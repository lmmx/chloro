# M6: base, head and head-pgo against rustfmt on a quiet host (the wave-5 head, E29–E31)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 6 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the wave-5 merges (E31 lexer front
end, E30 width rule; E29 refuted, record only) and on rustfmt 1.9.0, with the method and the
sweeps of [m5-quiet-host.md](m5-quiet-host.md) unchanged (measurement A, the cache, the
in-process and instruction counts on both samples with the per-file figure and the parse
phase, parallel items, the `CHLORO_TRACE` timeline, the nested-closure cliff, three noise
runs). Every figure in this file is **measured** in the windows named below; nothing is
estimated. The host is the 2.80 GHz host class of [../baseline.md](../baseline.md) and
M2–M5 (protocol §3: figures from different hosts are not compared; the "against M5" section
sets within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `2cb0ba3` (the integration branch after the wave-5 merges; one docs-only commit above the E30 merge `96c9abc`, the ledger's best-known-good, same code; the formatter code is M5's `e96b530` plus E31 and E30; E29 changed no code) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `2cb0ba3753e4abe51b90d2674c59f1fe7918a253` (worktree clean,
  `worktree_dirty=0`; `git diff --stat 96c9abc HEAD` touches five documentation files and no
  source; `git diff --stat e96b530 HEAD`, M5's code, touches 23 files: E31's `cst/lex.rs` and
  its harness, E30's width rule, E29's schedule scripts, the records); `base`
  `884f9c93f735676a752c67fab91d08959f410430` (`git worktree add /home/user/wt/base 884f9c9`,
  its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `5a3b591fa07cb5b9a4059cedb2c08a4d0acf212f655fdca8c24efcef63bd5d0b` | 2,217,072 |
| `head` bench | `target/release/examples/bench` | `960367538827f9f067e56bf95b64a14fa0d1341d9eea5a7749505b25e7a7edc4` | 1,903,136 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `00858a31ff4a46fdedd190e9223bb05cc4ada9bbd2750f3bf05b0b1b65ed3908` | 1,749,096 |
| `head` conform | `target/release/examples/conform` | `663d319fe540cc797aa0d2e73cf4c1af42338fbb4219ad580cf86e44b311b99c` | 1,878,496 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `bc58d147590d5da7b22332247fa070b79d5c56b0107afd716cb746f04ba52033` | 2,965,568 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `f648e5aaeed527e9b27480c897c2d81e9dd8b97c2e8b1b051eb5e82b50aae3a8` | 2,667,320 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes and the rustfmt hash equal the ones recorded in M1–M5: the
  default-profile build of `884f9c9` reproduces byte for byte on this toolchain.
- The `head` chloro binary is 2,217,072 bytes against 2,196,120 for M5's build and 2,516,048 for
  `base` (0.881x base; the CLI gained the lexer front end and the width rule);
  `head-pgo` is 2,965,568 bytes (1.179x base; M5's 2,921,072).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 19 lines for `head`
  (18 distinct crates, as at M5; the extra line is `cargo tree`'s `(*)` duplicate marker for
  `ra-ap-rustc_lexer`, which E31 names as a direct dependency and which `ra_ap_parser` already
  brought), 38 for `base`.
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, used by `pgo.sh`);
  hyperfine 2.0.0; valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v139 (the guest was rebooted
  between M5 and M6; same host class). The 2.80 GHz host class of M2–M5 and
  [../baseline.md](../baseline.md).
- **Load:** no other agent ran on the host (the three wave-5 agents had reported and their
  worktrees were removed; the E30 merge gates finished at 08:15 UTC and the profile-guided
  build at 08:18, which is why the 1-minute load average read 1.30 at the identity step;
  `env.txt` records no cargo, rustc, chloro, bench, valgrind, hyperfine or miri process at the
  start). The chain ran as one script (`scratchpad/m6/chain.sh`, M5's with the `m6` prefix and
  the new base commits for the diffs) with no gap between sweeps; the load average at the
  start of every later sweep is the harness's own processes from the previous sweep (the `mc`
  rows run chloro with 4 threads). `cpu3` columns are `/proc/stat` ticks of core 3 (USER_HZ
  100) over the sweep: steal / busy / idle.

| mark | time | load | cpu3 steal / busy / idle ticks since previous mark |
|---|---|---|---|
| a1-start | 2026-10-17T08:19:42Z | 1.30 1.23 1.90 | – |
| a1-end | 2026-10-17T08:26:35Z | 1.47 1.41 1.73 | 346 / 31661 / 9160 |
| a2-start | 2026-10-17T08:26:35Z | 1.47 1.41 1.73 | 0 / 0 / 1 |
| a2-end | 2026-10-17T08:32:59Z | 1.94 1.66 1.73 | 330 / 28362 / 9587 |
| a3-start | 2026-10-17T08:32:59Z | 1.94 1.66 1.73 | 0 / 1 / 0 |
| a3-end | 2026-10-17T08:39:56Z | 1.52 1.50 1.60 | 298 / 29036 / 12361 |
| dh-start | 2026-10-17T08:39:56Z | 1.52 1.50 1.60 | 0 / 0 / 0 |
| dh-end | 2026-10-17T08:42:53Z | 1.03 1.28 1.49 | 143 / 17337 / 18 |
| c-start | 2026-10-17T08:42:53Z | 1.03 1.28 1.49 | 0 / 0 / 1 |
| c-end | 2026-10-17T08:45:13Z | 2.79 2.15 1.81 | 137 / 13262 / 550 |
| d-start | 2026-10-17T08:45:13Z | 2.79 2.15 1.81 | 0 / 1 / 0 |
| d-bench-end | 2026-10-17T08:46:34Z | 1.47 1.88 1.74 | 86 / 8011 / 3 |
| d-profile-end | 2026-10-17T08:58:48Z | 1.00 1.08 1.33 | 852 / 70587 / 1896 |
| d-end | 2026-10-17T08:58:59Z | 1.00 1.08 1.33 | 7 / 1022 / 1 |
| e-start | 2026-10-17T08:58:59Z | 1.00 1.08 1.33 | 0 / 0 / 0 |
| e-end | 2026-10-17T08:59:42Z | 0.95 1.06 1.31 | 49 / 2740 / 1497 |
| t-start | 2026-10-17T08:59:42Z | 0.95 1.06 1.31 | 0 / 1 / 0 |
| t-end | 2026-10-17T09:00:10Z | 2.16 1.32 1.39 | 36 / 2395 / 404 |
| f-start | 2026-10-17T09:00:10Z | 2.16 1.32 1.39 | 0 / 0 / 1 |
| f-end | 2026-10-17T09:00:30Z | 1.83 1.30 1.38 | 11 / 981 / 986 |
| noise-1-start | 2026-10-17T09:00:30Z | 1.83 1.30 1.38 | 0 / 1 / 1 |
| noise-1-end | 2026-10-17T09:02:28Z | 1.76 1.48 1.44 | 95 / 8591 / 3108 |
| noise-2-start | 2026-10-17T09:02:28Z | 1.76 1.48 1.44 | 0 / 0 / 0 |
| noise-2-end | 2026-10-17T09:04:30Z | 2.04 1.78 1.56 | 111 / 9019 / 3066 |
| noise-3-start | 2026-10-17T09:04:30Z | 2.04 1.78 1.56 | 0 / 0 / 1 |
| noise-3-end | 2026-10-17T09:06:31Z | 2.28 2.02 1.68 | 98 / 8146 / 3816 |
| chain-end | 2026-10-17T09:06:31Z | 2.28 2.02 1.68 | 0 / 0 / 1 |

- Steal on core 3 is 0.7–1.2% of each sweep's ticks (sweep 1: 346 of 41,167; sweep 2: 330
  of 38,279; sweep 3: 298 of 41,695; the cache sweep: 137 of 13,949; the per-file profiles:
  852 of 73,335), between M4's 0.4–1.0% and M5's 0.8–1.5%. The pinned rows carry this steal.
- **This window's host sits between M4's and M5's.** The same `base` binary on the same bytes
  reads 4.36–4.86 MB/s in `bench -n 5` (M4 4.96–5.09, M5 3.68–4.03), 1014 ms on the cache copy
  (M4 909, M5 1280) and 5.87 s in `conform -i` (M4 5.70, M5 8.00); rustfmt's `fx-hir-ty`
  pinned mean is 1022 ms (M4 906, M5 1271), and sweep 1 took 6 min 53 s (M4 6:18, M5 8:54).
  Protocol §3 keeps every comparison inside one window, so the ratios below stand; the
  absolute milliseconds are not comparable with M4's or M5's, and the instruction counts,
  which do not depend on the window, are the figures to carry between records (the `base`
  count agrees with M4's and M5's to within 146 of 587,989,243).
- The whole chain ran as one script from 08:19:42 to 09:06:31 UTC with no gap between
  steps; the by-hand steps (cache, in-process, parallel items, trace, cliffs) wrote their
  exports under `benchmarks/results/m6-*` like the harness sweeps.

## Method

- All process-level figures: `benchmarks/compare_rustfmt.sh` with hyperfine 2.0 `-N -i`,
  `--metrics time_wall_clock,time_user,time_system,memory_peak_resident`, 2 warmup runs and
  **15 timed runs per command per round, 2 rounds with the command order reversed** (n = 30
  per cell), pinned rows on core 3, `mc` rows unpinned, `--mode check`. `report.py` merges
  the rounds; the tables below are built from the same JSON exports (min / mean ± σ wall ms
  / peak RSS MiB; CPU is mean user+sys ms over all threads).
- **The cache is disabled in every sweep except measurement C** by the harness itself:
  `compare_rustfmt.sh` passes `--no-cache` to any chloro binary whose `--help` lists the flag
  unless `--with-cache` is given (`env.txt` records `chloro_cache_flag=--no-cache`); `base`
  has no cache and gets no flag. `--no-cache` also turns off the opt-in unit cache (E15),
  which no sweep enables. Verified before the sweeps: `head -v --check --no-cache` prints
  `Cache: disabled (--no-cache)`.
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found): the
  copies M4 prepared and verified on this disk (M5 reused them) (same bytes; the five stdin copies are fixed
  points for every build, see Identity). The registry crates `tokio-1.53.3`,
  `regex-syntax-0.8.11`, `winnow-0.7.15` and `clap_builder-4.6.7` were present in
  `~/.cargo/registry`.
- Instruction counts: `benchmarks/profile.sh` as amended by E27 (the binary and the sample
  staged under a fixed-length path, so the count no longer depends on where they live),
  `--runs 2 --core 3 --per-file` on the fixture sample (49 files, every 25th of the sorted
  fixture list) and on the registry validation sample (`--sample-list
  benchmarks/samples/registry49.txt`, 49 files of the four registry crates); `--phase parse`
  on both samples for `head` and `head-pgo` (`base`'s `bench` has no parse phase). Per file
  means each file formatted in a `bench` process of its own, so the cross-file heap-layout
  term of EXP-020 is reported beside the whole-sample count rather than inside it.
- The schedule timeline: `benchmarks/schedule/diag.py` (E23) on the released binary's own
  `CHLORO_TRACE` output, `--check --no-cache`, 30 runs per crate at 4 threads unpinned
  (`head` and `head-pgo`) and 10 runs at 1 thread on core 3 (`head`); the medians per crate
  of wall, CPU, the format window, the tail after the first worker goes idle and the idle
  core time. The reconstruction from thread CPU clocks needs the patched build and is not
  rerun here (EXP-023 has it).
- The cliff shape: `benchmarks/cliffs/run.sh --shapes closures-expr --scales 1,2,4,8 --core
  3` (nesting depths 2, 4, 8 and 16 of expression closures, eight copies each), per binary,
  with the output compared to rustfmt's through `fmt_stdin` (`head`'s `fmt_stdin` for
  `head-pgo`, which has no profile-guided `fmt_stdin`).
- The results directories (gitignored, under `benchmarks/results/`):

| directory | command |
|---|---|
| `m6-a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `m6-a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `m6-a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `m6-d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `m6-c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `m6-d-inproc` | by-hand `taskset -c 3 bench -n 5 --root <head fixtures>` triples, `benchmarks/profile.sh --runs 2 --core 3 --per-file` per binary on both samples, `--phase parse` for head and head-pgo, `taskset -c 3 conform -i` per build (and `-V 1.8.0` for head) |
| `m6-e-parallel-items` | by-hand hyperfine, measurement E |
| `m6-t-trace` | `diag.py` per crate: `head4` (head, 4 threads, 30 runs), `head1` (head, 1 thread on core 3, 10 runs), `pgo4` (head-pgo, 4 threads, 30 runs) |
| `m6-f-cliffs` | `cliffs/run.sh --shapes closures-expr --scales 1,2,4,8 --core 3` per binary: `<binary>.ir.tsv`, `<binary>.compare.tsv`, `<binary>.log` (the fit) |
| `m6-noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |
| `m6-identity` | `env.txt`, `sha256.txt`, `sizes.txt`, `fixed-point.txt`, `loads.tsv` (the per-sweep load and `/proc/stat` lines) |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`); the `base` column shows sweep 1's base. The `ranges` column lists,
  in order, head vs base, head-pgo vs base, head vs rustfmt, head-pgo vs rustfmt: `below` =
  every run of the first command took less wall time than every run of the second; `overlap`
  = the min–max ranges intersect.
- The same `base` binary ran in sweeps 1 and 2: see the `1c` crate bullets for its drift
  between the two windows.
- Output agreement in every crate, files and stdin workload, recomputed by the harness with
  this record's binaries (`agreement_chloro=<sha16>` in each copy's `meta`): on formatted input
  `head` equals rustfmt on every file of every workload (15/15, 122/122, 33/33, 33/33, 28/28,
  49/49, 33/33, 57/57, 61/61, 287/287, 1/1 ×5) and on raw input the same with `tiny-test-data`
  289/289; `base` differs on one raw file (`parser/err/0024_many_type_parens.rs`, the R2 fix)
  and on no formatted file. Exit codes are 0 for every command on formatted input, 1 for both
  tools on raw input where files change (`reg-clap-builder` and the stdin rows exit 0).

## Crate level, formatted input, `--check` (measurement A)

### Crate level, all CPUs (`mc`), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 89.0 / 111.4 ± 18.4 / 24 | 25.4 / 28.7 ± 2.2 / 17 | 25.5 / 30.0 ± 2.5 / 18 | 296.7 / 332.7 ± 36.4 / 57 | 286.7 / 329.9 ± 30.3 / 57 | 0.257 | 0.228 | 0.086 | 0.091 | below; below; below; below | 244.1 | 91.1 | 95.8 | 327.6 | 0.373 | 0.349 | 0.278 | 0.296 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 188.7 / 247.7 ± 28.6 / 36 | 64.6 / 79.8 ± 8.8 / 16 | 60.9 / 68.7 ± 6.3 / 17 | 880.5 / 1034.0 ± 87.2 / 86 | 931.9 / 1028.3 ± 72.4 / 86 | 0.322 | 0.281 | 0.077 | 0.067 | below; below; below; below | 800.6 | 295.5 | 250.0 | 1021.8 | 0.369 | 0.320 | 0.289 | 0.246 | b[0]h[0]r[0]p[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 118.1 / 152.8 ± 21.4 / 20 | 27.3 / 32.0 ± 3.2 / 15 | 23.4 / 29.5 ± 3.7 / 16 | 309.8 / 378.2 ± 58.4 / 58 | 313.8 / 373.1 ± 53.2 / 58 | 0.209 | 0.195 | 0.084 | 0.079 | below; below; below; below | 271.7 | 94.7 | 88.0 | 373.8 | 0.348 | 0.320 | 0.253 | 0.239 | b[0]h[0]r[0]p[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 38.0 / 56.9 ± 11.5 / 13 | 15.5 / 17.9 ± 1.6 / 8 | 15.3 / 18.1 ± 2.2 / 8 | 130.2 / 159.7 ± 25.0 / 51 | 133.2 / 162.5 ± 23.9 / 51 | 0.315 | 0.323 | 0.112 | 0.111 | below; below; below; below | 155.3 | 58.7 | 56.3 | 156.2 | 0.378 | 0.373 | 0.376 | 0.352 | b[0]h[0]r[0]p[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 23.8 / 30.4 ± 4.1 / 10 | 10.3 / 11.4 ± 0.8 / 6 | 9.7 / 12.0 ± 1.8 / 7 | 78.1 / 97.1 ± 20.4 / 47 | 78.7 / 101.8 ± 20.3 / 47 | 0.373 | 0.350 | 0.117 | 0.118 | below; below; below; below | 83.5 | 34.3 | 34.5 | 95.2 | 0.411 | 0.364 | 0.360 | 0.344 | b[0]h[0]r[0]p[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 16.1 / 21.7 ± 3.3 / 7 | 7.2 / 9.1 ± 1.3 / 5 | 6.4 / 8.9 ± 1.8 / 6 | 42.7 / 55.5 ± 10.9 / 43 | 43.5 / 54.4 ± 6.6 / 43 | 0.421 | 0.376 | 0.165 | 0.164 | below; below; below; below | 49.6 | 24.0 | 20.5 | 54.7 | 0.485 | 0.402 | 0.440 | 0.391 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 130.9 / 163.8 ± 23.5 / 38 | 51.9 / 61.3 ± 8.1 / 23 | 51.0 / 62.9 ± 5.8 / 22 | 459.0 / 553.2 ± 89.8 / 66 | 464.5 / 579.3 ± 102.9 / 66 | 0.374 | 0.308 | 0.111 | 0.109 | below; below; below; below | 492.0 | 204.5 | 202.8 | 546.0 | 0.416 | 0.330 | 0.375 | 0.357 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 57.6 / 74.7 ± 9.4 / 18 | 24.7 / 28.7 ± 2.7 / 11 | 21.5 / 25.7 ± 2.6 / 12 | 191.2 / 227.1 ± 26.0 / 55 | 196.4 / 233.7 ± 36.8 / 55 | 0.384 | 0.363 | 0.126 | 0.110 | below; below; below; below | 217.0 | 90.9 | 83.4 | 223.4 | 0.419 | 0.412 | 0.407 | 0.362 | b[0]h[0]r[0]p[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 64.4 / 85.6 ± 12.9 / 18 | 23.4 / 27.7 ± 2.8 / 11 | 21.1 / 25.2 ± 2.9 / 12 | 183.5 / 217.1 ± 23.0 / 59 | 188.5 / 229.2 ± 41.9 / 59 | 0.323 | 0.309 | 0.127 | 0.110 | below; below; below; below | 251.9 | 92.9 | 84.1 | 213.2 | 0.369 | 0.346 | 0.436 | 0.373 | b[0]h[0]r[0]p[0] |

- Unpinned, `head/base` in mean: 0.209 (`fx-ide-db`) to 0.421 (`reg-tokio`), M5 0.219–0.441,
  M4 0.25–0.52; CPU 0.348–0.485. `head/rustfmt` 0.077 (`fx-hir-ty`) to 0.165 (`reg-tokio`), M5
  0.083–0.173, M4 0.092–0.179. Every head run is below every base run and every rustfmt run
  on all nine crates.
- `head-pgo/base` 0.195–0.376 and `head-pgo/rustfmt` 0.067–0.164 (M5 0.228–0.368 and
  0.066–0.148), every run below on all nine.
- Peak RSS of head on the nine crates unpinned: 5–23 MiB (M5 5–23) against base 7–38 and
  rustfmt 43–86; head-pgo 6–22.

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 192.4 / 248.4 ± 41.9 / 16 | 73.0 / 85.9 ± 19.2 / 9 | 65.0 / 72.8 ± 8.9 / 10 | 279.6 / 327.9 ± 51.0 / 57 | 277.0 / 336.5 ± 48.2 / 57 | 0.346 | 0.337 | 0.262 | 0.216 | below; below; below; below | 243.9 | 84.7 | 71.6 | 324.8 | 0.347 | 0.334 | 0.261 | 0.214 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 613.3 / 668.9 ± 42.9 / 18 | 220.6 / 257.9 ± 30.6 / 8 | 201.0 / 215.5 ± 16.0 / 8 | 892.2 / 1021.9 ± 97.9 / 86 | 890.7 / 1067.1 ± 123.0 / 86 | 0.386 | 0.288 | 0.252 | 0.202 | below; below; below; below | 661.2 | 254.9 | 214.5 | 1009.8 | 0.385 | 0.289 | 0.252 | 0.203 | b[0]h[0]r[0]p[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 212.8 / 283.0 ± 72.3 / 15 | 70.3 / 79.1 ± 9.2 / 9 | 61.5 / 66.6 ± 6.3 / 10 | 313.0 / 369.8 ± 51.1 / 58 | 299.8 / 366.0 ± 56.5 / 58 | 0.280 | 0.269 | 0.214 | 0.182 | below; below; below; below | 280.5 | 77.9 | 66.1 | 364.7 | 0.278 | 0.270 | 0.214 | 0.183 | b[0]h[0]r[0]p[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 111.6 / 139.1 ± 23.0 / 9 | 42.1 / 47.2 ± 5.9 / 5 | 40.5 / 50.7 ± 7.2 / 6 | 131.5 / 145.7 ± 20.3 / 51 | 137.0 / 156.0 ± 24.5 / 51 | 0.339 | 0.307 | 0.324 | 0.325 | below; below; below; below | 137.8 | 46.4 | 49.8 | 144.4 | 0.337 | 0.306 | 0.321 | 0.324 | b[0]h[0]r[0]p[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 64.4 / 75.0 ± 13.7 / 7 | 25.9 / 26.6 ± 1.5 / 5 | 24.1 / 25.2 ± 0.9 / 5 | 73.2 / 82.5 ± 17.6 / 47 | 73.9 / 90.6 ± 21.8 / 47 | 0.355 | 0.313 | 0.323 | 0.278 | below; below; below; below | 74.4 | 26.4 | 24.9 | 81.7 | 0.355 | 0.312 | 0.323 | 0.277 | b[0]h[0]r[0]p[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 34.9 / 42.3 ± 8.5 / 6 | 14.8 / 17.3 ± 2.0 / 4 | 13.8 / 14.6 ± 0.9 / 5 | 41.2 / 54.4 ± 11.6 / 43 | 41.5 / 50.6 ± 7.4 / 43 | 0.408 | 0.397 | 0.318 | 0.289 | below; below; below; below | 41.5 | 16.9 | 14.2 | 53.3 | 0.407 | 0.393 | 0.317 | 0.287 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 424.6 / 521.3 ± 101.2 / 29 | 146.9 / 175.9 ± 39.0 / 10 | 129.6 / 166.4 ± 28.4 / 11 | 441.3 / 543.3 ± 135.3 / 66 | 460.2 / 557.8 ± 86.8 / 66 | 0.337 | 0.318 | 0.324 | 0.298 | below; below; below; below | 515.9 | 174.0 | 163.5 | 540.6 | 0.337 | 0.317 | 0.322 | 0.297 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 169.2 / 187.1 ± 24.4 / 12 | 63.7 / 66.9 ± 4.1 / 6 | 57.3 / 63.4 ± 7.2 / 7 | 190.5 / 210.2 ± 25.3 / 55 | 193.7 / 211.3 ± 12.5 / 55 | 0.358 | 0.308 | 0.318 | 0.300 | below; below; below; below | 184.8 | 66.1 | 62.3 | 209.1 | 0.358 | 0.305 | 0.316 | 0.297 | b[0]h[0]r[0]p[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 188.6 / 225.9 ± 35.4 / 11 | 66.5 / 73.0 ± 10.1 / 7 | 60.1 / 67.0 ± 9.6 / 8 | 182.3 / 211.5 ± 35.1 / 59 | 177.3 / 208.4 ± 47.7 / 59 | 0.323 | 0.313 | 0.345 | 0.322 | below; below; below; below | 224.3 | 72.0 | 65.2 | 209.9 | 0.321 | 0.308 | 0.343 | 0.318 | b[0]h[0]r[0]p[0] |

- Pinned, `head/base` in mean: 0.280 (`fx-ide-db`) to 0.408 (`reg-tokio`), M5 0.294–0.462, M4
  0.38–0.60; CPU 0.278–0.407. `head/rustfmt` 0.214 (`fx-ide-db`) to 0.345 (`reg-winnow`), M5
  0.239–0.405, M4 0.26–0.44. `head-pgo/base` 0.269–0.397 (M5 0.285–0.429) and
  `head-pgo/rustfmt` 0.182–0.325 (M5 0.210–0.332). Every head and head-pgo run is below every
  base run and every rustfmt run on all nine crates.
- `base` between sweeps 1 and 2 (mean ratio, the same binary and bytes): 0.870x (`fx-hir`) to
  1.185x on the pinned rows and 0.949x–1.248x (`reg-regex-syntax`) unpinned (M5 0.883x–1.115x
  pinned); `pgo/base` and `pgo/rustfmt` are computed against sweep 2's own base and rustfmt
  runs.
- Peak RSS of head pinned: 4–10 MiB (M5 4–11) against base 6–29 and rustfmt 43–86.
- Spread within a cell: σ/mean of the pinned head rows is 5.6% (`fx-parser`) to 22% (`fx-hir`
  and `reg-regex-syntax`, each with one slow tail); rustfmt's 10–25%; base's 6.4–26%.

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 12.5 / 16.9 ± 2.7 / 5 | 5.8 / 7.2 ± 1.0 / 5 | 5.7 / 7.1 ± 1.5 / 5 | 49.7 / 61.8 ± 10.4 / 44 | 50.4 / 61.0 ± 9.9 / 44 | 0.426 | 0.434 | 0.117 | 0.117 | below; overlap; below; below | 32.7 | 15.9 | 14.9 | 60.0 | 0.486 | 0.450 | 0.265 | 0.249 | b[0]h[0]r[0]p[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 22.4 / 24.8 ± 2.6 / 5 | 11.4 / 14.2 ± 2.4 / 4 | 10.2 / 12.0 ± 1.9 / 5 | 49.8 / 61.6 ± 14.7 / 44 | 49.8 / 60.3 ± 11.4 / 44 | 0.572 | 0.450 | 0.230 | 0.200 | below; below; below; below | 24.2 | 13.6 | 11.8 | 60.9 | 0.563 | 0.454 | 0.224 | 0.198 | b[0]h[0]r[0]p[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 2.0 / 2.6 ± 0.3 / 4 | 1.9 / 2.7 ± 0.5 / 4 | 2.0 / 2.8 ± 0.6 / 4 | 10.5 / 14.5 ± 3.1 / 34 | 10.5 / 13.0 ± 2.4 / 34 | 1.063 | 1.051 | 0.187 | 0.212 | overlap; overlap; below; below | 2.3 | 2.5 | 2.6 | 14.1 | 1.088 | 1.053 | 0.180 | 0.209 | b[0]h[0]r[0]p[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.7 / 1.8 ± 0.1 / 4 | 1.7 / 1.8 ± 0.1 / 4 | 1.7 / 1.8 ± 0.1 / 4 | 9.7 / 11.7 ± 2.5 / 34 | 9.8 / 11.2 ± 0.6 / 34 | 0.994 | 0.991 | 0.151 | 0.164 | overlap; overlap; below; below | 1.6 | 1.6 | 1.7 | 11.5 | 0.999 | 1.001 | 0.143 | 0.155 | b[0]h[0]r[0]p[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 85.7 / 105.4 ± 22.3 / 10 | 22.7 / 28.5 ± 6.5 / 9 | – | 137.4 / 150.0 ± 15.0 / 43 | – | 0.271 | – | 0.190 | – | below; –; below; – | 103.9 | 27.6 | – | 148.5 | 0.265 | – | 0.186 | – | b[0]h[0]r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 78.9 / 86.5 ± 11.9 / 14 | 36.9 / 44.9 ± 12.2 / 14 | – | 89.2 / 96.7 ± 11.0 / 53 | – | 0.519 | – | 0.464 | – | below; –; below; – | 85.7 | 44.4 | – | 96.3 | 0.518 | – | 0.461 | – | b[0]h[0]r[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 64.5 / 67.4 ± 3.2 / 9 | 28.1 / 29.0 ± 0.6 / 9 | – | 87.0 / 97.6 ± 13.6 / 47 | – | 0.431 | – | 0.297 | – | below; –; below; – | 66.7 | 28.8 | – | 96.5 | 0.431 | – | 0.298 | – | b[0]h[0]r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 17.4 / 18.2 ± 0.5 / 5 | 7.1 / 7.4 ± 0.2 / 4 | – | 29.3 / 31.5 ± 1.7 / 40 | – | 0.408 | – | 0.236 | – | below; –; below; – | 17.9 | 7.2 | – | 31.0 | 0.403 | – | 0.233 | – | b[0]h[0]r[0] |
| stdin-config | 1c (3) | 1 | 169 | 30 | 25.9 / 28.3 ± 2.3 / 6 | 12.0 / 12.8 ± 0.6 / 5 | – | 41.8 / 47.8 ± 5.1 / 44 | – | 0.451 | – | 0.267 | – | below; –; below; – | 27.8 | 12.5 | – | 47.0 | 0.451 | – | 0.267 | – | b[0]h[0]r[0] |

- The 287-file invocation (`parser/test_data`, 67 KB in all): `head/base` 0.426 unpinned and
  0.572 pinned (M5 0.411 and 0.450; the pinned base mean fell from 34.7 to 24.8 ms between the
  windows while head's read 15.6 and 14.2); `head/rustfmt` 0.117 and 0.230 (M5 0.111 and 0.181);
  every head run below every base and rustfmt run; head-pgo's range overlaps base's unpinned
  (5.7–13.2 vs 12.5–50.3: one slow head-pgo run).
- Start-up on the 13-byte file: head 1.8 ms pinned (base 1.8, rustfmt 11.7) and 2.7 ms
  unpinned in mean (base 2.6, rustfmt 14.5); head-pgo 1.8 and 2.8 ms. Head against base
  overlaps on this row in every record; both are below every rustfmt run.
- One file on stdin (`fmt_stdin`, pinned): `head/base` 0.271 (`stdin-lints`, 640 KB) to 0.519
  (`stdin-nodes`), M5 0.312–0.506; `head/rustfmt` 0.190 to 0.464; every head run below every
  base and rustfmt run on all five. No PGO build of `fmt_stdin`, as in M4 and M5.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 625 | 30 | 539.5 / 614.1 ± 45.3 / 218 | 26.2 / 31.6 ± 3.0 / 17 | 0.052 | below | 606.7 | 101.8 | 0.168 | 15/15 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 541.2 / 612.3 ± 55.6 / 218 | 73.3 / 82.4 ± 14.1 / 9 | 0.135 | below | 606.1 | 81.6 | 0.135 | 15/15 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1251.8 / 1412.0 ± 99.8 / 117 | 64.2 / 88.5 ± 11.2 / 17 | 0.063 | below | 1395.8 | 323.3 | 0.232 | 122/122 | r[1] h[1] |
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1231.0 / 1429.1 ± 126.5 / 117 | 222.3 / 275.7 ± 49.9 / 8 | 0.193 | below | 1414.6 | 272.7 | 0.193 | 122/122 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 349.9 / 411.8 ± 54.3 / 66 | 25.8 / 30.4 ± 2.6 / 15 | 0.074 | below | 406.7 | 94.6 | 0.233 | 33/33 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 350.4 / 470.0 ± 103.5 / 66 | 69.7 / 90.5 ± 19.5 / 9 | 0.192 | below | 466.9 | 88.1 | 0.189 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 168.0 / 197.6 ± 22.0 / 57 | 15.7 / 19.6 ± 1.8 / 8 | 0.099 | below | 194.9 | 63.5 | 0.326 | 33/33 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 161.5 / 191.0 ± 23.5 / 57 | 41.7 / 44.2 ± 2.8 / 5 | 0.231 | below | 188.8 | 43.9 | 0.232 | 33/33 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 86.1 / 104.7 ± 16.1 / 48 | 11.2 / 13.4 ± 0.9 / 6 | 0.128 | below | 102.4 | 39.3 | 0.384 | 28/28 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 83.3 / 92.8 ± 13.7 / 48 | 26.1 / 27.4 ± 1.9 / 5 | 0.296 | below | 91.6 | 27.2 | 0.297 | 28/28 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 43.8 / 54.3 ± 8.2 / 44 | 7.0 / 8.5 ± 0.8 / 5 | 0.157 | below | 52.4 | 21.6 | 0.412 | 49/49 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 42.0 / 46.9 ± 6.4 / 44 | 14.9 / 17.9 ± 5.2 / 4 | 0.383 | below | 46.2 | 17.6 | 0.381 | 49/49 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1421.2 / 1613.8 ± 109.1 / 423 | 52.7 / 62.3 ± 7.0 / 22 | 0.039 | below | 1599.1 | 205.9 | 0.129 | 33/33 | r[1] h[1] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1433.7 / 1610.6 ± 123.8 / 423 | 152.1 / 183.6 ± 28.9 / 10 | 0.114 | below | 1595.5 | 179.6 | 0.113 | 33/33 | r[1] h[1] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 196.2 / 230.3 ± 28.8 / 55 | 24.9 / 27.8 ± 2.7 / 11 | 0.121 | below | 226.3 | 89.3 | 0.395 | 57/57 | r[0] h[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 199.0 / 226.7 ± 33.7 / 55 | 63.3 / 69.2 ± 7.3 / 6 | 0.305 | below | 224.4 | 68.8 | 0.307 | 57/57 | r[0] h[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 196.7 / 231.6 ± 35.3 / 60 | 24.7 / 28.0 ± 2.9 / 11 | 0.121 | below | 228.3 | 93.6 | 0.410 | 61/61 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 194.9 / 226.2 ± 31.4 / 60 | 72.2 / 95.7 ± 18.1 / 7 | 0.423 | below | 224.6 | 94.8 | 0.422 | 61/61 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 62.0 / 71.8 ± 10.2 / 44 | 6.1 / 7.5 ± 0.9 / 5 | 0.105 | below | 70.8 | 16.8 | 0.237 | 289/289 | r[1] h[1] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 62.0 / 67.9 ± 10.7 / 44 | 11.6 / 12.0 ± 0.4 / 4 | 0.176 | below | 67.4 | 11.7 | 0.174 | 289/289 | r[1] h[1] |
| tiny-one | mc (none) | 1 | 0 | 30 | 11.4 / 15.0 ± 2.6 / 34 | 2.3 / 3.3 ± 0.5 / 4 | 0.223 | below | 14.5 | 3.0 | 0.206 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 10.5 / 13.7 ± 2.4 / 34 | 1.8 / 1.9 ± 0.1 / 4 | 0.139 | below | 13.1 | 1.7 | 0.133 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 139.1 / 150.1 ± 14.2 / 43 | 23.6 / 24.6 ± 0.7 / 9 | 0.164 | below | 148.6 | 24.3 | 0.164 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 91.6 / 104.6 ± 12.7 / 53 | 37.4 / 39.6 ± 2.4 / 14 | 0.378 | below | 103.5 | 38.9 | 0.376 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 89.9 / 107.7 ± 15.5 / 47 | 28.7 / 33.1 ± 4.9 / 9 | 0.308 | below | 106.1 | 32.8 | 0.309 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 31.4 / 36.7 ± 5.7 / 40 | 7.3 / 7.8 ± 0.4 / 4 | 0.212 | below | 36.1 | 7.5 | 0.207 | 1/1 | r[0] h[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 44.6 / 50.8 ± 6.7 / 44 | 12.3 / 14.1 ± 1.7 / 5 | 0.277 | below | 50.0 | 13.8 | 0.276 | 1/1 | r[0] h[0] |

- On input as found, `head/rustfmt` reads 0.039 (`reg-regex-syntax`) to 0.157 (`reg-tokio`)
  unpinned and 0.114 to 0.423 pinned on the crates, 0.105 and 0.176 on the 287 files, 0.164–0.378
  on the stdin files; rustfmt's peak RSS rises to 117–423 MiB on the crates whose files change
  (it builds a diff per changed file). Every head run is below every rustfmt run on every
  row.
- Outputs identical to rustfmt's on every file of every raw workload (`tiny-test-data`
  289/289). Exit codes 1 for both tools where files change, 0 for both on `reg-clap-builder`,
  `tiny-one` and the stdin rows.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 13,901,559 bytes)
  copied to `/tmp/chloro-bench/m6-cache/tree` and rewritten once by `head --write --no-cache`;
  `head --check --no-cache` and `base --check` on the result exit 0. `CHLORO_CACHE_DIR` points
  at a scratch directory, which after a priming run holds the cache file
  `741a713fc3ccca89c6d255871bb460de.chloro-cache` of 44,184 bytes (the name is the key of this
  binary and configuration) and the executable's identity record `624014c8….chloro-exe` of
  180 bytes. `head -v --check` on the copy reports `(1226 entries)` and `Unit cache: off`; the
  1238 fixtures hold 1226 distinct contents by sha256. The `head-pgo` binary writes its own
  pair (`71eed34b….chloro-cache`, 44,184 bytes; a 184-byte `.chloro-exe`).
- Commands: as in M4 and M5 (`hyperfine -N -i -w 1 -r 15 --metrics …`, two rounds with the order
  reversed for `cold`, `warm`, `no-cache`, `base` (n = 30); one round under `taskset -c 3` for
  `warm`, `no-cache`, `base` (n = 15); one unpinned round for `head-pgo` (n = 15)).

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 317.3 / 363.8 ± 31.8 / 43 | 12.2 / 16.2 ± 2.9 / 5 | 295.9 / 355.5 ± 41.1 / 41 | 879.1 / 1014.2 ± 90.4 / 53 | 22.0 | 62.7 | 0.351 | 1352.3 | 29.9 | 1326.7 | 3464.8 |
| 1c | 15 | – | 22.0 / 32.1 ± 4.9 / 4 | 1063.3 / 1289.8 ± 196.6 / 22 | 2899.7 / 3370.9 ± 266.3 / 33 | 40.2 | 104.9 | 0.383 | – | 31.4 | 1279.7 | 3335.3 |

| variant | n | pgo-warm | pgo-no-cache | no-cache/warm | CPU pgo-warm | CPU pgo-no-cache |
|---|---|---|---|---|---|---|
| mc | 15 | 13.2 / 15.9 ± 2.0 / 5 | 284.4 / 307.8 ± 13.4 / 38 | 19.4 | 30.3 | 1136.8 |

- Warm `--check` of the 1238 formatted files takes 16.2 ms unpinned (22.0x below `--no-cache`,
  62.7x below base) and 32.1 ms on one core (40.2x, 104.9x; the pinned warm cell has a
  22.0–36.5 ms range, its mean above M5's 29.7 while its minimum is below); CPU 29.9 and 31.4
  ms; peak RSS 5 and 4 MiB against 41 and 22 MiB without the cache.
- `cold` (cache directory removed before every run) reads 363.8 ms against 355.5 ms for
  `no-cache` in mean (1.023x; ranges 317.3–435.3 vs 295.9–429.8 overlap): hashing the files and the
  binary and writing the cache and the identity record add 8 ms in mean over the formatter on
  4 threads in this window (M5: 28; M4: 16).
- `no-cache` head against base on the same copy: 0.351 unpinned (355.5 vs 1014.2 ms) and 0.383
  pinned (1289.8 vs 3370.9 ms); CPU 0.383 and 0.384. M5 read 0.349 and 0.389, M4 0.410 and
  0.425. Peak RSS of `cold` (43 MiB) and `no-cache` (41 MiB) is below base's 53 MiB unpinned;
  pinned, 22 MiB against 33 MiB.
- The profile-guided build: warm 15.9 ms unpinned (CPU 30.3 ms), `--no-cache` 307.8 ms (0.866x
  head's 355.5 ms in the earlier round of the same sweep; peak RSS 38 against 41 MiB).

## In process and instructions (measurement D)

`taskset -c 3 bench -n 5 --root chloro-core/tests/conformance/fixtures` (best of 5 in-process
formats of the 1238 fixtures, 13.9 MB; the same tree for all three binaries), three
interleaved triples in the order base, head, head-pgo:

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 3.188 | 4.36 |
| 1 | head | 1238 | 13.9 | 1.160 | 11.99 |
| 1 | head-pgo | 1238 | 13.9 | 0.936 | 14.85 |
| 2 | base | 1238 | 13.9 | 2.862 | 4.86 |
| 2 | head | 1238 | 13.9 | 1.054 | 13.19 |
| 2 | head-pgo | 1238 | 13.9 | 0.962 | 14.45 |
| 3 | base | 1238 | 13.9 | 2.947 | 4.72 |
| 3 | head | 1238 | 13.9 | 1.072 | 12.96 |
| 3 | head-pgo | 1238 | 13.9 | 0.920 | 15.10 |

- base: 3 runs, best time 2.862–3.188 s, 4.36–4.86 MB/s
- head: 3 runs, best time 1.054–1.160 s, 11.99–13.19 MB/s
- head-pgo: 3 runs, best time 0.920–0.962 s, 14.45–15.10 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, each binary's
compiled-in fixture root, `chloro` = head, `chloro-base` = base: chloro 13.12–13.74 MB/s over
its three runs, chloro-base 4.65–4.85 MB/s, non-overlapping.

### Instructions on both samples (`benchmarks/profile.sh --runs 2 --core 3 --per-file`)

Whole = the sample formatted by one `bench` process (the ledger's figure); per file = each
file in a `bench` process of its own, the minimum of 2 runs per file, summed, net of the
empty-sample process (316,247 instructions for head, 316,117 for head-pgo, 328,886 for base);
cross-file = whole net of the empty process minus per file (the heap layout one file leaves
the next, EXP-020). The registry sample is `benchmarks/samples/registry49.txt` (E27).

| sample | binary | whole, run 1 | whole, run 2 | per file (net) | cross-file | whole / base | whole / head |
|---|---|---|---|---|---|---|---|
| fixtures | base | 587,989,389 | 587,989,442 | 581,628,476 | +6,032,027 | 1.0000 | |
| fixtures | head | 202,822,485 | 202,824,156 | 202,145,564 | +360,674 | 0.3449 | 1.0000 |
| fixtures | head-pgo | 170,069,599 | 170,069,833 | 169,757,860 | −4,378 | 0.2892 | 0.8385 |
| registry | base | 688,588,992 | 688,589,271 | 681,815,947 | +6,444,159 | 1.0000 | |
| registry | head | 255,681,198 | 255,681,238 | 255,654,285 | −289,334 | 0.3713 | 1.0000 |
| registry | head-pgo | 219,987,557 | 219,989,296 | 219,684,596 | −13,156 | 0.3195 | 0.8604 |

### The parse-only phase (`--phase parse`, head and head-pgo; `base`'s `bench` has no phase option)

| sample | binary | whole, run 1 | run 2 | per file (net) | parse net of the empty process / whole net |
|---|---|---|---|---|---|
| fixtures | head | 68,604,394 | 68,605,852 | 70,974,632 | 68,286,856 / 202,506,238 = **33.7%** |
| fixtures | head-pgo | 60,004,178 | 60,004,187 | 62,369,760 | 59,686,747 / 169,753,482 = 35.2% |
| registry | head | 106,889,112 | 106,889,168 | 108,343,704 | 106,571,574 / 255,364,951 = **41.7%** |
| registry | head-pgo | 93,746,926 | 93,749,894 | 95,204,946 | 93,429,495 / 219,671,440 = 42.5% |

- `bench -n 5`, three interleaved triples: base 4.36–4.86 MB/s, head 11.99–13.19 MB/s, head-pgo
  14.45–15.10 MB/s. The three ranges are pairwise disjoint (protocol §3 rule for a
  single-threaded change: 3 of 3 pairs, non-overlapping); head/base 2.71x–2.75x on the
  per-round MB/s (M5 2.50x–2.67x, M4 2.20x–2.38x), head-pgo/head 1.10x–1.24x. The harness's
  own three pairs (same core, 08:40–08:43 UTC) give base 4.65–4.85 and head 13.12–13.74 MB/s,
  non-overlapping. Base reads 4.36–4.86 against M4's 4.96–5.09 and M5's 3.68–4.03 on the same
  bytes and binary (Load), so the MB/s are this window's.
- `conform -i` (hyperfine, 9 runs each, core 3, harness): head 2310.0 ± 145.0 ms
  (2038.4–2603.7) against base 6360.1 ± 441.4 ms (5476.1–6875.3), 0.363, every head run below
  every base run; CPU 2287.2 vs 6308.8 ms; peak RSS 49 vs 50 MiB; exit 0 on every run. By
  hand (`taskset -c 3 conform -i`, one run each): head **1220/1220 identical**, 0 not
  idempotent, `rustfmt_version=1.9.0`, diff lines +0 −0, 13.8 MB in 1.100 s (12.5 MB/s),
  2.254 s wall; at `-V 1.8.0` **1220/1220 identical**, 0 not idempotent, 13.8 MB in 1.055 s
  (13.0 MB/s), 2.173 s wall; base 1219/1220 identical, 0 not idempotent, diff lines +2 −2,
  13.8 MB in 2.909 s (4.7 MB/s), 5.871 s wall (0.384 head/base; M5 0.389, M4 0.454).
- `Ir` on the fixture sample (650,213 bytes): base 587,989,389, head 202,822,485 (**−65.51%**),
  head-pgo 170,069,599 (−71.08%; 0.8385x head); two runs of each agree to within 53 (base),
  1,671 (head) and 234 (head-pgo). The base count equals M4's and M5's to within 146; the head
  count equals the ledger's 202,822,327 at the E30 merge (the same code) to within 158. The
  profile-guided ratio is this training run's: M3 0.7825x its head, M4 0.8337x, M5 0.8264x,
  here 0.8385x.
- Per file, net of the empty process, the fixture sample reads 581,628,476 (base),
  202,145,564 (head, 0.3475x) and 169,757,860 (head-pgo, 0.2919x); the cross-file term is
  +6.03 M for base (1.0% of its whole), +0.36 M for head (0.18%; M5's head read −0.11 M: E31's
  lexer vectors, sized from the text, leave the next file another heap) and −0.004 M for
  head-pgo.
- The registry validation sample: base 688,588,992, head 255,681,198 (**−62.87%**; 0.3713x),
  head-pgo 219,987,557 (−68.05%; 0.8604x head); per file 681,815,947 / 255,654,285 (0.3750x) /
  219,684,596. The gap between the two samples narrowed from M5 (0.3758 against 0.4356 there;
  0.3449 against 0.3713 here): E31's lexer front end took 14.8% off the registry sample and
  8.1% off the fixture sample, because the registry files are plainer and the parser was a
  larger share of them.
- The parse-only phase is **33.7%** of head's instructions on the fixture sample and **41.7%**
  on the registry sample (M5: 39.1% and 46.6%; E31 took 20.7% and 23.6% off the phase). PGO
  cuts the phase by 0.8746x and 0.8770x against the whole's 0.8385x and 0.8604x, so the
  profile-guided build's parse share is slightly higher (35.2% and 42.5%). The unmined-tricks
  plan's parser-fork trigger (40%) is still crossed on the registry sample and no longer on
  the fixture sample.

## The schedule timeline on a quiet host (measurement T)

`benchmarks/schedule/diag.py` on the released binaries' own `CHLORO_TRACE` output, `--check
--no-cache` on the nine crate copies, 30 runs per crate at 4 threads unpinned (`head` and
`head-pgo`) and 10 runs at 1 thread on core 3 (`head`); each row is the median over runs. **Read
with EXP-029** ([../experiments/EXP-029-split-tail.md](../experiments/EXP-029-split-tail.md)): a
release build's trace has no record of a split file's item threads, so the wall view counts the
split file's whole span as one busy core and its item workers as idle. On patched traces the
idle core time in `fx-hir`'s tail is 0.7–1.6 core-ms and the schedule sits 1.1–1.5% above the
work-conserving bound; the `tail` and `lost tail` columns below for `fx-hir` and `fx-ide-db`
overstate the loss for that reason, and are kept for comparison with M5.

**head, 4 threads, unpinned, 30 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 29.8 | 96.4 | 26.4 | 26.2 | 87.1 | 0.825 | 5.92 | 0.226 | 236 | 1 | 15.8 | 1.7 | 1.90 | 84.9 |
| fx-hir-ty | 122 | 78.9 | 280.2 | 75.1 | 74.3 | 293.2 | 0.987 | 0.42 | 0.005 | 1 | 1 | 1.4 | 2.0 | 4.69 | 288.4 |
| fx-ide-db | 33 | 29.3 | 92.6 | 26.1 | 25.9 | 90.0 | 0.895 | 2.96 | 0.122 | 640 | 1 | 8.2 | 1.9 | 3.09 | 86.1 |
| fx-syntax | 33 | 18.9 | 59.4 | 15.7 | 15.4 | 58.0 | 0.948 | 0.32 | 0.022 | 1 | 0 | 1.1 | 1.6 | 1.49 | 56.3 |
| fx-parser | 28 | 11.8 | 35.2 | 9.1 | 8.9 | 32.5 | 0.916 | 0.39 | 0.043 | 2 | 0 | 1.1 | 1.7 | 0.88 | 31.6 |
| reg-tokio | 49 | 8.0 | 20.0 | 5.3 | 5.0 | 17.1 | 0.864 | 0.25 | 0.049 | 0 | 0 | 1.0 | 1.6 | 0.84 | 16.2 |
| reg-regex-syntax | 33 | 61.4 | 197.6 | 57.3 | 57.0 | 222.1 | 0.982 | 0.50 | 0.009 | 4 | 4 | 1.6 | 1.8 | 3.98 | 215.2 |
| reg-clap-builder | 57 | 26.1 | 84.3 | 23.0 | 22.7 | 87.3 | 0.962 | 0.32 | 0.015 | 0 | 2 | 1.3 | 1.8 | 2.76 | 83.8 |
| reg-winnow | 61 | 24.9 | 84.0 | 21.7 | 21.4 | 81.8 | 0.962 | 0.29 | 0.013 | 2 | 0 | 1.0 | 1.9 | 2.23 | 78.7 |

**head-pgo, 4 threads, unpinned, 30 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 30.3 | 94.5 | 26.2 | 26.0 | 85.5 | 0.807 | 6.47 | 0.247 | 236 | 1 | 17.7 | 2.0 | 2.13 | 83.0 |
| fx-hir-ty | 122 | 71.4 | 260.4 | 67.3 | 66.6 | 261.5 | 0.986 | 0.40 | 0.006 | 1 | 1 | 1.5 | 1.9 | 5.16 | 256.4 |
| fx-ide-db | 33 | 31.8 | 97.1 | 27.7 | 27.4 | 95.4 | 0.857 | 5.18 | 0.172 | 640 | 1 | 14.8 | 1.8 | 3.61 | 91.5 |
| fx-syntax | 33 | 17.8 | 56.0 | 14.8 | 14.6 | 54.8 | 0.942 | 0.31 | 0.022 | 1 | 0 | 1.1 | 1.8 | 1.37 | 53.3 |
| fx-parser | 28 | 11.9 | 35.4 | 9.0 | 8.7 | 31.9 | 0.912 | 0.39 | 0.041 | 2 | 0 | 1.2 | 1.6 | 0.83 | 31.1 |
| reg-tokio | 49 | 8.2 | 20.8 | 5.4 | 5.0 | 17.0 | 0.864 | 0.19 | 0.034 | 0 | 0 | 0.7 | 1.6 | 0.98 | 16.0 |
| reg-regex-syntax | 33 | 58.6 | 193.9 | 54.3 | 54.1 | 213.5 | 0.982 | 0.44 | 0.008 | 4 | 4 | 1.5 | 1.9 | 4.80 | 208.4 |
| reg-clap-builder | 57 | 26.9 | 87.2 | 23.6 | 23.3 | 89.4 | 0.959 | 0.35 | 0.014 | 1 | 2 | 1.3 | 1.8 | 2.82 | 86.4 |
| reg-winnow | 61 | 26.1 | 86.5 | 22.7 | 22.2 | 85.1 | 0.963 | 0.27 | 0.011 | 1 | 0 | 1.0 | 1.8 | 2.62 | 82.4 |

**head, 1 thread on core 3, 10 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 89.0 | 85.1 | 84.2 | 84.0 | 83.6 | 0.996 | 0.18 | 0.002 | 0 | 0 | 0.2 | 0.0 | 0.79 | 82.7 |
| fx-hir-ty | 122 | 301.6 | 299.5 | 296.9 | 296.2 | 295.7 | 0.998 | 0.19 | 0.001 | 0 | 0 | 0.2 | 0.0 | 4.45 | 291.2 |
| fx-ide-db | 33 | 76.3 | 75.8 | 71.3 | 71.1 | 70.8 | 0.996 | 0.15 | 0.002 | 0 | 0 | 0.1 | 0.0 | 1.33 | 69.3 |
| fx-syntax | 33 | 49.1 | 48.4 | 44.8 | 44.5 | 44.3 | 0.994 | 0.13 | 0.003 | 0 | 0 | 0.1 | 0.0 | 0.80 | 43.4 |
| fx-parser | 28 | 41.3 | 41.0 | 36.7 | 36.5 | 36.2 | 0.991 | 0.16 | 0.004 | 0 | 0 | 0.2 | 0.0 | 0.70 | 35.5 |
| reg-tokio | 49 | 22.5 | 21.7 | 17.0 | 16.6 | 16.3 | 0.981 | 0.15 | 0.009 | 0 | 0 | 0.1 | 0.0 | 0.58 | 15.7 |
| reg-regex-syntax | 33 | 178.8 | 177.7 | 173.6 | 173.3 | 173.0 | 0.998 | 0.15 | 0.001 | 0 | 0 | 0.2 | 0.0 | 2.19 | 170.4 |
| reg-clap-builder | 57 | 70.9 | 70.2 | 66.3 | 66.0 | 65.6 | 0.995 | 0.16 | 0.002 | 0 | 0 | 0.2 | 0.0 | 1.24 | 64.2 |
| reg-winnow | 61 | 89.3 | 88.0 | 83.7 | 83.2 | 82.8 | 0.995 | 0.17 | 0.002 | 0 | 0 | 0.2 | 0.0 | 1.50 | 81.2 |

- Seven of the nine crates keep every core busy to within 0.3–0.5 ms of the end (tail 0.5–4.9%
  of the run, worker utilisation 0.864–0.987). `fx-hir` reads a 5.92 ms tail (22.6%; M5 7.14,
  22.4%) and `fx-ide-db` 2.96 ms (12.2%; M5 5.07, 14.0%), both owned by the crate's split file:
  the first is the trace's blind spot (EXP-029), the second a real tail of about 1% of
  capacity on patched traces.
- At 1 thread the format window is 5.0–296 ms with utilisation 0.981–0.998; the 4-thread wall
  is 2.81x (`reg-tokio`, 22.5 → 8.0 ms) to 3.82x (`fx-hir-ty`, 301.6 → 78.9 ms) below the 1-thread
  wall; `reg-winnow` 3.59x, `reg-regex-syntax` 2.91x, `fx-hir` 2.99x, `fx-ide-db` 2.60x. The
  process outside the format window is 3–4 ms in every row.
- Against M5 on the same copies, the 1-thread format windows fell 7–24% (`fx-hir-ty` 306.1 →
  296.2 ms, `reg-regex-syntax` 233.7 → 173.3, `reg-clap-builder` 122.9 → 66.0, `fx-ide-db` 90.2 →
  71.1), the wave-5 levers and this window's faster host together; the instruction counts
  separate the two.

## The nested-closure cliff on the three builds (measurement F)

`benchmarks/cliffs/run.sh --shapes closures-expr --scales 1,2,4,8 --core 3` per binary (nesting
depths 2, 4, 8 and 16 of expression closures, eight copies each), the output compared with
rustfmt's through `fmt_stdin` (`head`'s for `head-pgo`).

| depth (scale) | input bytes | output bytes | base Ir | head Ir | head-pgo Ir | head / base | pgo / head |
|---|---|---|---|---|---|---|---|
| 2 (1) | 440 | 472 | 1,823,198 | 929,315 | 810,413 | 0.5097 | 0.8721 |
| 4 (2) | 648 | 680 | 2,756,453 | 1,322,272 | 1,124,713 | 0.4797 | 0.8506 |
| 8 (4) | 1064 | 1672 | 34,884,491 | 5,479,088 | 4,442,756 | 0.1571 | 0.8109 |
| 16 (8) | 2000 | 10672 | 242,459,586 | 5,513,069 | 4,773,093 | 0.0227 | 0.8658 |

Empty file (the fixed cost of a `bench` process): base 339,021, head 324,863, head-pgo 324,574.
- `base` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	1823198 2756453 34884491 242459586	2.483	2.589	3.586	1.663	3.609	1489368	1.476	0.704 3.837 2.809`; output identical to rustfmt's on 4 of 4 files.
- `head` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	929315 1322272 5479088 5513069	0.976	1.167	1.561	0.671	1.059	231698	0.301	0.723 2.369 0.009`; output identical to rustfmt's on 4 of 4 files.
- `head-pgo` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	810413 1124713 4442756 4773093	0.966	1.195	1.602	0.693	1.100	255243	0.301	0.720 2.364 0.111`; output identical to rustfmt's on 4 of 4 files.

- At depth 16 the start's binary spends 242.5 M instructions on a 2,000-byte file; the head
  spends 5.51 M (0.0227x; M5 5.69 M), 5.48 M at depth 8 (0.157x) and about half of base's at
  depths 2 and 4 (0.48–0.51x). The head's local slope between depths 8 and 16 is 0.008. The
  profile-guided build is 0.81–0.87x the head at every depth.
- Every output is identical to rustfmt's on all three builds (12 of 12 comparisons; rustfmt
  exit 0 on every file).

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), head with `--no-cache`, `hyperfine -N -i
-w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no `--threads`
option; on one file its static chunking gives the file to one thread, so the `base` column is
the sequential figure with the base's codegen. Sizes in KB (bytes / 1000).

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 91.3 / 107.6 ± 15.6 / 10 | 25.8 / 29.6 ± 4.4 / 9 | 20.0 / 24.7 ± 3.6 / 11 | 0.230 | 0.275 | below | 105.6 | 28.9 | 34.2 | 0.324 | b[0] t[0] h[0] |
| stdin-lints | 640 | 1c | 30 | 96.9 / 116.2 ± 16.9 / 11 | 27.4 / 32.3 ± 4.7 / 9 | 26.4 / 32.7 ± 5.6 / 9 | 0.281 | 0.278 | below | 113.6 | 30.9 | 31.8 | 0.280 | b[0] t[0] h[0] |
| stdin-nodes | 358 | mc | 30 | 86.6 / 107.7 ± 22.5 / 14 | 38.9 / 50.7 ± 9.4 / 14 | 29.2 / 38.4 ± 5.3 / 14 | 0.357 | 0.470 | below | 106.1 | 49.5 | 57.5 | 0.542 | b[0] t[0] h[0] |
| stdin-nodes | 358 | 1c | 30 | 83.7 / 96.6 ± 15.5 / 14 | 40.9 / 47.0 ± 7.6 / 14 | 40.8 / 44.7 ± 5.3 / 14 | 0.462 | 0.486 | below | 95.4 | 45.5 | 43.8 | 0.459 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | mc | 30 | 70.9 / 79.9 ± 13.7 / 9 | 31.3 / 36.9 ± 4.9 / 9 | 19.9 / 23.6 ± 2.9 / 9 | 0.296 | 0.462 | below | 77.9 | 36.0 | 39.8 | 0.511 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | 1c | 30 | 70.6 / 85.6 ± 16.2 / 9 | 32.5 / 38.1 ± 5.6 / 9 | 32.0 / 36.0 ± 5.9 / 9 | 0.421 | 0.445 | below | 83.3 | 36.3 | 35.1 | 0.422 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | mc | 30 | 20.0 / 25.3 ± 5.6 / 6 | 8.4 / 9.8 ± 1.3 / 5 | 6.8 / 8.0 ± 1.0 / 5 | 0.315 | 0.389 | below | 24.4 | 9.2 | 11.9 | 0.489 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | 1c | 30 | 20.6 / 26.7 ± 5.9 / 6 | 9.4 / 12.0 ± 2.4 / 5 | 9.9 / 11.7 ± 2.6 / 5 | 0.437 | 0.449 | below | 26.0 | 11.4 | 11.2 | 0.430 | b[0] t[0] h[0] |
| stdin-config | 169 | mc | 30 | 29.5 / 33.6 ± 5.1 / 6 | 13.9 / 16.9 ± 2.5 / 6 | 9.9 / 12.2 ± 1.0 / 6 | 0.363 | 0.503 | below | 32.4 | 16.4 | 19.6 | 0.605 | b[0] t[0] h[0] |
| stdin-config | 169 | 1c | 30 | 29.2 / 36.7 ± 6.3 / 6 | 15.7 / 18.1 ± 3.2 / 6 | 14.7 / 17.7 ± 3.4 / 6 | 0.482 | 0.494 | below | 35.7 | 17.1 | 17.2 | 0.482 | b[0] t[0] h[0] |

- Unpinned, `head/base` in mean: 0.230 (`stdin-lints`), 0.296 (`stdin-hir-lib`), 0.315
  (`stdin-hover-tests`), 0.357 (`stdin-nodes`), 0.363 (`stdin-config`); M5 0.274–0.423. Every
  head run is below every base run on all five files, pinned and unpinned.
- `head --threads 1` against base, unpinned: 0.275–0.503 (M5 0.320–0.551); pinned 0.278–0.494.
- The split against `head --threads 1`, unpinned: wall 0.640x (`stdin-hir-lib`, 23.6 vs
  36.9 ms) to 0.834x (`stdin-lints`, 24.7 vs 29.6), 0.757x on `stdin-nodes`, 0.816x on
  `stdin-hover-tests`, 0.722x on `stdin-config`; CPU 1.11x–1.29x the single thread's; peak RSS
  equal to 1.22x (`stdin-lints` 11 vs 9 MiB). Pinned to one core the split costs 0.95x–1.01x
  the single thread's wall.

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload; runs 1–3 are
`compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`
back to back at the end of the chain.

| run | 1c rustfmt | 1c head | 1c base | 1c head/base | 1c head/rustfmt | mc rustfmt | mc head | mc base | mc head/base | mc head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 892.2 / 1021.9 ± 97.9 | 220.6 / 257.9 ± 30.6 | 613.3 / 668.9 ± 42.9 | 0.386 | 0.252 | 880.5 / 1034.0 ± 87.2 | 64.6 / 79.8 ± 8.8 | 188.7 / 247.7 ± 28.6 | 0.322 | 0.077 |
| 1 | 862.6 / 1036.4 ± 111.3 | 223.7 / 257.8 ± 30.2 | 614.5 / 718.0 ± 107.4 | 0.359 | 0.249 | 898.4 / 1023.2 ± 89.2 | 67.6 / 76.4 ± 7.7 | 195.6 / 230.8 ± 27.9 | 0.331 | 0.075 |
| 2 | 896.6 / 1045.9 ± 107.0 | 226.7 / 288.8 ± 52.6 | 632.9 / 714.1 ± 71.5 | 0.404 | 0.276 | 877.3 / 1130.6 ± 178.4 | 65.0 / 80.3 ± 8.4 | 198.9 / 243.2 ± 24.9 | 0.330 | 0.071 |
| 3 | 893.2 / 1039.8 ± 106.5 | 228.1 / 267.1 ± 45.0 | 630.8 / 705.8 ± 71.3 | 0.378 | 0.257 | 948.1 / 1082.8 ± 101.8 | 66.8 / 80.6 ± 8.4 | 201.6 / 241.2 ± 30.2 | 0.334 | 0.074 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 257.8–288.8
  ms (12%), the `1c` base mean 668.9–718.0 ms (7%), the `1c` rustfmt mean 1021.9–1045.9 ms (2%);
  the `mc` head mean 76.4–80.6 ms (5%) and the `mc` base mean 230.8–247.7 ms (7%). The `1c`
  head/base ratio reads 0.359–0.404 and the `mc` ratio 0.322–0.334; head/rustfmt reads
  0.249–0.276 (`1c`) and 0.071–0.077 (`mc`). Every head run is below every base run and every
  rustfmt run in all four runs, pinned and unpinned.
- Instruction counts move by at most 1,671 of 203 M (0.0008%) between runs of the whole
  sample.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir` 1c: head vs base: below (73.0–138.9 vs 192.4–316.4 ms); head vs rustfmt: below (73.0–138.9 vs 279.6–513.7 ms); head-pgo vs base: below (65.0–108.4 vs 193.6–316.4 ms); head-pgo vs rustfmt: below (65.0–108.4 vs 277.0–456.3 ms).
- `fx-hir` mc: head vs base: below (25.4–34.0 vs 89.0–159.6 ms); head vs rustfmt: below (25.4–34.0 vs 296.7–467.1 ms); head-pgo vs base: below (25.5–35.6 vs 94.7–166.7 ms); head-pgo vs rustfmt: below (25.5–35.6 vs 286.7–414.8 ms).
- `fx-hir-ty` 1c: head vs base: below (220.6–337.5 vs 613.3–766.0 ms); head vs rustfmt: below (220.6–337.5 vs 892.2–1325.1 ms); head-pgo vs base: below (201.0–279.8 vs 632.2–1055.7 ms); head-pgo vs rustfmt: below (201.0–279.8 vs 890.7–1387.5 ms).
- `fx-hir-ty` mc: head vs base: below (64.6–98.1 vs 188.7–322.1 ms); head vs rustfmt: below (64.6–98.1 vs 880.5–1282.7 ms); head-pgo vs base: below (60.9–89.4 vs 202.1–319.0 ms); head-pgo vs rustfmt: below (60.9–89.4 vs 931.9–1227.7 ms).
- `fx-ide-db` 1c: head vs base: below (70.3–107.9 vs 212.8–453.2 ms); head vs rustfmt: below (70.3–107.9 vs 313.0–526.3 ms); head-pgo vs base: below (61.5–88.4 vs 216.6–349.2 ms); head-pgo vs rustfmt: below (61.5–88.4 vs 299.8–503.5 ms).
- `fx-ide-db` mc: head vs base: below (27.3–41.0 vs 118.1–199.8 ms); head vs rustfmt: below (27.3–41.0 vs 309.8–529.1 ms); head-pgo vs base: below (23.4–36.0 vs 124.5–203.1 ms); head-pgo vs rustfmt: below (23.4–36.0 vs 313.8–531.3 ms).
- `fx-parser` 1c: head vs base: below (25.9–34.1 vs 64.4–110.3 ms); head vs rustfmt: below (25.9–34.1 vs 73.2–147.1 ms); head-pgo vs base: below (24.1–27.4 vs 65.3–123.5 ms); head-pgo vs rustfmt: below (24.1–27.4 vs 73.9–151.9 ms).
- `fx-parser` mc: head vs base: below (10.3–13.4 vs 23.8–39.4 ms); head vs rustfmt: below (10.3–13.4 vs 78.1–150.3 ms); head-pgo vs base: below (9.7–17.3 vs 25.9–41.8 ms); head-pgo vs rustfmt: below (9.7–17.3 vs 78.7–147.2 ms).
- `fx-syntax` 1c: head vs base: below (42.1–67.0 vs 111.6–192.5 ms); head vs rustfmt: below (42.1–67.0 vs 131.5–217.7 ms); head-pgo vs base: below (40.5–60.2 vs 115.5–215.0 ms); head-pgo vs rustfmt: below (40.5–60.2 vs 137.0–253.5 ms).
- `fx-syntax` mc: head vs base: below (15.5–21.5 vs 38.0–82.0 ms); head vs rustfmt: below (15.5–21.5 vs 130.2–225.2 ms); head-pgo vs base: below (15.3–25.5 vs 41.7–80.4 ms); head-pgo vs rustfmt: below (15.3–25.5 vs 133.2–223.8 ms).
- `reg-clap-builder` 1c: head vs base: below (63.7–82.8 vs 169.2–294.9 ms); head vs rustfmt: below (63.7–82.8 vs 190.5–278.2 ms); head-pgo vs base: below (57.3–81.0 vs 167.6–302.7 ms); head-pgo vs rustfmt: below (57.3–81.0 vs 193.7–248.9 ms).
- `reg-clap-builder` mc: head vs base: below (24.7–34.8 vs 57.6–93.8 ms); head vs rustfmt: below (24.7–34.8 vs 191.2–304.7 ms); head-pgo vs base: below (21.5–30.6 vs 59.8–91.9 ms); head-pgo vs rustfmt: below (21.5–30.6 vs 196.4–323.0 ms).
- `reg-regex-syntax` 1c: head vs base: below (146.9–282.6 vs 424.6–826.0 ms); head vs rustfmt: below (146.9–282.6 vs 441.3–896.8 ms); head-pgo vs base: below (129.6–211.0 vs 419.9–715.4 ms); head-pgo vs rustfmt: below (129.6–211.0 vs 460.2–786.6 ms).
- `reg-regex-syntax` mc: head vs base: below (51.9–80.7 vs 130.9–227.3 ms); head vs rustfmt: below (51.9–80.7 vs 459.0–811.6 ms); head-pgo vs base: below (51.0–74.6 vs 158.2–255.7 ms); head-pgo vs rustfmt: below (51.0–74.6 vs 464.5–905.2 ms).
- `reg-tokio` 1c: head vs base: below (14.8–22.3 vs 34.9–66.4 ms); head vs rustfmt: below (14.8–22.3 vs 41.2–78.4 ms); head-pgo vs base: below (13.8–17.8 vs 34.0–49.1 ms); head-pgo vs rustfmt: below (13.8–17.8 vs 41.5–65.3 ms).
- `reg-tokio` mc: head vs base: below (7.2–11.0 vs 16.1–32.3 ms); head vs rustfmt: below (7.2–11.0 vs 42.7–88.1 ms); head-pgo vs base: below (6.4–14.1 vs 16.5–32.7 ms); head-pgo vs rustfmt: below (6.4–14.1 vs 43.5–66.6 ms).
- `reg-winnow` 1c: head vs base: below (66.5–110.1 vs 188.6–314.1 ms); head vs rustfmt: below (66.5–110.1 vs 182.3–349.0 ms); head-pgo vs base: below (60.1–104.6 vs 188.9–278.6 ms); head-pgo vs rustfmt: below (60.1–104.6 vs 177.3–394.4 ms).
- `reg-winnow` mc: head vs base: below (23.4–34.0 vs 64.4–108.0 ms); head vs rustfmt: below (23.4–34.0 vs 183.5–261.9 ms); head-pgo vs base: below (21.1–30.0 vs 63.6–113.8 ms); head-pgo vs rustfmt: below (21.1–30.0 vs 188.5–356.7 ms).
- `stdin-config` 1c: head vs base: below (12.0–15.0 vs 25.9–35.2 ms); head vs rustfmt: below (12.0–15.0 vs 41.8–63.3 ms).
- `stdin-hir-lib` 1c: head vs base: below (28.1–30.5 vs 64.5–79.0 ms); head vs rustfmt: below (28.1–30.5 vs 87.0–139.8 ms).
- `stdin-hover-tests` 1c: head vs base: below (7.1–7.9 vs 17.4–19.4 ms); head vs rustfmt: below (7.1–7.9 vs 29.3–35.7 ms).
- `stdin-lints` 1c: head vs base: below (22.7–47.0 vs 85.7–154.8 ms); head vs rustfmt: below (22.7–47.0 vs 137.4–197.4 ms).
- `stdin-nodes` 1c: head vs base: below (36.9–75.9 vs 78.9–120.3 ms); head vs rustfmt: below (36.9–75.9 vs 89.2–140.0 ms).
- `tiny-one` 1c: head vs base: overlap (1.7–2.0 vs 1.7–1.9 ms); head vs rustfmt: below (1.7–2.0 vs 9.7–18.0 ms); head-pgo vs base: overlap (1.7–2.1 vs 1.7–2.2 ms); head-pgo vs rustfmt: below (1.7–2.1 vs 9.8–12.3 ms).
- `tiny-one` mc: head vs base: overlap (1.9–3.7 vs 2.0–3.1 ms); head vs rustfmt: below (1.9–3.7 vs 10.5–19.2 ms); head-pgo vs base: overlap (2.0–3.7 vs 1.9–4.1 ms); head-pgo vs rustfmt: below (2.0–3.7 vs 10.5–18.3 ms).
- `tiny-test-data` 1c: head vs base: below (11.4–18.2 vs 22.4–33.0 ms); head vs rustfmt: below (11.4–18.2 vs 49.8–101.5 ms); head-pgo vs base: below (10.2–15.9 vs 22.1–40.9 ms); head-pgo vs rustfmt: below (10.2–15.9 vs 49.8–84.3 ms).
- `tiny-test-data` mc: head vs base: below (5.8–10.6 vs 12.5–21.4 ms); head vs rustfmt: below (5.8–10.6 vs 49.7–85.3 ms); head-pgo vs base: overlap (5.7–13.5 vs 11.9–22.0 ms); head-pgo vs rustfmt: below (5.7–13.5 vs 50.4–80.1 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS (`head-pgo` against the
`base` runs of sweep 2, its own window), as the table builder lists them:

- `stdin-hir-lib.1c` head vs base, rss: 8.9 vs 8.8 MiB (1.012); wall ranges below.
- `stdin-nodes.1c` head vs base, rss: 13.9 vs 13.8 MiB (1.011); wall ranges below.
- `tiny-one.1c` head vs base, min: 1.7 vs 1.7 ms (1.011); wall ranges overlap.
- `tiny-one.mc` head vs base, mean: 2.7 vs 2.6 ms (1.063); wall ranges overlap.
- `tiny-one.mc` head vs base, CPU: 2.5 vs 2.3 ms (1.088); wall ranges overlap.
- `tiny-one.mc` head vs base, rss: 3.7 vs 3.7 MiB (1.014); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, min: 1.7 vs 1.7 ms (1.021); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, CPU: 1.7 vs 1.7 ms (1.001); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, rss: 4.2 vs 3.7 MiB (1.142); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, mean: 2.8 vs 2.6 ms (1.051); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, min: 2.0 vs 1.9 ms (1.069); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, CPU: 2.6 vs 2.5 ms (1.053); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, rss: 4.3 vs 3.7 MiB (1.148); wall ranges overlap.
- `tiny-test-data.1c` head-pgo vs base, rss: 5.2 vs 4.9 MiB (1.050); wall ranges below.

- The `tiny-one` cells read above base in mean on the unpinned row (1.063 head, 1.051
  head-pgo, inside overlapping ranges, 1.9–4.6 against 2.0–3.3 ms) and head-pgo's start-up RSS is 0.5–0.6 MiB above
  base's (the profile-guided binary is 0.7 MB larger), as the 13-byte file has in every
  record. Two stdin rows read 0.1 MiB above base's RSS (8.9 vs 8.8, 13.9 vs 13.8) with wall
  below. No crate or 287-file cell has head or head-pgo above base or rustfmt in mean, minimum,
  CPU or wall range.

## Against M5

M5 ([m5-quiet-host.md](m5-quiet-host.md)) measured `d2f6fa0` (the code of `e96b530`, the wave-4
head) on this host class in a slow window; this record measures `2cb0ba3` (the code of
`96c9abc`: M5's head plus E31's lexer front end and E30's width rule) in a window between M4's
and M5's (Load). Protocol §3 keeps absolute figures within one window, so the records' ratios
are set side by side and the absolute figures are not; `base` is the same binary (same sha256)
in all records:

| ratio (means) | M4, `6b78728` | M5, `d2f6fa0` | M6, `2cb0ba3` |
|---|---|---|---|
| crates, `mc`, head/base | 0.25–0.52 | 0.219–0.441 | 0.209–0.421 |
| crates, `1c`, head/base (wall; CPU) | 0.38–0.60; 0.38–0.59 | 0.294–0.462; 0.297–0.470 | 0.280–0.408; 0.278–0.407 |
| crates, `mc`, pgo/base | 0.22–0.45 | 0.228–0.368 | 0.195–0.376 |
| crates, `1c`, pgo/base | 0.36–0.43 | 0.285–0.429 | 0.269–0.397 |
| crates, `mc`, head/rustfmt | 0.092–0.179 | 0.083–0.173 | 0.077–0.165 |
| crates, `1c`, head/rustfmt | 0.26–0.44 | 0.239–0.405 | 0.214–0.345 |
| crates, `mc`, pgo/rustfmt | 0.072–0.175 | 0.066–0.148 | 0.067–0.164 |
| crates, `1c`, pgo/rustfmt | 0.24–0.38 | 0.210–0.332 | 0.182–0.325 |
| crates, head below base and rustfmt in range, both modes | 8–9 of 9 | 9 of 9 | 9 of 9 |
| 287 small files, `mc`; `1c`, head/base | 0.442; 0.565 | 0.411; 0.450 | 0.426; 0.572 |
| stdin, head/base | 0.35–0.56 | 0.312–0.506 | 0.271–0.519 |
| split files, `mc`, head/base (wall; CPU) | 0.29–0.42; 0.43–0.66 | 0.274–0.423; 0.371–0.623 | 0.230–0.363; 0.324–0.605 |
| `bench -n 5`, head/base on the triples' MB/s | 2.20x–2.38x | 2.50x–2.67x | 2.71x–2.75x |
| `bench -n 5`, head; head-pgo MB/s (base in the window) | 11.22–11.79; 13.03–13.24 (4.96–5.09) | 9.33–10.38; 11.15–12.56 (3.68–4.03) | 11.99–13.19; 14.45–15.10 (4.36–4.86) |
| `Ir`, fixture sample, head/base; pgo/base | 0.4221; 0.3519 | 0.3758; 0.3106 | 0.3449; 0.2892 |
| `Ir`, registry sample, head/base; pgo/base | — | 0.4356; 0.3660 | 0.3713; 0.3195 |
| parse-only share of head's `Ir`, fixtures; registry | 37.3%; ~43% (E27) | 39.1%; 46.6% | 33.7%; 41.7% |
| nested-closure cliff, depth 16, head `Ir` | 105.4 M (before E26) | 5.69 M | 5.51 M |
| cache, warm vs no-cache vs base, `mc` | 16.9 / 372.7 / 909.4 ms | 17.8 / 447.4 / 1280.3 ms | 16.2 / 355.5 / 1014.2 ms |
| `conform -i`, head/base | 0.45 | 0.39 | 0.36 (harness), 0.38 (by hand) |
| `conform -i`, head identical | 1220/1220 (and at 1.8.0) | 1220/1220 (and at 1.8.0) | 1220/1220 (and at 1.8.0) |

- Every ratio moved with the instruction count again: the fixture-sample `Ir` ratio against
  base went 0.4221 → 0.3758 → 0.3449 over the three records (−8.2% between M5 and M6) and the
  pinned crate `head/rustfmt` range 0.26–0.44 → 0.239–0.405 → 0.214–0.345, `head/base`
  0.38–0.60 → 0.294–0.462 → 0.280–0.408. The registry sample moved more (0.4356 → 0.3713,
  −14.8%), which is E31's lexer: the registry files are plainer and the parser was a larger
  share of them.
- The in-process throughput against the start is now 2.71x–2.75x in one window (M5 2.50x–2.67x,
  M4 2.20x–2.38x); head reads 12.0–13.2 MB/s where M4's head read 11.2–11.8 on a host 3–5%
  faster for base, so the wave-5 levers are visible in wall time, not only in the count.
- `base` sits between M4's and M5's windows for every quantity (Load), so M6's absolute
  milliseconds are not M4's or M5's; the three records' ratios are the comparison.

## Harness notes

- `compare_rustfmt.sh` passes `--no-cache` to the head builds on its own (`chloro_cache_flag=
  --no-cache` in `env.txt`) and no flag to `base`, which rejects it. The flag also turns off
  the opt-in unit cache. The `--rustfmt` and `--registry` defaults resolved without being
  passed.
- The `identical outputs` figure is the binary's own (the harness recomputes the agreement when
  a copy is reused by another binary and records the binary's hash beside it,
  `agreement_chloro=<sha16>` in the copy's `meta`), so the appendix tables carry the right
  counts (every workload identical, `tiny-test-data` 289/289 raw).
- `benchmarks/pgo.sh` ran before the chain (08:15–08:18 UTC, on the quiet host): instrumented
  build, training, optimised build of `chloro` and `examples/bench` under `target/pgo/release/`.
  The profile differs per training run, so the profile-guided instruction count is this
  build's (0.8385x head; the build of the E31 merge read 171,001,243 on the same sample,
  0.8417x its head).
- The by-hand `bench -n 5` triples pass `--root /home/user/chloro/chloro-core/tests/conformance/
  fixtures` to all three binaries, so `base`'s `bench` reads the head's fixture tree. The
  harness's own `inproc-bench` rows use each binary's compiled-in root, as in M3–M5.
- `profile.sh` is E27's; the counts are comparable with the ledger's from E27 on (the ledger's
  `96c9abc` count, 202,822,327–202,822,529 over three runs, was taken by the same script on the
  same code before this chain; this record's two runs read 202,822,485 and 202,824,156).
- The trace rows at 4 threads are unpinned and the 1-thread rows pinned to core 3, as
  `collect.sh` does; `diag.py` records the 1-minute load average beside each run (0.95–2.2
  during the section, the chain's own processes). A release trace cannot see a split file's
  item threads (EXP-029); the two split-file crates' tail columns are kept for comparison with
  M5 and are not a measure of idle cores.
- The cliff run's `fmt_stdin` for `head-pgo` is head's (no profile-guided `fmt_stdin`); the
  profile-guided `bench`'s output bytes equal head's on every file (`out_bytes` 472, 680, 1672,
  10672).

### Output agreement, as the harness recomputed it with this record's binaries

| input | workloads | identical |
|---|---|---|
| formatted (`a1`, `a2`) | nine crates, 287 files, one file, five stdin files | 15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 287/287, 1/1, 1/1 ×5 |
| raw (`a3`) | the same, bytes as found | 15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 289/289, 1/1, 1/1 ×5 |
| the cliff files (`f`) | `closures-expr` at depths 2, 4, 8, 16 | 4/4 on base, head and head-pgo |

## Current State

- `benchmarks/compare_rustfmt.sh`, `report.py`, `profile.sh`, `schedule/diag.py`,
  `cliffs/run.sh` and `pgo.sh` ran to completion on every sweep (exit 0) on a host with no
  other agent; seven harness results directories and six by-hand directories hold the JSON
  exports, `commands.txt`, `env.txt`, `report.md`, the traces, the callgrind profiles and the
  loads log.
- On formatted input with all 4 CPUs, `head` takes 0.077–0.165x rustfmt's wall time on the
  nine crates and 0.209–0.421x base's; on one core 0.214–0.345x rustfmt's and 0.280–0.408x
  base's (CPU 0.278–0.407x). Every head run is below every rustfmt run and every base run on
  every crate, pinned and unpinned.
- `head-pgo` takes 0.067–0.164x rustfmt's wall time unpinned and 0.182–0.325x pinned;
  0.195–0.376x base's unpinned and 0.269–0.397x pinned; `Ir` 0.2892x base's and 0.8385x head's
  on the fixture sample, 0.3195x and 0.8604x on the registry sample.
- The warm cache answers `--check` on 1238 formatted files in 16.2 ms against 355.5 ms without
  it (22.0x) and 1014.2 ms for base (62.7x); 40.2x and 104.9x on one core.
- In process, head formats the fixtures at 11.99–13.19 MB/s against base's 4.36–4.86 MB/s
  (disjoint over 3 interleaved triples; 2.71x–2.75x), head-pgo at 14.45–15.10 MB/s; `conform
  -i` reports 1220/1220 identical for head at rustfmt 1.9.0 and at 1.8.0, 1219/1220 for base;
  the instruction count on the fixture sample is 202,822,485 (0.3449 of base, −65.5% from the
  start), 170,069,599 profile-guided (0.2892); on the registry sample 255,681,198 (0.3713)
  and 219,987,557 (0.3195).
- The parse-only phase is 33.7% of head's instructions on the fixture sample and 41.7% on
  the registry sample (M5: 39.1% and 46.6%).
- The nested-closure cliff: depth 16 costs 5.51 M instructions on head against 242.5 M on
  base (0.0227x), with identical output.
- The parallel-items split gives 0.230–0.363x base's wall time on the five largest files
  with 4 threads, at 0.324–0.605x the CPU and up to 1.22x the peak RSS.
- Output agreement with rustfmt, recomputed with these binaries: every file of every workload
  on both inputs and the cliff files.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure; the cliff comparison for `head-pgo` uses head's `fmt_stdin`.
- The `cold` cache variant has no pinned row; the profile-guided cache rows are unpinned only.
- No raw-input row for `head-pgo` (measurement A's raw sweep is head only, as in M1–M5).
- The by-hand `conform -i` runs have no peak RSS of their own (no `/usr/bin/time` in the
  container).
- The host window differs from M4's and M5's (Load); only ratios and instruction counts carry
  between the records.
- The trace section is the wall view only and cannot see a split file's item threads
  (EXP-029); the reconstruction from thread CPU clocks was not rerun on this head.
- `base`'s `bench` has no `--phase parse`, so the parser's share at the start is not
  measured here.
- The unit cache (`--unit-cache`, E15) and the resident process prototype (E21, on its
  branch) are not measured here.

## Appendix: `report.py` tables as written

The `report.md` of each harness results directory, as `report.py` wrote it.

### `m6-a1-head-base-formatted`

```text
date_utc=2026-10-17T08:19:42Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.30 1.23 1.90
cpu3_stat_start=398237 6236 49932 190588 7298 0 764 5066 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.47 1.41 1.73
cpu3_stat_end=427455 6236 52363 199718 7372 0 772 5412 0 0
cpu3_steal_ticks=346 cpu3_busy_ticks=31649 cpu3_idle_ticks=9130
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1021.9 ± 97.9 (892.2–1325.1) | 257.9 ± 30.6 (220.6–337.5) | 668.9 ± 42.9 (613.3–766.0) | 0.386 | below | 0.252 | 0.247 | below | 86 | 8 | 1009.8 | 254.9 | 0.252 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1034.0 ± 87.2 (880.5–1282.7) | 79.8 ± 8.8 (64.6–98.1) | 247.7 ± 28.6 (188.7–322.1) | 0.322 | below | 0.077 | 0.073 | below | 86 | 16 | 1021.8 | 295.5 | 0.289 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 327.9 ± 51.0 (279.6–513.7) | 85.9 ± 19.2 (73.0–138.9) | 248.4 ± 41.9 (192.4–316.4) | 0.346 | below | 0.262 | 0.261 | below | 57 | 9 | 324.8 | 84.7 | 0.261 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 332.7 ± 36.4 (296.7–467.1) | 28.7 ± 2.2 (25.4–34.0) | 111.4 ± 18.4 (89.0–159.6) | 0.257 | below | 0.086 | 0.086 | below | 57 | 17 | 327.6 | 91.1 | 0.278 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 369.8 ± 51.1 (313.0–526.3) | 79.1 ± 9.2 (70.3–107.9) | 283.0 ± 72.3 (212.8–453.2) | 0.280 | below | 0.214 | 0.225 | below | 58 | 9 | 364.7 | 77.9 | 0.214 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 378.2 ± 58.4 (309.8–529.1) | 32.0 ± 3.2 (27.3–41.0) | 152.8 ± 21.4 (118.1–199.8) | 0.209 | below | 0.084 | 0.088 | below | 58 | 15 | 373.8 | 94.7 | 0.253 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 82.5 ± 17.6 (73.2–147.1) | 26.6 ± 1.5 (25.9–34.1) | 75.0 ± 13.7 (64.4–110.3) | 0.355 | below | 0.323 | 0.353 | below | 47 | 5 | 81.7 | 26.4 | 0.323 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 97.1 ± 20.4 (78.1–150.3) | 11.4 ± 0.8 (10.3–13.4) | 30.4 ± 4.1 (23.8–39.4) | 0.373 | below | 0.117 | 0.133 | below | 47 | 6 | 95.2 | 34.3 | 0.360 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 145.7 ± 20.3 (131.5–217.7) | 47.2 ± 5.9 (42.1–67.0) | 139.1 ± 23.0 (111.6–192.5) | 0.339 | below | 0.324 | 0.320 | below | 51 | 5 | 144.4 | 46.4 | 0.321 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 159.7 ± 25.0 (130.2–225.2) | 17.9 ± 1.6 (15.5–21.5) | 56.9 ± 11.5 (38.0–82.0) | 0.315 | below | 0.112 | 0.119 | below | 51 | 8 | 156.2 | 58.7 | 0.376 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 210.2 ± 25.3 (190.5–278.2) | 66.9 ± 4.1 (63.7–82.8) | 187.1 ± 24.4 (169.2–294.9) | 0.358 | below | 0.318 | 0.334 | below | 55 | 6 | 209.1 | 66.1 | 0.316 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 227.1 ± 26.0 (191.2–304.7) | 28.7 ± 2.7 (24.7–34.8) | 74.7 ± 9.4 (57.6–93.8) | 0.384 | below | 0.126 | 0.129 | below | 55 | 11 | 223.4 | 90.9 | 0.407 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 543.3 ± 135.3 (441.3–896.8) | 175.9 ± 39.0 (146.9–282.6) | 521.3 ± 101.2 (424.6–826.0) | 0.337 | below | 0.324 | 0.333 | below | 66 | 10 | 540.6 | 174.0 | 0.322 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 553.2 ± 89.8 (459.0–811.6) | 61.3 ± 8.1 (51.9–80.7) | 163.8 ± 23.5 (130.9–227.3) | 0.374 | below | 0.111 | 0.113 | below | 66 | 23 | 546.0 | 204.5 | 0.375 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 54.4 ± 11.6 (41.2–78.4) | 17.3 ± 2.0 (14.8–22.3) | 42.3 ± 8.5 (34.9–66.4) | 0.408 | below | 0.318 | 0.359 | below | 43 | 4 | 53.3 | 16.9 | 0.317 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 55.5 ± 10.9 (42.7–88.1) | 9.1 ± 1.3 (7.2–11.0) | 21.7 ± 3.3 (16.1–32.3) | 0.421 | below | 0.165 | 0.169 | below | 43 | 5 | 54.7 | 24.0 | 0.440 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 211.5 ± 35.1 (182.3–349.0) | 73.0 ± 10.1 (66.5–110.1) | 225.9 ± 35.4 (188.6–314.1) | 0.323 | below | 0.345 | 0.365 | below | 59 | 7 | 209.9 | 72.0 | 0.343 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 217.1 ± 23.0 (183.5–261.9) | 27.7 ± 2.8 (23.4–34.0) | 85.6 ± 12.9 (64.4–108.0) | 0.323 | below | 0.127 | 0.128 | below | 59 | 11 | 213.2 | 92.9 | 0.436 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.7 ± 2.5 (9.7–18.0) | 1.8 ± 0.1 (1.7–2.0) | 1.8 ± 0.1 (1.7–1.9) | 0.994 | overlap | 0.151 | 0.173 | below | 34 | 4 | 11.5 | 1.6 | 0.143 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 14.5 ± 3.1 (10.5–19.2) | 2.7 ± 0.5 (1.9–3.7) | 2.6 ± 0.3 (2.0–3.1) | 1.063 | overlap | 0.187 | 0.184 | below | 34 | 4 | 14.1 | 2.5 | 0.180 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 61.6 ± 14.7 (49.8–101.5) | 14.2 ± 2.4 (11.4–18.2) | 24.8 ± 2.6 (22.4–33.0) | 0.572 | below | 0.230 | 0.228 | below | 44 | 4 | 60.9 | 13.6 | 0.224 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 61.8 ± 10.4 (49.7–85.3) | 7.2 ± 1.0 (5.8–10.6) | 16.9 ± 2.7 (12.5–21.4) | 0.426 | below | 0.117 | 0.118 | below | 44 | 5 | 60.0 | 15.9 | 0.265 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 169 | 30 | 47.8 ± 5.1 (41.8–63.3) | 12.8 ± 0.6 (12.0–15.0) | 28.3 ± 2.3 (25.9–35.2) | 0.451 | below | 0.267 | 0.288 | below | 44 | 5 | 47.0 | 12.5 | 0.267 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 97.6 ± 13.6 (87.0–139.8) | 29.0 ± 0.6 (28.1–30.5) | 67.4 ± 3.2 (64.5–79.0) | 0.431 | below | 0.297 | 0.323 | below | 47 | 9 | 96.5 | 28.8 | 0.298 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 31.5 ± 1.7 (29.3–35.7) | 7.4 ± 0.2 (7.1–7.9) | 18.2 ± 0.5 (17.4–19.4) | 0.408 | below | 0.236 | 0.242 | below | 40 | 4 | 31.0 | 7.2 | 0.233 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 150.0 ± 15.0 (137.4–197.4) | 28.5 ± 6.5 (22.7–47.0) | 105.4 ± 22.3 (85.7–154.8) | 0.271 | below | 0.190 | 0.165 | below | 43 | 9 | 148.5 | 27.6 | 0.186 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 96.7 ± 11.0 (89.2–140.0) | 44.9 ± 12.2 (36.9–75.9) | 86.5 ± 11.9 (78.9–120.3) | 0.519 | below | 0.464 | 0.414 | below | 53 | 14 | 96.3 | 44.4 | 0.461 | 1/1 | r[0] c[0] |

### `m6-a2-pgo-base-formatted`

```text
date_utc=2026-10-17T08:26:35Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.47 1.41 1.73
cpu3_stat_start=427456 6236 52364 199748 7372 0 772 5412 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/pgo/release/chloro sha256=bc58d147590d5da7
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=1.94 1.66 1.73
cpu3_stat_end=453867 6236 54302 209324 7375 0 782 5742 0 0
cpu3_steal_ticks=330 cpu3_busy_ticks=28349 cpu3_idle_ticks=9576
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1067.1 ± 123.0 (890.7–1387.5) | 215.5 ± 16.0 (201.0–279.8) | 748.0 ± 101.0 (632.2–1055.7) | 0.288 | below | 0.202 | 0.226 | below | 86 | 8 | 1055.3 | 214.5 | 0.203 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1028.3 ± 72.4 (931.9–1227.7) | 68.7 ± 6.3 (60.9–89.4) | 244.8 ± 30.1 (202.1–319.0) | 0.281 | below | 0.067 | 0.065 | below | 86 | 17 | 1017.9 | 250.0 | 0.246 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 336.5 ± 48.2 (277.0–456.3) | 72.8 ± 8.9 (65.0–108.4) | 216.2 ± 30.5 (193.6–316.4) | 0.337 | below | 0.216 | 0.235 | below | 57 | 10 | 334.6 | 71.6 | 0.214 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 329.9 ± 30.3 (286.7–414.8) | 30.0 ± 2.5 (25.5–35.6) | 131.9 ± 19.9 (94.7–166.7) | 0.228 | below | 0.091 | 0.089 | below | 57 | 18 | 323.6 | 95.8 | 0.296 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 366.0 ± 56.5 (299.8–503.5) | 66.6 ± 6.3 (61.5–88.4) | 247.6 ± 31.9 (216.6–349.2) | 0.269 | below | 0.182 | 0.205 | below | 58 | 10 | 361.5 | 66.1 | 0.183 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 373.1 ± 53.2 (313.8–531.3) | 29.5 ± 3.7 (23.4–36.0) | 151.5 ± 20.9 (124.5–203.1) | 0.195 | below | 0.079 | 0.075 | below | 58 | 16 | 367.7 | 88.0 | 0.239 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 90.6 ± 21.8 (73.9–151.9) | 25.2 ± 0.9 (24.1–27.4) | 80.5 ± 21.1 (65.3–123.5) | 0.313 | below | 0.278 | 0.326 | below | 47 | 5 | 89.9 | 24.9 | 0.277 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 101.8 ± 20.3 (78.7–147.2) | 12.0 ± 1.8 (9.7–17.3) | 34.3 ± 4.3 (25.9–41.8) | 0.350 | below | 0.118 | 0.123 | below | 47 | 7 | 100.3 | 34.5 | 0.344 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 156.0 ± 24.5 (137.0–253.5) | 50.7 ± 7.2 (40.5–60.2) | 164.8 ± 31.5 (115.5–215.0) | 0.307 | below | 0.325 | 0.296 | below | 51 | 6 | 153.6 | 49.8 | 0.324 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 162.5 ± 23.9 (133.2–223.8) | 18.1 ± 2.2 (15.3–25.5) | 55.9 ± 8.3 (41.7–80.4) | 0.323 | below | 0.111 | 0.115 | below | 51 | 8 | 160.1 | 56.3 | 0.352 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 211.3 ± 12.5 (193.7–248.9) | 63.4 ± 7.2 (57.3–81.0) | 206.1 ± 43.3 (167.6–302.7) | 0.308 | below | 0.300 | 0.296 | below | 55 | 7 | 209.8 | 62.3 | 0.297 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 233.7 ± 36.8 (196.4–323.0) | 25.7 ± 2.6 (21.5–30.6) | 70.9 ± 7.8 (59.8–91.9) | 0.363 | below | 0.110 | 0.110 | below | 55 | 12 | 230.4 | 83.4 | 0.362 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 557.8 ± 86.8 (460.2–786.6) | 166.4 ± 28.4 (129.6–211.0) | 523.0 ± 77.2 (419.9–715.4) | 0.318 | below | 0.298 | 0.282 | below | 66 | 11 | 550.5 | 163.5 | 0.297 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 579.3 ± 102.9 (464.5–905.2) | 62.9 ± 5.8 (51.0–74.6) | 204.4 ± 23.7 (158.2–255.7) | 0.308 | below | 0.109 | 0.110 | below | 66 | 22 | 568.8 | 202.8 | 0.357 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 50.6 ± 7.4 (41.5–65.3) | 14.6 ± 0.9 (13.8–17.8) | 36.9 ± 4.1 (34.0–49.1) | 0.397 | below | 0.289 | 0.332 | below | 43 | 5 | 49.6 | 14.2 | 0.287 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 54.4 ± 6.6 (43.5–66.6) | 8.9 ± 1.8 (6.4–14.1) | 23.7 ± 3.6 (16.5–32.7) | 0.376 | below | 0.164 | 0.148 | below | 43 | 6 | 52.3 | 20.5 | 0.391 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 208.4 ± 47.7 (177.3–394.4) | 67.0 ± 9.6 (60.1–104.6) | 214.1 ± 28.1 (188.9–278.6) | 0.313 | below | 0.322 | 0.339 | below | 59 | 8 | 205.3 | 65.2 | 0.318 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 229.2 ± 41.9 (188.5–356.7) | 25.2 ± 2.9 (21.1–30.0) | 81.8 ± 9.8 (63.6–113.8) | 0.309 | below | 0.110 | 0.112 | below | 59 | 12 | 225.4 | 84.1 | 0.373 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.2 ± 0.6 (9.8–12.3) | 1.8 ± 0.1 (1.7–2.1) | 1.9 ± 0.1 (1.7–2.2) | 0.991 | overlap | 0.164 | 0.175 | below | 34 | 4 | 10.9 | 1.7 | 0.155 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 13.0 ± 2.4 (10.5–18.3) | 2.8 ± 0.6 (2.0–3.7) | 2.6 ± 0.6 (1.9–4.1) | 1.051 | overlap | 0.212 | 0.194 | below | 34 | 4 | 12.5 | 2.6 | 0.209 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 60.3 ± 11.4 (49.8–84.3) | 12.0 ± 1.9 (10.2–15.9) | 26.7 ± 5.6 (22.1–40.9) | 0.450 | below | 0.200 | 0.204 | below | 44 | 5 | 59.3 | 11.8 | 0.198 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 61.0 ± 9.9 (50.4–80.1) | 7.1 ± 1.5 (5.7–13.5) | 16.4 ± 2.4 (11.9–22.0) | 0.434 | overlap | 0.117 | 0.113 | below | 44 | 5 | 59.6 | 14.9 | 0.249 | 287/287 | r[0] c[0] |

### `m6-a3-head-raw`

```text
date_utc=2026-10-17T08:32:59Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.94 1.66 1.73
cpu3_stat_start=453868 6236 54304 209336 7375 0 782 5742 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.52 1.50 1.60
cpu3_stat_end=478554 6236 58632 221690 7394 0 796 6040 0 0
cpu3_steal_ticks=298 cpu3_busy_ticks=29014 cpu3_idle_ticks=12354
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1429.1 ± 126.5 (1231.0–1657.3) | 275.7 ± 49.9 (222.3–414.4) | 0.193 | 0.181 | below | 117 | 8 | 1414.6 | 272.7 | 0.193 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1412.0 ± 99.8 (1251.8–1624.6) | 88.5 ± 11.2 (64.2–106.9) | 0.063 | 0.051 | below | 117 | 17 | 1395.8 | 323.3 | 0.232 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 612.3 ± 55.6 (541.2–741.7) | 82.4 ± 14.1 (73.3–130.1) | 0.135 | 0.135 | below | 218 | 9 | 606.1 | 81.6 | 0.135 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 614.1 ± 45.3 (539.5–722.4) | 31.6 ± 3.0 (26.2–37.9) | 0.052 | 0.049 | below | 218 | 17 | 606.7 | 101.8 | 0.168 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 470.0 ± 103.5 (350.4–686.9) | 90.5 ± 19.5 (69.7–142.3) | 0.192 | 0.199 | below | 66 | 9 | 466.9 | 88.1 | 0.189 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 411.8 ± 54.3 (349.9–578.2) | 30.4 ± 2.6 (25.8–37.1) | 0.074 | 0.074 | below | 66 | 15 | 406.7 | 94.6 | 0.233 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 92.8 ± 13.7 (83.3–136.2) | 27.4 ± 1.9 (26.1–34.1) | 0.296 | 0.313 | below | 48 | 5 | 91.6 | 27.2 | 0.297 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 104.7 ± 16.1 (86.1–144.0) | 13.4 ± 0.9 (11.2–15.3) | 0.128 | 0.130 | below | 48 | 6 | 102.4 | 39.3 | 0.384 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 191.0 ± 23.5 (161.5–260.2) | 44.2 ± 2.8 (41.7–51.4) | 0.231 | 0.258 | below | 57 | 5 | 188.8 | 43.9 | 0.232 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 197.6 ± 22.0 (168.0–248.4) | 19.6 ± 1.8 (15.7–23.1) | 0.099 | 0.094 | below | 57 | 8 | 194.9 | 63.5 | 0.326 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 226.7 ± 33.7 (199.0–333.1) | 69.2 ± 7.3 (63.3–90.3) | 0.305 | 0.318 | below | 55 | 6 | 224.4 | 68.8 | 0.307 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 230.3 ± 28.8 (196.2–296.0) | 27.8 ± 2.7 (24.9–34.9) | 0.121 | 0.127 | below | 55 | 11 | 226.3 | 89.3 | 0.395 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1610.6 ± 123.8 (1433.7–1926.2) | 183.6 ± 28.9 (152.1–260.1) | 0.114 | 0.106 | below | 423 | 10 | 1595.5 | 179.6 | 0.113 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1613.8 ± 109.1 (1421.2–1944.4) | 62.3 ± 7.0 (52.7–82.9) | 0.039 | 0.037 | below | 423 | 22 | 1599.1 | 205.9 | 0.129 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 46.9 ± 6.4 (42.0–64.3) | 17.9 ± 5.2 (14.9–30.1) | 0.383 | 0.354 | below | 44 | 4 | 46.2 | 17.6 | 0.381 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 54.3 ± 8.2 (43.8–72.0) | 8.5 ± 0.8 (7.0–10.6) | 0.157 | 0.161 | below | 44 | 5 | 52.4 | 21.6 | 0.412 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 226.2 ± 31.4 (194.9–323.9) | 95.7 ± 18.1 (72.2–135.1) | 0.423 | 0.370 | below | 60 | 7 | 224.6 | 94.8 | 0.422 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 231.6 ± 35.3 (196.7–325.1) | 28.0 ± 2.9 (24.7–36.7) | 0.121 | 0.126 | below | 60 | 11 | 228.3 | 93.6 | 0.410 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 13.7 ± 2.4 (10.5–18.4) | 1.9 ± 0.1 (1.8–2.1) | 0.139 | 0.170 | below | 34 | 4 | 13.1 | 1.7 | 0.133 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 15.0 ± 2.6 (11.4–21.2) | 3.3 ± 0.5 (2.3–4.1) | 0.223 | 0.200 | below | 34 | 4 | 14.5 | 3.0 | 0.206 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 67.9 ± 10.7 (62.0–117.5) | 12.0 ± 0.4 (11.6–13.5) | 0.176 | 0.187 | below | 44 | 4 | 67.4 | 11.7 | 0.174 | 289/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 71.8 ± 10.2 (62.0–102.6) | 7.5 ± 0.9 (6.1–10.1) | 0.105 | 0.099 | below | 44 | 5 | 70.8 | 16.8 | 0.237 | 289/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 50.8 ± 6.7 (44.6–68.9) | 14.1 ± 1.7 (12.3–18.8) | 0.277 | 0.275 | below | 44 | 5 | 50.0 | 13.8 | 0.276 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 107.7 ± 15.5 (89.9–147.7) | 33.1 ± 4.9 (28.7–47.4) | 0.308 | 0.319 | below | 47 | 9 | 106.1 | 32.8 | 0.309 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 36.7 ± 5.7 (31.4–51.3) | 7.8 ± 0.4 (7.3–9.0) | 0.212 | 0.234 | below | 40 | 4 | 36.1 | 7.5 | 0.207 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 150.1 ± 14.2 (139.1–209.5) | 24.6 ± 0.7 (23.6–26.4) | 0.164 | 0.170 | below | 43 | 9 | 148.6 | 24.3 | 0.164 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 104.6 ± 12.7 (91.6–142.7) | 39.6 ± 2.4 (37.4–48.7) | 0.378 | 0.408 | below | 53 | 14 | 103.5 | 38.9 | 0.376 | 1/1 | r[0] c[0] |

### `m6-d-inproc-harness`

```text
date_utc=2026-10-17T08:39:56Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.52 1.50 1.60
cpu3_stat_start=478561 6236 58635 221696 7394 0 796 6040 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.03 1.28 1.49
cpu3_stat_end=495703 6236 58820 221706 7531 0 801 6183 0 0
cpu3_steal_ticks=143 cpu3_busy_ticks=17327 cpu3_idle_ticks=10
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 2310.0 ± 145.0 (2038.4–2603.7) | 49 | 2287.2 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 6360.1 ± 441.4 (5476.1–6875.3) | 50 | 6308.8 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.060 s | 13.12 |
| 2 | chloro-base | 1238 | 13.9 | 2.988 s | 4.65 |
| 3 | chloro-base | 1238 | 13.9 | 2.871 s | 4.84 |
| 4 | chloro | 1238 | 13.9 | 1.012 s | 13.74 |
| 5 | chloro | 1238 | 13.9 | 1.015 s | 13.70 |
| 6 | chloro-base | 1238 | 13.9 | 2.869 s | 4.85 |

- chloro: 3 runs, 13.12–13.74 MB/s
- chloro-base: 3 runs, 4.65–4.85 MB/s

### `m6-noise-hir-ty-1`

```text
date_utc=2026-10-17T09:00:30Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.83 1.30 1.38
cpu3_stat_start=588934 6236 64562 227056 7572 0 833 7361 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.76 1.48 1.44
cpu3_stat_end=597193 6236 64891 230153 7573 0 834 7456 0 0
cpu3_steal_ticks=95 cpu3_busy_ticks=8588 cpu3_idle_ticks=3097
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1036.4 ± 111.3 (862.6–1302.2) | 257.8 ± 30.2 (223.7–366.5) | 718.0 ± 107.4 (614.5–1031.3) | 0.359 | below | 0.249 | 0.259 | below | 86 | 7 | 1026.3 | 255.3 | 0.249 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1023.2 ± 89.2 (898.4–1318.3) | 76.4 ± 7.7 (67.6–91.4) | 230.8 ± 27.9 (195.6–320.7) | 0.331 | below | 0.075 | 0.075 | below | 86 | 16 | 1010.2 | 280.7 | 0.278 | 122/122 | r[0] c[0] |

### `m6-noise-hir-ty-2`

```text
date_utc=2026-10-17T09:02:28Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=1.76 1.48 1.44
cpu3_stat_start=597194 6236 64894 230163 7573 0 834 7456 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=2.04 1.78 1.56
cpu3_stat_end=605847 6236 65254 233217 7573 0 835 7567 0 0
cpu3_steal_ticks=111 cpu3_busy_ticks=9013 cpu3_idle_ticks=3054
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1045.9 ± 107.0 (896.6–1251.0) | 288.8 ± 52.6 (226.7–444.6) | 714.1 ± 71.5 (632.9–892.4) | 0.404 | below | 0.276 | 0.253 | below | 86 | 8 | 1033.6 | 284.8 | 0.276 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1130.6 ± 178.4 (877.3–1748.4) | 80.3 ± 8.4 (65.0–98.7) | 243.2 ± 24.9 (198.9–291.6) | 0.330 | below | 0.071 | 0.074 | below | 86 | 16 | 1113.1 | 293.9 | 0.264 | 122/122 | r[0] c[0] |

### `m6-noise-hir-ty-3`

```text
date_utc=2026-10-17T09:04:30Z
commit=2cb0ba3753e4abe51b90d2674c59f1fe7918a253
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v139
loadavg_start=2.04 1.78 1.56
cpu3_stat_start=605848 6236 65256 233231 7573 0 835 7567 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=5a3b591fa07cb5b9
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=00858a31ff4a46fd
bench=/home/user/chloro/target/release/examples/bench sha256=960367538827f9f0
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=2.28 2.02 1.68
cpu3_stat_end=613666 6236 65573 237039 7573 0 839 7665 0 0
cpu3_steal_ticks=98 cpu3_busy_ticks=8135 cpu3_idle_ticks=3808
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1039.8 ± 106.5 (893.2–1311.3) | 267.1 ± 45.0 (228.1–411.7) | 705.8 ± 71.3 (630.8–909.0) | 0.378 | below | 0.257 | 0.255 | below | 86 | 8 | 1029.3 | 264.4 | 0.257 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1082.8 ± 101.8 (948.1–1369.5) | 80.6 ± 8.4 (66.8–99.2) | 241.2 ± 30.2 (201.6–332.1) | 0.334 | below | 0.074 | 0.070 | below | 86 | 17 | 1068.4 | 287.5 | 0.269 | 122/122 | r[0] c[0] |
