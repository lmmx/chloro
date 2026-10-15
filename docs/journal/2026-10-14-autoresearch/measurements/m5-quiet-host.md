# M5: base, head and head-pgo against rustfmt on a quiet host (the wave-4 head, E19–E28)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 5 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the wave-3 and wave-4 merges (E20
shared sink, E22 fused steps, E19 exact levers, E27 harness, E23 trace timeline, E28 memo
policy, E26 width lower bound, E25 scratch arena) and on rustfmt 1.9.0, with the method of
[m4-quiet-host.md](m4-quiet-host.md), [m3-quiet-host.md](m3-quiet-host.md),
[m2-quiet-host.md](m2-quiet-host.md) and [m1-quiet-host.md](m1-quiet-host.md), plus three
rows the wave added to the harness: the per-file and registry-sample instruction counts and
the parse-only phase of `profile.sh` (E27), the `CHLORO_TRACE` schedule timeline on a quiet
host (E23) and the nested-closure cliff shape (E19, E26). Every figure in this file is
**measured** in the windows named below; nothing is estimated. The host is the 2.80 GHz host
class of [../baseline.md](../baseline.md), M2, M3 and M4 (protocol §3: figures from different
hosts are not compared; the "against M4" section sets within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `d2f6fa0` (the integration branch after the E19–E28 merges; docs-only commits above the E25 merge `e96b530`, the ledger's best-known-good, same code; the formatter code is M4's `e640fbf` plus E20, E22, E19, E27, E23, E28, E26 and E25; E21 stays on its branch and E24 was rejected) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `d2f6fa06cc8c2511cdb59ca6b3a589d95818172b` (worktree clean,
  `worktree_dirty=0`; `git diff --stat 830345e HEAD` touches 12 files: E25's merge, the
  ledger, the handoff and `Cargo.lock`; `git diff --stat e640fbf HEAD`, M4's code, touches 69
  files); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git worktree add
  /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `f48f3593dc5bc8dbff2498984b10e2fa5f8d872cec29f9f1e386ac823d0bde00` | 2,196,120 |
| `head` bench | `target/release/examples/bench` | `59f34345395848fa3990d508f603f398409b061d7b6687a7d4fd11c6eca50d9b` | 1,882,072 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `908c217696a941540f7fb211debbe2e4cf18aef225d1b8945fae0c8bcced36eb` | 1,728,200 |
| `head` conform | `target/release/examples/conform` | `a065dbe6328dc4c4a36013015050d95875cf07ba8fe18a39036c46cc3a5b96bf` | 1,857,592 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `602ddf4ceeff1e0bcc6d2f04b41c5ab26060b9e05bb46da7f212d076649cf493` | 2,921,072 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `d4aac88216144833b1073104bfdb30f9e1b822e58b41bafe1458032d4a8b8274` | 2,623,168 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes and the rustfmt hash equal the ones recorded in M1–M4: the
  default-profile build of `884f9c9` reproduces byte for byte on this toolchain.
- The `head` chloro binary is 2,196,120 bytes against 2,147,536 for M4's `6b78728` build and
  2,516,048 for `base` (0.873x base; 1.023x M4's head: the CLI gained the trace timeline, the
  width bound, the arena and `bumpalo`); `head-pgo` is 2,921,072 bytes (1.161x base; M4's
  2,924,784).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 18 lines for `head`
  (M4: 17; E25 adds `bumpalo`, which has no transitive dependency), 38 for `base`.
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, used by `pgo.sh`);
  hyperfine 2.0.0; valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. The 2.80 GHz host class
  of M2, M3, M4 and [../baseline.md](../baseline.md).
- **Load:** no other agent ran on the host (E25's agent, the last of wave 4, had finished and
  its worktree was removed; the E25 merge gates and the profile-guided build had finished at
  16:31 UTC, one minute before the chain started, which is why the 1-minute load average read
  1.86 at the identity step; `env.txt` records no cargo, rustc, chloro, bench, valgrind,
  hyperfine or miri process at the start). The chain ran as one script
  (`scratchpad/m5/chain.sh`, reproduced in Method) with no gap between sweeps; the load
  average at the start of every later sweep is the harness's own processes from the previous
  sweep (the `mc` rows run chloro with 4 threads). `cpu3` columns are `/proc/stat` ticks of
  core 3 (USER_HZ 100) over the sweep: steal / busy / idle.

| mark | time | load | cpu3 steal / busy / idle ticks since previous mark |
|---|---|---|---|
| a1-start | 2026-10-15T16:32:33Z | 1.86 3.44 2.93 | – |
| a1-end | 2026-10-15T16:41:27Z | 1.44 2.04 2.42 | 643 / 38569 / 13967 |
| a2-start | 2026-10-15T16:41:27Z | 1.44 2.04 2.42 | 0 / 0 / 0 |
| a2-end | 2026-10-15T16:49:23Z | 1.91 2.02 2.27 | 584 / 35235 / 11735 |
| a3-start | 2026-10-15T16:49:23Z | 1.91 2.02 2.27 | 0 / 1 / 1 |
| a3-end | 2026-10-15T16:58:25Z | 1.17 1.28 1.76 | 457 / 35092 / 18646 |
| dh-start | 2026-10-15T16:58:25Z | 1.17 1.28 1.76 | 0 / 0 / 1 |
| dh-end | 2026-10-15T17:02:28Z | 1.00 1.12 1.58 | 261 / 23763 / 21 |
| c-start | 2026-10-15T17:02:28Z | 1.00 1.12 1.58 | 0 / 0 / 1 |
| c-end | 2026-10-15T17:05:26Z | 2.86 2.33 2.01 | 206 / 16861 / 763 |
| d-start | 2026-10-15T17:05:26Z | 2.86 2.33 2.01 | 0 / 0 / 1 |
| d-bench-end | 2026-10-15T17:07:06Z | 1.35 1.95 1.90 | 115 / 9891 / 4 |
| d-profile-end | 2026-10-15T17:22:16Z | 0.98 1.01 1.31 | 1327 / 87317 / 2193 |
| d-end | 2026-10-15T17:22:31Z | 0.98 1.01 1.30 | 17 / 1390 / 1 |
| e-start | 2026-10-15T17:22:31Z | 0.98 1.01 1.30 | 0 / 0 / 1 |
| e-end | 2026-10-15T17:23:26Z | 1.04 1.02 1.29 | 88 / 3715 / 1718 |
| t-start | 2026-10-15T17:23:26Z | 1.04 1.02 1.29 | 0 / 1 / 0 |
| t-end | 2026-10-15T17:24:03Z | 2.41 1.36 1.39 | 63 / 3089 / 517 |
| f-start | 2026-10-15T17:24:03Z | 2.41 1.36 1.39 | 0 / 1 / 0 |
| f-end | 2026-10-15T17:24:27Z | 1.93 1.33 1.37 | 24 / 1932 / 424 |
| noise-1-start | 2026-10-15T17:24:27Z | 1.93 1.33 1.37 | 0 / 0 / 1 |
| noise-1-end | 2026-10-15T17:26:52Z | 2.15 1.71 1.51 | 190 / 11068 / 3253 |
| noise-2-start | 2026-10-15T17:26:52Z | 2.15 1.71 1.51 | 0 / 0 / 0 |
| noise-2-end | 2026-10-15T17:29:13Z | 1.92 1.90 1.62 | 145 / 11139 / 2812 |
| noise-3-start | 2026-10-15T17:29:13Z | 1.92 1.90 1.62 | 0 / 0 / 0 |
| noise-3-end | 2026-10-15T17:31:44Z | 1.84 1.86 1.65 | 170 / 12027 / 2890 |
| chain-end | 2026-10-15T17:31:44Z | 1.84 1.86 1.65 | 0 / 0 / 1 |

- Steal on core 3 is 0.8–1.5% of each sweep's ticks (sweep 1: 643 of 53,179; sweep 2: 584
  of 47,554; sweep 3: 457 of 54,195; the cache sweep: 206 of 17,830; the per-file profiles:
  1,327 of 90,837), above M4's 0.4–1.0%. The pinned rows carry this steal.
- **The host is slower in this record's windows than in M4's, for every tool alike.** The
  same `base` binary on the same bytes reads 3.68–4.03 MB/s in `bench -n 5` against M4's
  4.96–5.09 (0.72–0.81x), 1280 ms on the cache copy against 909 (1.41x) and 8.00 s in
  `conform -i` against 5.70 (1.40x); rustfmt's `fx-hir-ty` pinned mean is 1271 ms against 906
  (1.40x), and sweep 1 took 8 min 54 s against M4's 6 min 18 s. Protocol §3 keeps every
  comparison inside one window, so the ratios below stand; the absolute milliseconds are not
  comparable with M4's, and the instruction counts, which do not depend on the window, are
  the figures to carry between records (the `base` count agrees with M4's to within 76 of
  587,989,243).
- The whole chain ran as one script from 16:32:33 to 17:31:44 UTC with no gap between
  steps; the by-hand steps (cache, in-process, parallel items, trace, cliffs) wrote their
  exports under `benchmarks/results/m5-*` like the harness sweeps.

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
  copies M4 prepared and verified on this disk (same bytes; the five stdin copies are fixed
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
| `m5-a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `m5-a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `m5-a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `m5-d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `m5-c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `m5-d-inproc` | by-hand `taskset -c 3 bench -n 5 --root <head fixtures>` triples, `benchmarks/profile.sh --runs 2 --core 3 --per-file` per binary on both samples, `--phase parse` for head and head-pgo, `taskset -c 3 conform -i` per build (and `-V 1.8.0` for head) |
| `m5-e-parallel-items` | by-hand hyperfine, measurement E |
| `m5-t-trace` | `diag.py` per crate: `head4` (head, 4 threads, 30 runs), `head1` (head, 1 thread on core 3, 10 runs), `pgo4` (head-pgo, 4 threads, 30 runs) |
| `m5-f-cliffs` | `cliffs/run.sh --shapes closures-expr --scales 1,2,4,8 --core 3` per binary: `<binary>.ir.tsv`, `<binary>.compare.tsv`, `<binary>.log` (the fit) |
| `m5-noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |
| `m5-identity` | `env.txt`, `sha256.txt`, `sizes.txt`, `fixed-point.txt`, `loads.tsv` (the per-sweep load and `/proc/stat` lines) |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`); the `base` column shows sweep 1's base. The `ranges` column lists,
  in order, head vs base, head-pgo vs base, head vs rustfmt, head-pgo vs rustfmt: `below` =
  every run of the first command took less wall time than every run of the second; `overlap`
  = the min–max ranges intersect.
- The same `base` binary ran in sweeps 1 and 2: see the `1c` crate bullets for its drift
  between the two windows.
- Output agreement in every crate, files and stdin workload, recomputed by the harness with
  this record's binaries (`agreement_chloro=<sha16>` in each copy's `meta`, the fix M4 owed):
  on formatted input `head` equals rustfmt on every file of every workload (15/15, 122/122,
  33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 287/287, 1/1 ×5) and on raw input the same
  with `tiny-test-data` 289/289; `base` differs on one raw file
  (`parser/err/0024_many_type_parens.rs`, the R2 fix) and on no formatted file. Exit codes
  are 0 for every command on formatted input, 1 for both tools on raw input where files
  change (`reg-clap-builder` and the stdin rows exit 0).

## Crate level, formatted input, `--check` (measurement A)

### Crate level, all CPUs (`mc`), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 102.2 / 146.4 ± 21.0 / 24 | 33.0 / 41.4 ± 4.7 / 17 | 32.8 / 40.2 ± 6.4 / 18 | 352.9 / 449.3 ± 82.8 / 57 | 367.0 / 431.0 ± 54.1 / 57 | 0.283 | 0.276 | 0.092 | 0.093 | below; below; below; below | 305.6 | 131.7 | 120.4 | 441.8 | 0.431 | 0.381 | 0.298 | 0.284 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 253.8 / 306.5 ± 28.1 / 36 | 81.4 / 104.2 ± 13.4 / 16 | 72.6 / 84.1 ± 7.6 / 17 | 1121.5 / 1262.2 ± 93.3 / 86 | 1099.7 / 1280.8 ± 118.8 / 86 | 0.340 | 0.267 | 0.083 | 0.066 | below; below; below; below | 974.1 | 379.3 | 306.8 | 1241.5 | 0.389 | 0.313 | 0.306 | 0.244 | b[0]h[0]r[0]p[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 142.5 / 183.2 ± 23.9 / 20 | 30.8 / 40.0 ± 6.8 / 15 | 31.1 / 40.3 ± 3.6 / 17 | 381.4 / 460.1 ± 52.2 / 58 | 381.2 / 445.5 ± 61.2 / 58 | 0.219 | 0.228 | 0.087 | 0.090 | below; below; below; below | 325.7 | 122.8 | 120.9 | 449.1 | 0.377 | 0.384 | 0.273 | 0.275 | b[0]h[0]r[0]p[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 45.8 / 72.0 ± 11.1 / 13 | 19.1 / 23.6 ± 3.3 / 8 | 17.6 / 20.6 ± 2.4 / 8 | 166.1 / 210.9 ± 30.9 / 51 | 168.8 / 201.6 ± 25.2 / 51 | 0.328 | 0.275 | 0.112 | 0.102 | below; below; below; below | 190.6 | 76.7 | 64.0 | 204.6 | 0.403 | 0.317 | 0.375 | 0.324 | b[0]h[0]r[0]p[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 35.0 / 44.4 ± 5.0 / 10 | 13.6 / 16.6 ± 2.7 / 6 | 12.1 / 14.4 ± 1.4 / 7 | 94.7 / 113.9 ± 19.5 / 47 | 96.3 / 118.6 ± 16.5 / 47 | 0.374 | 0.357 | 0.146 | 0.122 | below; below; below; below | 123.0 | 48.9 | 43.4 | 110.7 | 0.398 | 0.388 | 0.442 | 0.376 | b[0]h[0]r[0]p[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 22.3 / 26.9 ± 3.0 / 7 | 10.0 / 11.9 ± 0.8 / 5 | 8.5 / 10.7 ± 1.4 / 6 | 54.6 / 68.4 ± 11.5 / 43 | 54.8 / 72.3 ± 14.4 / 43 | 0.441 | 0.368 | 0.173 | 0.148 | below; below; below; below | 56.6 | 32.7 | 26.6 | 66.5 | 0.577 | 0.406 | 0.491 | 0.377 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 169.4 / 228.8 ± 27.4 / 38 | 72.0 / 84.6 ± 8.6 / 23 | 59.7 / 76.3 ± 8.6 / 25 | 570.0 / 696.5 ± 94.6 / 66 | 570.6 / 682.2 ± 58.1 / 66 | 0.370 | 0.355 | 0.121 | 0.112 | below; below; below; below | 683.8 | 285.4 | 252.2 | 685.8 | 0.417 | 0.377 | 0.416 | 0.376 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 84.6 / 103.6 ± 12.5 / 18 | 31.3 / 37.6 ± 4.6 / 12 | 29.8 / 34.0 ± 3.0 / 12 | 264.5 / 328.2 ± 48.0 / 55 | 246.6 / 310.7 ± 46.4 / 55 | 0.363 | 0.343 | 0.115 | 0.109 | below; below; below; below | 294.5 | 121.9 | 109.9 | 319.5 | 0.414 | 0.378 | 0.381 | 0.360 | b[0]h[0]r[0]p[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 89.5 / 107.6 ± 12.5 / 19 | 34.5 / 41.5 ± 4.0 / 12 | 29.2 / 33.4 ± 2.6 / 13 | 239.6 / 292.8 ± 36.7 / 59 | 239.3 / 286.2 ± 37.2 / 59 | 0.386 | 0.297 | 0.142 | 0.117 | below; below; below; below | 316.2 | 141.6 | 111.7 | 286.3 | 0.448 | 0.333 | 0.494 | 0.397 | b[0]h[0]r[0]p[0] |

- Unpinned, `head/base` in mean: 0.219 (`fx-ide-db`) to 0.441 (`reg-tokio`), M4 0.25–0.52;
  CPU 0.377–0.577. `head/rustfmt` 0.083 (`fx-hir-ty`) to 0.173 (`reg-tokio`), M4 0.092–0.179.
  Every head run is below every base run and every rustfmt run on all nine crates (M4: below
  base on eight of nine unpinned).
- `head-pgo/base` 0.228–0.368 and `head-pgo/rustfmt` 0.066–0.148 (M4 0.22–0.45 and
  0.072–0.175), every run below on all nine.
- Peak RSS of head on the nine crates unpinned: 5–23 MiB (M4 5–31; the 23 is
  `reg-regex-syntax`, whose 1.6 MB of files include four split ones) against base 7–38 and
  rustfmt 43–86.

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 238.7 / 298.7 ± 53.6 / 16 | 97.3 / 114.2 ± 25.5 / 9 | 84.8 / 99.3 ± 13.8 / 10 | 343.9 / 400.8 ± 58.4 / 57 | 347.9 / 415.0 ± 68.8 / 57 | 0.382 | 0.346 | 0.285 | 0.239 | below; below; below; below | 294.9 | 112.5 | 97.8 | 396.2 | 0.382 | 0.346 | 0.284 | 0.240 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 757.0 / 906.5 ± 97.5 / 18 | 293.2 / 380.6 ± 59.2 / 8 | 248.5 / 286.3 ± 25.6 / 8 | 1131.3 / 1271.2 ± 90.9 / 86 | 1077.2 / 1239.7 ± 104.8 / 86 | 0.420 | 0.323 | 0.299 | 0.231 | below; below; below; below | 890.9 | 373.7 | 281.7 | 1250.6 | 0.419 | 0.323 | 0.299 | 0.232 | b[0]h[0]r[0]p[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 262.7 / 358.2 ± 74.7 / 15 | 88.7 / 105.4 ± 18.9 / 9 | 79.4 / 90.3 ± 12.6 / 10 | 382.7 / 441.8 ± 47.9 / 58 | 382.8 / 430.1 ± 41.9 / 58 | 0.294 | 0.285 | 0.239 | 0.210 | below; below; below; below | 353.1 | 104.8 | 88.8 | 432.2 | 0.297 | 0.287 | 0.243 | 0.209 | b[0]h[0]r[0]p[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 145.3 / 170.8 ± 21.9 / 9 | 56.3 / 63.6 ± 6.4 / 6 | 48.9 / 67.2 ± 14.4 / 6 | 166.3 / 201.5 ± 31.0 / 51 | 165.4 / 216.7 ± 44.2 / 51 | 0.372 | 0.367 | 0.316 | 0.310 | below; below; below; below | 166.9 | 62.5 | 65.9 | 197.6 | 0.374 | 0.368 | 0.316 | 0.308 | b[0]h[0]r[0]p[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 81.5 / 99.0 ± 21.2 / 7 | 33.1 / 39.2 ± 7.0 / 5 | 29.5 / 33.7 ± 5.1 / 5 | 94.0 / 100.9 ± 7.7 / 47 | 92.8 / 114.1 ± 21.2 / 47 | 0.396 | 0.361 | 0.389 | 0.296 | below; below; below; below | 97.1 | 37.8 | 33.3 | 100.0 | 0.389 | 0.366 | 0.378 | 0.298 | b[0]h[0]r[0]p[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 41.6 / 45.2 ± 3.4 / 6 | 18.9 / 20.9 ± 1.9 / 4 | 17.2 / 19.6 ± 3.3 / 5 | 51.6 / 66.8 ± 14.0 / 43 | 51.4 / 59.0 ± 8.4 / 43 | 0.462 | 0.429 | 0.312 | 0.332 | below; below; below; below | 43.6 | 20.5 | 19.1 | 65.8 | 0.470 | 0.426 | 0.311 | 0.330 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 549.1 / 687.1 ± 115.4 / 29 | 221.4 / 270.9 ± 53.2 / 11 | 169.2 / 196.2 ± 24.8 / 11 | 569.5 / 675.5 ± 91.8 / 66 | 571.3 / 658.1 ± 73.9 / 66 | 0.394 | 0.300 | 0.401 | 0.298 | below; below; below; below | 673.3 | 266.3 | 192.8 | 669.0 | 0.396 | 0.299 | 0.398 | 0.296 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 210.3 / 243.7 ± 38.7 / 12 | 81.3 / 100.7 ± 16.7 / 6 | 72.8 / 84.9 ± 11.3 / 7 | 246.2 / 280.5 ± 36.7 / 56 | 240.9 / 303.5 ± 44.1 / 55 | 0.413 | 0.312 | 0.359 | 0.280 | below; below; below; below | 239.2 | 98.1 | 83.7 | 277.2 | 0.410 | 0.315 | 0.354 | 0.279 | b[0]h[0]r[0]p[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 239.6 / 276.9 ± 51.7 / 11 | 87.5 / 106.0 ± 22.0 / 7 | 79.9 / 88.0 ± 11.3 / 8 | 229.5 / 261.8 ± 28.4 / 59 | 228.9 / 294.4 ± 50.4 / 59 | 0.383 | 0.305 | 0.405 | 0.299 | below; below; below; below | 274.4 | 104.5 | 86.3 | 258.4 | 0.381 | 0.303 | 0.404 | 0.296 | b[0]h[0]r[0]p[0] |

- Pinned, `head/base` in mean: 0.294 (`fx-ide-db`) to 0.462 (`reg-tokio`), M4 0.38–0.60; CPU
  0.297–0.470 (M4 0.38–0.59). `head/rustfmt` 0.239 (`fx-ide-db`) to 0.405 (`reg-winnow`), M4
  0.26–0.44. `head-pgo/base` 0.285–0.429 (M4 0.36–0.43) and `head-pgo/rustfmt` 0.210–0.332
  (M4 0.24–0.38). Every head and head-pgo run is below every base run and every rustfmt run
  on all nine crates.
- `base` between sweeps 1 and 2 (mean ratio, the same binary and bytes): 0.883x
  (`fx-ide-db`) to 1.115x (`reg-clap-builder`) on the pinned rows and 0.909x–1.078x unpinned
  (M4: 0.960x–1.264x pinned); `pgo/base` and `pgo/rustfmt` are computed against sweep 2's
  own base and rustfmt runs.
- Peak RSS of head pinned: 4–11 MiB (M4 4–14) against base 6–29 and rustfmt 43–86.
- Spread within a cell: σ/mean of the pinned head rows is 5.3% (`reg-winnow`'s min-to-mean
  is wider) to 22% (`fx-hir`, 25.5 of 114.2 ms, one slow tail of 186 ms); rustfmt's 7–21%;
  base's 7.5–21%. `fx-hir-ty` head pinned reads 293.2–531.1 ms (1.81x max/min).

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 16.7 / 24.3 ± 6.8 / 5 | 7.8 / 10.0 ± 1.6 / 5 | 6.6 / 9.0 ± 1.7 / 5 | 70.2 / 89.9 ± 17.4 / 44 | 68.3 / 86.3 ± 11.4 / 44 | 0.411 | 0.389 | 0.111 | 0.104 | below; below; below; below | 43.4 | 21.6 | 20.1 | 85.4 | 0.498 | 0.456 | 0.253 | 0.239 | b[0]h[0]r[0]p[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 28.7 / 34.7 ± 8.1 / 5 | 14.6 / 15.6 ± 1.5 / 5 | 14.5 / 18.8 ± 3.2 / 5 | 71.5 / 86.1 ± 15.0 / 44 | 68.0 / 77.9 ± 11.7 / 44 | 0.450 | 0.561 | 0.181 | 0.242 | below; below; below; below | 34.2 | 15.3 | 18.5 | 84.3 | 0.447 | 0.559 | 0.182 | 0.241 | b[0]h[0]r[0]p[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 2.1 / 2.6 ± 0.3 / 4 | 2.0 / 2.8 ± 1.3 / 4 | 2.3 / 3.4 ± 0.8 / 4 | 13.9 / 16.2 ± 2.0 / 34 | 14.7 / 17.8 ± 2.2 / 34 | 1.067 | 0.888 | 0.175 | 0.188 | overlap; overlap; below; below | 2.4 | 2.5 | 3.0 | 15.6 | 1.014 | 0.916 | 0.158 | 0.173 | b[0]h[0]r[0]p[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.8 / 2.0 ± 0.1 / 4 | 1.8 / 1.9 ± 0.1 / 4 | 1.9 / 2.4 ± 0.7 / 4 | 13.1 / 14.3 ± 1.1 / 34 | 14.1 / 17.0 ± 3.0 / 34 | 0.957 | 0.877 | 0.131 | 0.141 | overlap; overlap; below; below | 1.8 | 1.7 | 2.1 | 13.8 | 0.958 | 1.007 | 0.123 | 0.128 | b[0]h[0]r[0]p[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 103.6 / 109.5 ± 7.8 / 10 | 30.9 / 34.1 ± 5.1 / 8 | – | 167.1 / 191.5 ± 21.8 / 43 | – | 0.312 | – | 0.178 | – | below; –; below; – | 109.0 | 33.7 | – | 188.4 | 0.310 | – | 0.179 | – | b[0]h[0]r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 107.0 / 131.8 ± 26.6 / 14 | 53.3 / 66.7 ± 11.6 / 14 | – | 118.8 / 134.2 ± 19.7 / 53 | – | 0.506 | – | 0.497 | – | below; –; below; – | 130.2 | 65.9 | – | 132.7 | 0.506 | – | 0.497 | – | b[0]h[0]r[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 82.6 / 96.0 ± 16.7 / 9 | 36.9 / 40.9 ± 3.3 / 8 | – | 108.5 / 127.1 ± 21.7 / 47 | – | 0.426 | – | 0.322 | – | below; –; below; – | 94.1 | 40.2 | – | 124.1 | 0.427 | – | 0.324 | – | b[0]h[0]r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 22.3 / 25.5 ± 4.3 / 5 | 8.8 / 11.0 ± 2.3 / 4 | – | 38.8 / 42.5 ± 4.9 / 40 | – | 0.430 | – | 0.258 | – | below; –; below; – | 24.6 | 10.6 | – | 41.9 | 0.432 | – | 0.254 | – | b[0]h[0]r[0] |
| stdin-config | 1c (3) | 1 | 169 | 30 | 32.5 / 36.6 ± 4.7 / 6 | 14.6 / 16.5 ± 1.9 / 5 | – | 52.1 / 65.8 ± 13.3 / 44 | – | 0.452 | – | 0.251 | – | below; –; below; – | 35.9 | 15.9 | – | 64.6 | 0.442 | – | 0.246 | – | b[0]h[0]r[0] |

- The 287-file invocation (`parser/test_data`, 67 KB in all): `head/base` 0.411 unpinned and
  0.450 pinned (M4 0.442 and 0.565); `head/rustfmt` 0.111 and 0.181 (M4 0.13 and 0.24);
  every head and head-pgo run below every base and rustfmt run.
- Start-up on the 13-byte file: head 1.9 ms pinned (base 2.0, rustfmt 14.3) and 2.8 ms
  unpinned in mean (base 2.6, rustfmt 16.2; head's range 2.0–9.0 ms holds one outlier, its
  minimum is below base's); head-pgo 2.4 and 3.4 ms. Head against base overlaps on this row
  in every record; both are below every rustfmt run.
- One file on stdin (`fmt_stdin`, pinned): `head/base` 0.312 (`stdin-lints`, 640 KB) to 0.506
  (`stdin-nodes`), M4 0.35–0.56; `head/rustfmt` 0.178 to 0.497 (`stdin-nodes`, whose 358 KB of
  generated code rustfmt formats in 134 ms); every head run below every base and rustfmt run
  on all five. No PGO build of `fmt_stdin`, as in M4.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 625 | 30 | 689.1 / 857.2 ± 115.7 / 218 | 33.7 / 43.0 ± 4.8 / 17 | 0.050 | below | 846.2 | 139.9 | 0.165 | 15/15 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 675.6 / 774.8 ± 74.2 / 218 | 95.8 / 114.6 ± 22.0 / 9 | 0.148 | below | 764.3 | 113.1 | 0.148 | 15/15 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1572.5 / 1773.1 ± 139.8 / 117 | 81.5 / 87.9 ± 8.0 / 16 | 0.050 | below | 1745.1 | 327.1 | 0.187 | 122/122 | r[1] h[1] |
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1601.7 / 1738.2 ± 113.4 / 117 | 289.1 / 349.7 ± 48.5 / 8 | 0.201 | below | 1718.1 | 345.5 | 0.201 | 122/122 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 441.5 / 533.6 ± 82.6 / 66 | 33.7 / 43.3 ± 5.1 / 15 | 0.081 | below | 524.5 | 131.8 | 0.251 | 33/33 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 429.9 / 522.5 ± 58.9 / 66 | 96.2 / 129.8 ± 19.0 / 9 | 0.248 | below | 514.8 | 127.8 | 0.248 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 220.1 / 263.0 ± 39.2 / 57 | 19.5 / 23.3 ± 2.3 / 8 | 0.089 | below | 259.2 | 77.7 | 0.300 | 33/33 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 215.6 / 241.1 ± 19.9 / 57 | 52.3 / 62.7 ± 11.0 / 5 | 0.260 | below | 236.2 | 61.0 | 0.258 | 33/33 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 106.1 / 125.2 ± 19.0 / 48 | 14.3 / 15.6 ± 1.2 / 6 | 0.125 | below | 122.7 | 47.6 | 0.387 | 28/28 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 103.3 / 114.0 ± 9.8 / 48 | 32.2 / 40.9 ± 10.2 / 5 | 0.359 | below | 111.6 | 40.1 | 0.360 | 28/28 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 53.2 / 66.5 ± 12.8 / 43 | 8.2 / 10.6 ± 2.6 / 5 | 0.159 | below | 65.0 | 26.0 | 0.400 | 49/49 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 52.7 / 60.5 ± 8.1 / 43 | 19.1 / 20.4 ± 1.5 / 4 | 0.337 | below | 59.3 | 19.8 | 0.333 | 49/49 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1805.4 / 2096.1 ± 179.0 / 423 | 78.6 / 98.7 ± 33.1 / 23 | 0.047 | below | 2071.2 | 305.5 | 0.147 | 33/33 | r[1] h[1] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1823.0 / 2146.8 ± 255.8 / 423 | 205.4 / 232.8 ± 25.6 / 11 | 0.108 | below | 2125.3 | 229.9 | 0.108 | 33/33 | r[1] h[1] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 272.4 / 319.1 ± 49.8 / 56 | 31.1 / 41.4 ± 6.6 / 11 | 0.130 | below | 314.3 | 132.3 | 0.421 | 57/57 | r[0] h[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 244.9 / 311.1 ± 56.6 / 55 | 82.3 / 96.9 ± 15.1 / 6 | 0.311 | below | 308.3 | 95.3 | 0.309 | 57/57 | r[0] h[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 254.2 / 305.9 ± 50.1 / 60 | 30.2 / 34.3 ± 3.3 / 11 | 0.112 | below | 301.6 | 118.2 | 0.392 | 61/61 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 249.8 / 310.9 ± 56.2 / 60 | 89.3 / 118.3 ± 19.4 / 7 | 0.381 | below | 306.6 | 116.8 | 0.381 | 61/61 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 94.2 / 119.8 ± 18.2 / 44 | 8.1 / 9.5 ± 0.8 / 5 | 0.079 | below | 117.3 | 22.1 | 0.189 | 289/289 | r[1] h[1] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 84.6 / 109.3 ± 27.3 / 44 | 14.9 / 16.9 ± 2.4 / 4 | 0.155 | below | 107.9 | 16.4 | 0.152 | 289/289 | r[1] h[1] |
| tiny-one | mc (none) | 1 | 0 | 30 | 14.7 / 17.1 ± 2.0 / 34 | 2.3 / 3.1 ± 0.6 / 4 | 0.184 | below | 16.5 | 2.8 | 0.172 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 13.6 / 16.2 ± 2.1 / 34 | 1.8 / 2.5 ± 0.3 / 4 | 0.153 | below | 15.7 | 2.3 | 0.144 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 167.3 / 217.4 ± 37.6 / 43 | 32.5 / 40.1 ± 6.3 / 8 | 0.185 | below | 214.5 | 39.1 | 0.182 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 117.8 / 155.3 ± 25.1 / 53 | 51.3 / 67.5 ± 15.1 / 14 | 0.435 | below | 153.2 | 66.7 | 0.436 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 116.0 / 134.7 ± 17.7 / 47 | 39.3 / 42.9 ± 3.7 / 8 | 0.319 | below | 133.0 | 42.4 | 0.319 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 40.3 / 51.0 ± 9.5 / 40 | 9.2 / 10.7 ± 1.6 / 4 | 0.209 | below | 49.8 | 10.1 | 0.203 | 1/1 | r[0] h[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 57.3 / 73.3 ± 13.9 / 44 | 15.6 / 16.8 ± 0.9 / 5 | 0.230 | below | 71.1 | 16.4 | 0.231 | 1/1 | r[0] h[0] |

- On input as found, `head/rustfmt` reads 0.047 (`reg-regex-syntax`) to 0.159 (`reg-tokio`)
  unpinned and 0.108 to 0.401 pinned on the crates, 0.079 and 0.155 on the 287 files, 0.185–0.435
  on the stdin files; rustfmt's peak RSS rises to 117–423 MiB on the crates whose files change
  (it builds a diff per changed file). Every head run is below every rustfmt run on every
  row.
- Outputs identical to rustfmt's on every file of every raw workload (`tiny-test-data`
  289/289). Exit codes 1 for both tools where files change, 0 for both on `reg-clap-builder`,
  `tiny-one` and the stdin rows.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 13,901,559 bytes)
  copied to `/tmp/chloro-bench/m5-cache/tree` and rewritten once by `head --write --no-cache`;
  `head --check --no-cache` and `base --check` on the result exit 0. `CHLORO_CACHE_DIR` points
  at a scratch directory, which after a priming run holds the cache file
  `d004ad25b03eb62ecffb8f22fbe2c31d.chloro-cache` of 44,184 bytes (the name is the key of this
  binary and configuration; M4's build wrote `3d84c0c3…` of the same size) and the executable's
  identity record `51af18f6….chloro-exe` of 180 bytes. `head -v --check` on the copy reports
  `(1226 entries)` and `Unit cache: off`; the 1238 fixtures hold 1226 distinct contents by
  sha256 (as in M4). The `head-pgo` binary writes its own pair (`9b3f2c2a….chloro-cache`,
  44,184 bytes; a 184-byte `.chloro-exe`).
- Commands: `hyperfine -N -i -w 1 -r 15 --metrics … -n cold --prepare "rm -rf $C" "head
  --check $T" -n warm --prepare true "head --check $T" -n no-cache --prepare true "head --check
  --no-cache $T" -n base --prepare true "base --check $T"`, two rounds with the order reversed
  (n = 30); then one round under `taskset -c 3` for `warm`, `no-cache`, `base` (n = 15); then
  one unpinned round for `head-pgo` (`pgo-warm`, `pgo-no-cache`, n = 15, its own cache
  directory). `cold` is not in the pinned round.

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 418.8 / 475.4 ± 41.6 / 41 | 13.8 / 17.8 ± 3.1 / 5 | 380.3 / 447.4 ± 43.7 / 38 | 1139.7 / 1280.3 ± 102.6 / 52 | 25.2 | 72.1 | 0.349 | 1778.6 | 34.3 | 1660.7 | 4431.5 |
| 1c | 15 | – | 26.9 / 29.7 ± 2.6 / 4 | 1405.2 / 1665.6 ± 178.8 / 22 | 3788.7 / 4285.8 ± 331.1 / 33 | 56.1 | 144.3 | 0.389 | – | 28.3 | 1646.3 | 4240.2 |

| variant | n | pgo-warm | pgo-no-cache | no-cache/warm | CPU pgo-warm | CPU pgo-no-cache |
|---|---|---|---|---|---|---|
| mc | 15 | 16.2 / 19.7 ± 2.1 / 6 | 353.5 / 400.0 ± 23.5 / 42 | 20.3 | 37.9 | 1476.2 |

- Warm `--check` of the 1238 formatted files takes 17.8 ms unpinned (25.2x below `--no-cache`,
  72.1x below base) and 29.7 ms on one core (56.1x below `--no-cache`, 144.3x below base); CPU
  34.3 and 28.3 ms; peak RSS 5 and 4 MiB against 38 and 22 MiB without the cache. M4 measured
  16.9 and 24.4 ms on the same copy's bytes with the same cache code (E12's): the warm path
  is hashing 13.9 MB and process start-up, and this window's host is slower (Load).
- `cold` (cache directory removed before every run) reads 475.4 ms against 447.4 ms for
  `no-cache` in mean (1.063x; ranges 418.8–608.6 vs 380.3–575.4 ms overlap): hashing the
  files and the binary and writing the cache and the identity record add 28 ms in mean over
  the formatter on 4 threads in this window (M4: 16 ms).
- `no-cache` head against base on the same copy: 0.349 unpinned (447.4 vs 1280.3 ms) and 0.389
  pinned (1665.6 vs 4285.8 ms); CPU 0.375 and 0.388. M4 read 0.410 and 0.425, M3 0.518 and
  0.536. Peak RSS of `cold` (41 MiB) and `no-cache` (38 MiB) is below base's 52 MiB unpinned;
  pinned, 22 MiB against 33 MiB.
- The profile-guided build: warm 19.7 ms unpinned (CPU 37.9 ms), `--no-cache` 400.0 ms (0.894x
  head's 447.4 ms in the earlier round of the same sweep; peak RSS 42 against 38 MiB).
- The base means in this sweep (1280.3 ms unpinned, 4285.8 pinned) are 1.41x and 1.40x M4's
  (909.4, 3069.7) for the same binary and bytes: the window (Load). The ratios are
  within-window.

## In process and instructions (measurement D)

`taskset -c 3 bench -n 5 --root chloro-core/tests/conformance/fixtures` (best of 5 in-process
formats of the 1238 fixtures, 13.9 MB; the same tree for all three binaries), three
interleaved triples in the order base, head, head-pgo:

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 3.781 | 3.68 |
| 1 | head | 1238 | 13.9 | 1.490 | 9.33 |
| 1 | head-pgo | 1238 | 13.9 | 1.247 | 11.15 |
| 2 | base | 1238 | 13.9 | 3.572 | 3.89 |
| 2 | head | 1238 | 13.9 | 1.339 | 10.38 |
| 2 | head-pgo | 1238 | 13.9 | 1.159 | 12.00 |
| 3 | base | 1238 | 13.9 | 3.446 | 4.03 |
| 3 | head | 1238 | 13.9 | 1.382 | 10.06 |
| 3 | head-pgo | 1238 | 13.9 | 1.107 | 12.56 |

- base: 3 runs, best time 3.446–3.781 s, 3.68–4.03 MB/s
- head: 3 runs, best time 1.339–1.490 s, 9.33–10.38 MB/s
- head-pgo: 3 runs, best time 1.107–1.247 s, 11.15–12.56 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, each binary's
compiled-in fixture root, `chloro` = head, `chloro-base` = base:

- chloro: 1238 files, 13.9 MB: 1.318s, 10.54 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 3.941s, 3.53 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 3.850s, 3.61 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.443s, 9.63 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.635s, 8.50 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 4.294s, 3.24 MB/s (best of 5)

### Instructions on both samples (`benchmarks/profile.sh --runs 2 --core 3 --per-file`)

Whole = the sample formatted by one `bench` process (the ledger's figure); per file = each
file in a `bench` process of its own, the minimum of 2 runs per file, summed, net of the
empty-sample process (316,079 instructions for head, 318,244 for head-pgo, 328,886 for base);
cross-file = whole net of the empty process minus per file (the heap layout one file leaves
the next, EXP-020). The registry sample is `benchmarks/samples/registry49.txt` (E27).

| sample | binary | whole, run 1 | whole, run 2 | per file (net) | cross-file | whole / base | whole / head |
|---|---|---|---|---|---|---|---|
| fixtures | base | 587,989,319 | 587,989,329 | 581,626,432 | +6,034,001 | 1.0000 | |
| fixtures | head | 220,958,799 | 220,957,089 | 220,754,884 | −113,874 | 0.3758 | 1.0000 |
| fixtures | head-pgo | 182,602,721 | 182,602,785 | 182,500,927 | −216,450 | 0.3106 | 0.8264 |
| registry | base | 688,589,271 | 688,591,126 | 681,815,299 | +6,445,086 | 1.0000 | |
| registry | head | 299,933,543 | 299,935,266 | 298,836,329 | +781,135 | 0.4356 | 1.0000 |
| registry | head-pgo | 252,010,500 | 252,011,837 | 251,058,976 | +633,280 | 0.3660 | 0.8402 |

### The parse-only phase (`--phase parse`, head and head-pgo; `base`'s `bench` has no phase option)

| sample | binary | whole, run 1 | run 2 | per file (net) | parse net of the empty process / whole net |
|---|---|---|---|---|---|
| fixtures | head | 86,532,466 | 86,533,457 | 87,406,979 | 86,215,096 / 220,641,010 = **39.1%** |
| fixtures | head-pgo | 72,731,413 | 72,733,154 | 73,597,422 | 72,411,855 / 182,284,477 = 39.7% |
| registry | head | 139,892,728 | 139,894,541 | 140,273,760 | 139,575,358 / 299,617,464 = **46.6%** |
| registry | head-pgo | 117,497,531 | 117,499,047 | 117,866,940 | 117,177,973 / 251,692,256 = 46.6% |

- `bench -n 5`, three interleaved triples: base 3.68–4.03 MB/s, head 9.33–10.38 MB/s, head-pgo
  11.15–12.56 MB/s. The three ranges are pairwise disjoint (protocol §3 rule for a
  single-threaded change: 3 of 3 pairs, non-overlapping); head/base 2.50x–2.67x on the
  per-round MB/s (M4 2.20x–2.38x), head-pgo/head 1.16x–1.25x (M4 1.11x–1.16x). The harness's
  own three pairs (same core, 16:58–17:02 UTC) give base 3.24–3.61 and head 8.50–10.54 MB/s,
  non-overlapping. Base reads 3.68–4.03 against M4's 4.96–5.09 on the same bytes and binary
  (Load), so the MB/s are this window's; the instruction ratios below do not depend on it.
- `conform -i` (hyperfine, 9 runs each, core 3, harness): head 3374.6 ± 342.3 ms
  (2987.1–4117.4) against base 8283.3 ± 325.7 ms (7787.5–8672.3), 0.407, every head run below
  every base run; CPU 3335.6 vs 8176.6 ms; peak RSS 48 vs 51 MiB; exit 0 on every run. By
  hand (`taskset -c 3 conform -i`, one run each): head **1220/1220 identical**, 0 not
  idempotent, `rustfmt_version=1.9.0`, diff lines +0 −0, 13.8 MB in 1.516 s (9.1 MB/s),
  3.111 s wall (user 3.006 s); at `-V 1.8.0` **1220/1220 identical**, 0 not idempotent, 13.8 MB
  in 1.453 s (9.5 MB/s), 2.957 s wall; base 1219/1220 identical, 0 not idempotent, diff lines
  +2 −2, 13.8 MB in 3.951 s (3.5 MB/s), 8.003 s wall (0.389 head/base; M4 0.454).
- `Ir` on the fixture sample (650,213 bytes): base 587,989,319, head 220,957,089 (**−62.42%**),
  head-pgo 182,602,721 (−68.94%; 0.8264x head); two runs of each agree to within 10 (base),
  1,710 (head) and 64 (head-pgo). The base count equals M4's run 1 (587,989,243) to within 76,
  M3's to within 120 and M2's 587,989,243 to within 76; the head count's run 2 is 1,560 below
  the 220,958,649 the ledger records for `e96b530` (the same code). The profile-guided ratio
  is this training run's: M3's PGO read 0.7825x its head, M4's 0.8337x, here 0.8264x.
- Per file, net of the empty process, the fixture sample reads 581,626,432 (base),
  220,754,884 (head, 0.3795x) and 182,500,927 (head-pgo, 0.3138x); the cross-file term is
  +6.03 M for base (1.0% of its whole) and −0.11 M and −0.22 M for the two heads (E20's heap
  layout term: the whole-sample figure of the heads is now slightly *below* their per-file
  sum, so no part of the heads' whole-sample gain is layout luck).
- The registry validation sample (49 files of tokio, regex-syntax, clap_builder, winnow):
  base 688,589,271, head 299,933,543 (**−56.44%**; 0.4356x), head-pgo 252,010,500 (−63.40%;
  0.8402x head); per file 681,815,299 / 298,836,329 (0.4383x) / 251,058,976. The programme's
  gain on the registry sample is smaller than on the fixture sample (0.4356 against 0.3758):
  the registry files are larger and plainer (fewer nested closures and chains, longer `impl`
  blocks), so the parser is a larger share of them.
- The parse-only phase is **39.1%** of head's instructions on the fixture sample and **46.6%**
  on the registry sample (E27 measured 37.3% and about 43% on the code before E26 and E25:
  the formatter shrank, the parser did not). PGO cuts the parse phase by the same factor as
  the whole (0.8405x and 0.8399x against 0.8264x and 0.8402x), so the profile-guided build
  leaves the share unchanged (39.7% and 46.6%). The unmined-tricks plan's parser-fork
  trigger (T9: 40% of the instructions) is crossed on the registry sample and reached on the
  fixture sample.
- The largest per-file counts on the fixture sample are `handlers/notification.rs`
  (13,042,858 net for 22,455 bytes) and `slow-tests/ratoml.rs` (8,599,680 for 22,465 bytes):
  the same bytes cost 1.5x more in the first (deeply nested method chains and closures), the
  shape the cliff section measures.

## The schedule timeline on a quiet host (measurement T, E23 follow-up)

`benchmarks/schedule/diag.py` on the released binaries' own `CHLORO_TRACE` output
(`chloro/src/cli/trace.rs`, wall-clock stamps), `--check --no-cache` on the nine crate copies,
30 runs per crate at 4 threads unpinned for `head` and `head-pgo`, 10 runs at 1 thread on
core 3 for `head`; each row is the median over runs of a quantity derived per run. This is
the wall view EXP-023 asked to be read on a quiet host; its reconstruction from thread CPU
clocks needs the patched build (`trace_patch.py`) and is not rerun here.

**head, 4 threads, unpinned, 30 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 37.4 | 121.8 | 33.1 | 32.9 | 109.4 | 0.831 | 7.14 | 0.224 | 236 | 1 | 19.0 | 1.7 | 2.08 | 107.3 |
| fx-hir-ty | 122 | 90.9 | 335.4 | 86.5 | 85.5 | 334.9 | 0.988 | 0.28 | 0.003 | 1 | 1 | 1.0 | 2.2 | 5.29 | 328.7 |
| fx-ide-db | 33 | 39.6 | 125.5 | 36.1 | 35.8 | 126.8 | 0.884 | 5.07 | 0.140 | 640 | 1 | 14.7 | 2.0 | 2.86 | 123.0 |
| fx-syntax | 33 | 21.9 | 70.5 | 18.9 | 18.6 | 69.1 | 0.953 | 0.33 | 0.018 | 1 | 0 | 1.2 | 1.8 | 1.52 | 67.5 |
| fx-parser | 28 | 14.9 | 46.6 | 12.1 | 11.9 | 44.2 | 0.930 | 0.45 | 0.037 | 2 | 0 | 1.3 | 1.7 | 0.97 | 43.0 |
| reg-tokio | 49 | 11.3 | 29.3 | 8.0 | 7.5 | 25.1 | 0.886 | 0.28 | 0.034 | 0 | 0 | 1.0 | 2.0 | 1.31 | 23.7 |
| reg-regex-syntax | 33 | 84.4 | 283.6 | 79.6 | 79.2 | 305.7 | 0.984 | 0.66 | 0.007 | 11 | 4 | 2.0 | 2.4 | 5.47 | 299.6 |
| reg-clap-builder | 57 | 41.6 | 129.6 | 36.6 | 36.1 | 138.5 | 0.971 | 0.32 | 0.009 | 1 | 2 | 1.2 | 2.4 | 3.20 | 134.4 |
| reg-winnow | 61 | 37.3 | 129.1 | 33.1 | 32.6 | 125.3 | 0.969 | 0.39 | 0.011 | 2 | 0 | 1.3 | 2.5 | 3.06 | 122.8 |

**head-pgo, 4 threads, unpinned, 30 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 42.3 | 135.4 | 36.8 | 36.6 | 125.5 | 0.857 | 8.23 | 0.234 | 236 | 1 | 17.5 | 2.2 | 2.58 | 122.7 |
| fx-hir-ty | 122 | 99.5 | 363.3 | 94.5 | 93.4 | 368.5 | 0.988 | 0.35 | 0.004 | 1 | 1 | 1.3 | 2.5 | 6.21 | 362.4 |
| fx-ide-db | 33 | 37.1 | 109.9 | 33.0 | 32.3 | 102.3 | 0.810 | 7.81 | 0.221 | 640 | 1 | 23.2 | 2.1 | 3.15 | 98.7 |
| fx-syntax | 33 | 22.9 | 70.0 | 19.1 | 18.8 | 70.3 | 0.945 | 0.33 | 0.018 | 1 | 0 | 1.2 | 2.5 | 1.61 | 68.6 |
| fx-parser | 28 | 14.2 | 42.8 | 10.9 | 10.6 | 39.3 | 0.926 | 0.41 | 0.035 | 2 | 0 | 1.2 | 1.5 | 0.84 | 38.5 |
| reg-tokio | 49 | 9.9 | 26.6 | 6.9 | 6.5 | 22.6 | 0.884 | 0.19 | 0.028 | 0 | 0 | 0.7 | 1.9 | 1.15 | 21.4 |
| reg-regex-syntax | 33 | 78.5 | 253.1 | 71.5 | 71.1 | 280.7 | 0.983 | 0.54 | 0.008 | 4 | 4 | 1.8 | 2.1 | 5.31 | 272.7 |
| reg-clap-builder | 57 | 34.4 | 108.5 | 30.1 | 29.6 | 114.7 | 0.963 | 0.32 | 0.011 | 0 | 2 | 1.2 | 2.3 | 3.62 | 110.6 |
| reg-winnow | 61 | 29.6 | 99.7 | 26.1 | 25.6 | 98.4 | 0.966 | 0.30 | 0.012 | 2 | 0 | 1.1 | 2.0 | 2.57 | 95.2 |

**head, 1 thread on core 3, 10 runs** (medians over runs; ms; `T` = process span from `main` to the last mark, `F` = format window from spawn to join, `busy` = sum of the files' spans over all workers, `util` = mean worker utilisation over `F`, `tail` = `F`'s end minus the first worker's exit, `owner` = the file still running at that moment, `lost tail` = idle core-ms during the tail)

| crate | files | wall | CPU | T | F | busy | util | tail | tail / T | owner KB | split files | lost tail (core-ms) | lost ramp | read | fmt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 106.5 | 103.3 | 101.4 | 101.2 | 100.9 | 0.997 | 0.18 | 0.002 | 0 | 0 | 0.2 | 0.0 | 0.93 | 99.9 |
| fx-hir-ty | 122 | 311.5 | 309.0 | 306.9 | 306.1 | 305.6 | 0.998 | 0.16 | 0.001 | 0 | 0 | 0.2 | 0.0 | 4.42 | 300.7 |
| fx-ide-db | 33 | 95.8 | 95.0 | 90.5 | 90.2 | 89.9 | 0.997 | 0.15 | 0.002 | 0 | 0 | 0.2 | 0.0 | 1.56 | 88.1 |
| fx-syntax | 33 | 57.4 | 57.0 | 52.5 | 52.2 | 51.9 | 0.994 | 0.14 | 0.003 | 0 | 0 | 0.1 | 0.0 | 0.89 | 50.9 |
| fx-parser | 28 | 54.0 | 53.5 | 49.0 | 48.7 | 48.3 | 0.992 | 0.16 | 0.003 | 0 | 0 | 0.2 | 0.0 | 0.96 | 47.3 |
| reg-tokio | 49 | 23.2 | 22.3 | 18.3 | 17.9 | 17.6 | 0.985 | 0.13 | 0.007 | 0 | 0 | 0.1 | 0.0 | 0.76 | 16.8 |
| reg-regex-syntax | 33 | 240.7 | 238.4 | 234.1 | 233.7 | 233.3 | 0.998 | 0.18 | 0.001 | 0 | 0 | 0.2 | 0.0 | 3.05 | 229.9 |
| reg-clap-builder | 57 | 129.0 | 125.9 | 123.3 | 122.9 | 122.4 | 0.996 | 0.19 | 0.002 | 0 | 0 | 0.2 | 0.0 | 2.17 | 120.0 |
| reg-winnow | 61 | 144.4 | 131.4 | 132.3 | 131.8 | 131.3 | 0.997 | 0.20 | 0.001 | 0 | 0 | 0.2 | 0.0 | 2.17 | 128.9 |

- At 4 threads, seven of the nine crates keep every core busy to within 0.3–0.7 ms of the
  end (tail 0.3–3.7% of the run, worker utilisation 0.886–0.988). Two do not: `fx-hir`'s
  tail is 7.14 ms (22.4% of its 33.1 ms; 19.0 idle core-ms) and `fx-ide-db`'s 5.07 ms (14.0%;
  14.7 idle core-ms), and in both the file still running when the first worker goes idle is
  the crate's one split file (`hir/src/lib.rs`, 236 KB; `ide-db`'s 640 KB `lints.rs`). The
  split spreads a file's top-level items over the four threads, but the file's own
  sequential parts (its reading, parsing, the text between items and the stitching) and its
  largest items run on one; with few other files left to fill the other cores, that
  remainder is the tail. EXP-023's unit queue across files was stopped because its
  simulated tail was at most 12.9%; these two crates measure above that bound on the quiet
  host, so the lever that remains is inside the split file (sub-item units, or parsing the
  split file's items in parallel with the other files), not between files.
- The profile-guided build shows the same two tails (23.4% and 22.1%; `fx-ide-db`'s split
  file finishes later relative to the others, 7.81 ms) and otherwise the same shape; its
  format windows are 0.80–0.95x head's.
- At 1 thread the format window is 7.5–307 ms with utilisation 0.985–0.998 and no tail (the
  worker is alone); the 4-thread wall is 2.05x (`reg-tokio`, 23.2 → 11.3 ms, whose 8 ms of
  formatting is of the order of the 3–4 ms the process spends outside the format window)
  to 3.87x (`reg-winnow`, 144.4 → 37.3 ms) below the 1-thread wall; `fx-hir-ty` 3.43x,
  `reg-regex-syntax` 2.85x, `fx-hir` 2.85x, `fx-ide-db` 2.42x. The process outside the
  format window (arguments, walk, sort, spawn, join, output) is 3–5 ms in every row.
- Reading the files is 1–6 ms of CPU per crate; formatting 24–363 ms; hashing 0 (no cache);
  output 0.1–0.4 ms.

## The nested-closure cliff on the three builds (measurement F)

`benchmarks/cliffs/run.sh --shapes closures-expr --scales 1,2,4,8 --core 3` per binary:
eight copies of an expression closure nested to depth 2, 4, 8 and 16 (`gen.py`), each file
formatted once by `bench` under callgrind (`env -i`, `setarch -R`, pinned), the output
compared with rustfmt's through `fmt_stdin`.

| depth (scale) | input bytes | output bytes | base Ir | head Ir | head-pgo Ir | head / base | pgo / head |
|---|---|---|---|---|---|---|---|
| 2 (1) | 440 | 472 | 1,823,147 | 976,535 | 845,682 | 0.5356 | 0.8660 |
| 4 (2) | 648 | 680 | 2,756,453 | 1,385,856 | 1,170,952 | 0.5028 | 0.8449 |
| 8 (4) | 1064 | 1672 | 34,886,103 | 5,570,698 | 4,501,065 | 0.1597 | 0.8080 |
| 16 (8) | 2000 | 10672 | 242,459,629 | 5,691,015 | 4,894,907 | 0.0235 | 0.8601 |

Empty file (the fixed cost of a `bench` process): base 340,400, head 324,816, head-pgo 328,811.
- `base` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	1823147 2756453 34886103 242459629	2.483	2.589	3.587	1.664	3.609	1489335	1.476	0.704 3.838 2.809`; output identical to rustfmt's on 4 of 4 files.
- `head` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	976535 1385856 5570698 5691015	0.964	1.143	1.529	0.659	1.046	237871	0.301	0.703 2.306 0.033`; output identical to rustfmt's on 4 of 4 files.
- `head-pgo` fit (`gen.py fit`: shape, Ir by scale, k_raw, k_net, k_bytes, k_out, p, a, p_out, local slopes): `closures-expr	845682 1170952 4501065 4894907	0.954	1.174	1.574	0.682	1.088	263412	0.301	0.704 2.309 0.130`; output identical to rustfmt's on 4 of 4 files.

- At depth 16 the start's binary spends 242.5 M instructions on a 2,000-byte file (its
  fit's exponent is 3.6: the work grows with the fourth power of the nesting depth); the
  head spends 5.69 M (0.0235x), 5.57 M at depth 8 (0.160x) and the same as base's order at
  depths 2 and 4 (0.50–0.54x). The head's local slope between depths 8 and 16 is 0.03: the
  cost no longer grows with the depth once E26's bound skips the one-line attempts that
  cannot fit. The profile-guided build is 0.81–0.87x the head at every depth.
- Every output is identical to rustfmt's on all three builds (12 of 12 comparisons;
  rustfmt exit 0 on every file).
- The ledger's cliff figures (5,617,971 at depth 8 and 5,736,051 at depth 16 at `830345e`)
  were taken with E27's path staging by `run.sh`'s own invocation; this run's 5,570,698 and
  5,691,015 are E25's merge on top (−0.8%).

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), head with `--no-cache`, `hyperfine -N -i
-w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no `--threads`
option; on one file its static chunking gives the file to one thread, so the `base` column is
the sequential figure with the base's codegen. Sizes in KB (bytes / 1000).

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 109.9 / 135.2 ± 22.3 / 10 | 32.2 / 43.2 ± 10.2 / 9 | 26.5 / 37.0 ± 6.0 / 11 | 0.274 | 0.320 | below | 132.0 | 41.4 | 48.9 | 0.371 | b[0] t[0] h[0] |
| stdin-lints | 640 | 1c | 30 | 109.6 / 124.0 ± 16.6 / 11 | 32.3 / 35.4 ± 2.8 / 9 | 33.3 / 40.5 ± 7.4 / 9 | 0.326 | 0.286 | below | 122.4 | 34.3 | 39.3 | 0.321 | b[0] t[0] h[0] |
| stdin-nodes | 358 | mc | 30 | 111.1 / 137.1 ± 19.6 / 14 | 52.8 / 75.6 ± 16.2 / 14 | 43.5 / 58.0 ± 9.3 / 14 | 0.423 | 0.551 | below | 131.1 | 72.5 | 81.7 | 0.623 | b[0] t[0] h[0] |
| stdin-nodes | 358 | 1c | 30 | 113.0 / 131.5 ± 15.8 / 14 | 58.2 / 69.9 ± 11.4 / 14 | 58.2 / 73.1 ± 15.2 / 14 | 0.556 | 0.532 | below | 128.1 | 67.8 | 71.1 | 0.556 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | mc | 30 | 84.9 / 106.0 ± 18.4 / 9 | 38.8 / 47.7 ± 7.1 / 9 | 27.7 / 32.8 ± 4.1 / 9 | 0.310 | 0.450 | below | 104.3 | 45.8 | 52.7 | 0.505 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | 1c | 30 | 88.0 / 102.3 ± 11.6 / 9 | 40.7 / 48.7 ± 7.9 / 9 | 40.8 / 48.5 ± 8.3 / 9 | 0.475 | 0.476 | below | 100.8 | 46.8 | 47.3 | 0.469 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | mc | 30 | 24.1 / 28.4 ± 3.8 / 6 | 10.7 / 12.9 ± 1.9 / 5 | 8.5 / 10.3 ± 1.1 / 5 | 0.361 | 0.453 | below | 27.4 | 12.2 | 15.2 | 0.553 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | 1c | 30 | 25.6 / 31.2 ± 5.2 / 6 | 11.4 / 15.2 ± 3.7 / 5 | 11.3 / 13.2 ± 4.7 / 5 | 0.424 | 0.488 | overlap | 30.1 | 14.1 | 11.9 | 0.395 | b[0] t[0] h[0] |
| stdin-config | 169 | mc | 30 | 33.5 / 43.2 ± 7.5 / 6 | 16.8 / 19.3 ± 2.7 / 5 | 12.5 / 14.8 ± 2.1 / 6 | 0.343 | 0.448 | below | 41.4 | 18.4 | 22.4 | 0.541 | b[0] t[0] h[0] |
| stdin-config | 169 | 1c | 30 | 34.1 / 40.6 ± 8.7 / 6 | 17.6 / 20.6 ± 3.1 / 5 | 17.3 / 19.4 ± 2.1 / 5 | 0.477 | 0.508 | below | 39.1 | 19.6 | 18.4 | 0.471 | b[0] t[0] h[0] |

- Unpinned, `head/base` in mean: 0.274 (`stdin-lints`), 0.310 (`stdin-hir-lib`), 0.343
  (`stdin-config`), 0.361 (`stdin-hover-tests`), 0.423 (`stdin-nodes`); M4 0.290–0.415. Every
  head run is below every base run on all five files unpinned and on four pinned
  (`stdin-hover-tests` pinned overlaps: head 13.2 ± 4.7 ms holds one slow run).
- `head --threads 1` against base, unpinned: 0.320–0.551 (M4 0.332–0.551); pinned
  0.286–0.532.
- The split against `head --threads 1`, unpinned: wall 0.688x (`stdin-hir-lib`, 32.8 vs
  47.7 ms) to 0.857x (`stdin-lints`, 37.0 vs 43.2), 0.767x on `stdin-nodes` and `stdin-config`,
  0.798x on `stdin-hover-tests`; CPU 1.13x–1.25x the single thread's; peak RSS equal to 1.22x
  (`stdin-lints` 11 vs 9 MiB). Pinned to one core the split costs 0.87x–1.14x the single
  thread's wall (the four threads share the core), as in M4.

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload; runs 1–3 are
`compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`
back to back at the end of the chain.

| run | 1c rustfmt | 1c head | 1c base | 1c head/base | 1c head/rustfmt | mc rustfmt | mc head | mc base | mc head/base | mc head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 1131.3 / 1271.2 ± 90.9 | 293.2 / 380.6 ± 59.2 | 757.0 / 906.5 ± 97.5 | 0.420 | 0.299 | 1121.5 / 1262.2 ± 93.3 | 81.4 / 104.2 ± 13.4 | 253.8 / 306.5 ± 28.1 | 0.340 | 0.083 |
| 1 | 1093.9 / 1232.7 ± 85.3 | 292.5 / 337.1 ± 35.9 | 725.6 / 863.6 ± 73.6 | 0.390 | 0.274 | 1079.9 / 1278.1 ± 108.6 | 79.6 / 89.7 ± 8.0 | 245.5 / 278.2 ± 23.3 | 0.322 | 0.070 |
| 2 | 1093.2 / 1239.8 ± 99.5 | 279.3 / 330.2 ± 43.9 | 758.3 / 841.3 ± 50.8 | 0.393 | 0.266 | 1067.8 / 1206.0 ± 65.3 | 82.1 / 94.5 ± 9.0 | 223.8 / 291.8 ± 42.0 | 0.324 | 0.078 |
| 3 | 1169.9 / 1327.4 ± 132.7 | 291.7 / 332.5 ± 39.2 | 803.2 / 984.8 ± 157.9 | 0.338 | 0.250 | 1114.6 / 1266.2 ± 90.9 | 90.3 / 111.7 ± 12.1 | 261.4 / 301.3 ± 28.9 | 0.371 | 0.088 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 330.2–380.6
  ms (15%; run 0, the chain's first sweep, is the slowest), the `1c` base mean 841.3–984.8 ms
  (17%), the `1c` rustfmt mean 1232.7–1327.4 ms (8%); the `mc` head mean 89.7–111.7 ms (25%)
  and the `mc` base mean 278.2–306.5 ms (10%). The `1c` head/base ratio reads 0.338–0.420
  and the `mc` ratio 0.322–0.371; head/rustfmt reads 0.250–0.299 (`1c`) and 0.070–0.088
  (`mc`). Every head run is below every base run and every rustfmt run in all four runs,
  pinned and unpinned.
- The spread between runs is wider than M4's (6%/10%/7% and 5%/5%): this window's host (see
  Load) moves every tool by tens of ms between runs; the ratios move by a tenth of their
  value and the range statements do not change.
- Instruction counts move by at most 1,710 of 221 M (0.0008%) between runs of the whole
  sample, and the per-file sum by 20,436 (0.009%) between the two per-file runs.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir` 1c: head vs base: below (97.3–186.4 vs 238.7–439.9 ms); head vs rustfmt: below (97.3–186.4 vs 343.9–605.9 ms); head-pgo vs base: below (84.8–128.4 vs 245.6–407.3 ms); head-pgo vs rustfmt: below (84.8–128.4 vs 347.9–585.8 ms).
- `fx-hir` mc: head vs base: below (33.0–51.8 vs 102.2–187.3 ms); head vs rustfmt: below (33.0–51.8 vs 352.9–688.5 ms); head-pgo vs base: below (32.8–55.8 vs 119.1–185.3 ms); head-pgo vs rustfmt: below (32.8–55.8 vs 367.0–574.7 ms).
- `fx-hir-ty` 1c: head vs base: below (293.2–531.1 vs 757.0–1206.7 ms); head vs rustfmt: below (293.2–531.1 vs 1131.3–1479.5 ms); head-pgo vs base: below (248.5–348.0 vs 794.2–1041.2 ms); head-pgo vs rustfmt: below (248.5–348.0 vs 1077.2–1475.3 ms).
- `fx-hir-ty` mc: head vs base: below (81.4–135.8 vs 253.8–360.2 ms); head vs rustfmt: below (81.4–135.8 vs 1121.5–1492.9 ms); head-pgo vs base: below (72.6–98.8 vs 241.2–424.7 ms); head-pgo vs rustfmt: below (72.6–98.8 vs 1099.7–1548.0 ms).
- `fx-ide-db` 1c: head vs base: below (88.7–162.3 vs 262.7–508.3 ms); head vs rustfmt: below (88.7–162.3 vs 382.7–558.1 ms); head-pgo vs base: below (79.4–120.1 vs 262.3–413.1 ms); head-pgo vs rustfmt: below (79.4–120.1 vs 382.8–550.0 ms).
- `fx-ide-db` mc: head vs base: below (30.8–59.3 vs 142.5–246.4 ms); head vs rustfmt: below (30.8–59.3 vs 381.4–601.4 ms); head-pgo vs base: below (31.1–44.6 vs 144.7–227.9 ms); head-pgo vs rustfmt: below (31.1–44.6 vs 381.2–622.0 ms).
- `fx-parser` 1c: head vs base: below (33.1–66.2 vs 81.5–165.4 ms); head vs rustfmt: below (33.1–66.2 vs 94.0–127.7 ms); head-pgo vs base: below (29.5–45.5 vs 80.3–140.8 ms); head-pgo vs rustfmt: below (29.5–45.5 vs 92.8–183.1 ms).
- `fx-parser` mc: head vs base: below (13.6–25.1 vs 35.0–53.7 ms); head vs rustfmt: below (13.6–25.1 vs 94.7–178.8 ms); head-pgo vs base: below (12.1–16.9 vs 31.7–53.0 ms); head-pgo vs rustfmt: below (12.1–16.9 vs 96.3–155.4 ms).
- `fx-syntax` 1c: head vs base: below (56.3–79.6 vs 145.3–234.9 ms); head vs rustfmt: below (56.3–79.6 vs 166.3–290.0 ms); head-pgo vs base: below (48.9–85.1 vs 136.9–268.9 ms); head-pgo vs rustfmt: below (48.9–85.1 vs 165.4–324.1 ms).
- `fx-syntax` mc: head vs base: below (19.1–34.5 vs 45.8–98.1 ms); head vs rustfmt: below (19.1–34.5 vs 166.1–297.1 ms); head-pgo vs base: below (17.6–28.9 vs 58.8–98.3 ms); head-pgo vs rustfmt: below (17.6–28.9 vs 168.8–270.7 ms).
- `reg-clap-builder` 1c: head vs base: below (81.3–145.3 vs 210.3–368.1 ms); head vs rustfmt: below (81.3–145.3 vs 246.2–379.4 ms); head-pgo vs base: below (72.8–124.4 vs 215.9–372.3 ms); head-pgo vs rustfmt: below (72.8–124.4 vs 240.9–417.3 ms).
- `reg-clap-builder` mc: head vs base: below (31.3–48.2 vs 84.6–134.5 ms); head vs rustfmt: below (31.3–48.2 vs 264.5–469.0 ms); head-pgo vs base: below (29.8–40.1 vs 80.3–137.4 ms); head-pgo vs rustfmt: below (29.8–40.1 vs 246.6–430.1 ms).
- `reg-regex-syntax` 1c: head vs base: below (221.4–440.8 vs 549.1–1021.0 ms); head vs rustfmt: below (221.4–440.8 vs 569.5–968.8 ms); head-pgo vs base: below (169.2–262.8 vs 523.2–952.1 ms); head-pgo vs rustfmt: below (169.2–262.8 vs 571.3–820.0 ms).
- `reg-regex-syntax` mc: head vs base: below (72.0–106.4 vs 169.4–277.7 ms); head vs rustfmt: below (72.0–106.4 vs 570.0–1017.3 ms); head-pgo vs base: below (59.7–95.6 vs 179.8–260.0 ms); head-pgo vs rustfmt: below (59.7–95.6 vs 570.6–777.9 ms).
- `reg-tokio` 1c: head vs base: below (18.9–25.0 vs 41.6–54.6 ms); head vs rustfmt: below (18.9–25.0 vs 51.6–102.0 ms); head-pgo vs base: below (17.2–30.8 vs 41.1–61.6 ms); head-pgo vs rustfmt: below (17.2–30.8 vs 51.4–79.3 ms).
- `reg-tokio` mc: head vs base: below (10.0–13.6 vs 22.3–34.0 ms); head vs rustfmt: below (10.0–13.6 vs 54.6–93.1 ms); head-pgo vs base: below (8.5–14.2 vs 19.2–81.1 ms); head-pgo vs rustfmt: below (8.5–14.2 vs 54.8–103.9 ms).
- `reg-winnow` 1c: head vs base: below (87.5–161.9 vs 239.6–424.2 ms); head vs rustfmt: below (87.5–161.9 vs 229.5–332.3 ms); head-pgo vs base: below (79.9–127.3 vs 239.9–392.1 ms); head-pgo vs rustfmt: below (79.9–127.3 vs 228.9–429.1 ms).
- `reg-winnow` mc: head vs base: below (34.5–48.2 vs 89.5–136.4 ms); head vs rustfmt: below (34.5–48.2 vs 239.6–385.5 ms); head-pgo vs base: below (29.2–39.9 vs 88.9–137.2 ms); head-pgo vs rustfmt: below (29.2–39.9 vs 239.3–391.9 ms).
- `stdin-config` 1c: head vs base: below (14.6–23.8 vs 32.5–52.5 ms); head vs rustfmt: below (14.6–23.8 vs 52.1–93.2 ms).
- `stdin-hir-lib` 1c: head vs base: below (36.9–51.5 vs 82.6–143.4 ms); head vs rustfmt: below (36.9–51.5 vs 108.5–193.1 ms).
- `stdin-hover-tests` 1c: head vs base: below (8.8–15.1 vs 22.3–38.8 ms); head vs rustfmt: below (8.8–15.1 vs 38.8–60.9 ms).
- `stdin-lints` 1c: head vs base: below (30.9–51.6 vs 103.6–137.8 ms); head vs rustfmt: below (30.9–51.6 vs 167.1–246.3 ms).
- `stdin-nodes` 1c: head vs base: below (53.3–96.6 vs 107.0–205.2 ms); head vs rustfmt: below (53.3–96.6 vs 118.8–193.0 ms).
- `tiny-one` 1c: head vs base: overlap (1.8–2.0 vs 1.8–2.2 ms); head vs rustfmt: below (1.8–2.0 vs 13.1–17.7 ms); head-pgo vs base: overlap (1.9–5.6 vs 1.9–13.7 ms); head-pgo vs rustfmt: below (1.9–5.6 vs 14.1–22.8 ms).
- `tiny-one` mc: head vs base: overlap (2.0–9.0 vs 2.1–3.3 ms); head vs rustfmt: below (2.0–9.0 vs 13.9–21.0 ms); head-pgo vs base: overlap (2.3–5.9 vs 2.7–6.4 ms); head-pgo vs rustfmt: below (2.3–5.9 vs 14.7–23.7 ms).
- `tiny-test-data` 1c: head vs base: below (14.6–21.7 vs 28.7–54.8 ms); head vs rustfmt: below (14.6–21.7 vs 71.5–114.3 ms); head-pgo vs base: below (14.5–23.2 vs 28.9–47.0 ms); head-pgo vs rustfmt: below (14.5–23.2 vs 68.0–112.0 ms).
- `tiny-test-data` mc: head vs base: below (7.8–13.4 vs 16.7–50.3 ms); head vs rustfmt: below (7.8–13.4 vs 70.2–136.5 ms); head-pgo vs base: below (6.6–13.2 vs 18.5–34.5 ms); head-pgo vs rustfmt: below (6.6–13.2 vs 68.3–115.5 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS (`head-pgo` against the
`base` runs of sweep 2, its own window), as the table builder lists them:

- `tiny-one.mc` head vs base, mean: 2.8 vs 2.6 ms (1.067); wall ranges overlap.
- `tiny-one.mc` head vs base, CPU: 2.5 vs 2.4 ms (1.014); wall ranges overlap.
- `tiny-one.mc` head vs base, rss: 3.6 vs 3.6 MiB (1.014); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, min: 1.9 vs 1.9 ms (1.037); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, CPU: 2.1 vs 2.1 ms (1.007); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, rss: 4.3 vs 3.7 MiB (1.143); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, rss: 4.2 vs 3.7 MiB (1.154); wall ranges overlap.
- `tiny-test-data.1c` head-pgo vs base, rss: 5.3 vs 4.9 MiB (1.074); wall ranges below.
- `tiny-test-data.mc` head-pgo vs base, rss: 5.4 vs 5.2 MiB (1.041); wall ranges below.

- The `tiny-one` cells read above base in mean on one row (1.067 unpinned head, inside a
  2.0–9.0 ms range against 2.1–3.3) and head-pgo's start-up RSS is 0.5–0.6 MiB above base's
  (the profile-guided binary is 0.7 MB larger), as the 13-byte file has in every record.
  No crate, stdin or 287-file cell has head or head-pgo above base or rustfmt in mean,
  minimum, CPU or wall range; head-pgo's RSS on the 287 files is 0.2–0.4 MiB above base's
  (5.3 vs 4.9 MiB).

## Against M4

M4 ([m4-quiet-host.md](m4-quiet-host.md)) measured `6b78728` (the code of `e640fbf`, the
E12–E18 head) on the 2.80 GHz host class; this record measures `d2f6fa0` (the code of
`e96b530`: M4's head plus E20, E22, E19, E27, E23, E28, E26 and E25) on the same host class
(same `model name`, `nproc`, `MemTotal` and kernel) in a slower window (Load). Protocol §3
keeps absolute figures within one window, so the two records' ratios are set side by side and
the absolute figures are not; `base` is the same binary (same sha256) in both:

| ratio (means) | M4, `6b78728` | M5, `d2f6fa0` |
|---|---|---|
| crates, `mc`, head/base | 0.25–0.52 | 0.219–0.441 |
| crates, `1c`, head/base (wall; CPU) | 0.38–0.60; 0.38–0.59 | 0.294–0.462; 0.297–0.470 |
| crates, `mc`, pgo/base | 0.22–0.45 | 0.228–0.368 |
| crates, `1c`, pgo/base | 0.36–0.43 | 0.285–0.429 |
| crates, `mc`, head/rustfmt | 0.092–0.179 | 0.083–0.173 |
| crates, `1c`, head/rustfmt | 0.26–0.44 | 0.239–0.405 |
| crates, `mc`, pgo/rustfmt | 0.072–0.175 | 0.066–0.148 |
| crates, `1c`, pgo/rustfmt | 0.24–0.38 | 0.210–0.332 |
| crates, `1c`, head below base in range | 9 of 9 | 9 of 9 |
| crates, `mc`, head below base in range | 8 of 9 | 9 of 9 |
| crates, `1c` and `mc`, head below rustfmt in range | 9 of 9 | 9 of 9 |
| 287 small files, `mc`; `1c`, head/base | 0.442; 0.565 | 0.411; 0.450 |
| stdin, head/base | 0.35–0.56 | 0.312–0.506 |
| split files, `mc`, head/base (wall; CPU) | 0.29–0.42; 0.43–0.66 | 0.274–0.423; 0.371–0.623 |
| `bench -n 5`, head/base on the triples' MB/s | 11.22–11.79 vs 4.96–5.09 (2.20x–2.38x) | 9.33–10.38 vs 3.68–4.03 (2.50x–2.67x) |
| `bench -n 5`, head-pgo MB/s | 13.03–13.24 | 11.15–12.56 |
| `Ir`, fixture sample, head/base; pgo/base | 0.4221; 0.3519 | 0.3758; 0.3106 |
| `Ir`, registry sample, head/base; pgo/base | — (322,841,637 before E19: 0.4688) | 0.4356; 0.3660 |
| parse-only share of head's `Ir`, fixtures; registry | 37.3%; ~43% (E27, before E26 and E25) | 39.1%; 46.6% |
| nested-closure cliff, depth 16, head `Ir` | 105.4 M (before E26) | 5.69 M (base 242.5 M) |
| cache, warm vs no-cache vs base, `mc` | 16.9 / 372.7 / 909.4 ms (22.1x, 53.8x) | 17.8 / 447.4 / 1280.3 ms (25.2x, 72.1x) |
| `conform -i`, head/base | 0.45 | 0.39 (by hand), 0.41 (harness) |
| `conform -i`, head identical | 1220/1220 (and at 1.8.0) | 1220/1220 (and at 1.8.0) |

- Every ratio moved with the instruction count: the two records' fixture-sample `Ir` ratios
  against base are 0.4221 and 0.3758 (−57.8% and −62.4% from the start; −11.0% between the
  records), and the pinned crate `head/base` range moved from 0.38–0.60 to 0.29–0.46,
  `head/rustfmt` from 0.26–0.44 to 0.24–0.41, `pgo/rustfmt` from 0.24–0.38 to 0.21–0.33. The
  unpinned `mc` rows now read below base in range on all nine crates (M4: eight).
- The crates with the deepest chains and closures moved most: `fx-ide-db` from 0.38 of base
  pinned and 0.25 unpinned in M4 to 0.294 and 0.219; `fx-hir-ty` from 0.52 unpinned to 0.340;
  `reg-tokio` (small files, little nesting) least, 0.60 → 0.462 pinned.
- `base` reads 0.72–0.81x M4's throughput in this record's windows for the same binary and
  bytes (`bench`, cache, `conform`, and rustfmt alike), so the wall ratios here are this
  window's; the instruction ratios, which do not depend on the window, are the figures to
  carry forward. Where head is memory-light and base memory-heavy a slower host can flatter
  the ratio; the `Ir` ratio moved by −11.0% and the pinned wall ratio by −14% to −23%, so
  up to half of the wall movement may be the window.

## Harness notes

- `compare_rustfmt.sh` passes `--no-cache` to the head builds on its own (`chloro_cache_flag=
  --no-cache` in `env.txt`) and no flag to `base`, which rejects it. The flag also turns off
  the opt-in unit cache. The `--rustfmt` and `--registry` defaults resolved without being
  passed.
- The `identical outputs` figure is now the binary's own: since `bf05b31` (the fix M4 owed)
  the harness recomputes the agreement whenever a copy is reused by a binary other than the
  one that made it and records the binary's hash beside it (`agreement_chloro=<sha16>` in the
  copy's `meta`: `f48f3593…` for head's 22 workload copies, `908c2176…` for the five stdin
  copies, which `fmt_stdin` formats). The appendix tables therefore carry the right counts
  (every workload identical, `tiny-test-data` 289/289 raw).
- `benchmarks/pgo.sh` ran before the chain (16:14–16:19 UTC, during the E25 merge gates;
  training output does not depend on timing): instrumented build, training, optimised build
  of `chloro` and `examples/bench` under `target/pgo/release/`. The profile differs per
  training run, so the profile-guided instruction count is this build's (M4's build of M4's
  head read 0.8337x its head; this one 0.8264x).
- The by-hand `bench -n 5` triples pass `--root /home/user/chloro/chloro-core/tests/conformance/
  fixtures` to all three binaries, so `base`'s `bench` reads the head's fixture tree (its
  compiled-in default is the base worktree's copy; same count and size, 1238 files, 13.9 MB).
  The harness's own `inproc-bench` rows use each binary's compiled-in root, as in M3 and M4.
- `profile.sh` is E27's: the binary and the sample are staged under a fixed-length path, so
  the counts are comparable with the ledger's from E27 on (the ledger's `e96b530` count,
  220,958,649, was taken by the same script on the same code before this chain; this
  record's two runs read 220,957,089 and 220,958,799). `--per-file` runs 50 `bench` processes
  per run (49 files and the empty sample); the per-file section took 15 minutes for the ten
  invocations.
- The trace rows at 4 threads are unpinned (the CLI's default placement) and the 1-thread
  rows pinned to core 3, as `collect.sh` does; `diag.py` records the 1-minute load average
  beside each run (1.0–2.4 during the section, the chain's own processes).
- The cliff run's `fmt_stdin` for `head-pgo` is head's (no profile-guided `fmt_stdin`), so the
  `head-pgo` comparison checks the head's output, not the profile-guided binary's; the
  profile-guided `bench`'s output bytes are recorded in its `ir.tsv` and equal head's on
  every file (`out_bytes` 472, 680, 1672, 10672).

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
- On formatted input with all 4 CPUs, `head` takes 0.083–0.173x rustfmt's wall time on the
  nine crates and 0.219–0.441x base's; on one core 0.239–0.405x rustfmt's and 0.294–0.462x
  base's (CPU 0.297–0.470x). Every head run is below every rustfmt run and every base run on
  every crate, pinned and unpinned.
- `head-pgo` takes 0.066–0.148x rustfmt's wall time unpinned and 0.210–0.332x pinned;
  0.228–0.368x base's unpinned and 0.285–0.429x pinned; `Ir` 0.3106x base's and 0.8264x
  head's on the fixture sample, 0.3660x and 0.8402x on the registry sample.
- The warm cache answers `--check` on 1238 formatted files in 17.8 ms against 447.4 ms without
  it (25.2x) and 1280.3 ms for base (72.1x); 56.1x and 144.3x on one core.
- In process, head formats the fixtures at 9.33–10.38 MB/s against base's 3.68–4.03 MB/s
  (disjoint over 3 interleaved triples; 2.50x–2.67x), head-pgo at 11.15–12.56 MB/s; `conform
  -i` reports 1220/1220 identical for head at rustfmt 1.9.0 and at 1.8.0, 1219/1220 for base;
  the instruction count on the fixture sample is 220,957,089 (0.3758 of base, −62.4% from
  the start), 182,602,721 profile-guided (0.3106); on the registry sample 299,933,543 (0.4356)
  and 252,010,500 (0.3660).
- The parse-only phase is 39.1% of head's instructions on the fixture sample and 46.6% on
  the registry sample: the parser is now the largest single component, above the plan's
  fork trigger on the registry sample.
- The nested-closure cliff is gone on the three builds' comparison: depth 16 costs 5.69 M
  instructions on head against 242.5 M on base (0.0235x), with identical output.
- The parallel-items split gives 0.274–0.423x base's wall time on the five largest files
  with 4 threads, at 0.371–0.623x the CPU and up to 1.22x the peak RSS.
- The four-core schedule has no tail on seven crates and a 14–22% tail on the two whose one
  split file (`hir/src/lib.rs`, `lints.rs`) is the last thing running; the lever is inside the
  split file.
- Output agreement with rustfmt, recomputed with these binaries: every file of every workload
  on both inputs and the cliff files.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure; the cliff comparison for `head-pgo` uses head's `fmt_stdin`.
- The `cold` cache variant has no pinned row; the profile-guided cache rows are unpinned only.
- No raw-input row for `head-pgo` (measurement A's raw sweep is head only, as in M1–M4).
- The by-hand `conform -i` runs have no peak RSS of their own (no `/usr/bin/time` in the
  container).
- The host was 20–40% slower than in M4's windows for every tool; the absolute milliseconds
  of this record are not comparable with M4's, only the ratios and the instruction counts are.
  A re-run of measurement A in a window where `base`'s `bench` reads M4's 4.96–5.09 MB/s
  would settle how much of the wall-ratio movement between M4 and M5 is the window.
- The trace section is the wall view only; the reconstruction from thread CPU clocks
  (`sim.py`, the patched build) was not rerun on this head.
- `base`'s `bench` has no `--phase parse`, so the parser's share at the start is not
  measured here (EXP-017 and the theory records bound it).
- The unit cache (`--unit-cache`, E15) and the resident process prototype (E21, on its
  branch) are not measured here; their figures are in EXP-015 and EXP-021.

## Appendix: `report.py` tables as written

The `report.md` of each harness results directory, as `report.py` wrote it.

### `m5-a1-head-base-formatted`

```text
date_utc=2026-10-15T16:32:34Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.86 3.44 2.93
cpu3_stat_start=2080907 23876 210067 318557 8446 0 4598 40215 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.44 2.04 2.42
cpu3_stat_end=2116306 23876 213211 332507 8486 0 4621 40858 0 0
cpu3_steal_ticks=643 cpu3_busy_ticks=38543 cpu3_idle_ticks=13950
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1271.2 ± 90.9 (1131.3–1479.5) | 380.6 ± 59.2 (293.2–531.1) | 906.5 ± 97.5 (757.0–1206.7) | 0.420 | below | 0.299 | 0.259 | below | 86 | 8 | 1250.6 | 373.7 | 0.299 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1262.2 ± 93.3 (1121.5–1492.9) | 104.2 ± 13.4 (81.4–135.8) | 306.5 ± 28.1 (253.8–360.2) | 0.340 | below | 0.083 | 0.073 | below | 86 | 16 | 1241.5 | 379.3 | 0.306 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 400.8 ± 58.4 (343.9–605.9) | 114.2 ± 25.5 (97.3–186.4) | 298.7 ± 53.6 (238.7–439.9) | 0.382 | below | 0.285 | 0.283 | below | 57 | 9 | 396.2 | 112.5 | 0.284 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 449.3 ± 82.8 (352.9–688.5) | 41.4 ± 4.7 (33.0–51.8) | 146.4 ± 21.0 (102.2–187.3) | 0.283 | below | 0.092 | 0.094 | below | 57 | 17 | 441.8 | 131.7 | 0.298 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 441.8 ± 47.9 (382.7–558.1) | 105.4 ± 18.9 (88.7–162.3) | 358.2 ± 74.7 (262.7–508.3) | 0.294 | below | 0.239 | 0.232 | below | 58 | 9 | 432.2 | 104.8 | 0.243 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 460.1 ± 52.2 (381.4–601.4) | 40.0 ± 6.8 (30.8–59.3) | 183.2 ± 23.9 (142.5–246.4) | 0.219 | below | 0.087 | 0.081 | below | 58 | 15 | 449.1 | 122.8 | 0.273 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 100.9 ± 7.7 (94.0–127.7) | 39.2 ± 7.0 (33.1–66.2) | 99.0 ± 21.2 (81.5–165.4) | 0.396 | below | 0.389 | 0.353 | below | 47 | 5 | 100.0 | 37.8 | 0.378 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 113.9 ± 19.5 (94.7–178.8) | 16.6 ± 2.7 (13.6–25.1) | 44.4 ± 5.0 (35.0–53.7) | 0.374 | below | 0.146 | 0.144 | below | 47 | 6 | 110.7 | 48.9 | 0.442 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 201.5 ± 31.0 (166.3–290.0) | 63.6 ± 6.4 (56.3–79.6) | 170.8 ± 21.9 (145.3–234.9) | 0.372 | below | 0.316 | 0.338 | below | 51 | 6 | 197.6 | 62.5 | 0.316 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 210.9 ± 30.9 (166.1–297.1) | 23.6 ± 3.3 (19.1–34.5) | 72.0 ± 11.1 (45.8–98.1) | 0.328 | below | 0.112 | 0.115 | below | 51 | 8 | 204.6 | 76.7 | 0.375 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 280.5 ± 36.7 (246.2–379.4) | 100.7 ± 16.7 (81.3–145.3) | 243.7 ± 38.7 (210.3–368.1) | 0.413 | below | 0.359 | 0.330 | below | 56 | 6 | 277.2 | 98.1 | 0.354 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 328.2 ± 48.0 (264.5–469.0) | 37.6 ± 4.6 (31.3–48.2) | 103.6 ± 12.5 (84.6–134.5) | 0.363 | below | 0.115 | 0.118 | below | 55 | 12 | 319.5 | 121.9 | 0.381 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 675.5 ± 91.8 (569.5–968.8) | 270.9 ± 53.2 (221.4–440.8) | 687.1 ± 115.4 (549.1–1021.0) | 0.394 | below | 0.401 | 0.389 | below | 66 | 11 | 669.0 | 266.3 | 0.398 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 696.5 ± 94.6 (570.0–1017.3) | 84.6 ± 8.6 (72.0–106.4) | 228.8 ± 27.4 (169.4–277.7) | 0.370 | below | 0.121 | 0.126 | below | 66 | 23 | 685.8 | 285.4 | 0.416 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 66.8 ± 14.0 (51.6–102.0) | 20.9 ± 1.9 (18.9–25.0) | 45.2 ± 3.4 (41.6–54.6) | 0.462 | below | 0.312 | 0.366 | below | 43 | 4 | 65.8 | 20.5 | 0.311 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 68.4 ± 11.5 (54.6–93.1) | 11.9 ± 0.8 (10.0–13.6) | 26.9 ± 3.0 (22.3–34.0) | 0.441 | below | 0.173 | 0.183 | below | 43 | 5 | 66.5 | 32.7 | 0.491 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 261.8 ± 28.4 (229.5–332.3) | 106.0 ± 22.0 (87.5–161.9) | 276.9 ± 51.7 (239.6–424.2) | 0.383 | below | 0.405 | 0.381 | below | 59 | 7 | 258.4 | 104.5 | 0.404 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 292.8 ± 36.7 (239.6–385.5) | 41.5 ± 4.0 (34.5–48.2) | 107.6 ± 12.5 (89.5–136.4) | 0.386 | below | 0.142 | 0.144 | below | 59 | 12 | 286.3 | 141.6 | 0.494 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 14.3 ± 1.1 (13.1–17.7) | 1.9 ± 0.1 (1.8–2.0) | 2.0 ± 0.1 (1.8–2.2) | 0.957 | overlap | 0.131 | 0.135 | below | 34 | 4 | 13.8 | 1.7 | 0.123 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 16.2 ± 2.0 (13.9–21.0) | 2.8 ± 1.3 (2.0–9.0) | 2.6 ± 0.3 (2.1–3.3) | 1.067 | overlap | 0.175 | 0.147 | below | 34 | 4 | 15.6 | 2.5 | 0.158 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 86.1 ± 15.0 (71.5–114.3) | 15.6 ± 1.5 (14.6–21.7) | 34.7 ± 8.1 (28.7–54.8) | 0.450 | below | 0.181 | 0.204 | below | 44 | 5 | 84.3 | 15.3 | 0.182 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 89.9 ± 17.4 (70.2–136.5) | 10.0 ± 1.6 (7.8–13.4) | 24.3 ± 6.8 (16.7–50.3) | 0.411 | below | 0.111 | 0.111 | below | 44 | 5 | 85.4 | 21.6 | 0.253 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 169 | 30 | 65.8 ± 13.3 (52.1–93.2) | 16.5 ± 1.9 (14.6–23.8) | 36.6 ± 4.7 (32.5–52.5) | 0.452 | below | 0.251 | 0.279 | below | 44 | 5 | 64.6 | 15.9 | 0.246 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 127.1 ± 21.7 (108.5–193.1) | 40.9 ± 3.3 (36.9–51.5) | 96.0 ± 16.7 (82.6–143.4) | 0.426 | below | 0.322 | 0.340 | below | 47 | 8 | 124.1 | 40.2 | 0.324 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 42.5 ± 4.9 (38.8–60.9) | 11.0 ± 2.3 (8.8–15.1) | 25.5 ± 4.3 (22.3–38.8) | 0.430 | below | 0.258 | 0.226 | below | 40 | 4 | 41.9 | 10.6 | 0.254 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 191.5 ± 21.8 (167.1–246.3) | 34.1 ± 5.1 (30.9–51.6) | 109.5 ± 7.8 (103.6–137.8) | 0.312 | below | 0.178 | 0.185 | below | 43 | 8 | 188.4 | 33.7 | 0.179 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 134.2 ± 19.7 (118.8–193.0) | 66.7 ± 11.6 (53.3–96.6) | 131.8 ± 26.6 (107.0–205.2) | 0.506 | below | 0.497 | 0.449 | below | 53 | 14 | 132.7 | 65.9 | 0.497 | 1/1 | r[0] c[0] |

### `m5-a2-pgo-base-formatted`

```text
date_utc=2026-10-15T16:41:27Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.44 2.04 2.42
cpu3_stat_start=2116307 23876 213215 332523 8486 0 4621 40858 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/pgo/release/chloro sha256=602ddf4ceeff1e0b
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=1.91 2.02 2.27
cpu3_stat_end=2148912 23876 215815 344240 8490 0 4647 41442 0 0
cpu3_steal_ticks=584 cpu3_busy_ticks=35205 cpu3_idle_ticks=11717
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1239.7 ± 104.8 (1077.2–1475.3) | 286.3 ± 25.6 (248.5–348.0) | 885.3 ± 64.1 (794.2–1041.2) | 0.323 | below | 0.231 | 0.231 | below | 86 | 8 | 1216.8 | 281.7 | 0.232 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1280.8 ± 118.8 (1099.7–1548.0) | 84.1 ± 7.6 (72.6–98.8) | 314.9 ± 50.9 (241.2–424.7) | 0.267 | below | 0.066 | 0.066 | below | 86 | 17 | 1258.3 | 306.8 | 0.244 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 415.0 ± 68.8 (347.9–585.8) | 99.3 ± 13.8 (84.8–128.4) | 286.6 ± 41.0 (245.6–407.3) | 0.346 | below | 0.239 | 0.244 | below | 57 | 10 | 407.1 | 97.8 | 0.240 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 431.0 ± 54.1 (367.0–574.7) | 40.2 ± 6.4 (32.8–55.8) | 145.7 ± 17.4 (119.1–185.3) | 0.276 | below | 0.093 | 0.089 | below | 57 | 18 | 424.1 | 120.4 | 0.284 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 430.1 ± 41.9 (382.8–550.0) | 90.3 ± 12.6 (79.4–120.1) | 316.4 ± 37.4 (262.3–413.1) | 0.285 | below | 0.210 | 0.207 | below | 58 | 10 | 425.0 | 88.8 | 0.209 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 445.5 ± 61.2 (381.2–622.0) | 40.3 ± 3.6 (31.1–44.6) | 176.3 ± 19.8 (144.7–227.9) | 0.228 | below | 0.090 | 0.082 | below | 58 | 17 | 439.5 | 120.9 | 0.275 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 114.1 ± 21.2 (92.8–183.1) | 33.7 ± 5.1 (29.5–45.5) | 93.4 ± 14.7 (80.3–140.8) | 0.361 | below | 0.296 | 0.318 | below | 47 | 5 | 111.6 | 33.3 | 0.298 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 118.6 ± 16.5 (96.3–155.4) | 14.4 ± 1.4 (12.1–16.9) | 40.4 ± 4.6 (31.7–53.0) | 0.357 | below | 0.122 | 0.126 | below | 47 | 7 | 115.4 | 43.4 | 0.376 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 216.7 ± 44.2 (165.4–324.1) | 67.2 ± 14.4 (48.9–85.1) | 183.0 ± 40.7 (136.9–268.9) | 0.367 | below | 0.310 | 0.295 | below | 51 | 6 | 214.2 | 65.9 | 0.308 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 201.6 ± 25.2 (168.8–270.7) | 20.6 ± 2.4 (17.6–28.9) | 74.8 ± 10.5 (58.8–98.3) | 0.275 | below | 0.102 | 0.104 | below | 51 | 8 | 197.2 | 64.0 | 0.324 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 303.5 ± 44.1 (240.9–417.3) | 84.9 ± 11.3 (72.8–124.4) | 271.8 ± 42.0 (215.9–372.3) | 0.312 | below | 0.280 | 0.302 | below | 55 | 7 | 299.4 | 83.7 | 0.279 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 310.7 ± 46.4 (246.6–430.1) | 34.0 ± 3.0 (29.8–40.1) | 99.1 ± 11.6 (80.3–137.4) | 0.343 | below | 0.109 | 0.121 | below | 55 | 12 | 304.9 | 109.9 | 0.360 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 658.1 ± 73.9 (571.3–820.0) | 196.2 ± 24.8 (169.2–262.8) | 654.4 ± 104.5 (523.2–952.1) | 0.300 | below | 0.298 | 0.296 | below | 66 | 11 | 650.4 | 192.8 | 0.296 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 682.2 ± 58.1 (570.6–777.9) | 76.3 ± 8.6 (59.7–95.6) | 215.0 ± 22.5 (179.8–260.0) | 0.355 | below | 0.112 | 0.105 | below | 66 | 25 | 670.5 | 252.2 | 0.376 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 59.0 ± 8.4 (51.4–79.3) | 19.6 ± 3.3 (17.2–30.8) | 45.6 ± 5.1 (41.1–61.6) | 0.429 | below | 0.332 | 0.335 | below | 43 | 5 | 57.9 | 19.1 | 0.330 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 72.3 ± 14.4 (54.8–103.9) | 10.7 ± 1.4 (8.5–14.2) | 29.0 ± 10.9 (19.2–81.1) | 0.368 | below | 0.148 | 0.155 | below | 43 | 6 | 70.5 | 26.6 | 0.377 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 294.4 ± 50.4 (228.9–429.1) | 88.0 ± 11.3 (79.9–127.3) | 288.6 ± 40.3 (239.9–392.1) | 0.305 | below | 0.299 | 0.349 | below | 59 | 8 | 291.8 | 86.3 | 0.296 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 286.2 ± 37.2 (239.3–391.9) | 33.4 ± 2.6 (29.2–39.9) | 112.3 ± 13.7 (88.9–137.2) | 0.297 | below | 0.117 | 0.122 | below | 59 | 13 | 281.5 | 111.7 | 0.397 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 17.0 ± 3.0 (14.1–22.8) | 2.4 ± 0.7 (1.9–5.6) | 2.7 ± 2.1 (1.9–13.7) | 0.877 | overlap | 0.141 | 0.136 | below | 34 | 4 | 16.4 | 2.1 | 0.128 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 17.8 ± 2.2 (14.7–23.7) | 3.4 ± 0.8 (2.3–5.9) | 3.8 ± 0.7 (2.7–6.4) | 0.888 | overlap | 0.188 | 0.157 | below | 34 | 4 | 17.1 | 3.0 | 0.173 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 77.9 ± 11.7 (68.0–112.0) | 18.8 ± 3.2 (14.5–23.2) | 33.5 ± 3.8 (28.9–47.0) | 0.561 | below | 0.242 | 0.214 | below | 44 | 5 | 76.7 | 18.5 | 0.241 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 86.3 ± 11.4 (68.3–115.5) | 9.0 ± 1.7 (6.6–13.2) | 23.0 ± 3.6 (18.5–34.5) | 0.389 | below | 0.104 | 0.096 | below | 44 | 5 | 84.1 | 20.1 | 0.239 | 287/287 | r[0] c[0] |

### `m5-a3-head-raw`

```text
date_utc=2026-10-15T16:49:23Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.91 2.02 2.27
cpu3_stat_start=2148913 23876 215817 344262 8490 0 4647 41442 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.17 1.28 1.76
cpu3_stat_end=2178096 23876 221696 362899 8497 0 4664 41899 0 0
cpu3_steal_ticks=457 cpu3_busy_ticks=35062 cpu3_idle_ticks=18637
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1738.2 ± 113.4 (1601.7–1995.4) | 349.7 ± 48.5 (289.1–488.0) | 0.201 | 0.181 | below | 117 | 8 | 1718.1 | 345.5 | 0.201 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1773.1 ± 139.8 (1572.5–2197.1) | 87.9 ± 8.0 (81.5–120.9) | 0.050 | 0.052 | below | 117 | 16 | 1745.1 | 327.1 | 0.187 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 774.8 ± 74.2 (675.6–1014.4) | 114.6 ± 22.0 (95.8–159.8) | 0.148 | 0.142 | below | 218 | 9 | 764.3 | 113.1 | 0.148 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 857.2 ± 115.7 (689.1–1093.3) | 43.0 ± 4.8 (33.7–50.5) | 0.050 | 0.049 | below | 218 | 17 | 846.2 | 139.9 | 0.165 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 522.5 ± 58.9 (429.9–630.7) | 129.8 ± 19.0 (96.2–164.9) | 0.248 | 0.224 | below | 66 | 9 | 514.8 | 127.8 | 0.248 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 533.6 ± 82.6 (441.5–722.5) | 43.3 ± 5.1 (33.7–54.2) | 0.081 | 0.076 | below | 66 | 15 | 524.5 | 131.8 | 0.251 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 114.0 ± 9.8 (103.3–140.9) | 40.9 ± 10.2 (32.2–66.3) | 0.359 | 0.312 | below | 48 | 5 | 111.6 | 40.1 | 0.360 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 125.2 ± 19.0 (106.1–179.8) | 15.6 ± 1.2 (14.3–19.0) | 0.125 | 0.135 | below | 48 | 6 | 122.7 | 47.6 | 0.387 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 241.1 ± 19.9 (215.6–300.7) | 62.7 ± 11.0 (52.3–86.2) | 0.260 | 0.243 | below | 57 | 5 | 236.2 | 61.0 | 0.258 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 263.0 ± 39.2 (220.1–340.9) | 23.3 ± 2.3 (19.5–29.3) | 0.089 | 0.089 | below | 57 | 8 | 259.2 | 77.7 | 0.300 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 311.1 ± 56.6 (244.9–493.3) | 96.9 ± 15.1 (82.3–141.3) | 0.311 | 0.336 | below | 55 | 6 | 308.3 | 95.3 | 0.309 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 319.1 ± 49.8 (272.4–431.8) | 41.4 ± 6.6 (31.1–54.1) | 0.130 | 0.114 | below | 56 | 11 | 314.3 | 132.3 | 0.421 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 2146.8 ± 255.8 (1823.0–2790.7) | 232.8 ± 25.6 (205.4–302.0) | 0.108 | 0.113 | below | 423 | 11 | 2125.3 | 229.9 | 0.108 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 2096.1 ± 179.0 (1805.4–2547.5) | 98.7 ± 33.1 (78.6–265.1) | 0.047 | 0.044 | below | 423 | 23 | 2071.2 | 305.5 | 0.147 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 60.5 ± 8.1 (52.7–84.4) | 20.4 ± 1.5 (19.1–24.7) | 0.337 | 0.362 | below | 43 | 4 | 59.3 | 19.8 | 0.333 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 66.5 ± 12.8 (53.2–98.7) | 10.6 ± 2.6 (8.2–19.2) | 0.159 | 0.154 | below | 43 | 5 | 65.0 | 26.0 | 0.400 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 310.9 ± 56.2 (249.8–454.2) | 118.3 ± 19.4 (89.3–149.6) | 0.381 | 0.358 | below | 60 | 7 | 306.6 | 116.8 | 0.381 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 305.9 ± 50.1 (254.2–479.9) | 34.3 ± 3.3 (30.2–42.8) | 0.112 | 0.119 | below | 60 | 11 | 301.6 | 118.2 | 0.392 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 16.2 ± 2.1 (13.6–19.8) | 2.5 ± 0.3 (1.8–3.1) | 0.153 | 0.136 | below | 34 | 4 | 15.7 | 2.3 | 0.144 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 17.1 ± 2.0 (14.7–21.2) | 3.1 ± 0.6 (2.3–5.0) | 0.184 | 0.157 | below | 34 | 4 | 16.5 | 2.8 | 0.172 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 109.3 ± 27.3 (84.6–175.8) | 16.9 ± 2.4 (14.9–25.8) | 0.155 | 0.176 | below | 44 | 4 | 107.9 | 16.4 | 0.152 | 289/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 119.8 ± 18.2 (94.2–164.3) | 9.5 ± 0.8 (8.1–11.0) | 0.079 | 0.086 | below | 44 | 5 | 117.3 | 22.1 | 0.189 | 289/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 73.3 ± 13.9 (57.3–101.0) | 16.8 ± 0.9 (15.6–19.1) | 0.230 | 0.273 | below | 44 | 5 | 71.1 | 16.4 | 0.231 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 134.7 ± 17.7 (116.0–189.3) | 42.9 ± 3.7 (39.3–56.9) | 0.319 | 0.339 | below | 47 | 8 | 133.0 | 42.4 | 0.319 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 51.0 ± 9.5 (40.3–73.4) | 10.7 ± 1.6 (9.2–16.9) | 0.209 | 0.230 | below | 40 | 4 | 49.8 | 10.1 | 0.203 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 217.4 ± 37.6 (167.3–292.5) | 40.1 ± 6.3 (32.5–52.8) | 0.185 | 0.194 | below | 43 | 8 | 214.5 | 39.1 | 0.182 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 155.3 ± 25.1 (117.8–209.6) | 67.5 ± 15.1 (51.3–99.3) | 0.435 | 0.435 | below | 53 | 14 | 153.2 | 66.7 | 0.436 | 1/1 | r[0] c[0] |

### `m5-d-inproc-harness`

```text
date_utc=2026-10-15T16:58:26Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.17 1.28 1.76
cpu3_stat_start=2178107 23876 221700 362905 8497 0 4664 41899 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.00 1.12 1.58
cpu3_stat_end=2201583 23876 221976 362916 8655 0 4669 42160 0 0
cpu3_steal_ticks=261 cpu3_busy_ticks=23752 cpu3_idle_ticks=11
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 3374.6 ± 342.3 (2987.1–4117.4) | 48 | 3335.6 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 8283.3 ± 325.7 (7787.5–8672.3) | 51 | 8176.6 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.318 s | 10.54 |
| 2 | chloro-base | 1238 | 13.9 | 3.941 s | 3.53 |
| 3 | chloro-base | 1238 | 13.9 | 3.850 s | 3.61 |
| 4 | chloro | 1238 | 13.9 | 1.443 s | 9.63 |
| 5 | chloro | 1238 | 13.9 | 1.635 s | 8.50 |
| 6 | chloro-base | 1238 | 13.9 | 4.294 s | 3.24 |

- chloro: 3 runs, 8.50–10.54 MB/s
- chloro-base: 3 runs, 3.24–3.61 MB/s

### `m5-noise-hir-ty-1`

```text
date_utc=2026-10-15T17:24:27Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.93 1.33 1.37
cpu3_stat_start=2318340 23876 229367 368551 8702 0 4724 44000 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=2.15 1.71 1.51
cpu3_stat_end=2328937 23876 229827 371793 8703 0 4730 44190 0 0
cpu3_steal_ticks=190 cpu3_busy_ticks=11057 cpu3_idle_ticks=3242
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1232.7 ± 85.3 (1093.9–1452.4) | 337.1 ± 35.9 (292.5–469.5) | 863.6 ± 73.6 (725.6–1044.4) | 0.390 | below | 0.274 | 0.267 | below | 86 | 8 | 1210.8 | 330.1 | 0.273 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1278.1 ± 108.6 (1079.9–1528.1) | 89.7 ± 8.0 (79.6–110.8) | 278.2 ± 23.3 (245.5–343.6) | 0.322 | below | 0.070 | 0.074 | below | 86 | 16 | 1258.1 | 330.5 | 0.263 | 122/122 | r[0] c[0] |

### `m5-noise-hir-ty-2`

```text
date_utc=2026-10-15T17:26:52Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=2.15 1.71 1.51
cpu3_stat_start=2328939 23876 229830 371805 8703 0 4730 44190 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.92 1.90 1.62
cpu3_stat_end=2339587 23876 230310 374604 8703 0 4736 44335 0 0
cpu3_steal_ticks=145 cpu3_busy_ticks=11128 cpu3_idle_ticks=2799
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1239.8 ± 99.5 (1093.2–1532.2) | 330.2 ± 43.9 (279.3–490.7) | 841.3 ± 50.8 (758.3–945.3) | 0.393 | below | 0.266 | 0.255 | below | 86 | 8 | 1224.5 | 326.3 | 0.266 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1206.0 ± 65.3 (1067.8–1348.7) | 94.5 ± 9.0 (82.1–116.1) | 291.8 ± 42.0 (223.8–382.6) | 0.324 | below | 0.078 | 0.077 | below | 86 | 16 | 1188.1 | 350.0 | 0.295 | 122/122 | r[0] c[0] |

### `m5-noise-hir-ty-3`

```text
date_utc=2026-10-15T17:29:13Z
commit=d2f6fa06cc8c2511cdb59ca6b3a589d95818172b
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.92 1.90 1.62
cpu3_stat_start=2339589 23876 230311 374617 8703 0 4736 44335 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=f48f3593dc5bc8db
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=908c217696a94154
bench=/home/user/chloro/target/release/examples/bench sha256=59f34345395848fa
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.84 1.86 1.65
cpu3_stat_end=2351109 23876 230811 377495 8703 0 4740 44505 0 0
cpu3_steal_ticks=170 cpu3_busy_ticks=12020 cpu3_idle_ticks=2878
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 1327.4 ± 132.7 (1169.9–1577.5) | 332.5 ± 39.2 (291.7–447.2) | 984.8 ± 157.9 (803.2–1349.7) | 0.338 | below | 0.250 | 0.249 | below | 86 | 8 | 1309.9 | 327.9 | 0.250 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1266.2 ± 90.9 (1114.6–1488.8) | 111.7 ± 12.1 (90.3–140.2) | 301.3 ± 28.9 (261.4–373.4) | 0.371 | below | 0.088 | 0.081 | below | 86 | 16 | 1245.7 | 408.0 | 0.328 | 122/122 | r[0] c[0] |
