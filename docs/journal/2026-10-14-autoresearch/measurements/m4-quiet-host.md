# M4: base, head and head-pgo against rustfmt on a quiet host (the final head, E12–E18)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 4 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the overnight merges (E12 CLI
waste, E13 PGO in the release workflow, E14 allocator measurement, E15 unit cache, E16
scanner paths, E17 parse floor, E18 dead candidates) and on rustfmt 1.9.0, with the method of
[m3-quiet-host.md](m3-quiet-host.md), [m2-quiet-host.md](m2-quiet-host.md) and
[m1-quiet-host.md](m1-quiet-host.md). Every figure in this file is **measured** in the
windows named below; nothing is estimated. The host is the 2.80 GHz host class of
[../baseline.md](../baseline.md), M2 and M3 (protocol §3: figures from different hosts are
not compared; the "against M3" section sets within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `6b78728` (the integration branch after the E12–E18 merges; three docs-only commits above the ledger's best-known-good `e640fbf`, same code; the formatter code is M3's `4867896` plus R5, E12, E15, E17, E18 and E16, with E13 touching the release workflow and E14 adding off-by-default features) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `6b78728ab59fde69663c8b4ca9ec32ca55748c08` (worktree clean,
  `worktree_dirty=0`; `git diff --stat e640fbf HEAD` touches three Markdown files under
  `docs/journal/` and no source); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git
  worktree add /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `37d1d72d07248151bf2f0a03d8cc8fcb7b8d0a087a6c03cd6a43d8b424763033` | 2,147,536 |
| `head` bench | `target/release/examples/bench` | `819d6f68ebd5c415721c4fd32466d0a61e253d3c51b804d48b44d4a2c3cf69ac` | 1,842,304 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `93ee7b03e8fd238f140e58f0d9ebb1f5c72a42098895b849394fb1889e9029a2` | 1,684,744 |
| `head` conform | `target/release/examples/conform` | `b287b617bae7fcf92a9158e81be16ae561cbe14a403c4da8689ea1f1328e5fd4` | 1,814,192 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `218d001c5d831ec6c541d3df10766768cb949598bf65686ff05170ed3b67247b` | 2,924,784 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `1f78c6bca6534ad6ce0a8f9431f3200988fcfa03aa6c7fc9ce762d08d868b378` | 2,631,544 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes and the rustfmt hash equal the ones recorded in M1, M2 and M3: the
  default-profile build of `884f9c9` reproduces byte for byte on this toolchain.
- The `head` chloro binary is 2,147,536 bytes against 2,010,696 for M3's `4867896` build and
  2,516,048 for `base` (0.854x base; 1.068x M3's head: the CLI gained the unit cache, the
  identity stamp and the E16 paths); `head-pgo` is 2,924,784 bytes (1.162x base, 1.078x M3's
  2,714,112).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 17 lines for `head`,
  38 for `base` (the M2 and M3 counts; E14's allocator crates are behind off-by-default
  features and do not appear).
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, used by `pgo.sh`);
  hyperfine 2.0.0; valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. The 2.80 GHz host class
  of M2, M3 and [../baseline.md](../baseline.md).
- **Load:** no other agent ran on the host (the overnight experiment agents had all finished
  and their worktrees were cleaned before the first command). The 1-minute load average read
  0.01 at the identity step. The chain ran as one script
  (`scratchpad/m4/chain.sh`, reproduced in Method) with no gap between sweeps; the load
  average at the start of every later sweep is the harness's own processes from the previous
  sweep (the `mc` rows run chloro with 4 threads). `cpu3` columns are `/proc/stat` ticks of
  core 3 (USER_HZ 100) over the sweep: steal / busy / idle. A first start of the chain at
  04:28 UTC was stopped after one minute because the five stdin workload copies on this disk
  predated the harness's stdin preparation (two of the five were not fixed points: exit 1
  for `base`, `head` and `head-pgo` alike); the five copies were rebuilt through rustfmt's
  stdin (`--groups c -r 1`), verified as fixed points for rustfmt, `base`, `head` and
  `head-pgo` (exit 0 on all five), and the chain restarted from the identity step. Nothing of
  the first start is used.

| mark | time | load | cpu3 steal / busy / idle ticks since previous mark |
|---|---|---|---|
| a1-start | 2026-10-15T04:30:43Z | 0.87 0.70 1.35 | – |
| a1-end | 2026-10-15T04:37:01Z | 1.64 1.55 1.53 | 251 / 29492 / 8020 |
| a2-start | 2026-10-15T04:37:01Z | 1.64 1.55 1.53 | 0 / 0 / 0 |
| a2-end | 2026-10-15T04:43:00Z | 1.95 1.80 1.64 | 263 / 26273 / 9288 |
| a3-start | 2026-10-15T04:43:00Z | 1.95 1.80 1.64 | 0 / 0 / 0 |
| a3-end | 2026-10-15T04:49:41Z | 1.07 1.28 1.44 | 234 / 26971 / 12744 |
| dh-start | 2026-10-15T04:49:41Z | 1.07 1.28 1.44 | 0 / 0 / 1 |
| dh-end | 2026-10-15T04:52:31Z | 1.11 1.21 1.39 | 161 / 16883 / 18 |
| c-start | 2026-10-15T04:52:31Z | 1.11 1.21 1.39 | 0 / 0 / 0 |
| c-end | 2026-10-15T04:54:45Z | 2.70 2.01 1.67 | 112 / 12577 / 602 |
| d-start | 2026-10-15T04:54:45Z | 2.70 2.01 1.67 | 0 / 0 / 1 |
| d-bench-end | 2026-10-15T04:56:03Z | 1.44 1.77 1.62 | 55 / 7817 / 3 |
| d-profile-end | 2026-10-15T04:56:31Z | 1.27 1.69 1.59 | 17 / 2685 / 27 |
| d-end | 2026-10-15T04:56:42Z | 1.23 1.67 1.59 | 5 / 1096 / 2 |
| e-start | 2026-10-15T04:56:42Z | 1.23 1.67 1.59 | 0 / 0 / 0 |
| e-end | 2026-10-15T04:57:24Z | 1.14 1.59 1.56 | 25 / 2744 / 1502 |
| noise-1-start | 2026-10-15T04:57:24Z | 1.14 1.59 1.56 | 0 / 0 / 1 |
| noise-1-end | 2026-10-15T04:59:12Z | 1.80 1.68 1.59 | 58 / 7473 / 3222 |
| noise-2-start | 2026-10-15T04:59:12Z | 1.80 1.68 1.59 | 0 / 0 / 1 |
| noise-2-end | 2026-10-15T05:01:02Z | 1.76 1.68 1.60 | 86 / 8424 / 2428 |
| noise-3-start | 2026-10-15T05:01:02Z | 1.76 1.68 1.60 | 0 / 0 / 0 |
| noise-3-end | 2026-10-15T05:02:54Z | 1.91 1.76 1.63 | 88 / 8181 / 2979 |
| chain-end | 2026-10-15T05:02:54Z | 1.91 1.76 1.63 | 0 / 0 / 0 |

- Steal on core 3 is 0.4–1.0% of each sweep's ticks (sweep 1: 251 of 37,739; sweep 2: 263
  of 36,380; sweep 3: 234 of 40,189; the cache sweep: 112 of 12,845), at or below M3's
  0.60–0.85%. The pinned rows carry this steal.
- The whole chain ran as one script from 04:30:43 to its last noise run with no gap between
  steps; the by-hand steps (cache, in-process, parallel items) wrote their exports under
  `benchmarks/results/m4-*` like the harness sweeps.

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
  has no cache and gets no flag. Since E15 `--no-cache` also turns off the opt-in unit cache,
  which no sweep enables. Verified before the sweeps: `head -v --check --no-cache` prints
  `Cache: disabled (--no-cache)`.
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found). The
  crate and files copies are the ones on this disk from the earlier records (same
  preparation, same bytes: `tiny-test-data` 287 formatted and 289 raw files); the five
  stdin copies were rebuilt as described under Load and are fixed points for every build
  (`stdin-hir-lib` 236,066 bytes and `stdin-config` 169,196, M3's sizes). The registry
  crates `tokio-1.53.3`, `regex-syntax-0.8.11`, `winnow-0.7.15` and `clap_builder-4.6.7`
  were present in `~/.cargo/registry`.
- The results directories (gitignored, under `benchmarks/results/`):

| directory | command |
|---|---|
| `m4-a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `m4-a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `m4-a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `m4-d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `m4-c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `m4-d-inproc` | by-hand `taskset -c 3 bench -n 5 --root <head fixtures>` triples, `benchmarks/profile.sh --runs 2 --core 3` per binary, `taskset -c 3 conform -i` per build (and `-V 1.8.0` for head) |
| `m4-e-parallel-items` | by-hand hyperfine, measurement E |
| `m4-noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |
| `m4-identity` | `env.txt`, `sha256.txt`, `sizes.txt`, `fixed-point.txt`, `loads.tsv` (the per-sweep load and `/proc/stat` lines) |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`); the `base` column shows sweep 1's base. The `ranges` column lists,
  in order, head vs base, head-pgo vs base, head vs rustfmt, head-pgo vs rustfmt: `below` =
  every run of the first command took less wall time than every run of the second; `overlap`
  = the min–max ranges intersect.
- The same `base` binary ran in sweeps 1 and 2: see the `1c` crate bullets for its drift
  between the two windows (0.960x–1.264x on the pinned rows).
- Output agreement in every crate, files and stdin workload, recomputed with this record's
  binaries after the sweeps (the harness's stored figure is stale when a copy is reused; see
  Harness notes): on formatted input `head` equals rustfmt on every file of every workload
  (15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 287/287, 1/1 ×5) and on raw
  input the same with `tiny-test-data` 289/289; `base` differs on one raw file
  (`parser/err/0024_many_type_parens.rs`, the R2 fix) and on no formatted file. Exit codes are 0 for every command on formatted input, 1 for both tools on
  raw input where files change (`reg-clap-builder` and the stdin rows exit 0).

## Crate level, formatted input, `--check` (measurement A)

### Crate level, all CPUs (`mc`), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 87.0 / 102.5 ± 11.4 / 24 | 27.5 / 32.1 ± 3.0 / 18 | 26.1 / 30.4 ± 3.0 / 18 | 283.1 / 311.4 ± 28.0 / 56 | 281.8 / 315.4 ± 38.7 / 57 | 0.313 | 0.293 | 0.103 | 0.096 | below; below; below; below | 219.6 | 106.9 | 96.0 | 306.9 | 0.487 | 0.430 | 0.348 | 0.308 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 176.5 / 209.8 ± 18.1 / 35 | 75.0 / 84.5 ± 6.2 / 19 | 67.6 / 72.9 ± 4.9 / 19 | 856.2 / 915.9 ± 39.5 / 85 | 899.6 / 1013.3 ± 86.5 / 85 | 0.403 | 0.340 | 0.092 | 0.072 | below; below; below; below | 671.0 | 314.2 | 270.8 | 908.0 | 0.468 | 0.400 | 0.346 | 0.270 | b[0]h[0]r[0]p[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 118.4 / 137.9 ± 15.8 / 20 | 29.4 / 35.0 ± 3.6 / 20 | 26.0 / 31.9 ± 3.5 / 20 | 305.2 / 342.2 ± 38.2 / 58 | 308.4 / 351.7 ± 32.5 / 58 | 0.254 | 0.223 | 0.102 | 0.091 | below; below; below; below | 243.1 | 105.9 | 97.2 | 337.5 | 0.436 | 0.380 | 0.314 | 0.281 | b[0]h[0]r[0]p[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 36.2 / 48.9 ± 7.0 / 13 | 16.2 / 17.7 ± 1.0 / 8 | 15.6 / 17.7 ± 1.6 / 8 | 129.1 / 144.5 ± 17.2 / 51 | 133.1 / 155.4 ± 18.3 / 51 | 0.361 | 0.327 | 0.122 | 0.114 | below; below; below; below | 126.5 | 59.3 | 57.4 | 141.7 | 0.468 | 0.417 | 0.418 | 0.377 | b[0]h[0]r[0]p[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 21.6 / 25.8 ± 2.6 / 10 | 10.7 / 12.9 ± 2.0 / 6 | 11.2 / 13.0 ± 1.4 / 7 | 73.3 / 84.5 ± 14.3 / 47 | 78.4 / 89.9 ± 18.4 / 47 | 0.499 | 0.447 | 0.152 | 0.145 | overlap; below; below; below | 72.2 | 39.4 | 38.7 | 83.4 | 0.545 | 0.485 | 0.472 | 0.437 | b[0]h[0]r[0]p[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 13.7 / 18.2 ± 1.9 / 6 | 7.4 / 8.4 ± 0.8 / 5 | 6.9 / 8.0 ± 0.9 / 6 | 41.0 / 47.1 ± 5.9 / 43 | 41.0 / 46.1 ± 6.1 / 43 | 0.462 | 0.396 | 0.179 | 0.175 | below; below; below; below | 39.6 | 22.5 | 20.9 | 45.9 | 0.567 | 0.477 | 0.489 | 0.462 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 118.6 / 143.1 ± 19.2 / 37 | 64.9 / 74.8 ± 6.6 / 31 | 54.3 / 62.6 ± 8.6 / 31 | 449.3 / 494.9 ± 33.5 / 66 | 460.6 / 503.6 ± 31.3 / 66 | 0.523 | 0.435 | 0.151 | 0.124 | below; below; below; below | 445.3 | 248.4 | 208.2 | 489.6 | 0.558 | 0.456 | 0.507 | 0.419 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 52.2 / 70.1 ± 13.1 / 18 | 25.0 / 30.1 ± 3.4 / 12 | 23.7 / 27.2 ± 3.1 / 13 | 193.9 / 214.7 ± 15.8 / 55 | 195.4 / 224.7 ± 26.1 / 55 | 0.430 | 0.382 | 0.140 | 0.121 | below; below; below; below | 195.8 | 95.3 | 86.2 | 211.6 | 0.487 | 0.420 | 0.450 | 0.387 | b[0]h[0]r[0]p[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 60.4 / 70.7 ± 8.3 / 18 | 25.9 / 27.4 ± 1.3 / 12 | 23.9 / 27.3 ± 2.1 / 12 | 182.3 / 208.4 ± 22.6 / 59 | 184.8 / 205.6 ± 14.6 / 58 | 0.387 | 0.321 | 0.131 | 0.133 | below; below; below; below | 214.1 | 93.9 | 91.9 | 205.1 | 0.439 | 0.372 | 0.458 | 0.452 | b[0]h[0]r[0]p[0] |

- `head/base` on all CPUs: 0.254 (`fx-ide-db`) to 0.523 (`reg-regex-syntax`); every head run is
  below every base run on eight crates; the ranges overlap on `fx-parser` (10.7–21.9 vs
  21.6–32.6 ms). M3 read 0.299–0.582 with two overlaps.
- `head/rustfmt` on all CPUs: 0.092 (`fx-hir-ty`) to 0.179 (`reg-tokio`); every head run and
  every head-pgo run is below every rustfmt run on all nine crates.
- `pgo/base` on all CPUs: 0.223 (`fx-ide-db`) to 0.447 (`fx-parser`), every head-pgo run below
  every base run on all nine crates; `pgo/rustfmt` 0.072 (`fx-hir-ty`) to 0.175 (`reg-tokio`).
  `head-pgo` is below `head` in mean on seven crates (0.837x on `reg-regex-syntax`, 0.863x on
  `fx-hir-ty`, 0.903x–0.955x on four, 0.996x on `reg-winnow`) and level with it on `fx-syntax`
  (17.7 vs 17.7 ms) and `fx-parser` (13.0 vs 12.9 ms): with four threads the per-file split
  hides part of the single-thread gain the `1c` rows show.
- CPU `head/base` on all CPUs: 0.436 (`fx-ide-db`) to 0.567 (`reg-tokio`); CPU `pgo/base`
  0.372–0.485. The CPU ratio is above the wall ratio on every crate (the 4-thread rows pay the
  E1 split in CPU and get it back in wall).
- Peak RSS on all CPUs: head 5–31 MiB against base 6–37 MiB and rustfmt 43–85 MiB; head is
  below base on eight crates and 0.3 MiB above it on `fx-ide-db` (19.8 vs 19.5 MiB); head-pgo
  6–31 MiB, below sweep 2's base on eight crates and 0.2 MiB below it on `fx-ide-db`.

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 183.0 / 202.2 ± 18.8 / 16 | 83.6 / 86.3 ± 3.6 / 9 | 77.2 / 83.7 ± 10.6 / 10 | 275.0 / 302.6 ± 27.3 / 56 | 279.4 / 311.6 ± 29.3 / 56 | 0.427 | 0.398 | 0.285 | 0.269 | below; below; below; below | 201.1 | 85.9 | 82.7 | 298.4 | 0.427 | 0.399 | 0.288 | 0.269 | b[0]h[0]r[0]p[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 577.6 / 614.5 ± 30.8 / 18 | 261.1 / 283.0 ± 16.3 / 8 | 234.5 / 256.3 ± 20.6 / 9 | 846.8 / 905.9 ± 38.3 / 85 | 879.6 / 980.6 ± 73.0 / 85 | 0.461 | 0.389 | 0.312 | 0.261 | below; below; below; below | 611.4 | 280.2 | 254.7 | 899.4 | 0.458 | 0.390 | 0.311 | 0.263 | b[0]h[0]r[0]p[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 213.3 / 224.7 ± 17.2 / 15 | 82.6 / 85.3 ± 2.7 / 10 | 74.8 / 81.4 ± 3.3 / 11 | 300.6 / 323.6 ± 23.6 / 58 | 308.2 / 334.8 ± 29.7 / 58 | 0.379 | 0.362 | 0.264 | 0.243 | below; below; below; below | 222.8 | 84.7 | 80.3 | 320.5 | 0.380 | 0.360 | 0.264 | 0.242 | b[0]h[0]r[0]p[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 109.6 / 120.7 ± 15.3 / 9 | 48.3 / 52.5 ± 5.6 / 6 | 44.4 / 46.3 ± 1.2 / 6 | 128.2 / 139.5 ± 20.8 / 51 | 126.7 / 141.3 ± 11.7 / 51 | 0.435 | 0.394 | 0.377 | 0.328 | below; below; below; below | 119.1 | 51.5 | 45.9 | 138.7 | 0.432 | 0.394 | 0.371 | 0.328 | b[0]h[0]r[0]p[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 63.3 / 68.2 ± 4.8 / 7 | 29.6 / 30.8 ± 0.8 / 5 | 28.0 / 32.3 ± 5.6 / 5 | 74.0 / 78.5 ± 9.4 / 47 | 75.3 / 91.7 ± 13.5 / 47 | 0.451 | 0.416 | 0.392 | 0.353 | below; below; below; below | 67.2 | 30.5 | 31.7 | 77.7 | 0.454 | 0.415 | 0.393 | 0.351 | b[0]h[0]r[0]p[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 33.7 / 35.6 ± 4.1 / 6 | 17.1 / 21.2 ± 2.9 / 4 | 16.2 / 17.1 ± 0.8 / 5 | 41.3 / 48.5 ± 7.4 / 43 | 43.6 / 52.6 ± 8.8 / 43 | 0.595 | 0.381 | 0.437 | 0.326 | below; below; below; below | 35.3 | 20.8 | 16.7 | 47.6 | 0.590 | 0.381 | 0.437 | 0.327 | b[0]h[0]r[0]p[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 402.9 / 444.2 ± 34.2 / 29 | 199.2 / 223.4 ± 20.7 / 14 | 167.1 / 195.4 ± 29.4 / 15 | 442.0 / 503.3 ± 47.0 / 66 | 450.5 / 517.3 ± 65.3 / 66 | 0.503 | 0.421 | 0.444 | 0.378 | below; below; below; below | 441.1 | 221.4 | 194.0 | 497.9 | 0.502 | 0.422 | 0.445 | 0.379 | b[0]h[0]r[0]p[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 167.1 / 184.2 ± 27.2 / 12 | 74.9 / 80.5 ± 7.6 / 6 | 67.4 / 77.9 ± 10.6 / 7 | 195.9 / 210.9 ± 15.3 / 55 | 191.7 / 219.2 ± 26.3 / 55 | 0.437 | 0.427 | 0.381 | 0.356 | below; below; below; below | 182.2 | 79.8 | 76.7 | 208.8 | 0.438 | 0.426 | 0.382 | 0.353 | b[0]h[0]r[0]p[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 191.2 / 213.2 ± 19.6 / 11 | 77.8 / 89.1 ± 12.3 / 7 | 70.2 / 79.2 ± 10.8 / 8 | 181.6 / 203.5 ± 22.4 / 59 | 179.4 / 206.7 ± 29.4 / 58 | 0.418 | 0.387 | 0.438 | 0.383 | below; below; below; below | 209.2 | 87.8 | 78.3 | 200.8 | 0.420 | 0.386 | 0.437 | 0.383 | b[0]h[0]r[0]p[0] |

- Per core (both tools on core 3), `head/base` wall: 0.379 (`fx-ide-db`) to 0.595
  (`reg-tokio`); CPU `head/base` 0.380–0.590. Every head run is below every base run on all
  nine crates. The CPU ratio and the wall ratio agree to within 0.01 on every row (the pinned
  process is not waiting for its core). Without `reg-tokio` (head 21.2 ± 2.9 ms with a
  17.1 ms minimum; its head-pgo row reads 17.1 ± 0.8) the range is 0.379–0.503.
- Per core, `head/rustfmt` wall: 0.264 (`fx-ide-db`) to 0.444 (`reg-regex-syntax`); CPU
  0.264–0.445. Every head run is below every rustfmt run on all nine crates. M3 read 0.324–0.540.
- Per core, `pgo/base` wall: 0.362 (`fx-ide-db`) to 0.427 (`reg-clap-builder`), every head-pgo
  run below every base run on all nine crates; `pgo/rustfmt` 0.243 (`fx-ide-db`) to 0.383
  (`reg-winnow`), every head-pgo run below every rustfmt run on all nine. The PGO gain against
  head on one core is 0.808x (`reg-tokio`) to 0.970x (`fx-hir`) in mean on eight crates and
  1.050x on `fx-parser` (32.3 ± 5.6 against 30.8 ± 0.8 ms; the head-pgo run has a 50.2 ms
  maximum against a 28.0 ms minimum).
- Peak RSS on one core: head 4–14 MiB against base 6–29 MiB and rustfmt 43–85 MiB; head is below
  base on every crate; head-pgo 5–15 MiB, below sweep 2's base on every crate and 0.6–0.8 MiB
  above head.
- The same `base` binary ran in sweeps 1 and 2: its crate means in sweep 2 lie within
  0.960x–1.264x of sweep 1 on the pinned rows (`reg-winnow` 204.7 vs 213.2 ms; `reg-tokio` 45.0
  vs 35.6 ms) and 1.007x–1.204x unpinned; sweep 2 ran under a higher load average (1.95 at its
  start against 0.87, the tail of sweep 1's 4-thread rows), which is why every `pgo/base`
  ratio is taken within sweep 2.

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 12.7 / 17.6 ± 2.9 / 5 | 6.2 / 7.8 ± 0.7 / 5 | 6.2 / 7.4 ± 0.8 / 5 | 52.1 / 61.6 ± 11.3 / 44 | 52.7 / 64.9 ± 12.8 / 44 | 0.442 | 0.418 | 0.126 | 0.113 | below; below; below; below | 34.7 | 17.8 | 16.4 | 60.4 | 0.514 | 0.474 | 0.295 | 0.256 | b[0]h[0]r[0]p[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 22.1 / 22.9 ± 0.5 / 5 | 12.3 / 12.9 ± 0.5 / 5 | 11.7 / 13.3 ± 1.5 / 5 | 50.6 / 54.1 ± 5.4 / 44 | 53.6 / 61.6 ± 7.8 / 44 | 0.565 | 0.518 | 0.239 | 0.216 | below; below; below; below | 22.6 | 12.7 | 12.8 | 53.8 | 0.563 | 0.508 | 0.236 | 0.213 | b[0]h[0]r[0]p[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 1.9 / 2.3 ± 0.4 / 4 | 2.1 / 2.4 ± 0.4 / 4 | 2.0 / 2.3 ± 0.3 / 4 | 12.0 / 14.4 ± 1.8 / 34 | 11.9 / 13.7 ± 1.8 / 34 | 1.042 | 0.914 | 0.169 | 0.169 | overlap; overlap; below; below | 2.1 | 2.2 | 2.1 | 13.9 | 1.058 | 0.962 | 0.159 | 0.160 | b[0]h[0]r[0]p[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.8 / 1.9 ± 0.1 / 4 | 1.7 / 1.8 ± 0.1 / 4 | 1.8 / 2.2 ± 0.9 / 4 | 10.7 / 11.9 ± 1.8 / 34 | 11.5 / 13.4 ± 2.8 / 34 | 0.965 | 1.119 | 0.153 | 0.164 | overlap; overlap; below; below | 1.8 | 1.7 | 1.9 | 11.4 | 0.962 | 1.030 | 0.148 | 0.150 | b[0]h[0]r[0]p[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 83.3 / 94.4 ± 15.8 / 10 | 29.9 / 32.6 ± 3.5 / 9 | – | 138.4 / 146.6 ± 10.9 / 43 | – | 0.345 | – | 0.222 | – | below; –; below; – | 93.6 | 32.3 | – | 145.2 | 0.345 | – | 0.222 | – | b[0]h[0]r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 79.8 / 84.3 ± 8.8 / 14 | 42.1 / 46.1 ± 8.6 / 13 | – | 86.4 / 92.3 ± 2.9 / 53 | – | 0.547 | – | 0.500 | – | below; –; below; – | 84.0 | 45.8 | – | 91.6 | 0.545 | – | 0.500 | – | b[0]h[0]r[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 64.1 / 69.1 ± 5.6 / 9 | 33.5 / 34.4 ± 0.5 / 8 | – | 87.7 / 91.5 ± 5.0 / 47 | – | 0.499 | – | 0.376 | – | below; –; below; – | 67.9 | 34.2 | – | 90.4 | 0.504 | – | 0.379 | – | b[0]h[0]r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 17.4 / 19.3 ± 2.4 / 5 | 8.4 / 9.6 ± 1.6 / 4 | – | 30.9 / 32.9 ± 3.7 / 40 | – | 0.499 | – | 0.292 | – | below; –; below; – | 18.9 | 9.3 | – | 32.3 | 0.493 | – | 0.289 | – | b[0]h[0]r[0] |
| stdin-config | 1c (3) | 1 | 169 | 30 | 26.2 / 27.4 ± 0.6 / 6 | 13.9 / 15.3 ± 2.1 / 5 | – | 42.6 / 45.4 ± 1.9 / 44 | – | 0.557 | – | 0.337 | – | below; –; below; – | 27.1 | 15.0 | – | 45.0 | 0.554 | – | 0.333 | – | b[0]h[0]r[0] |

- `tiny-one` (one 13-byte file, process start-up): head 1.8 ms pinned and 2.4 ms unpinned
  against base 1.9 and 2.3 ms; head-pgo 2.2 and 2.3 ms; rustfmt 11.9 and 14.4 ms. `head/base`
  reads 1.042 unpinned and 0.965 pinned, `pgo/base` 0.914 and 1.119 (the pinned head-pgo row
  has one 6.4 ms run against a 1.8 ms minimum; σ 0.9 ms); every one of these has overlapping
  ranges, as in M3. Every head and head-pgo run is below every rustfmt run.
- `tiny-test-data` (287 files): head/base 0.442 unpinned and 0.565 pinned, every head run
  below every base run in both; pgo/base 0.418 and 0.518 (below in both); head/rustfmt 0.126
  and 0.239, every head run below every rustfmt run. M3 read 0.605 and 0.672 against base:
  E12's per-file work (one canonicalisation per root, one sort) shows most on the
  many-small-files row.
- stdin (`examples/fmt_stdin`, no threads, no file I/O, no cache code, no PGO build):
  head/base 0.345 (`stdin-lints`) to 0.557 (`stdin-config`); every head run is below every
  base run on all five files (M3: four of five). head/rustfmt 0.222 (`stdin-lints`) to 0.500
  (`stdin-nodes`), every head run below every rustfmt run on all five files. CPU ratios equal
  the wall ratios to within 0.01. Against M3's stdin rows (0.412–0.619 of base) the same bytes
  read 0.345–0.557.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 625 | 30 | 578.7 / 632.2 ± 46.8 / 218 | 31.4 / 34.9 ± 3.0 / 17 | 0.055 | below | 626.8 | 115.3 | 0.184 | 15/15 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 549.0 / 601.7 ± 41.8 / 217 | 86.5 / 95.9 ± 12.0 / 9 | 0.159 | below | 595.0 | 93.6 | 0.157 | 15/15 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1240.2 / 1318.7 ± 60.6 / 116 | 76.1 / 87.6 ± 9.3 / 19 | 0.066 | below | 1306.2 | 321.3 | 0.246 | 122/122 | r[1] h[1] |
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1187.1 / 1337.6 ± 80.1 / 116 | 269.6 / 294.9 ± 21.9 / 8 | 0.220 | below | 1326.2 | 290.9 | 0.219 | 122/122 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 350.4 / 388.9 ± 43.2 / 66 | 30.6 / 36.3 ± 4.7 / 19 | 0.093 | below | 384.4 | 108.1 | 0.281 | 33/33 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 339.5 / 372.4 ± 30.9 / 66 | 84.5 / 90.8 ± 4.9 / 10 | 0.244 | below | 367.1 | 89.6 | 0.244 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 165.2 / 201.3 ± 31.2 / 57 | 16.7 / 19.5 ± 1.5 / 8 | 0.097 | below | 199.2 | 65.8 | 0.330 | 33/33 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 162.3 / 203.7 ± 42.0 / 57 | 49.1 / 52.1 ± 7.1 / 6 | 0.256 | below | 202.4 | 51.8 | 0.256 | 33/33 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 88.5 / 101.8 ± 15.0 / 48 | 12.9 / 15.3 ± 1.6 / 6 | 0.151 | below | 100.2 | 46.4 | 0.464 | 28/28 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 84.7 / 87.8 ± 1.7 / 48 | 30.4 / 38.5 ± 7.7 / 5 | 0.439 | below | 87.2 | 37.4 | 0.429 | 28/28 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 42.1 / 52.0 ± 9.9 / 43 | 7.7 / 9.3 ± 1.2 / 5 | 0.178 | below | 51.4 | 24.5 | 0.476 | 49/49 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 41.7 / 45.0 ± 6.0 / 43 | 16.5 / 17.4 ± 1.1 / 4 | 0.385 | below | 44.6 | 16.9 | 0.379 | 49/49 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1392.4 / 1531.6 ± 103.9 / 423 | 64.9 / 81.6 ± 10.1 / 30 | 0.053 | below | 1520.1 | 275.5 | 0.181 | 33/33 | r[1] h[1] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1415.4 / 1517.6 ± 90.1 / 423 | 201.5 / 215.3 ± 21.1 / 14 | 0.142 | below | 1507.1 | 213.7 | 0.142 | 33/33 | r[1] h[1] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 189.5 / 220.4 ± 27.6 / 55 | 24.2 / 27.9 ± 3.2 / 12 | 0.127 | below | 218.3 | 91.4 | 0.419 | 57/57 | r[0] h[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 196.4 / 210.9 ± 17.0 / 55 | 75.0 / 80.2 ± 8.7 / 6 | 0.380 | below | 209.5 | 79.5 | 0.379 | 57/57 | r[0] h[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 196.9 / 225.2 ± 22.7 / 60 | 25.7 / 28.8 ± 3.2 / 12 | 0.128 | below | 221.4 | 98.7 | 0.446 | 61/61 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 194.7 / 220.4 ± 38.3 / 59 | 80.9 / 89.1 ± 12.8 / 7 | 0.404 | below | 218.4 | 87.3 | 0.400 | 61/61 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 62.9 / 72.1 ± 12.7 / 44 | 6.5 / 7.3 ± 1.1 / 5 | 0.101 | below | 70.8 | 17.2 | 0.243 | 288/289 | r[1] h[1] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 63.6 / 69.4 ± 10.4 / 44 | 12.7 / 14.8 ± 2.2 / 4 | 0.213 | below | 68.4 | 14.4 | 0.211 | 288/289 | r[1] h[1] |
| tiny-one | mc (none) | 1 | 0 | 30 | 11.2 / 13.6 ± 2.1 / 34 | 2.1 / 3.3 ± 2.4 / 4 | 0.241 | overlap | 12.9 | 2.7 | 0.206 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.4 / 11.8 ± 0.3 / 34 | 1.7 / 1.8 ± 0.1 / 4 | 0.152 | below | 11.6 | 1.7 | 0.143 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 139.3 / 152.6 ± 15.9 / 43 | 30.3 / 34.4 ± 6.1 / 9 | 0.225 | below | 150.1 | 33.3 | 0.222 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 89.7 / 99.3 ± 10.3 / 53 | 41.5 / 44.7 ± 4.0 / 13 | 0.450 | below | 97.9 | 44.2 | 0.451 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 91.1 / 100.0 ± 10.1 / 47 | 32.7 / 35.2 ± 5.1 / 8 | 0.352 | below | 99.0 | 34.9 | 0.352 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 31.9 / 34.0 ± 4.4 / 40 | 8.6 / 10.8 ± 2.6 / 4 | 0.317 | below | 33.5 | 10.1 | 0.300 | 1/1 | r[0] h[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 45.5 / 50.5 ± 6.2 / 44 | 14.4 / 15.8 ± 1.6 / 5 | 0.314 | below | 49.7 | 15.2 | 0.306 | 1/1 | r[0] h[0] |

- rustfmt `--check` on unformatted input includes the line diff of every changed file:
  `reg-regex-syntax` 1517.6 ms and 423 MiB pinned against 503.3 ms and 66 MiB on formatted
  input (sweep 1); `fx-hir` 601.7 ms and 217 MiB against 302.6 ms and 56 MiB. head's figures
  on the two inputs differ by less than 20% on every row but the 13-byte file (the raw/formatted
  mean ratio lies between 0.819 on `reg-tokio` `1c` and 1.188 on `fx-parser` `1c`; `tiny-one`
  `mc` reads 1.336 on a 3.3 ± 2.4 ms mean with one slow run).
- Every head run is below every rustfmt run on 26 of the 27 raw rows; `tiny-one` `mc` overlaps
  (2.1–14.9 vs 11.2–20.0 ms: one head run of 14.9 ms against a 2.1 ms minimum).
- The `identical outputs` column of this table is the harness's figure, which it computes
  when it prepares a copy and stores in the copy's `meta`; the raw copies on this disk were
  prepared on 2026-10-14 with the binary of that day, so `tiny-test-data` shows 288/289 (M2's
  figure, before the R2 fixes). Recomputed on the same bytes with this record's `head`
  (rustfmt and `head --write --no-cache` on two copies, `diff -rq`): 289 of 289 identical;
  `base` differs on `parser/err/0024_many_type_parens.rs`, the file R2 fixed. The agreement
  counts for every workload and both inputs, recomputed with this record's binaries after the
  sweeps, are under Harness notes.
- The ratios in this table (0.053–0.450) are not like-for-like and are not the ones to quote;
  the formatted-input tables above are.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 13,901,559 bytes)
  copied to `/tmp/chloro-bench/m4-cache/tree` and rewritten once by `head --write --no-cache`;
  `head --check --no-cache` and `base --check` on the result exit 0. `CHLORO_CACHE_DIR` points
  at a scratch directory, which after a priming run holds the cache file
  `3d84c0c30d3a4a38ee7e7e02e31663d1.chloro-cache` of 44,184 bytes (the name is the key of this
  binary and configuration; M3's build wrote `56497adc…` of 44,152 bytes) and, since E12, the
  executable's identity record `b1154dec39779d683654cfa9d8db3191.chloro-exe` of 180 bytes.
  `head -v --check` on the copy reports `(1226 entries)` and `Unit cache: off`; the 1238
  fixtures hold 1226 distinct contents by sha256 (M3 counted 1230 with a different hash set;
  the entries equal the distinct contents in both records). The `head-pgo` binary writes its
  own pair (`18a31c31….chloro-cache`, 44,184 bytes; a 184-byte `.chloro-exe`).
- Commands: `hyperfine -N -i -w 1 -r 15 --metrics … -n cold --prepare "rm -rf $C" "head
  --check $T" -n warm --prepare true "head --check $T" -n no-cache --prepare true "head --check
  --no-cache $T" -n base --prepare true "base --check $T"`, two rounds with the order reversed
  (n = 30); then one round under `taskset -c 3` for `warm`, `no-cache`, `base` (n = 15); then
  one unpinned round for `head-pgo` (`pgo-warm`, `pgo-no-cache`, n = 15, its own cache
  directory). `cold` is not in the pinned round.

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 355.6 / 389.0 ± 26.4 / 48 | 12.8 / 16.9 ± 3.4 / 5 | 341.4 / 372.7 ± 21.3 / 44 | 800.9 / 909.4 ± 83.0 / 53 | 22.1 | 53.8 | 0.410 | 1448.7 | 29.9 | 1396.1 | 3188.3 |
| 1c | 15 | – | 22.5 / 24.4 ± 2.1 / 4 | 1235.6 / 1305.9 ± 60.9 / 23 | 2881.6 / 3069.7 ± 134.1 / 33 | 53.5 | 125.7 | 0.425 | – | 23.6 | 1298.4 | 3049.1 |

| variant | n | pgo-warm | pgo-no-cache | no-cache/warm | CPU pgo-warm | CPU pgo-no-cache |
|---|---|---|---|---|---|---|
| mc | 15 | 12.0 / 13.9 ± 1.3 / 5 | 301.3 / 316.9 ± 10.4 / 40 | 22.8 | 27.2 | 1195.7 |

- Warm `--check` of the 1238 formatted files takes 16.9 ms unpinned (22.1x below `--no-cache`,
  53.8x below base) and 24.4 ms on one core (53.5x below `--no-cache`, 125.7x below base); CPU
  29.9 and 23.6 ms; peak RSS 5 and 4 MiB against 44 and 23 MiB without the cache. M3 measured
  28.9 and 37.1 ms on the same copy's bytes; the 12.0 and 12.7 ms removed are E12's work (one
  canonicalisation per root instead of per file, one sort, the executable's digest read from
  the identity record instead of hashed from the 2 MB binary on every run).
- `cold` (cache directory removed before every run) reads 389.0 ms against 372.7 ms for
  `no-cache` in mean (1.044x; ranges 355.6–489.0 vs 341.4–423.8 ms overlap): hashing the
  files and the binary and writing the cache and the identity record add 16 ms in mean over
  the formatter on 4 threads in this window (M3: 12 ms).
- `no-cache` head against base on the same copy: 0.410 unpinned (372.7 vs 909.4 ms) and 0.425
  pinned (1305.9 vs 3069.7 ms); CPU 0.438 and 0.426. M3 read 0.518 and 0.536. Peak RSS of
  `cold` (48 MiB) and `no-cache` (44 MiB) is below base's 53 MiB unpinned; pinned, 23 MiB
  against 33 MiB.
- The profile-guided build: warm 13.9 ms unpinned (CPU 27.2 ms), `--no-cache` 316.9 ms (0.850x
  head's 372.7 ms in the earlier round of the same sweep; peak RSS 40 against 44 MiB).
- The base means in this sweep (909.4 ms unpinned, 3069.7 pinned) are above M3's (838.8,
  2916.9) for the same binary and bytes; the load average at this sweep's end read 2.70 (its
  own 4-thread processes). The ratios are within-window.

## In process and instructions (measurement D)

`taskset -c 3 bench -n 5 --root chloro-core/tests/conformance/fixtures` (best of 5 in-process
formats of the 1238 fixtures, 13.9 MB; the same tree for all three binaries), three
interleaved triples in the order base, head, head-pgo:

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 2.798 | 4.97 |
| 1 | head | 1238 | 13.9 | 1.203 | 11.56 |
| 1 | head-pgo | 1238 | 13.9 | 1.059 | 13.13 |
| 2 | base | 1238 | 13.9 | 2.803 | 4.96 |
| 2 | head | 1238 | 13.9 | 1.179 | 11.79 |
| 2 | head-pgo | 1238 | 13.9 | 1.050 | 13.24 |
| 3 | base | 1238 | 13.9 | 2.731 | 5.09 |
| 3 | head | 1238 | 13.9 | 1.239 | 11.22 |
| 3 | head-pgo | 1238 | 13.9 | 1.067 | 13.03 |

- base: 3 runs, best time 2.731–2.803 s, 4.96–5.09 MB/s
- head: 3 runs, best time 1.179–1.239 s, 11.22–11.79 MB/s
- head-pgo: 3 runs, best time 1.050–1.067 s, 13.03–13.24 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, each binary's
compiled-in fixture root, `chloro` = head, `chloro-base` = base:

- chloro: 1238 files, 13.9 MB: 1.279s, 10.87 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.648s, 5.25 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.762s, 5.03 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.203s, 11.56 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.191s, 11.68 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.813s, 4.94 MB/s (best of 5)

### Instructions on the 49-file sample (`benchmarks/profile.sh --runs 2 --core 3`)

| binary | run 1 Ir | run 2 Ir | Ir / base | Ir / head |
|---|---|---|---|---|
| base | 587,989,243 | 587,989,266 | 1.0000 | |
| head | 248,202,137 | 248,203,822 | 0.4221 | 1.0000 |
| head-pgo | 206,922,054 | 206,922,104 | 0.3519 | 0.8337 |

- `bench -n 5`, three interleaved triples: base 4.96–5.09 MB/s, head 11.22–11.79 MB/s, head-pgo
  13.03–13.24 MB/s. The three ranges are pairwise disjoint (protocol §3 rule for a
  single-threaded change: 3 of 3 pairs, non-overlapping); head/base 2.20x–2.38x on the
  per-round MB/s, head-pgo/head 1.11x–1.16x. The harness's own three pairs (same core, 04:49
  UTC) give base 4.94–5.25 and head 10.87–11.68 MB/s, non-overlapping. M3 measured base
  5.22–5.27 and head 9.39–9.54 on this host class; base reads 3–5% lower in this window, so
  the head/base ratio (2.2–2.4x against M3's 1.8x) carries that drift, and the within-window
  ratio of the two heads' instruction counts (0.794) is the better guide to tonight's change
  than the two records' MB/s.
- `conform -i` (hyperfine, 9 runs each, core 3, harness): head 2587.2 ± 86.6 ms
  (2468.2–2713.6) against base 5703.2 ± 99.2 ms (5555.8–5868.9), 0.454, every head run below
  every base run; CPU 2564.8 vs 5656.6 ms; peak RSS 46 vs 50 MiB; exit 0 on every run. By
  hand (`taskset -c 3 conform -i`, one run each): head **1220/1220 identical**, 0 not
  idempotent, `rustfmt_version=1.9.0`, diff lines +0 −0, 13.8 MB in 1.286 s (10.7 MB/s),
  2.638 s wall (user 2.561 s); at `-V 1.8.0` **1220/1220 identical**, 0 not idempotent, 13.8 MB
  in 1.389 s (9.9 MB/s), 2.846 s wall; base 1219/1220 identical, 0 not idempotent, diff lines
  +2 −2, 13.8 MB in 2.736 s (5.0 MB/s), 5.512 s wall.
- `Ir` on the 49-file sample (650,213 bytes, `env -i`, `setarch -R`): base 587,989,243, head
  248,202,137 (−57.79%), head-pgo 206,922,054 (−64.81%; 0.8337x head); two runs of each agree
  to within 23 instructions (base), 1,685 (head) and 50 (head-pgo). The base count equals M3's
  run 1 (587,989,199) to within 44 and M2's 587,989,243 exactly; the head count's run 1 is 43
  above the 248,202,094 the ledger records for `e640fbf`. The profile-guided count is this
  training run's: M3's PGO build of M3's head read 0.7825x its head and E13's reproduction
  0.784x; here 0.8337x, so the profile is worth less on the E16–E18 code (whose hot paths the
  levers already shortened) than it was before them.

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), head with `--no-cache`, `hyperfine -N -i
-w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no `--threads`
option; on one file its static chunking gives the file to one thread, so the `base` column is
the sequential figure with the base's codegen. Sizes in KB (bytes / 1000).

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 89.7 / 108.2 ± 18.6 / 10 | 31.1 / 36.0 ± 5.0 / 10 | 24.0 / 31.4 ± 4.3 / 15 | 0.290 | 0.332 | below | 106.7 | 35.2 | 46.4 | 0.434 | b[0] t[0] h[0] |
| stdin-lints | 640 | 1c | 30 | 89.6 / 105.3 ± 19.8 / 10 | 33.1 / 38.0 ± 8.0 / 10 | 33.3 / 36.2 ± 4.2 / 10 | 0.344 | 0.361 | below | 104.3 | 37.5 | 34.6 | 0.332 | b[0] t[0] h[0] |
| stdin-nodes | 358 | mc | 30 | 83.0 / 92.5 ± 13.3 / 14 | 43.8 / 51.0 ± 9.1 / 14 | 33.1 / 38.3 ± 7.0 / 14 | 0.414 | 0.551 | below | 91.6 | 50.1 | 60.0 | 0.655 | b[0] t[0] h[0] |
| stdin-nodes | 358 | 1c | 30 | 84.2 / 89.3 ± 8.9 / 14 | 44.9 / 49.8 ± 5.6 / 14 | 45.3 / 50.7 ± 7.6 / 14 | 0.568 | 0.557 | below | 88.4 | 49.0 | 49.9 | 0.565 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | mc | 30 | 68.0 / 76.8 ± 10.0 / 9 | 34.7 / 40.4 ± 7.0 / 9 | 21.3 / 24.3 ± 2.9 / 9 | 0.317 | 0.525 | below | 75.9 | 39.6 | 43.0 | 0.566 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | 1c | 30 | 68.7 / 76.1 ± 11.7 / 9 | 36.5 / 37.8 ± 1.7 / 9 | 36.3 / 41.0 ± 6.4 / 9 | 0.538 | 0.497 | below | 75.4 | 37.3 | 40.4 | 0.536 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | mc | 30 | 19.4 / 23.0 ± 4.1 / 6 | 9.7 / 11.4 ± 2.1 / 5 | 6.9 / 8.0 ± 1.1 / 5 | 0.350 | 0.497 | below | 21.9 | 10.8 | 12.9 | 0.588 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | 1c | 30 | 20.1 / 23.2 ± 4.5 / 6 | 10.9 / 13.1 ± 2.2 / 5 | 11.4 / 12.5 ± 1.2 / 5 | 0.539 | 0.566 | below | 22.7 | 12.6 | 12.1 | 0.536 | b[0] t[0] h[0] |
| stdin-config | 169 | mc | 30 | 27.2 / 33.9 ± 7.2 / 6 | 14.8 / 17.3 ± 3.3 / 5 | 10.6 / 14.1 ± 2.3 / 6 | 0.415 | 0.511 | below | 33.4 | 16.9 | 21.5 | 0.643 | b[0] t[0] h[0] |
| stdin-config | 169 | 1c | 30 | 28.0 / 30.2 ± 3.3 / 6 | 15.5 / 18.5 ± 3.1 / 5 | 15.5 / 18.5 ± 3.0 / 5 | 0.614 | 0.615 | below | 29.7 | 18.1 | 18.0 | 0.607 | b[0] t[0] h[0] |

- Unpinned, `head/base` in mean: 0.290 (`stdin-lints`), 0.317 (`stdin-hir-lib`), 0.350
  (`stdin-hover-tests`), 0.414 (`stdin-nodes`), 0.415 (`stdin-config`); on the minima
  2.51x–3.73x (base min / head min: 89.7/24.0, 83.0/33.1, 68.0/21.3, 19.4/6.9, 27.2/10.6). Every
  head run is below every base run on all five files, pinned and unpinned (M3: all five
  unpinned, four pinned).
- `head --threads 1` against base, unpinned: 0.332–0.551 (the codegen and the E4, E7b, E3,
  E16, E17 and E18 substrate on one thread; M3 read 0.412–0.670); pinned: 0.361–0.615.
- The split against `head --threads 1`, unpinned: wall 0.603x (`stdin-hir-lib`, 24.3 vs
  40.4 ms) to 0.872x (`stdin-lints`, 31.4 vs 36.0 ms), the ranges below on `stdin-hir-lib` and
  overlapping on the other four; CPU 1.09x (`stdin-hir-lib`) to 1.32x (`stdin-lints`, 46.4 vs
  35.2 ms). M3 read the split's CPU cost at 1.03x–1.19x; with E16's cheaper per-item work the
  fixed cost of the split (the item scan, the per-unit replay, four threads' start-up) is a
  larger share.
- CPU of the split, unpinned `head` against base: 0.434 (`stdin-lints`) to 0.655
  (`stdin-nodes`); every unpinned head row is below base in CPU. Peak RSS head vs base: 1.41x
  on `stdin-lints` (14.7 vs 10.4 MiB, the E1 split, M3's 1.44x), 0.96x–1.07x on the other four.
- Pinned (4 threads on one core), `head/base` in mean: 0.344–0.614; `head` against `head
  --threads 1` pinned: 0.95 (`stdin-lints`, `stdin-hover-tests`) to 1.08 (`stdin-hir-lib`,
  41.0 vs 37.8 ms), ranges overlapping on all five.
- Exit codes: `base`, `head-t1` and `head` exit 0 on every run of every file (the five copies
  are fixed points for every build; see Method). hyperfine warned twice that a first timed run
  of `base` was slower than the rest (25.1 and 23.7 ms on `stdin-hover-tests` and
  `stdin-config`); those runs are in the n = 30.

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload; runs 1–3 are
`compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`
back to back at the end of the chain.

| run | 1c rustfmt | 1c head | 1c base | 1c head/base | 1c head/rustfmt | mc rustfmt | mc head | mc base | mc head/base | mc head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 846.8 / 905.9 ± 38.3 | 261.1 / 283.0 ± 16.3 | 577.6 / 614.5 ± 30.8 | 0.461 | 0.312 | 856.2 / 915.9 ± 39.5 | 75.0 / 84.5 ± 6.2 | 176.5 / 209.8 ± 18.1 | 0.403 | 0.092 |
| 1 | 846.7 / 915.2 ± 42.0 | 262.0 / 279.0 ± 17.8 | 600.0 / 645.3 ± 49.1 | 0.432 | 0.305 | 857.4 / 940.3 ± 65.7 | 73.7 / 80.8 ± 8.1 | 191.3 / 209.0 ± 12.6 | 0.387 | 0.086 |
| 2 | 865.2 / 948.3 ± 53.8 | 259.2 / 280.5 ± 20.1 | 589.8 / 638.7 ± 28.9 | 0.439 | 0.296 | 863.3 / 951.6 ± 60.2 | 72.5 / 80.5 ± 6.5 | 181.0 / 219.3 ± 20.0 | 0.367 | 0.085 |
| 3 | 879.9 / 965.2 ± 46.5 | 265.3 / 297.2 ± 30.2 | 605.3 / 675.0 ± 74.7 | 0.440 | 0.308 | 864.9 / 969.2 ± 56.8 | 74.9 / 82.2 ± 5.1 | 183.9 / 212.0 ± 16.2 | 0.388 | 0.085 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 279.0–297.2 ms
  (6%), the `1c` base mean 614.5–675.0 ms (10%), the `1c` rustfmt mean 905.9–965.2 ms (7%);
  the `mc` head mean 80.5–84.5 ms (5%) and the `mc` base mean 209.0–219.3 ms (5%). The `1c`
  head/base ratio reads 0.432–0.461 and the `mc` ratio 0.367–0.403; head/rustfmt reads
  0.296–0.312 (`1c`) and 0.085–0.092 (`mc`). Every head run is below every base run and every
  rustfmt run in all four runs, pinned and unpinned.
- The three noise runs came at the end of the chain under its highest load averages (1.76–1.91
  at their ends; the `mc` rows of the chain's own earlier sweeps); the base and rustfmt means
  drift upward across them (base `1c` 645 → 675 ms) while head's stay within 18 ms.
- Within one run of 30, max/min on this host reaches 1.50x for rustfmt (`fx-parser` `1c`
  74.0–127.1 ms, sweep 1), 1.37x for base (`fx-parser` `1c` 63.3–87.0 ms) and 2.05x for head
  (`fx-parser` `mc` 10.7–21.9 ms); σ/mean lies between 1.2% and 13.8% on the pinned crate rows
  of the three commands (`fx-hir-ty` base the narrowest at 5.0%, `reg-tokio` head the widest
  at 13.7%) and between 4.7% and 15.5% on the unpinned head crate rows (`fx-parser` the widest).
- Instruction counts move by at most 1,685 of 248 M (0.0007%) between runs.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir` 1c: head vs base: below (83.6–103.6 vs 183.0–254.5 ms); head vs rustfmt: below (83.6–103.6 vs 275.0–379.0 ms); head-pgo vs base: below (77.2–119.6 vs 192.2–288.0 ms); head-pgo vs rustfmt: below (77.2–119.6 vs 279.4–388.4 ms).
- `fx-hir` mc: head vs base: below (27.5–39.4 vs 87.0–128.1 ms); head vs rustfmt: below (27.5–39.4 vs 283.1–388.4 ms); head-pgo vs base: below (26.1–39.5 vs 87.2–131.8 ms); head-pgo vs rustfmt: below (26.1–39.5 vs 281.8–440.3 ms).
- `fx-hir-ty` 1c: head vs base: below (261.1–317.5 vs 577.6–706.4 ms); head vs rustfmt: below (261.1–317.5 vs 846.8–1009.1 ms); head-pgo vs base: below (234.5–312.4 vs 601.7–826.9 ms); head-pgo vs rustfmt: below (234.5–312.4 vs 879.6–1155.9 ms).
- `fx-hir-ty` mc: head vs base: below (75.0–95.3 vs 176.5–246.7 ms); head vs rustfmt: below (75.0–95.3 vs 856.2–1016.1 ms); head-pgo vs base: below (67.6–84.2 vs 172.3–280.0 ms); head-pgo vs rustfmt: below (67.6–84.2 vs 899.6–1233.8 ms).
- `fx-ide-db` 1c: head vs base: below (82.6–97.6 vs 213.3–279.8 ms); head vs rustfmt: below (82.6–97.6 vs 300.6–394.0 ms); head-pgo vs base: below (74.8–92.2 vs 213.9–249.0 ms); head-pgo vs rustfmt: below (74.8–92.2 vs 308.2–430.5 ms).
- `fx-ide-db` mc: head vs base: below (29.4–46.9 vs 118.4–191.0 ms); head vs rustfmt: below (29.4–46.9 vs 305.2–481.9 ms); head-pgo vs base: below (26.0–39.4 vs 125.8–180.9 ms); head-pgo vs rustfmt: below (26.0–39.4 vs 308.4–425.1 ms).
- `fx-parser` 1c: head vs base: below (29.6–33.7 vs 63.3–87.0 ms); head vs rustfmt: below (29.6–33.7 vs 74.0–127.1 ms); head-pgo vs base: below (28.0–50.2 vs 66.2–114.4 ms); head-pgo vs rustfmt: below (28.0–50.2 vs 75.3–125.9 ms).
- `fx-parser` mc: head vs base: overlap (10.7–21.9 vs 21.6–32.6 ms); head vs rustfmt: below (10.7–21.9 vs 73.3–125.5 ms); head-pgo vs base: below (11.2–18.2 vs 25.1–33.4 ms); head-pgo vs rustfmt: below (11.2–18.2 vs 78.4–146.2 ms).
- `fx-syntax` 1c: head vs base: below (48.3–71.9 vs 109.6–175.1 ms); head vs rustfmt: below (48.3–71.9 vs 128.2–205.1 ms); head-pgo vs base: below (44.4–50.9 vs 112.2–151.5 ms); head-pgo vs rustfmt: below (44.4–50.9 vs 126.7–182.7 ms).
- `fx-syntax` mc: head vs base: below (16.2–20.2 vs 36.2–69.7 ms); head vs rustfmt: below (16.2–20.2 vs 129.1–192.2 ms); head-pgo vs base: below (15.6–20.7 vs 38.6–72.1 ms); head-pgo vs rustfmt: below (15.6–20.7 vs 133.1–202.5 ms).
- `reg-clap-builder` 1c: head vs base: below (74.9–102.2 vs 167.1–287.6 ms); head vs rustfmt: below (74.9–102.2 vs 195.9–263.2 ms); head-pgo vs base: below (67.4–108.4 vs 166.4–239.4 ms); head-pgo vs rustfmt: below (67.4–108.4 vs 191.7–290.5 ms).
- `reg-clap-builder` mc: head vs base: below (25.0–38.9 vs 52.2–126.8 ms); head vs rustfmt: below (25.0–38.9 vs 193.9–251.9 ms); head-pgo vs base: below (23.7–38.3 vs 60.3–94.8 ms); head-pgo vs rustfmt: below (23.7–38.3 vs 195.4–292.6 ms).
- `reg-regex-syntax` 1c: head vs base: below (199.2–277.2 vs 402.9–546.6 ms); head vs rustfmt: below (199.2–277.2 vs 442.0–620.8 ms); head-pgo vs base: below (167.1–270.5 vs 418.3–601.4 ms); head-pgo vs rustfmt: below (167.1–270.5 vs 450.5–723.8 ms).
- `reg-regex-syntax` mc: head vs base: below (64.9–90.9 vs 118.6–176.2 ms); head vs rustfmt: below (64.9–90.9 vs 449.3–581.1 ms); head-pgo vs base: below (54.3–93.5 vs 123.7–181.3 ms); head-pgo vs rustfmt: below (54.3–93.5 vs 460.6–566.8 ms).
- `reg-tokio` 1c: head vs base: below (17.1–25.3 vs 33.7–53.2 ms); head vs rustfmt: below (17.1–25.3 vs 41.3–66.1 ms); head-pgo vs base: below (16.2–20.0 vs 34.7–66.9 ms); head-pgo vs rustfmt: below (16.2–20.0 vs 43.6–72.3 ms).
- `reg-tokio` mc: head vs base: below (7.4–10.7 vs 13.7–21.3 ms); head vs rustfmt: below (7.4–10.7 vs 41.0–65.4 ms); head-pgo vs base: below (6.9–10.9 vs 14.8–30.9 ms); head-pgo vs rustfmt: below (6.9–10.9 vs 41.0–66.7 ms).
- `reg-winnow` 1c: head vs base: below (77.8–125.1 vs 191.2–267.0 ms); head vs rustfmt: below (77.8–125.1 vs 181.6–261.9 ms); head-pgo vs base: below (70.2–113.6 vs 187.6–270.9 ms); head-pgo vs rustfmt: below (70.2–113.6 vs 179.4–300.3 ms).
- `reg-winnow` mc: head vs base: below (25.9–30.3 vs 60.4–94.9 ms); head vs rustfmt: below (25.9–30.3 vs 182.3–261.1 ms); head-pgo vs base: below (23.9–32.4 vs 62.3–122.9 ms); head-pgo vs rustfmt: below (23.9–32.4 vs 184.8–238.5 ms).
- `stdin-config` 1c: head vs base: below (13.9–21.7 vs 26.2–29.5 ms); head vs rustfmt: below (13.9–21.7 vs 42.6–51.3 ms).
- `stdin-hir-lib` 1c: head vs base: below (33.5–35.5 vs 64.1–83.7 ms); head vs rustfmt: below (33.5–35.5 vs 87.7–110.1 ms).
- `stdin-hover-tests` 1c: head vs base: below (8.4–13.9 vs 17.4–28.2 ms); head vs rustfmt: below (8.4–13.9 vs 30.9–49.3 ms).
- `stdin-lints` 1c: head vs base: below (29.9–48.7 vs 83.3–139.4 ms); head vs rustfmt: below (29.9–48.7 vs 138.4–187.1 ms).
- `stdin-nodes` 1c: head vs base: below (42.1–77.5 vs 79.8–126.4 ms); head vs rustfmt: below (42.1–77.5 vs 86.4–98.5 ms).
- `tiny-one` 1c: head vs base: overlap (1.7–2.0 vs 1.8–2.1 ms); head vs rustfmt: below (1.7–2.0 vs 10.7–21.1 ms); head-pgo vs base: overlap (1.8–6.4 vs 1.8–2.2 ms); head-pgo vs rustfmt: below (1.8–6.4 vs 11.5–27.3 ms).
- `tiny-one` mc: head vs base: overlap (2.1–3.9 vs 1.9–3.3 ms); head vs rustfmt: below (2.1–3.9 vs 12.0–20.0 ms); head-pgo vs base: overlap (2.0–3.5 vs 2.1–6.0 ms); head-pgo vs rustfmt: below (2.0–3.5 vs 11.9–17.1 ms).
- `tiny-test-data` 1c: head vs base: below (12.3–14.4 vs 22.1–24.1 ms); head vs rustfmt: below (12.3–14.4 vs 50.6–78.5 ms); head-pgo vs base: below (11.7–18.1 vs 23.1–38.1 ms); head-pgo vs rustfmt: below (11.7–18.1 vs 53.6–82.5 ms).
- `tiny-test-data` mc: head vs base: below (6.2–9.5 vs 12.7–22.8 ms); head vs rustfmt: below (6.2–9.5 vs 52.1–99.2 ms); head-pgo vs base: below (6.2–9.5 vs 12.4–23.8 ms); head-pgo vs rustfmt: below (6.2–9.5 vs 52.7–91.4 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS (`head-pgo` against the
`base` runs of sweep 2, its own window), as the table builder lists them:

- `fx-ide-db.mc` head vs base, rss: 19.8 vs 19.5 MiB (1.012); wall ranges below.
- `tiny-one.1c` head vs base, rss: 3.7 vs 3.7 MiB (1.016); wall ranges overlap.
- `tiny-one.mc` head vs base, mean: 2.4 vs 2.3 ms (1.042); wall ranges overlap.
- `tiny-one.mc` head vs base, min: 2.1 vs 1.9 ms (1.063); wall ranges overlap.
- `tiny-one.mc` head vs base, CPU: 2.2 vs 2.1 ms (1.058); wall ranges overlap.
- `tiny-one.mc` head vs base, rss: 3.7 vs 3.7 MiB (1.009); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, mean: 2.2 vs 2.0 ms (1.119); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, CPU: 1.9 vs 1.8 ms (1.030); wall ranges overlap.
- `tiny-one.1c` head-pgo vs base, rss: 4.3 vs 3.7 MiB (1.179); wall ranges overlap.
- `tiny-one.mc` head-pgo vs base, rss: 4.3 vs 3.7 MiB (1.166); wall ranges overlap.
- `tiny-test-data.1c` head-pgo vs base, rss: 5.2 vs 4.8 MiB (1.074); wall ranges below.
- `tiny-test-data.mc` head-pgo vs base, rss: 5.5 vs 5.1 MiB (1.074); wall ranges below.

From the parallel-items table (head with 4 threads against base on one file), peak RSS only:

- `stdin-lints` mc, head vs base, rss: 14.7 vs 10.4 (1.41); wall ranges below.
- `stdin-nodes` mc, head vs base, rss: 13.9 vs 13.8 (1.01); wall ranges below.
- `stdin-nodes` 1c, head vs base, rss: 14.1 vs 13.9 (1.01); wall ranges below.
- `stdin-hir-lib` mc, head vs base, rss: 8.9 vs 8.8 (1.02); wall ranges below.
- `stdin-hir-lib` 1c, head vs base, rss: 9.0 vs 8.8 (1.02); wall ranges below.
- `stdin-config` mc, head vs base, rss: 6.3 vs 5.9 (1.07); wall ranges below.

- Wall-time means above base: `tiny-one` only (head 1.042 unpinned, head-pgo 1.119 pinned; a
  13-byte file, process start-up, every range overlapping, the other two `tiny-one` cells
  below base). No crate, files, stdin, cache or in-process row reads head or head-pgo above
  base in mean or minimum wall. No chloro build reads above rustfmt in mean, minimum, CPU or
  peak RSS on any workload, formatted or raw.
- CPU above base: the same two `tiny-one` cells (1.058, 1.030).
- Peak RSS above base: `stdin-lints` unpinned (14.7 vs 10.4 MiB, the E1 split, M3's 1.44x),
  five split rows within 0.4 MiB of base, head on `fx-ide-db` unpinned (+0.3 MiB) and
  `tiny-one` pinned (+0.06 MiB), head-pgo on `tiny-one` (+0.6 MiB) and `tiny-test-data`
  (+0.4 MiB). No other crate row reads `head` above base in peak RSS; no cache row and no
  in-process row reads above base.

## Against M3

M3 ([m3-quiet-host.md](m3-quiet-host.md)) measured `4867896` (the E3 + R2 head) on the 2.80
GHz host class; this record measures `6b78728` (M3's head plus R5, E12, E15, E17, E18, E16;
E13 and E14 change no formatter code) on the same host class (same `model name`, `nproc`,
`MemTotal` and kernel). Protocol §3 keeps absolute figures within one window, so the two
records' ratios are set side by side and the absolute figures are not; `base` is the same
binary (same sha256) in both:

| ratio (means) | M3, `4867896` | M4, `6b78728` |
|---|---|---|
| crates, `mc`, head/base | 0.30–0.58 | 0.25–0.52 |
| crates, `1c`, head/base (wall; CPU) | 0.46–0.60; 0.46–0.59 | 0.38–0.60; 0.38–0.59 |
| crates, `mc`, pgo/base | 0.26–0.50 | 0.22–0.45 |
| crates, `1c`, pgo/base | 0.40–0.51 | 0.36–0.43 |
| crates, `mc`, head/rustfmt | 0.105–0.203 | 0.092–0.179 |
| crates, `1c`, head/rustfmt | 0.32–0.54 | 0.26–0.44 |
| crates, `1c`, pgo/rustfmt | 0.28–0.44 | 0.24–0.38 |
| crates, `1c`, head below base in range | 9 of 9 | 9 of 9 |
| crates, `mc`, head below base in range | 7 of 9 | 8 of 9 |
| crates, `1c`, head below rustfmt in range | 9 of 9 | 9 of 9 |
| 287 small files, `mc`; `1c`, head/base | 0.605; 0.672 | 0.442; 0.565 |
| stdin, head/base | 0.41–0.62 | 0.35–0.56 |
| split files, `mc`, head/base (wall; CPU) | 0.29–0.47; 0.46–0.76 | 0.29–0.42; 0.43–0.66 |
| `bench -n 5`, head/base on the triples' MB/s | 9.39–9.54 vs 5.22–5.27 (1.79x–1.81x) | 11.22–11.79 vs 4.96–5.09 (2.20x–2.38x) |
| `bench -n 5`, head-pgo MB/s | 11.28–11.59 | 13.03–13.24 |
| `Ir`, head/base; pgo/base | 0.5319; 0.4162 | 0.4221; 0.3519 |
| cache, warm vs no-cache vs base, `mc` | 28.9 / 434.3 / 838.8 ms (15.0x, 29.0x) | 16.9 / 372.7 / 909.4 ms (22.1x, 53.8x) |
| `conform -i`, head/base | 0.56 | 0.45 |
| `conform -i`, head identical | 1220/1220 | 1220/1220 (and 1220/1220 at 1.8.0) |

- Every ratio moved in the direction of the instruction count: the two records' `Ir` ratios
  against base are 0.5319 and 0.4221 (−46.8% and −57.8%), and the pinned crate `head/base`
  range moved from 0.46–0.60 to 0.38–0.60 (0.38–0.50 without the one noisy `reg-tokio` cell),
  `head/rustfmt` from 0.32–0.54 to 0.26–0.44, `pgo/rustfmt` from 0.28–0.44 to 0.24–0.38.
- The rows E12 targets moved most: the 287-file invocation from 0.605/0.672 of base to
  0.442/0.565, and the warm cache from 28.9 to 16.9 ms unpinned (37.1 to 24.4 pinned), while
  the `--no-cache` figure fell with the formatter (434.3 to 372.7 ms).
- `base` reads slower in this record's windows than in M3's for the same binary and bytes
  (`bench` 4.96–5.09 against 5.22–5.27 MB/s; cache sweep 909 against 839 ms; `conform -i`
  5.70 against 5.49 s), 3–8%, so the `head/base` wall ratios here are slightly flattering and
  the `Ir` ratios, which do not depend on the window, are the figures to carry forward.
- In M3 the unpinned crate ranges of head and base overlapped on two crates; here on one
  (`fx-parser`). The `tiny-one` cells read above base in mean on one row each (1.042 unpinned
  head, 1.119 pinned head-pgo), inside overlapping ranges, as the 13-byte file has in every
  record.

## Harness notes

- `compare_rustfmt.sh` passes `--no-cache` to the head builds on its own (`chloro_cache_flag=
  --no-cache` in `env.txt`) and no flag to `base`, which rejects it. Since E15 the flag also
  turns off the opt-in unit cache. The `--rustfmt` and `--registry` defaults resolved without
  being passed.
- `benchmarks/pgo.sh` ran to completion before the chain (instrumented build, training,
  optimised build of `chloro` and `examples/bench` under `target/pgo/release/`); the profile
  differs per training run, so the profile-guided instruction count is this build's, not
  E13's or M3's (M3 244,712,537; E13's reproduction 245,255,322).
- **The harness's `identical outputs` figure is stale when a copy is reused.** `prepare()`
  computes the agreement once, when it makes a copy, with the chloro binary it was given, and
  stores it in the copy's `meta`; a later run that reuses the copy (no `--fresh`) reports that
  stored figure. The copies on this disk were made on 2026-10-14 with that day's binary, so the
  raw `tiny-test-data` row reports 288/289 (M2's figure, before R2) for a head that produces
  289 of 289. The agreement counts of this record were recomputed with this record's binaries
  (`scratchpad/m4/agree.sh`: rustfmt and `--write --no-cache` on fresh copies of every
  workload, `diff -rq`) and are in the table below. A harness fix (recompute the agreement at
  run time, or record the binary's hash with it) is owed; it was not made while the chain was
  running, since bash reads a running script incrementally.
- The chain's first start (04:28 UTC) was stopped after one minute: the five stdin copies
  predated the harness's stdin preparation and two were not fixed points. They were rebuilt
  with `--groups c -r 1` (rustfmt through stdin, as the harness does since M3) and verified
  (exit 0 for rustfmt, `base`, `head` and `head-pgo` on all five) before the restart. The crate
  and files copies were not rebuilt: their bytes are rustfmt's output for the same inputs and
  the same rustfmt binary, which is what the preparation produces.
- The by-hand `bench -n 5` triples pass `--root /home/user/chloro/chloro-core/tests/conformance/
  fixtures` to all three binaries, so `base`'s `bench` reads the head's fixture tree (its
  compiled-in default is the base worktree's copy; same count and size, 1238 files, 13.9 MB).
  The harness's own `inproc-bench` rows use each binary's compiled-in root, as in M3.
- `/usr/bin/time` is not in the container; the by-hand `conform -i` runs record wall, user and
  sys through bash's `time`, and the peak RSS of `conform -i` comes from the harness's
  hyperfine rows in `m4-d-inproc-harness`.
- `--baseline` requires all four baseline binaries and `--chloro` requires `fmt_stdin`, `bench`
  and `conform` to exist even for groups that do not run them; the PGO sweep used head's three
  examples (the harness defaults) to satisfy the check.
- The chain was one script with no gaps; the first monitor of its progress buffered its output
  (a `cut` in the pipeline), which changed nothing in the measurements.

### Output agreement, recomputed with this record's binaries

`scratchpad/m4/agree.sh`, after the sweeps: for every workload copy, rustfmt (crate root, file list, or stdin as the harness runs it) on one fresh copy, `head --write --no-cache` on a second and `base --write` on a third, then `diff -rq`. Counts are files whose bytes equal rustfmt's output.

| input | workload | files | head identical | base identical | files where base differs |
|---|---|---|---|---|---|
| formatted | `fx-hir-ty` | 122 | 122 | 122 | – |
| formatted | `fx-hir` | 15 | 15 | 15 | – |
| formatted | `fx-ide-db` | 33 | 33 | 33 | – |
| formatted | `fx-parser` | 28 | 28 | 28 | – |
| formatted | `fx-syntax` | 33 | 33 | 33 | – |
| formatted | `reg-clap-builder` | 57 | 57 | 57 | – |
| formatted | `reg-regex-syntax` | 33 | 33 | 33 | – |
| formatted | `reg-tokio` | 49 | 49 | 49 | – |
| formatted | `reg-winnow` | 61 | 61 | 61 | – |
| formatted | `stdin-config` | 1 | 1 | 1 | – |
| formatted | `stdin-hir-lib` | 1 | 1 | 1 | – |
| formatted | `stdin-hover-tests` | 1 | 1 | 1 | – |
| formatted | `stdin-lints` | 1 | 1 | 1 | – |
| formatted | `stdin-nodes` | 1 | 1 | 1 | – |
| formatted | `tiny-one` | 1 | 1 | 1 | – |
| formatted | `tiny-test-data` | 287 | 287 | 287 | – |
| raw | `fx-hir-ty` | 122 | 122 | 122 | – |
| raw | `fx-hir` | 15 | 15 | 15 | – |
| raw | `fx-ide-db` | 33 | 33 | 33 | – |
| raw | `fx-parser` | 28 | 28 | 28 | – |
| raw | `fx-syntax` | 33 | 33 | 33 | – |
| raw | `reg-clap-builder` | 57 | 57 | 57 | – |
| raw | `reg-regex-syntax` | 33 | 33 | 33 | – |
| raw | `reg-tokio` | 49 | 49 | 49 | – |
| raw | `reg-winnow` | 61 | 61 | 61 | – |
| raw | `stdin-config` | 1 | 1 | 1 | – |
| raw | `stdin-hir-lib` | 1 | 1 | 1 | – |
| raw | `stdin-hover-tests` | 1 | 1 | 1 | – |
| raw | `stdin-lints` | 1 | 1 | 1 | – |
| raw | `stdin-nodes` | 1 | 1 | 1 | – |
| raw | `tiny-one` | 1 | 1 | 1 | – |
| raw | `tiny-test-data` | 289 | 289 | 288 | `parser/err/0024_many_type_parens.rs` |

- `head` equals rustfmt on every file of every workload on both inputs (1,037 files formatted, 1,039 raw); `base` differs on one raw file, `parser/err/0024_many_type_parens.rs` (the R2 fix), and on none of the formatted copies (which are rustfmt's own output and a fixed point for every build).

## Current State

- `benchmarks/compare_rustfmt.sh`, `report.py`, `profile.sh` and `pgo.sh` ran to completion on
  every sweep (exit 0) on a host with no other agent; seven harness results directories and
  four by-hand directories hold the JSON exports, `commands.txt`, `env.txt`, `report.md` and
  the loads log.
- On formatted input with all 4 CPUs, `head` takes 0.092–0.179x rustfmt's wall time on the
  nine crates and 0.25–0.52x base's; on one core 0.26–0.44x rustfmt's and 0.38–0.60x base's
  (CPU 0.38–0.59x). Every head run is below every rustfmt run and every base run on every
  crate pinned; unpinned, below every rustfmt run on every crate and below every base run on
  eight of nine.
- `head-pgo` takes 0.072–0.175x rustfmt's wall time unpinned and 0.24–0.38x pinned; 0.22–0.45x
  base's unpinned and 0.36–0.43x pinned; `Ir` 0.3519x base's and 0.8337x head's.
- The warm cache answers `--check` on 1238 formatted files in 16.9 ms against 372.7 ms without
  it (22.1x) and 909.4 ms for base (53.8x); 53.5x and 125.7x on one core.
- In process, head formats the fixtures at 11.22–11.79 MB/s against base's 4.96–5.09 MB/s
  (disjoint over 3 interleaved triples), head-pgo at 13.03–13.24 MB/s; `conform -i` reports
  1220/1220 identical for head at rustfmt 1.9.0 and at 1.8.0, 1219/1220 for base; the
  instruction count on the sample is 248,202,137 (0.4221 of base), 206,922,054 profile-guided.
- The parallel-items split gives 0.29–0.42x base's wall time on the five largest files with
  4 threads, at 0.43–0.66x the CPU and up to 1.41x the peak RSS.
- Output agreement with rustfmt, recomputed with these binaries: every file of every workload
  on both inputs.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure.
- The `cold` cache variant has no pinned row; the profile-guided cache rows are unpinned only.
- No raw-input row for `head-pgo` (measurement A's raw sweep is head only, as in M1–M3).
- The by-hand `conform -i` runs have no peak RSS of their own (no `/usr/bin/time` in the
  container).
- The harness still reports a copy's stored agreement figure; the fix (recompute at run time,
  or store the binary hash with it) is owed and is not in this record's head.
- The unit cache (`--unit-cache`, E15) is not measured here; its figures are in EXP-015.

## Appendix: `report.py` tables as written

The `report.md` of each harness results directory, as `report.py` wrote it.

### `m4-a1-head-base-formatted`

```text
date_utc=2026-10-15T04:30:43Z
commit=6b78728ab59fde69663c8b4ca9ec32ca55748c08
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=0.87 0.70 1.35
cpu3_stat_start=618577 0 66596 567273 3203 0 2273 6750 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.64 1.55 1.53
cpu3_stat_end=645757 0 68894 575283 3219 0 2278 7001 0 0
cpu3_steal_ticks=251 cpu3_busy_ticks=29478 cpu3_idle_ticks=8010
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 905.9 ± 38.3 (846.8–1009.1) | 283.0 ± 16.3 (261.1–317.5) | 614.5 ± 30.8 (577.6–706.4) | 0.461 | below | 0.312 | 0.308 | below | 85 | 8 | 899.4 | 280.2 | 0.311 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 915.9 ± 39.5 (856.2–1016.1) | 84.5 ± 6.2 (75.0–95.3) | 209.8 ± 18.1 (176.5–246.7) | 0.403 | below | 0.092 | 0.088 | below | 85 | 19 | 908.0 | 314.2 | 0.346 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 302.6 ± 27.3 (275.0–379.0) | 86.3 ± 3.6 (83.6–103.6) | 202.2 ± 18.8 (183.0–254.5) | 0.427 | below | 0.285 | 0.304 | below | 56 | 9 | 298.4 | 85.9 | 0.288 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 311.4 ± 28.0 (283.1–388.4) | 32.1 ± 3.0 (27.5–39.4) | 102.5 ± 11.4 (87.0–128.1) | 0.313 | below | 0.103 | 0.097 | below | 56 | 18 | 306.9 | 106.9 | 0.348 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 323.6 ± 23.6 (300.6–394.0) | 85.3 ± 2.7 (82.6–97.6) | 224.7 ± 17.2 (213.3–279.8) | 0.379 | below | 0.264 | 0.275 | below | 58 | 10 | 320.5 | 84.7 | 0.264 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 342.2 ± 38.2 (305.2–481.9) | 35.0 ± 3.6 (29.4–46.9) | 137.9 ± 15.8 (118.4–191.0) | 0.254 | below | 0.102 | 0.096 | below | 58 | 20 | 337.5 | 105.9 | 0.314 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 78.5 ± 9.4 (74.0–127.1) | 30.8 ± 0.8 (29.6–33.7) | 68.2 ± 4.8 (63.3–87.0) | 0.451 | below | 0.392 | 0.401 | below | 47 | 5 | 77.7 | 30.5 | 0.393 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 84.5 ± 14.3 (73.3–125.5) | 12.9 ± 2.0 (10.7–21.9) | 25.8 ± 2.6 (21.6–32.6) | 0.499 | overlap | 0.152 | 0.145 | below | 47 | 6 | 83.4 | 39.4 | 0.472 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 139.5 ± 20.8 (128.2–205.1) | 52.5 ± 5.6 (48.3–71.9) | 120.7 ± 15.3 (109.6–175.1) | 0.435 | below | 0.377 | 0.377 | below | 51 | 6 | 138.7 | 51.5 | 0.371 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 144.5 ± 17.2 (129.1–192.2) | 17.7 ± 1.0 (16.2–20.2) | 48.9 ± 7.0 (36.2–69.7) | 0.361 | below | 0.122 | 0.125 | below | 51 | 8 | 141.7 | 59.3 | 0.418 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 210.9 ± 15.3 (195.9–263.2) | 80.5 ± 7.6 (74.9–102.2) | 184.2 ± 27.2 (167.1–287.6) | 0.437 | below | 0.381 | 0.382 | below | 55 | 6 | 208.8 | 79.8 | 0.382 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 214.7 ± 15.8 (193.9–251.9) | 30.1 ± 3.4 (25.0–38.9) | 70.1 ± 13.1 (52.2–126.8) | 0.430 | below | 0.140 | 0.129 | below | 55 | 12 | 211.6 | 95.3 | 0.450 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 503.3 ± 47.0 (442.0–620.8) | 223.4 ± 20.7 (199.2–277.2) | 444.2 ± 34.2 (402.9–546.6) | 0.503 | below | 0.444 | 0.451 | below | 66 | 14 | 497.9 | 221.4 | 0.445 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 494.9 ± 33.5 (449.3–581.1) | 74.8 ± 6.6 (64.9–90.9) | 143.1 ± 19.2 (118.6–176.2) | 0.523 | below | 0.151 | 0.144 | below | 66 | 31 | 489.6 | 248.4 | 0.507 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 48.5 ± 7.4 (41.3–66.1) | 21.2 ± 2.9 (17.1–25.3) | 35.6 ± 4.1 (33.7–53.2) | 0.595 | below | 0.437 | 0.415 | below | 43 | 4 | 47.6 | 20.8 | 0.437 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 47.1 ± 5.9 (41.0–65.4) | 8.4 ± 0.8 (7.4–10.7) | 18.2 ± 1.9 (13.7–21.3) | 0.462 | below | 0.179 | 0.181 | below | 43 | 5 | 45.9 | 22.5 | 0.489 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 203.5 ± 22.4 (181.6–261.9) | 89.1 ± 12.3 (77.8–125.1) | 213.2 ± 19.6 (191.2–267.0) | 0.418 | below | 0.438 | 0.428 | below | 59 | 7 | 200.8 | 87.8 | 0.437 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 208.4 ± 22.6 (182.3–261.1) | 27.4 ± 1.3 (25.9–30.3) | 70.7 ± 8.3 (60.4–94.9) | 0.387 | below | 0.131 | 0.142 | below | 59 | 12 | 205.1 | 93.9 | 0.458 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.9 ± 1.8 (10.7–21.1) | 1.8 ± 0.1 (1.7–2.0) | 1.9 ± 0.1 (1.8–2.1) | 0.965 | overlap | 0.153 | 0.163 | below | 34 | 4 | 11.4 | 1.7 | 0.148 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 14.4 ± 1.8 (12.0–20.0) | 2.4 ± 0.4 (2.1–3.9) | 2.3 ± 0.4 (1.9–3.3) | 1.042 | overlap | 0.169 | 0.173 | below | 34 | 4 | 13.9 | 2.2 | 0.159 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 54.1 ± 5.4 (50.6–78.5) | 12.9 ± 0.5 (12.3–14.4) | 22.9 ± 0.5 (22.1–24.1) | 0.565 | below | 0.239 | 0.243 | below | 44 | 5 | 53.8 | 12.7 | 0.236 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 61.6 ± 11.3 (52.1–99.2) | 7.8 ± 0.7 (6.2–9.5) | 17.6 ± 2.9 (12.7–22.8) | 0.442 | below | 0.126 | 0.120 | below | 44 | 5 | 60.4 | 17.8 | 0.295 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 169 | 30 | 45.4 ± 1.9 (42.6–51.3) | 15.3 ± 2.1 (13.9–21.7) | 27.4 ± 0.6 (26.2–29.5) | 0.557 | below | 0.337 | 0.327 | below | 44 | 5 | 45.0 | 15.0 | 0.333 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 91.5 ± 5.0 (87.7–110.1) | 34.4 ± 0.5 (33.5–35.5) | 69.1 ± 5.6 (64.1–83.7) | 0.499 | below | 0.376 | 0.382 | below | 47 | 8 | 90.4 | 34.2 | 0.379 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 32.9 ± 3.7 (30.9–49.3) | 9.6 ± 1.6 (8.4–13.9) | 19.3 ± 2.4 (17.4–28.2) | 0.499 | below | 0.292 | 0.273 | below | 40 | 4 | 32.3 | 9.3 | 0.289 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 146.6 ± 10.9 (138.4–187.1) | 32.6 ± 3.5 (29.9–48.7) | 94.4 ± 15.8 (83.3–139.4) | 0.345 | below | 0.222 | 0.216 | below | 43 | 9 | 145.2 | 32.3 | 0.222 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 92.3 ± 2.9 (86.4–98.5) | 46.1 ± 8.6 (42.1–77.5) | 84.3 ± 8.8 (79.8–126.4) | 0.547 | below | 0.500 | 0.488 | below | 53 | 13 | 91.6 | 45.8 | 0.500 | 1/1 | r[0] c[0] |

### `m4-a2-pgo-base-formatted`

```text
date_utc=2026-10-15T04:37:01Z
commit=6b78728ab59fde69663c8b4ca9ec32ca55748c08
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.64 1.55 1.53
cpu3_stat_start=645763 0 68898 575294 3219 0 2278 7001 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/pgo/release/chloro sha256=218d001c5d831ec6
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=1.95 1.80 1.64
cpu3_stat_end=670116 0 70792 584575 3221 0 2292 7263 0 0
cpu3_steal_ticks=262 cpu3_busy_ticks=26247 cpu3_idle_ticks=9281
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 980.6 ± 73.0 (879.6–1155.9) | 256.3 ± 20.6 (234.5–312.4) | 659.8 ± 49.9 (601.7–826.9) | 0.389 | below | 0.261 | 0.267 | below | 85 | 9 | 968.4 | 254.7 | 0.263 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 1013.3 ± 86.5 (899.6–1233.8) | 72.9 ± 4.9 (67.6–84.2) | 214.7 ± 23.9 (172.3–280.0) | 0.340 | below | 0.072 | 0.075 | below | 85 | 19 | 1003.2 | 270.8 | 0.270 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 311.6 ± 29.3 (279.4–388.4) | 83.7 ± 10.6 (77.2–119.6) | 210.5 ± 17.6 (192.2–288.0) | 0.398 | below | 0.269 | 0.276 | below | 56 | 10 | 307.7 | 82.7 | 0.269 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 315.4 ± 38.7 (281.8–440.3) | 30.4 ± 3.0 (26.1–39.5) | 103.7 ± 11.5 (87.2–131.8) | 0.293 | below | 0.096 | 0.093 | below | 57 | 18 | 311.6 | 96.0 | 0.308 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 334.8 ± 29.7 (308.2–430.5) | 81.4 ± 3.3 (74.8–92.2) | 224.9 ± 9.9 (213.9–249.0) | 0.362 | below | 0.243 | 0.243 | below | 58 | 11 | 331.6 | 80.3 | 0.242 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 351.7 ± 32.5 (308.4–425.1) | 31.9 ± 3.5 (26.0–39.4) | 142.6 ± 13.4 (125.8–180.9) | 0.223 | below | 0.091 | 0.084 | below | 58 | 20 | 346.5 | 97.2 | 0.281 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 91.7 ± 13.5 (75.3–125.9) | 32.3 ± 5.6 (28.0–50.2) | 77.7 ± 13.8 (66.2–114.4) | 0.416 | below | 0.353 | 0.371 | below | 47 | 5 | 90.4 | 31.7 | 0.351 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 89.9 ± 18.4 (78.4–146.2) | 13.0 ± 1.4 (11.2–18.2) | 29.2 ± 2.4 (25.1–33.4) | 0.447 | below | 0.145 | 0.143 | below | 47 | 7 | 88.4 | 38.7 | 0.437 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 141.3 ± 11.7 (126.7–182.7) | 46.3 ± 1.2 (44.4–50.9) | 117.5 ± 8.6 (112.2–151.5) | 0.394 | below | 0.328 | 0.351 | below | 51 | 6 | 139.7 | 45.9 | 0.328 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 155.4 ± 18.3 (133.1–202.5) | 17.7 ± 1.6 (15.6–20.7) | 54.1 ± 10.1 (38.6–72.1) | 0.327 | below | 0.114 | 0.117 | below | 51 | 8 | 151.9 | 57.4 | 0.377 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 219.2 ± 26.3 (191.7–290.5) | 77.9 ± 10.6 (67.4–108.4) | 182.7 ± 19.3 (166.4–239.4) | 0.427 | below | 0.356 | 0.352 | below | 55 | 7 | 217.5 | 76.7 | 0.353 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 224.7 ± 26.1 (195.4–292.6) | 27.2 ± 3.1 (23.7–38.3) | 71.2 ± 7.1 (60.3–94.8) | 0.382 | below | 0.121 | 0.121 | below | 55 | 13 | 222.7 | 86.2 | 0.387 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 517.3 ± 65.3 (450.5–723.8) | 195.4 ± 29.4 (167.1–270.5) | 463.6 ± 49.0 (418.3–601.4) | 0.421 | below | 0.378 | 0.371 | below | 66 | 15 | 512.4 | 194.0 | 0.379 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 503.6 ± 31.3 (460.6–566.8) | 62.6 ± 8.6 (54.3–93.5) | 144.1 ± 15.3 (123.7–181.3) | 0.435 | below | 0.124 | 0.118 | below | 66 | 31 | 497.4 | 208.2 | 0.419 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 52.6 ± 8.8 (43.6–72.3) | 17.1 ± 0.8 (16.2–20.0) | 45.0 ± 9.8 (34.7–66.9) | 0.381 | below | 0.326 | 0.371 | below | 43 | 5 | 51.1 | 16.7 | 0.327 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 46.1 ± 6.1 (41.0–66.7) | 8.0 ± 0.9 (6.9–10.9) | 20.3 ± 4.1 (14.8–30.9) | 0.396 | below | 0.175 | 0.169 | below | 43 | 6 | 45.2 | 20.9 | 0.462 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 206.7 ± 29.4 (179.4–300.3) | 79.2 ± 10.8 (70.2–113.6) | 204.7 ± 19.3 (187.6–270.9) | 0.387 | below | 0.383 | 0.391 | below | 58 | 8 | 204.5 | 78.3 | 0.383 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 205.6 ± 14.6 (184.8–238.5) | 27.3 ± 2.1 (23.9–32.4) | 85.0 ± 12.9 (62.3–122.9) | 0.321 | below | 0.133 | 0.129 | below | 58 | 12 | 203.2 | 91.9 | 0.452 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 13.4 ± 2.8 (11.5–27.3) | 2.2 ± 0.9 (1.8–6.4) | 2.0 ± 0.1 (1.8–2.2) | 1.119 | overlap | 0.164 | 0.153 | below | 34 | 4 | 12.5 | 1.9 | 0.150 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 13.7 ± 1.8 (11.9–17.1) | 2.3 ± 0.3 (2.0–3.5) | 2.5 ± 0.7 (2.1–6.0) | 0.914 | overlap | 0.169 | 0.165 | below | 34 | 4 | 13.2 | 2.1 | 0.160 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 61.6 ± 7.8 (53.6–82.5) | 13.3 ± 1.5 (11.7–18.1) | 25.7 ± 3.7 (23.1–38.1) | 0.518 | below | 0.216 | 0.219 | below | 44 | 5 | 60.3 | 12.8 | 0.213 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 64.9 ± 12.8 (52.7–91.4) | 7.4 ± 0.8 (6.2–9.5) | 17.6 ± 3.1 (12.4–23.8) | 0.418 | below | 0.113 | 0.118 | below | 44 | 5 | 64.0 | 16.4 | 0.256 | 287/287 | r[0] c[0] |

### `m4-a3-head-raw`

```text
date_utc=2026-10-15T04:43:00Z
commit=aeb32802b502db075ea7bdd4b0cec5a7630a7acc
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.95 1.80 1.64
cpu3_stat_start=670123 0 70796 584581 3221 0 2292 7264 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.07 1.28 1.44
cpu3_stat_end=692764 0 75106 597320 3296 0 2302 7498 0 0
cpu3_steal_ticks=234 cpu3_busy_ticks=26951 cpu3_idle_ticks=12739
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1337.6 ± 80.1 (1187.1–1486.8) | 294.9 ± 21.9 (269.6–378.2) | 0.220 | 0.227 | below | 116 | 8 | 1326.2 | 290.9 | 0.219 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1318.7 ± 60.6 (1240.2–1474.9) | 87.6 ± 9.3 (76.1–112.3) | 0.066 | 0.061 | below | 116 | 19 | 1306.2 | 321.3 | 0.246 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 601.7 ± 41.8 (549.0–685.2) | 95.9 ± 12.0 (86.5–142.0) | 0.159 | 0.158 | below | 217 | 9 | 595.0 | 93.6 | 0.157 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 632.2 ± 46.8 (578.7–760.3) | 34.9 ± 3.0 (31.4–45.8) | 0.055 | 0.054 | below | 218 | 17 | 626.8 | 115.3 | 0.184 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 372.4 ± 30.9 (339.5–437.0) | 90.8 ± 4.9 (84.5–102.4) | 0.244 | 0.249 | below | 66 | 10 | 367.1 | 89.6 | 0.244 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 388.9 ± 43.2 (350.4–537.3) | 36.3 ± 4.7 (30.6–46.1) | 0.093 | 0.087 | below | 66 | 19 | 384.4 | 108.1 | 0.281 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 87.8 ± 1.7 (84.7–91.6) | 38.5 ± 7.7 (30.4–51.2) | 0.439 | 0.359 | below | 48 | 5 | 87.2 | 37.4 | 0.429 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 101.8 ± 15.0 (88.5–138.7) | 15.3 ± 1.6 (12.9–20.4) | 0.151 | 0.145 | below | 48 | 6 | 100.2 | 46.4 | 0.464 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 203.7 ± 42.0 (162.3–275.3) | 52.1 ± 7.1 (49.1–77.9) | 0.256 | 0.302 | below | 57 | 6 | 202.4 | 51.8 | 0.256 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 201.3 ± 31.2 (165.2–262.7) | 19.5 ± 1.5 (16.7–23.7) | 0.097 | 0.101 | below | 57 | 8 | 199.2 | 65.8 | 0.330 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 210.9 ± 17.0 (196.4–251.3) | 80.2 ± 8.7 (75.0–122.6) | 0.380 | 0.382 | below | 55 | 6 | 209.5 | 79.5 | 0.379 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 220.4 ± 27.6 (189.5–304.1) | 27.9 ± 3.2 (24.2–36.4) | 0.127 | 0.127 | below | 55 | 12 | 218.3 | 91.4 | 0.419 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1517.6 ± 90.1 (1415.4–1728.2) | 215.3 ± 21.1 (201.5–294.5) | 0.142 | 0.142 | below | 423 | 14 | 1507.1 | 213.7 | 0.142 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1531.6 ± 103.9 (1392.4–1964.5) | 81.6 ± 10.1 (64.9–103.1) | 0.053 | 0.047 | below | 423 | 30 | 1520.1 | 275.5 | 0.181 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 45.0 ± 6.0 (41.7–67.2) | 17.4 ± 1.1 (16.5–20.4) | 0.385 | 0.395 | below | 43 | 4 | 44.6 | 16.9 | 0.379 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 52.0 ± 9.9 (42.1–69.1) | 9.3 ± 1.2 (7.7–12.4) | 0.178 | 0.183 | below | 43 | 5 | 51.4 | 24.5 | 0.476 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 220.4 ± 38.3 (194.7–341.7) | 89.1 ± 12.8 (80.9–128.1) | 0.404 | 0.415 | below | 59 | 7 | 218.4 | 87.3 | 0.400 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 225.2 ± 22.7 (196.9–276.6) | 28.8 ± 3.2 (25.7–37.6) | 0.128 | 0.130 | below | 60 | 12 | 221.4 | 98.7 | 0.446 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.8 ± 0.3 (11.4–12.2) | 1.8 ± 0.1 (1.7–1.9) | 0.152 | 0.151 | below | 34 | 4 | 11.6 | 1.7 | 0.143 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 13.6 ± 2.1 (11.2–17.7) | 3.3 ± 2.4 (2.1–12.3) | 0.241 | 0.188 | overlap | 34 | 4 | 12.9 | 2.7 | 0.206 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 69.4 ± 10.4 (63.6–107.6) | 14.8 ± 2.2 (12.7–19.9) | 0.213 | 0.200 | below | 44 | 4 | 68.4 | 14.4 | 0.211 | 288/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 72.1 ± 12.7 (62.9–111.2) | 7.3 ± 1.1 (6.5–10.9) | 0.101 | 0.103 | below | 44 | 5 | 70.8 | 17.2 | 0.243 | 288/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 50.5 ± 6.2 (45.5–74.5) | 15.8 ± 1.6 (14.4–21.7) | 0.314 | 0.316 | below | 44 | 5 | 49.7 | 15.2 | 0.306 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 100.0 ± 10.1 (91.1–142.8) | 35.2 ± 5.1 (32.7–52.3) | 0.352 | 0.359 | below | 47 | 8 | 99.0 | 34.9 | 0.352 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 34.0 ± 4.4 (31.9–55.8) | 10.8 ± 2.6 (8.6–19.5) | 0.317 | 0.271 | below | 40 | 4 | 33.5 | 10.1 | 0.300 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 152.6 ± 15.9 (139.3–192.2) | 34.4 ± 6.1 (30.3–51.5) | 0.225 | 0.217 | below | 43 | 9 | 150.1 | 33.3 | 0.222 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 99.3 ± 10.3 (89.7–127.2) | 44.7 ± 4.0 (41.5–60.1) | 0.450 | 0.463 | below | 53 | 13 | 97.9 | 44.2 | 0.451 | 1/1 | r[0] c[0] |

### `m4-d-inproc-harness`

```text
date_utc=2026-10-15T04:49:41Z
commit=aeb32802b502db075ea7bdd4b0cec5a7630a7acc
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.07 1.28 1.44
cpu3_stat_start=692770 0 75109 597326 3296 0 2302 7498 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.11 1.21 1.39
cpu3_stat_end=709473 0 75285 597335 3296 0 2304 7659 0 0
cpu3_steal_ticks=161 cpu3_busy_ticks=16879 cpu3_idle_ticks=9
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 2587.2 ± 86.6 (2468.2–2713.6) | 46 | 2564.8 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 5703.2 ± 99.2 (5555.8–5868.9) | 50 | 5656.6 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.279 s | 10.87 |
| 2 | chloro-base | 1238 | 13.9 | 2.648 s | 5.25 |
| 3 | chloro-base | 1238 | 13.9 | 2.762 s | 5.03 |
| 4 | chloro | 1238 | 13.9 | 1.203 s | 11.56 |
| 5 | chloro | 1238 | 13.9 | 1.191 s | 11.68 |
| 6 | chloro-base | 1238 | 13.9 | 2.813 s | 4.94 |

- chloro: 3 runs, 10.87–11.68 MB/s
- chloro-base: 3 runs, 4.94–5.25 MB/s

### `m4-noise-hir-ty-1`

```text
date_utc=2026-10-15T04:57:25Z
commit=aeb32802b502db075ea7bdd4b0cec5a7630a7acc
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.14 1.59 1.56
cpu3_stat_start=735470 0 76200 599483 3299 0 2312 7873 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.80 1.68 1.59
cpu3_stat_end=742651 0 76486 602699 3299 0 2313 7930 0 0
cpu3_steal_ticks=57 cpu3_busy_ticks=7467 cpu3_idle_ticks=3216
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 915.2 ± 42.0 (846.7–1046.3) | 279.0 ± 17.8 (262.0–318.9) | 645.3 ± 49.1 (600.0–771.4) | 0.432 | below | 0.305 | 0.309 | below | 85 | 8 | 908.1 | 276.5 | 0.305 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 940.3 ± 65.7 (857.4–1094.9) | 80.8 ± 8.1 (73.7–106.9) | 209.0 ± 12.6 (191.3–240.7) | 0.387 | below | 0.086 | 0.086 | below | 85 | 18 | 934.1 | 298.7 | 0.320 | 122/122 | r[0] c[0] |

### `m4-noise-hir-ty-2`

```text
date_utc=2026-10-15T04:59:12Z
commit=aeb32802b502db075ea7bdd4b0cec5a7630a7acc
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.80 1.68 1.59
cpu3_stat_start=742656 0 76487 602706 3299 0 2313 7935 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.76 1.68 1.60
cpu3_stat_end=750728 0 76835 605123 3300 0 2314 8017 0 0
cpu3_steal_ticks=82 cpu3_busy_ticks=8420 cpu3_idle_ticks=2417
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 948.3 ± 53.8 (865.2–1078.2) | 280.5 ± 20.1 (259.2–338.0) | 638.7 ± 28.9 (589.8–705.9) | 0.439 | below | 0.296 | 0.300 | below | 85 | 8 | 935.7 | 278.4 | 0.298 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 951.6 ± 60.2 (863.3–1114.6) | 80.5 ± 6.5 (72.5–96.0) | 219.3 ± 20.0 (181.0–272.0) | 0.367 | below | 0.085 | 0.084 | below | 85 | 18 | 943.0 | 294.0 | 0.312 | 122/122 | r[0] c[0] |

### `m4-noise-hir-ty-3`

```text
date_utc=2026-10-15T05:01:02Z
commit=aeb32802b502db075ea7bdd4b0cec5a7630a7acc
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.76 1.68 1.60
cpu3_stat_start=750729 0 76836 605135 3300 0 2314 8017 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=37d1d72d07248151
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=93ee7b03e8fd238f
bench=/home/user/chloro/target/release/examples/bench sha256=819d6f68ebd5c415
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.91 1.76 1.63
cpu3_stat_end=758590 0 77152 608101 3300 0 2315 8105 0 0
cpu3_steal_ticks=88 cpu3_busy_ticks=8177 cpu3_idle_ticks=2966
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 965.2 ± 46.5 (879.9–1075.0) | 297.2 ± 30.2 (265.3–378.6) | 675.0 ± 74.7 (605.3–1005.4) | 0.440 | below | 0.308 | 0.301 | below | 85 | 8 | 956.0 | 293.9 | 0.307 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 969.2 ± 56.8 (864.9–1064.5) | 82.2 ± 5.1 (74.9–93.0) | 212.0 ± 16.2 (183.9–252.7) | 0.388 | below | 0.085 | 0.087 | below | 85 | 19 | 960.9 | 302.7 | 0.315 | 122/122 | r[0] c[0] |
