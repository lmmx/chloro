# M2: base, head and head-pgo against rustfmt on a quiet host (E7b integration head)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 2 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the E7b merge and on rustfmt 1.9.0,
with the method of [m1-quiet-host.md](m1-quiet-host.md). Every figure in this file is
**measured** in the windows named below; nothing is estimated. The host is the 2.80 GHz host
class of [../baseline.md](../baseline.md) and
[../experiments/EXP-007b-flat-port.md](../experiments/EXP-007b-flat-port.md), not the
2.10 GHz host of M1 (protocol §3: figures from different hosts are not compared; the
"against M1" section sets only within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `8d418ee` (the integration branch after the E7b merge: E6 cache, E1 parallel items, E2 `[profile.release]` `lto = "fat"`, `codegen-units = 1`, E4 green reads, E8 `rustfmt_version`, E7b flat substrate) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script now also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `8d418eee2de08c56e1324bc424b1aa8e96929638` (worktree clean, `worktree_dirty=0`
  in every `env.txt`); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git worktree add
  /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `bda41f77264a302147977119425a4c12b3b1885a4849b044a5e1569738a6d58d` | 1,928,728 |
| `head` bench | `target/release/examples/bench` | `cbcc51c4c2aa9bc9539ac4a7f7f6be07a8b7ad5acc2b3953a0bff5b4222b1115` | 1,723,832 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `bc225b1e65ec9185873ea50c43d75b054acfcf1ab776c1c10e43408c09606a81` | 1,565,160 |
| `head` conform | `target/release/examples/conform` | `51dc1a34058ad7bfaba438fd32af9812ace922d10f0d707e2d922129a0b9873f` | 1,693,584 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `711a973433c3bb01b1884c84171e971128ce251786de9a475fd2f26155d660bf` | 2,592,152 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `60c8001a15c7049bbed5748e07652ebdfe97c750e0e163ae1e60f9818185b759` | 2,387,912 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes equal the ones recorded in [m1-quiet-host.md](m1-quiet-host.md) and
  the chloro hash equals the one in [../baseline.md](../baseline.md): the default-profile
  build of `884f9c9` reproduces byte for byte on this toolchain.
- The `head` chloro binary is 1,928,728 bytes against 2,048,304 for M1's `dc20cd7` build and
  2,516,048 for `base` (0.767x base); `head-pgo` is 2,592,152 bytes (1.030x base, 0.851x M1's
  3,047,112).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 17 lines for `head`,
  38 for `base`.
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, installed for `pgo.sh`);
  hyperfine 2.0.0 (`cargo install hyperfine --locked`); valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. This is the 2.80 GHz
  host class of [../baseline.md](../baseline.md) and of the E4, E7 and E7b records, not the
  2.10 GHz host of M1 and EXP-001.
- **Load:** no other agent ran on the host. The 1-minute load average read 0.34 before the
  first process of the session; the builds (`base`, `head`, `pgo.sh` with its training runs),
  `cargo install hyperfine` and the harness's workload preparation preceded the sweeps, so the
  load at the first sweep's start is their tail plus the preparation's own rustfmt runs. The
  load average at the start of every later sweep is the harness's own processes from the
  previous sweep (the `mc` rows run chloro with 4 threads). `cpu3` columns are `/proc/stat`
  ticks of core 3 (USER_HZ 100) over the sweep: steal / busy / idle.

| results directory | start (UTC) | load average at start | at end | cpu3 steal / busy / idle ticks |
|---|---|---|---|---|
| `a1-head-base-formatted` | 2026-10-14T20:22:05Z | 1.14 2.01 1.16 | 1.75 1.88 1.40 | 510 / 33360 / 8142 |
| `a2-pgo-base-formatted` | 2026-10-14T20:29:07Z | 1.75 1.88 1.40 | 2.11 1.90 1.54 | 396 / 27487 / 8743 |
| `a3-head-raw` | 2026-10-14T20:35:14Z | 2.11 1.90 1.54 | 1.26 1.42 1.43 | 423 / 29511 / 11204 |
| `d-inproc-harness` | 2026-10-14T20:42:06Z | 1.24 1.41 1.43 | 1.02 1.23 1.35 | 227 / 19424 / 12 |
| `c-cache` | 2026-10-14T20:45:24Z | 1.02 1.23 1.35 | 2.54 2.03 1.66 | 196 / 13959 / 627 |
| `d-inproc` | 2026-10-14T20:47:52Z | 2.54 2.03 1.66 | 1.20 1.69 1.58 | 183 / 12420 / 41 |
| `e-parallel-items` | 2026-10-14T20:49:59Z | 1.20 1.69 1.58 | 1.15 1.60 1.55 | 54 / 3196 / 1687 |
| `noise-hir-ty-1` | 2026-10-14T20:50:48Z | 1.15 1.60 1.55 | 1.54 1.62 1.56 | 149 / 8202 / 3152 |
| `noise-hir-ty-2` | 2026-10-14T20:52:44Z | 1.54 1.62 1.56 | 1.83 1.70 1.59 | 131 / 8109 / 3329 |
| `noise-hir-ty-3` | 2026-10-14T20:54:40Z | 1.83 1.70 1.59 | 1.95 1.87 1.67 | 129 / 8561 / 2739 |

- Steal on core 3 is 1.0–1.4% of the sweep's ticks (sweep 1: 510 of 42,012; sweep 2: 396 of
  36,626; sweep 3: 423 of 41,138; the noise runs: 129–149 of 11,429–11,569). The pinned rows carry this
  steal.
- The sweeps ran back to back in one chain (`a1`, `a2`, `a3`, `d-inproc-harness`, `c-cache`,
  `d-inproc`, `e-parallel-items`, the three noise runs) from 20:22 to 20:56 UTC with
  no suspension.

## Method

- All process-level figures: `benchmarks/compare_rustfmt.sh` with hyperfine 2.0 `-N -i`,
  `--metrics time_wall_clock,time_user,time_system,memory_peak_resident`, 2 warmup runs and
  **15 timed runs per command per round, 2 rounds with the command order reversed** (n = 30
  per cell), pinned rows on core 3, `mc` rows unpinned, `--mode check`. `report.py` merges
  the rounds; the tables below are built from the same JSON exports (min / mean ± σ wall ms
  / peak RSS MiB; CPU is mean user+sys ms over all threads).
- **The cache is disabled in every sweep except measurement C** by the harness itself: since
  M1 `compare_rustfmt.sh` passes `--no-cache` to any chloro binary whose `--help` lists the
  flag unless `--with-cache` is given (`env.txt` records `chloro_cache_flag=--no-cache`);
  `base` has no cache and gets no flag. No environment workaround is in use. Verified before
  the sweeps: `head -v --check --no-cache` prints `Cache: disabled (--no-cache)`.
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found), made by
  a 1-run pass of the harness before the sweeps. The registry crates `tokio-1.53.3`,
  `regex-syntax-0.8.11`, `winnow-0.7.15` and `clap_builder-4.6.7` were absent from this
  container's `~/.cargo/registry` and were fetched with a scratch `Cargo.toml` pinning the
  four versions (`cargo fetch`).
- The results directories (gitignored, under `benchmarks/results/`):

| directory | command |
|---|---|
| `a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `d-inproc` | by-hand `taskset -c 3 bench -n 5` triples and `benchmarks/profile.sh --runs 2 --core 3` per binary |
| `e-parallel-items` | by-hand hyperfine, measurement E |
| `noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`). The `ranges` column lists, in order, head vs base, head-pgo vs base,
  head vs rustfmt, head-pgo vs rustfmt: `below` = every run of the first command took less
  wall time than every run of the second; `overlap` = the min–max ranges intersect.
- Output agreement in every crate, files and stdin workload: `identical_outputs` equals the
  file count (15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 287/287, 1/1
  ×5) on formatted input; on raw input the same, except `tiny-test-data` 288/289. Exit codes
  are 0 for every command on formatted input, 1 for both tools on raw input where files
  change (`reg-clap-builder` and the stdin rows exit 0).

## Crate level, formatted input, `--check` (measurement A)

### Crate level, all CPUs (`mc`), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 90.5 / 104.9 ± 12.2 / 24 | 40.1 / 49.8 ± 7.2 / 18 | 38.5 / 44.4 ± 4.7 / 17 | 279.3 / 322.0 ± 31.7 / 57 | 282.7 / 325.1 ± 28.5 / 56 | 0.475 | 0.394 | 0.155 | 0.137 | below; below; below; below | 233.4 | 162.2 | 148.2 | 316.3 | 0.695 | 0.600 | 0.513 | 0.462 | b[0]h[0]r[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 195.1 / 232.6 ± 25.4 / 35 | 109.1 / 126.4 ± 12.7 / 21 | 93.5 / 101.9 ± 6.1 / 22 | 873.5 / 968.4 ± 62.3 / 85 | 886.7 / 976.7 ± 55.3 / 85 | 0.543 | 0.468 | 0.130 | 0.104 | below; below; below; below | 736.9 | 468.5 | 376.4 | 955.6 | 0.636 | 0.546 | 0.490 | 0.393 | b[0]h[0]r[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 102.6 / 140.2 ± 17.6 / 21 | 56.7 / 70.8 ± 8.2 / 19 | 41.3 / 50.9 ± 5.7 / 20 | 306.3 / 352.3 ± 32.3 / 58 | 317.5 / 344.8 ± 28.0 / 58 | 0.505 | 0.364 | 0.201 | 0.148 | below; below; below; below | 251.2 | 185.9 | 145.9 | 343.0 | 0.740 | 0.577 | 0.542 | 0.431 | b[0]h[0]r[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 42.8 / 51.4 ± 6.4 / 13 | 21.6 / 24.5 ± 2.1 / 8 | 19.1 / 22.7 ± 2.6 / 9 | 128.3 / 148.5 ± 16.4 / 50 | 131.3 / 147.7 ± 15.6 / 50 | 0.476 | 0.477 | 0.165 | 0.154 | below; below; below; below | 135.7 | 82.6 | 74.5 | 145.8 | 0.608 | 0.579 | 0.566 | 0.517 | b[0]h[0]r[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 23.2 / 29.8 ± 4.0 / 10 | 14.7 / 17.2 ± 2.4 / 6 | 12.4 / 14.1 ± 0.8 / 7 | 74.3 / 85.8 ± 10.9 / 47 | 73.9 / 83.4 ± 11.2 / 47 | 0.578 | 0.453 | 0.201 | 0.169 | overlap; below; below; below | 83.1 | 53.4 | 43.3 | 82.9 | 0.642 | 0.521 | 0.643 | 0.528 | b[0]h[0]r[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 15.4 / 19.7 ± 2.4 / 7 | 8.1 / 9.6 ± 1.5 / 5 | 7.7 / 8.9 ± 0.9 / 6 | 41.2 / 48.3 ± 6.2 / 43 | 41.8 / 47.3 ± 6.3 / 43 | 0.486 | 0.446 | 0.198 | 0.188 | below; below; below; below | 45.1 | 26.3 | 23.8 | 46.8 | 0.584 | 0.542 | 0.563 | 0.515 | b[0]h[0]r[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 125.4 / 160.1 ± 25.9 / 37 | 83.2 / 99.2 ± 9.4 / 31 | 66.4 / 75.7 ± 8.0 / 33 | 451.2 / 520.5 ± 44.7 / 66 | 448.9 / 482.4 ± 20.5 / 66 | 0.620 | 0.531 | 0.191 | 0.157 | below; below; below; below | 483.2 | 340.1 | 256.6 | 511.1 | 0.704 | 0.575 | 0.665 | 0.539 | b[0]h[0]r[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 57.3 / 70.2 ± 8.1 / 18 | 32.4 / 38.2 ± 2.9 / 12 | 29.3 / 35.8 ± 3.5 / 13 | 195.6 / 227.5 ± 27.9 / 55 | 192.9 / 219.6 ± 20.0 / 55 | 0.544 | 0.536 | 0.168 | 0.163 | below; below; below; below | 198.4 | 128.2 | 117.8 | 224.3 | 0.646 | 0.619 | 0.572 | 0.547 | b[0]h[0]r[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 66.4 / 76.8 ± 6.0 / 18 | 33.6 / 37.2 ± 2.4 / 11 | 28.9 / 32.0 ± 2.6 / 12 | 180.3 / 207.3 ± 21.3 / 59 | 185.1 / 206.0 ± 16.3 / 59 | 0.484 | 0.439 | 0.179 | 0.155 | below; below; below; below | 226.5 | 130.9 | 111.4 | 203.7 | 0.578 | 0.513 | 0.642 | 0.552 | b[0]h[0]r[0] |

- `head/base` on all CPUs: 0.475 (`fx-hir`) to 0.620 (`reg-regex-syntax`); every head run is
  below every base run on eight crates; the ranges overlap on `fx-parser` (14.7–23.7 vs
  23.2–39.9 ms).
- `head/rustfmt` on all CPUs: 0.130 (`fx-hir-ty`) to 0.201 (`fx-ide-db`, `fx-parser`); every
  head run and every head-pgo run is below every rustfmt run on all nine crates.
- `pgo/base` on all CPUs: 0.364 (`fx-ide-db`) to 0.536 (`reg-clap-builder`), every head-pgo
  run below every base run on all nine crates; `pgo/rustfmt` 0.104 (`fx-hir-ty`) to 0.188
  (`reg-tokio`). `head-pgo` is below `head` in mean on all nine crates (the closest:
  `fx-syntax` 22.7 vs 24.5 ms, `reg-tokio` 8.9 vs 9.6 ms).
- CPU `head/base` on all CPUs: 0.578 (`reg-winnow`) to 0.740 (`fx-ide-db`); CPU `pgo/base`
  0.513–0.619. The CPU ratio is above the wall ratio on every crate (the 4-thread rows pay
  the E1 split in CPU and get it back in wall).
- Peak RSS on all CPUs: head 5–31 MiB against base 7–37 MiB and rustfmt 43–85 MiB; head is
  below base on every crate (`fx-ide-db` 19 vs 21 MiB, the crate where M1 read head above
  base); head-pgo 6–33 MiB, below base on every crate.

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 191.5 / 227.5 ± 29.1 / 16 | 125.2 / 138.1 ± 12.1 / 9 | 111.8 / 121.6 ± 8.5 / 10 | 282.2 / 315.2 ± 33.8 / 57 | 276.5 / 308.1 ± 30.3 / 57 | 0.607 | 0.575 | 0.438 | 0.395 | below; below; below; below | 223.4 | 136.2 | 120.3 | 310.7 | 0.610 | 0.576 | 0.438 | 0.394 | b[0]h[0]r[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 594.5 / 665.5 ± 56.7 / 18 | 394.1 / 435.3 ± 29.8 / 8 | 347.2 / 375.6 ± 24.5 / 9 | 870.6 / 982.3 ± 64.8 / 85 | 871.2 / 983.8 ± 61.2 / 85 | 0.654 | 0.542 | 0.443 | 0.382 | below; below; below; below | 659.0 | 428.0 | 370.4 | 965.3 | 0.649 | 0.541 | 0.443 | 0.382 | b[0]h[0]r[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 214.8 / 249.8 ± 24.6 / 15 | 143.5 / 166.9 ± 16.4 / 10 | 110.7 / 120.4 ± 12.2 / 11 | 299.5 / 348.0 ± 33.9 / 58 | 303.7 / 327.8 ± 23.3 / 58 | 0.668 | 0.484 | 0.480 | 0.367 | overlap; below; below; below | 244.4 | 163.7 | 118.4 | 343.0 | 0.670 | 0.486 | 0.477 | 0.364 | b[0]h[0]r[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 116.9 / 146.3 ± 27.1 / 9 | 66.0 / 77.8 ± 14.2 / 6 | 57.8 / 60.6 ± 2.1 / 6 | 128.0 / 150.6 ± 22.7 / 50 | 126.7 / 141.7 ± 14.2 / 50 | 0.532 | 0.511 | 0.516 | 0.428 | overlap; below; below; below | 142.5 | 76.6 | 60.0 | 147.4 | 0.538 | 0.512 | 0.520 | 0.431 | b[0]h[0]r[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 64.7 / 75.7 ± 10.5 / 7 | 38.8 / 44.5 ± 5.8 / 5 | 34.2 / 36.2 ± 2.1 / 5 | 72.8 / 79.8 ± 8.4 / 47 | 73.2 / 86.2 ± 12.2 / 47 | 0.588 | 0.497 | 0.558 | 0.420 | below; below; below; below | 74.1 | 43.7 | 35.5 | 79.0 | 0.590 | 0.501 | 0.553 | 0.420 | b[0]h[0]r[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 33.5 / 37.7 ± 6.0 / 6 | 19.9 / 21.2 ± 0.8 / 4 | 17.7 / 21.0 ± 3.9 / 5 | 39.3 / 44.8 ± 7.2 / 43 | 39.6 / 42.1 ± 2.2 / 43 | 0.562 | 0.575 | 0.473 | 0.499 | below; below; below; below | 37.1 | 20.7 | 20.6 | 44.3 | 0.559 | 0.574 | 0.468 | 0.494 | b[0]h[0]r[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 414.4 / 469.9 ± 39.9 / 29 | 267.7 / 308.6 ± 34.6 / 14 | 207.3 / 220.4 ± 15.2 / 15 | 459.3 / 506.3 ± 37.9 / 66 | 453.4 / 487.6 ± 30.2 / 66 | 0.657 | 0.485 | 0.609 | 0.452 | below; below; below; below | 462.2 | 303.8 | 217.6 | 500.0 | 0.657 | 0.486 | 0.608 | 0.451 | b[0]h[0]r[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 166.0 / 190.6 ± 25.3 / 12 | 99.1 / 112.7 ± 12.5 / 6 | 89.6 / 94.4 ± 4.7 / 7 | 192.5 / 217.8 ± 24.2 / 55 | 196.4 / 225.0 ± 28.7 / 55 | 0.591 | 0.522 | 0.517 | 0.420 | below; below; below; below | 187.6 | 110.5 | 93.3 | 214.2 | 0.589 | 0.520 | 0.516 | 0.423 | b[0]h[0]r[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 188.5 / 213.0 ± 19.9 / 11 | 107.0 / 121.7 ± 13.1 / 7 | 92.7 / 101.3 ± 9.8 / 8 | 181.1 / 199.1 ± 13.2 / 59 | 179.8 / 197.5 ± 13.5 / 59 | 0.571 | 0.476 | 0.611 | 0.513 | below; below; below; below | 209.9 | 118.9 | 99.4 | 195.6 | 0.567 | 0.476 | 0.608 | 0.515 | b[0]h[0]r[0] |

- Per core (both tools on core 3), `head/base` wall: 0.532 (`fx-syntax`) to 0.668
  (`fx-ide-db`); CPU `head/base` 0.538–0.670. Every head run is below every base run on seven
  crates; the ranges overlap on `fx-ide-db` (143.5–214.9 vs 214.8–309.8 ms) and `fx-syntax`
  (66.0–121.5 vs 116.9–208.2 ms). The CPU ratio and the wall ratio agree to within 0.01 on
  every row (the pinned process is not waiting for its core).
- Per core, `head/rustfmt` wall: 0.438 (`fx-hir`) to 0.611 (`reg-winnow`); CPU 0.438–0.608.
  Every head run is below every rustfmt run on all nine crates.
- Per core, `pgo/base` wall: 0.476 (`reg-winnow`) to 0.575 (`fx-hir`, `reg-tokio`), every
  head-pgo run below every base run on all nine crates; `pgo/rustfmt` 0.367 (`fx-ide-db`) to
  0.513 (`reg-winnow`), every head-pgo run below every rustfmt run on all nine. The PGO
  build's mean wall and mean CPU are below head's on every crate (`reg-tokio` the closest:
  21.0 vs 21.2 ms wall, 20.6 vs 20.7 ms CPU).
- `head` min is below `base` min on every crate, pinned and unpinned; `head-pgo` min is below
  `base` min on every crate.
- Peak RSS per core: head 4–14 MiB against base 6–29 MiB and rustfmt 43–85 MiB.

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 12.6 / 15.3 ± 2.2 / 5 | 8.6 / 10.2 ± 1.3 / 4 | 7.8 / 9.0 ± 0.9 / 5 | 49.4 / 60.2 ± 11.2 / 44 | 49.4 / 55.6 ± 9.1 / 44 | 0.663 | 0.625 | 0.169 | 0.161 | overlap; below; below; below | 31.0 | 20.4 | 18.0 | 59.0 | 0.659 | 0.637 | 0.346 | 0.328 | b[0]h[0]r[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 22.2 / 26.0 ± 4.8 / 5 | 14.7 / 16.1 ± 1.7 / 4 | 14.1 / 14.6 ± 0.5 / 5 | 48.5 / 54.5 ± 5.5 / 44 | 48.5 / 52.3 ± 4.7 / 44 | 0.619 | 0.605 | 0.295 | 0.280 | overlap; below; below; below | 25.4 | 15.8 | 14.3 | 53.7 | 0.622 | 0.608 | 0.294 | 0.278 | b[0]h[0]r[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 1.9 / 2.5 ± 0.5 / 4 | 2.1 / 2.5 ± 0.3 / 4 | 1.9 / 2.2 ± 0.3 / 4 | 9.4 / 12.2 ± 1.8 / 34 | 9.1 / 10.8 ± 1.6 / 34 | 0.996 | 1.001 | 0.206 | 0.199 | overlap; overlap; below; below | 2.2 | 2.3 | 2.0 | 11.7 | 1.032 | 0.986 | 0.196 | 0.191 | b[0]h[0]r[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.7 / 2.0 ± 0.7 / 4 | 1.7 / 2.0 ± 0.3 / 4 | 1.7 / 1.9 ± 0.3 / 4 | 8.9 / 10.8 ± 2.4 / 34 | 9.0 / 10.4 ± 1.5 / 34 | 1.033 | 0.814 | 0.188 | 0.186 | overlap; overlap; below; below | 1.7 | 1.8 | 1.8 | 10.2 | 1.090 | 0.818 | 0.180 | 0.176 | b[0]h[0]r[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 84.0 / 92.3 ± 10.2 / 10 | 69.4 / 79.6 ± 12.5 / 10 | – | 139.0 / 158.4 ± 16.1 / 43 | – | 0.862 | – | 0.502 | – | overlap; –; below; – | 91.4 | 78.4 | – | 155.3 | 0.859 | – | 0.505 | – | b[0]h[0]r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 78.8 / 91.3 ± 10.7 / 14 | 49.7 / 54.5 ± 7.0 / 14 | – | 87.1 / 94.4 ± 7.3 / 53 | – | 0.597 | – | 0.577 | – | below; –; below; – | 89.0 | 53.9 | – | 92.9 | 0.606 | – | 0.581 | – | b[0]h[0]r[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 65.4 / 75.8 ± 10.3 / 9 | 44.4 / 49.0 ± 4.0 / 8 | – | 90.9 / 102.7 ± 14.7 / 47 | – | 0.646 | – | 0.477 | – | below; –; below; – | 74.6 | 47.3 | – | 101.2 | 0.634 | – | 0.467 | – | b[0]h[0]r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 17.6 / 19.1 ± 2.7 / 5 | 11.7 / 13.9 ± 3.1 / 4 | – | 28.7 / 33.8 ± 6.4 / 40 | – | 0.728 | – | 0.411 | – | overlap; –; below; – | 18.2 | 13.5 | – | 33.4 | 0.742 | – | 0.405 | – | b[0]h[0]r[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 25.9 / 28.9 ± 3.3 / 6 | 18.4 / 20.9 ± 3.3 / 5 | – | 41.9 / 48.3 ± 8.6 / 44 | – | 0.722 | – | 0.433 | – | overlap; –; below; – | 28.0 | 20.2 | – | 47.5 | 0.720 | – | 0.425 | – | b[0]h[0]r[0] |

- `tiny-one` (one 13-byte file, process start-up): head 2.0 ms pinned and 2.5 ms unpinned
  against base 2.0 and 2.5 ms; head-pgo 1.9 and 2.2 ms; rustfmt 10.8 and 12.2 ms. `head/base`
  reads 0.996 unpinned and 1.033 pinned, `pgo/base` 1.001 and 0.814; every one of these has
  overlapping ranges. Every head and head-pgo run is below every rustfmt run.
- `tiny-test-data` (287 files): head/base 0.663 unpinned and 0.619 pinned with overlapping
  ranges; pgo/base 0.625 and 0.605, every head-pgo run below every base run; head/rustfmt
  0.169 and 0.295, every head run below every rustfmt run.
- stdin (`examples/fmt_stdin`, no threads, no file I/O, no cache code, no PGO build):
  head/base 0.597 (`stdin-nodes`) to 0.862 (`stdin-lints`); every head run is below every
  base run on `stdin-nodes` and `stdin-hir-lib`, the other three overlap. head/rustfmt
  0.411 (`stdin-hover-tests`) to 0.577 (`stdin-nodes`), every head run below every rustfmt
  run on all five files. CPU ratios equal the wall ratios to within 0.02.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 625 | 30 | 549.4 / 593.6 ± 20.7 / 218 | 39.7 / 46.9 ± 4.1 / 16 | 0.079 | below | 585.4 | 156.8 | 0.268 | 15/15 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 539.6 / 582.8 ± 36.3 / 218 | 127.9 / 149.1 ± 34.2 / 9 | 0.256 | below | 577.3 | 145.3 | 0.252 | 15/15 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1222.1 / 1296.8 ± 50.8 / 116 | 107.3 / 115.1 ± 6.2 / 22 | 0.089 | below | 1278.9 | 429.6 | 0.336 | 122/122 | r[1] h[1] |
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1217.7 / 1310.5 ± 49.9 / 116 | 393.7 / 443.4 ± 33.5 / 8 | 0.338 | below | 1290.6 | 432.7 | 0.335 | 122/122 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 344.6 / 396.4 ± 32.5 / 66 | 55.0 / 73.7 ± 8.4 / 19 | 0.186 | below | 390.1 | 196.1 | 0.503 | 33/33 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 350.8 / 382.8 ± 25.2 / 66 | 148.9 / 171.9 ± 30.3 / 10 | 0.449 | below | 376.2 | 169.7 | 0.451 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 166.0 / 187.1 ± 21.9 / 57 | 22.2 / 25.2 ± 2.3 / 8 | 0.135 | below | 184.5 | 85.7 | 0.464 | 33/33 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 160.5 / 175.2 ± 14.3 / 57 | 65.6 / 68.0 ± 2.2 / 6 | 0.388 | below | 173.6 | 67.4 | 0.388 | 33/33 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 82.0 / 93.5 ± 10.8 / 48 | 13.8 / 15.2 ± 1.4 / 6 | 0.162 | below | 91.1 | 48.0 | 0.527 | 28/28 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 83.0 / 94.2 ± 8.1 / 48 | 39.7 / 44.3 ± 5.0 / 5 | 0.470 | below | 91.6 | 43.3 | 0.473 | 28/28 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 42.4 / 45.2 ± 2.8 / 43 | 8.2 / 10.1 ± 1.1 / 5 | 0.224 | below | 43.9 | 27.9 | 0.635 | 49/49 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 40.5 / 44.8 ± 6.0 / 43 | 19.9 / 20.5 ± 0.6 / 4 | 0.457 | below | 44.1 | 20.2 | 0.458 | 49/49 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1421.9 / 1491.6 ± 48.4 / 423 | 82.1 / 95.3 ± 10.0 / 32 | 0.064 | below | 1470.8 | 332.3 | 0.226 | 33/33 | r[1] h[1] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1391.7 / 1503.0 ± 70.5 / 423 | 261.3 / 278.7 ± 17.1 / 14 | 0.185 | below | 1485.3 | 275.9 | 0.186 | 33/33 | r[1] h[1] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 194.3 / 223.4 ± 25.5 / 55 | 31.5 / 40.3 ± 7.7 / 12 | 0.180 | below | 218.9 | 127.2 | 0.581 | 57/57 | r[0] h[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 194.3 / 220.3 ± 31.5 / 55 | 99.5 / 104.5 ± 6.0 / 6 | 0.474 | below | 217.3 | 103.5 | 0.476 | 57/57 | r[0] h[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 188.2 / 212.3 ± 17.1 / 60 | 32.6 / 36.3 ± 2.9 / 11 | 0.171 | below | 207.8 | 127.3 | 0.613 | 61/61 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 183.1 / 203.6 ± 22.2 / 60 | 107.3 / 116.1 ± 7.7 / 7 | 0.570 | below | 201.3 | 113.6 | 0.565 | 61/61 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 60.3 / 67.0 ± 13.9 / 44 | 8.1 / 9.1 ± 0.8 / 4 | 0.135 | below | 66.0 | 18.9 | 0.286 | 288/289 | r[1] h[1] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 60.5 / 68.8 ± 10.7 / 44 | 14.8 / 17.6 ± 2.0 / 4 | 0.255 | below | 68.0 | 17.2 | 0.253 | 288/289 | r[1] h[1] |
| tiny-one | mc (none) | 1 | 0 | 30 | 9.6 / 11.1 ± 2.1 / 34 | 2.0 / 2.4 ± 0.2 / 4 | 0.212 | below | 10.7 | 2.2 | 0.203 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 7.7 / 9.1 ± 1.8 / 34 | 1.6 / 1.7 ± 0.1 / 4 | 0.191 | below | 8.3 | 1.6 | 0.194 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 136.7 / 147.5 ± 12.3 / 43 | 68.9 / 72.6 ± 6.9 / 10 | 0.492 | below | 146.2 | 72.0 | 0.492 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 88.2 / 97.7 ± 11.8 / 53 | 48.6 / 53.1 ± 5.2 / 13 | 0.544 | below | 96.1 | 51.2 | 0.533 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 88.0 / 96.0 ± 11.7 / 47 | 44.4 / 46.8 ± 2.5 / 8 | 0.487 | below | 94.1 | 45.8 | 0.486 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 28.8 / 32.8 ± 5.1 / 40 | 11.7 / 15.7 ± 4.8 / 4 | 0.477 | below | 32.0 | 15.0 | 0.470 | 1/1 | r[0] h[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 42.9 / 45.0 ± 1.6 / 44 | 18.6 / 19.4 ± 0.5 / 5 | 0.432 | below | 44.5 | 19.2 | 0.430 | 1/1 | r[0] h[0] |

- rustfmt `--check` on unformatted input includes the line diff of every changed file:
  `reg-regex-syntax` 1503.0 ms and 423 MiB pinned against 506.3 ms and 66 MiB on formatted
  input (sweep 1); `fx-hir` 582.8 ms and 218 MiB against 315.2 ms and 57 MiB. head's figures
  on the two inputs differ by less than 16% on every row (`fx-hir-ty` `1c` 443.4 raw vs
  435.3 formatted; `reg-regex-syntax` `1c` 278.7 vs 308.6; the largest differences are
  `tiny-one` `1c` 1.7 vs 2.0, `fx-syntax` `1c` 68.0 vs 77.8 and `stdin-hover-tests` 15.7 vs
  13.9).
- Every head run is below every rustfmt run on every raw row (27 of 27 `below`).
- The ratios in this table (0.064–0.570) are not like-for-like and are not the ones to quote;
  the formatted-input tables above are.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 14,190,149 bytes)
  copied to a scratch directory and rewritten once by `head --write --no-cache`; `head --check
  --no-cache` on the result exits 0. `CHLORO_CACHE_DIR` points at a scratch directory, which
  holds one file `083aac67e014bbc689da1858fea5dac3.chloro-cache` of 44,152 bytes after a run
  (the file name is the key of this binary; M1's `dc20cd7` build wrote `eb8c3d0c…`, the same
  size).
- Commands: `hyperfine -N -i -w 1 -r 15 --metrics … -n cold --prepare "rm -rf $C" "head
  --check $T" -n warm --prepare "" "head --check $T" -n no-cache --prepare "" "head --check
  --no-cache $T" -n base --prepare "" "base --check $T"`, two rounds with the order reversed
  (n = 30); then one round under `taskset -c 3` for `warm`, `no-cache`, `base` (n = 15).
  `cold` is not in the pinned round.

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 505.4 / 556.5 ± 35.3 / 51 | 26.3 / 30.4 ± 3.9 / 5 | 478.9 / 523.4 ± 23.6 / 46 | 832.9 / 917.2 ± 56.9 / 53 | 17.2 | 30.2 | 0.571 | 2041.3 | 42.3 | 1953.5 | 3114.9 |
| 1c | 15 | – | 37.1 / 38.5 ± 2.0 / 5 | 1789.2 / 1952.4 ± 73.8 / 23 | 2980.5 / 3171.2 ± 104.3 / 33 | 50.7 | 82.3 | 0.616 | – | 37.3 | 1925.7 | 3121.4 |

- Warm `--check` of the 1238 formatted files takes 30.4 ms unpinned (17.2x below `--no-cache`,
  30.2x below base) and 38.5 ms on one core (50.7x below `--no-cache`, 82.3x below base); CPU
  42.3 and 37.3 ms; peak RSS 5 MiB against 46 MiB without the cache.
- `cold` (cache directory removed before every run) reads 556.5 ms against 523.4 ms for
  `no-cache` in mean (1.063x; ranges 505.4–653.2 vs 478.9–564.9 ms overlap): hashing and the
  cache write add 33 ms in mean over the formatter on 4 threads in this window.
- `no-cache` head against base on the same copy: 0.571 unpinned (523.4 vs 917.2 ms) and 0.616
  pinned (1952.4 vs 3171.2 ms); CPU 0.627 and 0.617. Peak RSS of `cold` (51 MiB) and
  `no-cache` (46 MiB) is below base's 53 MiB unpinned; pinned, 23 MiB against 33 MiB.
- M1 measured, on the other host, warm 24.5 ms against 650.8 ms `--no-cache` (26.6x) and 773.4
  ms base (31.6x) unpinned, and 24.9 against 2342.3 and 2815.2 ms pinned (94x, 113x); the
  warm figure here is above M1's and the `--no-cache` figure is below it, so the
  `no-cache/warm` ratio reads lower.

## In process and instructions (measurement D)

`taskset -c 3 bench -n 5` (best of 5 in-process formats of the 1238 fixtures, 13.9 MB), three interleaved triples in the order base, head, head-pgo:

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 2.809 | 4.95 |
| 1 | head | 1238 | 13.9 | 1.843 | 7.54 |
| 1 | head-pgo | 1238 | 13.9 | 1.500 | 9.27 |
| 2 | base | 1238 | 13.9 | 2.765 | 5.03 |
| 2 | head | 1238 | 13.9 | 1.916 | 7.25 |
| 2 | head-pgo | 1238 | 13.9 | 1.522 | 9.13 |
| 3 | base | 1238 | 13.9 | 2.817 | 4.94 |
| 3 | head | 1238 | 13.9 | 1.799 | 7.73 |
| 3 | head-pgo | 1238 | 13.9 | 1.560 | 8.91 |

- base: 3 runs, best time 2.765–2.817 s, 4.94–5.03 MB/s
- head: 3 runs, best time 1.799–1.916 s, 7.25–7.73 MB/s
- head-pgo: 3 runs, best time 1.500–1.560 s, 8.91–9.27 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, `chloro` = head, `chloro-base` = base:

- chloro: 1238 files, 13.9 MB: 1.781s, 7.81 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.735s, 5.08 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.651s, 5.24 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.833s, 7.59 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.807s, 7.69 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.848s, 4.88 MB/s (best of 5)

### Instructions on the 49-file sample (`benchmarks/profile.sh --runs 2 --core 3`)

| binary | run 1 Ir | run 2 Ir | Ir / base |
|---|---|---|---|
| base | 587,989,243 | 587,990,996 | 1.0000 |
| head | 390,689,247 | 390,689,313 | 0.6645 |
| head-pgo | 306,889,907 | 306,889,893 | 0.5219 |

- `bench -n 5`, three interleaved triples: base 4.94–5.03 MB/s, head 7.25–7.73 MB/s, head-pgo
  8.91–9.27 MB/s. The three ranges are pairwise disjoint (protocol §3 rule for a
  single-threaded change: 3 of 3 pairs, non-overlapping); head/base 1.44x–1.56x on the
  per-round MB/s, head-pgo/head 1.15x–1.26x. The harness's own three pairs (same core, 20:42
  UTC) give base 4.88–5.24 and head 7.59–7.81 MB/s, non-overlapping.
- `conform -i` (hyperfine, 9 runs each, core 3): head 3785.3 ± 135.8 ms (3580.5–3916.1)
  against base 5838.6 ± 160.4 ms (5493.8–6012.4), 0.648, every head run below every base
  run; CPU 3741.6 vs 5768.9 ms; peak RSS 46 vs 51 MiB; exit 0 on every run. By hand after the chain (`taskset -c 3 conform -i`, one run each): head
  1219/1220 identical, 0 not idempotent, `rustfmt_version=1.9.0`, 13.8 MB in 1.857 s; base
  1219/1220 identical, 0 not idempotent, 13.8 MB in 2.798 s.
- `Ir` on the 49-file sample (650,213 bytes, `env -i`, `setarch -R`): base 587,989,243, head
  390,689,247 (−33.55%), head-pgo 306,889,893 (−47.81%; 0.7855x head); two runs of each
  agree to within 1,753 instructions. The base count is 69 above M1's 587,989,174 on the
  other host and 667 above the 587,988,576 of [../baseline.md](../baseline.md) for the same
  binary hash; the head count is 1,017 below the 390,690,264 the orchestrator recorded for
  the E7b merge in [../experiments.md](../experiments.md).

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), head with `--no-cache`, `hyperfine -N -i
-w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no `--threads`
option; on one file its static chunking gives the file to one thread, so the `base` column is
the sequential figure with the base's codegen.

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 86.3 / 95.8 ± 7.8 / 10 | 71.9 / 79.2 ± 7.9 / 10 | 47.0 / 53.8 ± 5.3 / 15 | 0.562 | 0.827 | below | 93.5 | 77.1 | 85.8 | 0.917 | b[0] t[0] h[0] |
| stdin-lints | 640 | 1c | 30 | 88.8 / 93.0 ± 3.9 / 11 | 71.5 / 74.4 ± 2.4 / 10 | 70.8 / 76.8 ± 8.7 / 10 | 0.826 | 0.801 | overlap | 91.3 | 73.5 | 75.0 | 0.822 | b[0] t[0] h[0] |
| stdin-nodes | 358 | mc | 30 | 81.7 / 92.2 ± 15.0 / 14 | 49.7 / 55.1 ± 6.5 / 14 | 34.2 / 37.2 ± 1.9 / 14 | 0.403 | 0.598 | below | 89.7 | 54.1 | 61.2 | 0.683 | b[0] t[0] h[0] |
| stdin-nodes | 358 | 1c | 30 | 83.2 / 95.4 ± 12.2 / 14 | 50.8 / 59.0 ± 7.2 / 14 | 51.6 / 53.1 ± 1.2 / 14 | 0.557 | 0.618 | below | 93.2 | 57.5 | 52.5 | 0.563 | b[0] t[0] h[0] |
| stdin-hir-lib | 228 | mc | 30 | 68.6 / 73.8 ± 4.5 / 9 | 46.8 / 51.1 ± 5.2 / 9 | 25.4 / 28.5 ± 2.6 / 9 | 0.386 | 0.692 | below | 72.7 | 50.0 | 56.0 | 0.770 | b[1] t[1] h[1] |
| stdin-hir-lib | 228 | 1c | 30 | 69.3 / 74.7 ± 5.0 / 9 | 47.4 / 52.0 ± 6.0 / 9 | 46.3 / 49.8 ± 5.4 / 9 | 0.666 | 0.696 | overlap | 73.0 | 50.9 | 48.9 | 0.670 | b[1] t[1] h[1] |
| stdin-hover-tests | 211 | mc | 30 | 19.3 / 21.6 ± 3.1 / 6 | 12.9 / 14.5 ± 2.5 / 5 | 8.7 / 9.7 ± 0.6 / 6 | 0.447 | 0.671 | below | 21.0 | 13.8 | 17.2 | 0.820 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | 1c | 30 | 19.6 / 22.9 ± 3.9 / 6 | 14.1 / 15.5 ± 1.3 / 5 | 14.1 / 16.4 ± 2.3 / 5 | 0.716 | 0.677 | overlap | 22.1 | 14.7 | 15.6 | 0.705 | b[0] t[0] h[0] |
| stdin-config | 167 | mc | 30 | 27.5 / 31.3 ± 4.3 / 6 | 19.9 / 21.9 ± 2.8 / 5 | 12.8 / 15.5 ± 4.5 / 6 | 0.496 | 0.702 | overlap | 30.2 | 21.3 | 24.5 | 0.809 | b[1] t[1] h[1] |
| stdin-config | 167 | 1c | 30 | 29.1 / 35.1 ± 6.5 / 6 | 20.6 / 22.7 ± 1.5 / 5 | 20.8 / 22.7 ± 2.1 / 5 | 0.647 | 0.648 | overlap | 33.7 | 22.1 | 21.9 | 0.651 | b[1] t[1] h[1] |

- Unpinned, `head/base` in mean: 0.386 (`stdin-hir-lib`), 0.403 (`stdin-nodes`), 0.447
  (`stdin-hover-tests`), 0.496 (`stdin-config`), 0.562 (`stdin-lints`); on the minima
  1.84x–2.70x (base min / head min: 86.3/47.0, 81.7/34.2, 68.6/25.4, 19.3/8.7, 27.5/12.8).
  Every head run is below every base run on `stdin-lints`, `stdin-nodes`, `stdin-hir-lib` and
  `stdin-hover-tests`; `stdin-config` overlaps (12.8–36.5 vs 27.5–44.2 ms, one head run of 30
  at 36.5 ms).
- `head --threads 1` against base, unpinned: 0.598–0.827 (the codegen and the E4 and E7b
  substrate on one thread); pinned: 0.618–0.801.
- CPU of the split, unpinned `head` against base: 0.683 (`stdin-nodes`) to 0.917
  (`stdin-lints`); every unpinned head row is below base in CPU. Against `head --threads 1`
  the split costs 1.11x (`stdin-lints`, 85.8 vs 77.1 ms) to 1.25x (`stdin-hover-tests`, 17.2
  vs 13.8 ms) CPU. Peak RSS head vs base: 1.44x on `stdin-lints` (15.1 vs 10.5 MiB), 1.00x–1.05x
  on the other four.
- Pinned (4 threads on one core), `head/base` in mean: 0.557–0.826; `head` against `head
  --threads 1` pinned: 0.90 (`stdin-nodes`) to 1.06 (`stdin-hover-tests`, 16.4 vs 15.5 ms),
  ranges overlapping on all five.
- Exit codes: `base`, `head-t1` and `head` exit 1 on every run of `stdin-hir-lib` and
  `stdin-config` and 0 on the other three files; see Harness notes. The CLI outputs of `base`
  and `head` on those two files are byte-identical (`cmp`), so the comparison is between two
  builds doing the same work.

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload.

| run | load at start | `1c` rustfmt | `1c` head | `1c` base | `1c` head/base | `1c` head/rustfmt | `mc` rustfmt | `mc` head | `mc` base | `mc` head/base | `mc` head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 1.14 2.01 1.16 | 871 / 982 ± 65 | 394 / 435 ± 30 | 595 / 665 ± 57 | 0.654 | 0.443 | 873 / 968 ± 62 | 109 / 126 ± 13 | 195 / 233 ± 25 | 0.543 | 0.130 |
| 1 | 1.15 1.60 1.55 | 874 / 929 ± 41 | 383 / 414 ± 27 | 603 / 663 ± 34 | 0.625 | 0.445 | 871 / 942 ± 39 | 105 / 118 ± 11 | 187 / 217 ± 25 | 0.542 | 0.125 |
| 2 | 1.54 1.62 1.56 | 866 / 945 ± 50 | 383 / 412 ± 22 | 593 / 637 ± 34 | 0.646 | 0.436 | 906 / 979 ± 60 | 106 / 114 ± 6 | 183 / 211 ± 16 | 0.542 | 0.117 |
| 3 | 1.83 1.70 1.59 | 852 / 931 ± 55 | 377 / 409 ± 26 | 596 / 663 ± 45 | 0.616 | 0.439 | 866 / 929 ± 38 | 104 / 114 ± 6 | 189 / 214 ± 15 | 0.531 | 0.122 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 409–435 ms
  (6%), the `1c` base mean 637–665 ms (4%), the `1c` rustfmt mean 929–982 ms (6%); the `mc`
  head mean 114–126 ms (11%) and the `mc` base mean 211–233 ms (10%). The `1c` head/base
  ratio reads 0.616–0.654 and the `mc` ratio 0.531–0.543; head/rustfmt reads 0.436–0.445
  (`1c`) and 0.117–0.130 (`mc`). Every head run is below every base run and every rustfmt run
  in all four runs, pinned and unpinned.
- Within one run of 30, max/min on this host reaches 1.28x for rustfmt (`fx-hir-ty` `1c`
  870.6–1118.1 ms, sweep 1), 1.36x for base (`1c` 594.5–806.4 ms) and 1.30x for head (`1c`
  394.1–513.9 ms); σ/mean lies between 4% and 19% on the pinned crate rows of the three
  commands (`reg-tokio` head the narrowest at 3.8%, `fx-syntax` base the widest at 18.5%) and
  between 6% and 16% on the unpinned head crate rows.
- Instruction counts move by at most 1,753 of 588 M (0.0003%) between runs.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir` 1c: head vs base: below (125.2–170.3 vs 191.5–293.5 ms); head vs rustfmt: below (125.2–170.3 vs 282.2–432.1 ms); head-pgo vs base: below (111.8–142.3 vs 189.6–273.0 ms); head-pgo vs rustfmt: below (111.8–142.3 vs 276.5–373.6 ms).
- `fx-hir` mc: head vs base: below (40.1–68.2 vs 90.5–136.1 ms); head vs rustfmt: below (40.1–68.2 vs 279.3–394.9 ms); head-pgo vs base: below (38.5–55.3 vs 90.4–142.3 ms); head-pgo vs rustfmt: below (38.5–55.3 vs 282.7–409.3 ms).
- `fx-hir-ty` 1c: head vs base: below (394.1–513.9 vs 594.5–806.4 ms); head vs rustfmt: below (394.1–513.9 vs 870.6–1118.1 ms); head-pgo vs base: below (347.2–447.2 vs 611.9–824.8 ms); head-pgo vs rustfmt: below (347.2–447.2 vs 871.2–1141.8 ms).
- `fx-hir-ty` mc: head vs base: below (109.1–151.7 vs 195.1–294.6 ms); head vs rustfmt: below (109.1–151.7 vs 873.5–1108.9 ms); head-pgo vs base: below (93.5–121.1 vs 180.0–264.2 ms); head-pgo vs rustfmt: below (93.5–121.1 vs 886.7–1099.8 ms).
- `fx-ide-db` 1c: head vs base: overlap (143.5–214.9 vs 214.8–309.8 ms); head vs rustfmt: below (143.5–214.9 vs 299.5–429.8 ms); head-pgo vs base: below (110.7–175.8 vs 218.5–314.4 ms); head-pgo vs rustfmt: below (110.7–175.8 vs 303.7–386.5 ms).
- `fx-ide-db` mc: head vs base: below (56.7–87.5 vs 102.6–186.7 ms); head vs rustfmt: below (56.7–87.5 vs 306.3–425.8 ms); head-pgo vs base: below (41.3–65.9 vs 121.3–191.8 ms); head-pgo vs rustfmt: below (41.3–65.9 vs 317.5–452.3 ms).
- `fx-parser` 1c: head vs base: below (38.8–61.8 vs 64.7–102.2 ms); head vs rustfmt: below (38.8–61.8 vs 72.8–104.6 ms); head-pgo vs base: below (34.2–43.8 vs 65.4–123.5 ms); head-pgo vs rustfmt: below (34.2–43.8 vs 73.2–116.4 ms).
- `fx-parser` mc: head vs base: overlap (14.7–23.7 vs 23.2–39.9 ms); head vs rustfmt: below (14.7–23.7 vs 74.3–115.9 ms); head-pgo vs base: below (12.4–16.1 vs 25.4–38.4 ms); head-pgo vs rustfmt: below (12.4–16.1 vs 73.9–122.7 ms).
- `fx-syntax` 1c: head vs base: overlap (66.0–121.5 vs 116.9–208.2 ms); head vs rustfmt: below (66.0–121.5 vs 128.0–212.9 ms); head-pgo vs base: below (57.8–66.9 vs 109.7–155.2 ms); head-pgo vs rustfmt: below (57.8–66.9 vs 126.7–182.5 ms).
- `fx-syntax` mc: head vs base: below (21.6–29.5 vs 42.8–71.5 ms); head vs rustfmt: below (21.6–29.5 vs 128.3–220.8 ms); head-pgo vs base: below (19.1–27.9 vs 37.8–60.0 ms); head-pgo vs rustfmt: below (19.1–27.9 vs 131.3–196.5 ms).
- `reg-clap-builder` 1c: head vs base: below (99.1–144.6 vs 166.0–268.3 ms); head vs rustfmt: below (99.1–144.6 vs 192.5–286.6 ms); head-pgo vs base: below (89.6–111.0 vs 167.3–239.9 ms); head-pgo vs rustfmt: below (89.6–111.0 vs 196.4–327.1 ms).
- `reg-clap-builder` mc: head vs base: below (32.4–43.1 vs 57.3–85.4 ms); head vs rustfmt: below (32.4–43.1 vs 195.6–314.5 ms); head-pgo vs base: below (29.3–44.4 vs 56.4–82.3 ms); head-pgo vs rustfmt: below (29.3–44.4 vs 192.9–277.8 ms).
- `reg-regex-syntax` 1c: head vs base: below (267.7–400.3 vs 414.4–573.1 ms); head vs rustfmt: below (267.7–400.3 vs 459.3–626.9 ms); head-pgo vs base: below (207.3–271.5 vs 418.0–560.4 ms); head-pgo vs rustfmt: below (207.3–271.5 vs 453.4–569.4 ms).
- `reg-regex-syntax` mc: head vs base: below (83.2–118.9 vs 125.4–235.0 ms); head vs rustfmt: below (83.2–118.9 vs 451.2–633.7 ms); head-pgo vs base: below (66.4–104.3 vs 121.9–197.6 ms); head-pgo vs rustfmt: below (66.4–104.3 vs 448.9–538.3 ms).
- `reg-tokio` 1c: head vs base: below (19.9–24.4 vs 33.5–54.3 ms); head vs rustfmt: below (19.9–24.4 vs 39.3–65.5 ms); head-pgo vs base: below (17.7–29.3 vs 33.9–53.8 ms); head-pgo vs rustfmt: below (17.7–29.3 vs 39.6–50.9 ms).
- `reg-tokio` mc: head vs base: below (8.1–15.3 vs 15.4–25.6 ms); head vs rustfmt: below (8.1–15.3 vs 41.2–64.1 ms); head-pgo vs base: below (7.7–10.9 vs 14.8–29.0 ms); head-pgo vs rustfmt: below (7.7–10.9 vs 41.8–65.5 ms).
- `reg-winnow` 1c: head vs base: below (107.0–153.9 vs 188.5–268.1 ms); head vs rustfmt: below (107.0–153.9 vs 181.1–233.3 ms); head-pgo vs base: below (92.7–125.1 vs 186.5–260.7 ms); head-pgo vs rustfmt: below (92.7–125.1 vs 179.8–227.9 ms).
- `reg-winnow` mc: head vs base: below (33.6–42.0 vs 66.4–90.4 ms); head vs rustfmt: below (33.6–42.0 vs 180.3–272.6 ms); head-pgo vs base: below (28.9–38.3 vs 60.2–91.2 ms); head-pgo vs rustfmt: below (28.9–38.3 vs 185.1–250.3 ms).
- `stdin-config` 1c: head vs base: overlap (18.4–33.4 vs 25.9–41.1 ms); head vs rustfmt: below (18.4–33.4 vs 41.9–69.2 ms).
- `stdin-hir-lib` 1c: head vs base: below (44.4–64.2 vs 65.4–99.2 ms); head vs rustfmt: below (44.4–64.2 vs 90.9–156.1 ms).
- `stdin-hover-tests` 1c: head vs base: overlap (11.7–20.4 vs 17.6–32.8 ms); head vs rustfmt: below (11.7–20.4 vs 28.7–49.0 ms).
- `stdin-lints` 1c: head vs base: overlap (69.4–116.5 vs 84.0–126.6 ms); head vs rustfmt: below (69.4–116.5 vs 139.0–204.5 ms).
- `stdin-nodes` 1c: head vs base: below (49.7–72.7 vs 78.8–116.1 ms); head vs rustfmt: below (49.7–72.7 vs 87.1–112.9 ms).
- `tiny-one` 1c: head vs base: overlap (1.7–2.5 vs 1.7–5.4 ms); head vs rustfmt: below (1.7–2.5 vs 8.9–17.4 ms); head-pgo vs base: overlap (1.7–2.6 vs 1.7–2.6 ms); head-pgo vs rustfmt: below (1.7–2.6 vs 9.0–14.8 ms).
- `tiny-one` mc: head vs base: overlap (2.1–3.6 vs 1.9–4.5 ms); head vs rustfmt: below (2.1–3.6 vs 9.4–15.6 ms); head-pgo vs base: overlap (1.9–3.1 vs 1.9–2.7 ms); head-pgo vs rustfmt: below (1.9–3.1 vs 9.1–17.5 ms).
- `tiny-test-data` 1c: head vs base: overlap (14.7–22.9 vs 22.2–38.0 ms); head vs rustfmt: below (14.7–22.9 vs 48.5–72.3 ms); head-pgo vs base: below (14.1–16.0 vs 21.8–36.3 ms); head-pgo vs rustfmt: below (14.1–16.0 vs 48.5–71.1 ms).
- `tiny-test-data` mc: head vs base: overlap (8.6–14.1 vs 12.6–22.2 ms); head vs rustfmt: below (8.6–14.1 vs 49.4–92.4 ms); head-pgo vs base: below (7.8–11.2 vs 11.5–19.8 ms); head-pgo vs rustfmt: below (7.8–11.2 vs 49.4–86.2 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS. No chloro build reads
above rustfmt in mean, minimum, CPU or peak RSS on any workload, formatted or raw; no cell of
the nine crates, the five stdin rows, the cache sweep or the in-process measurements reads
head or head-pgo above base; the list is `tiny-one`, `tiny-test-data` and the peak RSS of
the split files.

- `tiny-test-data` 1c, formatted, head-pgo vs base, rss: 4.9 vs 4.8 (1.014); wall ranges below.
- `tiny-one` mc, formatted, head vs base, min: 2.1 vs 1.9 (1.094); wall ranges overlap.
- `tiny-one` mc, formatted, head vs base, cpu: 2.3 vs 2.2 (1.032); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, mean: 2.2 vs 2.1 (1.001); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, min: 1.9 vs 1.9 (1.011); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, rss: 3.8 vs 3.7 (1.031); wall ranges overlap.
- `tiny-one` 1c, formatted, head vs base, mean: 2.0 vs 2.0 (1.033); wall ranges overlap.
- `tiny-one` 1c, formatted, head vs base, cpu: 1.8 vs 1.7 (1.090); wall ranges overlap.
- `tiny-one` 1c, formatted, head-pgo vs base, min: 1.7 vs 1.7 (1.004); wall ranges overlap.
- `tiny-one` 1c, formatted, head-pgo vs base, rss: 3.9 vs 3.7 (1.053); wall ranges overlap.
- `stdin-lints` mc, parallel items, head vs base, rss: 15.1 vs 10.5 (1.436); wall ranges below.
- `stdin-nodes` mc, parallel items, head vs base, rss: 13.9 vs 13.9 (1.001); wall ranges below.
- `stdin-nodes` 1c, parallel items, head vs base, rss: 14.0 vs 14.0 (1.002); wall ranges below.
- `stdin-hir-lib` mc, parallel items, head vs base, rss: 8.9 vs 8.8 (1.004); wall ranges below.
- `stdin-hir-lib` 1c, parallel items, head vs base, rss: 8.9 vs 8.9 (1.005); wall ranges overlap.
- `stdin-config` mc, parallel items, head vs base, rss: 6.2 vs 5.9 (1.051); wall ranges overlap.

- Wall-time means above base: `tiny-one` pinned (head 1.033, 2.0 vs 2.0 ms at one decimal)
  and unpinned (head-pgo 1.001); a 13-byte file, process start-up; ranges overlap. No other
  row, pinned or unpinned, reads a chloro build above base in mean wall.
- CPU above base: `tiny-one` only (head 1.032 unpinned, 1.090 pinned; 2.3 vs 2.2 and 1.8 vs
  1.7 ms). The split files, which M1 recorded at 1.00–1.23x base's CPU, read 0.68–0.92x on
  this head.
- Peak RSS above base: `stdin-lints` unpinned (15.1 vs 10.5 MiB, the E1 split), five split
  rows within 0.3 MiB of base, and head-pgo on `tiny-one` and `tiny-test-data` (+0.1–0.2
  MiB). No crate row, no cache row and no in-process row reads above base in peak RSS.

## Against M1

M1 ([m1-quiet-host.md](m1-quiet-host.md)) measured `dc20cd7` (E6, E1, E2) on the 2.10 GHz
host; this record measures `8d418ee` (M1's head plus E4, E8 and E7b) on the 2.80 GHz host.
The hosts differ, so the absolute figures of the two records are not compared (protocol §3);
`base` is the same binary (same sha256) in both, and each record's ratios are within one
window on one host, so the ratios are set side by side:

| ratio (means) | M1, `dc20cd7` on the 2.10 GHz host | M2, `8d418ee` on the 2.80 GHz host |
|---|---|---|
| crates, `mc`, head/base | 0.52–0.88 | 0.48–0.62 |
| crates, `1c`, head/base (wall; CPU) | 0.76–0.98; 0.76–0.99 | 0.53–0.67; 0.54–0.67 |
| crates, `mc`, pgo/base | 0.48–0.77 | 0.36–0.54 |
| crates, `1c`, pgo/base | 0.67–0.96 | 0.48–0.58 |
| crates, `mc`, head/rustfmt | 0.149–0.283 | 0.130–0.201 |
| crates, `1c`, head/rustfmt | 0.57–0.89 | 0.44–0.61 |
| crates, `1c`, pgo/rustfmt | 0.50–0.83 | 0.37–0.51 |
| crates, `1c`, head below base in range | 0 of 9 | 7 of 9 |
| crates, `1c`, head below rustfmt in range | 2 of 9 | 9 of 9 |
| stdin, head/base | 0.88–1.00 | 0.60–0.86 |
| split files, `mc`, head/base (wall; CPU) | 0.57–0.69; 1.00–1.23 | 0.39–0.56; 0.68–0.92 |
| `bench -n 5`, head/base on the triples' MB/s | 6.53–6.94 vs 5.44–5.99 (1.09x–1.28x) | 7.25–7.73 vs 4.94–5.03 (1.44x–1.56x) |
| `bench -n 5`, head-pgo MB/s | 6.91–8.14 | 8.91–9.27 |
| `Ir`, head/base; pgo/base | 0.9425; 0.7623 | 0.6645; 0.5219 |
| cache, warm vs no-cache vs base, `mc` | 24.5 / 650.8 / 773.4 ms (26.6x, 31.6x) | 30.4 / 523.4 / 917.2 ms (17.2x, 30.2x) |
| `conform -i`, head/base | 0.93 | 0.65 |

- The M1 head/base ratios on the pinned crates (0.76–0.98) and this record's (0.53–0.67) differ
  by the E4 and E7b work between the two heads; the two records' `Ir` ratios (0.9425 and
  0.6645) move in the same direction by a similar amount (−5.75% to −33.55% against base).
- In M1 the pinned crate ranges of head and base overlapped on all nine crates; here seven of
  nine are disjoint. In M1 the pinned head ranges were below rustfmt's on two crates; here on
  all nine.

## Harness notes

- `compare_rustfmt.sh` passes `--no-cache` to the head builds on its own (`chloro_cache_flag=
  --no-cache` in `env.txt`) and no flag to `base`, which rejects it; the `env -u HOME` workaround
  of M1 is not needed and was not used. The `--rustfmt` and `--registry` defaults resolved
  without being passed.
- `benchmarks/pgo.sh "$PWD/target" cli` ran to completion: instrumented build, training (13
  `.profraw` files merged), optimised build of `chloro` and, since M1, of `examples/bench`
  under `target/pgo/release/`. The relative-root failure of M1 is gone (the script makes the
  root absolute).
- The chloro CLI `--check` exits 1 for `base` and `head` alike on the formatted stdin copies
  of `stdin-hir-lib` and `stdin-config` (`/tmp/chloro-bench/formatted/<w>/tree/input.rs`),
  although rustfmt on stdin and `examples/fmt_stdin` both return those copies unchanged
  (`identical_outputs=1/1`, `diff` empty). The CLI's print mode rewrites 1084 lines of
  `stdin-hir-lib` and 299 lines of `stdin-config`; the two builds' CLI outputs are identical
  to each other, the CLI and `fmt_stdin` both start from `Config::default()`, and the result
  does not depend on the working directory. The measurement E rows for those two files time
  a full format in every build; the exit code is in the table.
- `--baseline` requires all four baseline binaries and `--chloro` requires `fmt_stdin`, `bench`
  and `conform` to exist even for groups that do not run them; the PGO sweep used head's three
  examples (the harness defaults) to satisfy the check.
- The three registry crates other than `clap_builder` are not in a fresh container's cargo
  registry; the script exits 2 at `prepare` until they are fetched.
- The harness's `conform -i` validation log records only the exit status, not the agreement
  counts; the counts in measurement D come from a by-hand run after the chain.
- hyperfine is not in a fresh container; `cargo install hyperfine --locked` built 2.0.0.

## Current State

- `benchmarks/compare_rustfmt.sh`, `report.py`, `profile.sh` and `pgo.sh` ran to completion on
  every sweep (exit 0) on a host with no other agent; seven harness results directories and
  three by-hand directories hold the JSON exports, `commands.txt`, `env.txt` and `report.md`.
- On formatted input with all 4 CPUs, `head` takes 0.130–0.201x rustfmt's wall time on the
  nine crates and 0.48–0.62x base's; on one core 0.44–0.61x rustfmt's and 0.53–0.67x base's
  (CPU 0.54–0.67x). Every head run is below every rustfmt run on every crate, pinned and
  unpinned.
- `head-pgo` takes 0.104–0.188x rustfmt's wall time unpinned and 0.37–0.51x pinned; 0.36–0.54x
  base's unpinned and 0.48–0.58x pinned; `Ir` 0.5219x base's and 0.7855x head's.
- The warm cache answers `--check` on 1238 formatted files in 30.4 ms against 523.4 ms without
  it (17.2x) and 917.2 ms for base (30.2x); 50.7x and 82.3x on one core.
- In process, head formats the fixtures at 7.25–7.73 MB/s against base's 4.94–5.03 MB/s
  (disjoint over 3 interleaved triples), head-pgo at 8.91–9.27 MB/s.
- The parallel-items split gives 0.39–0.56x base's wall time on the five largest files with
  4 threads, at 0.68–0.92x the CPU and up to 1.44x the peak RSS.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure.
- The `cold` cache variant has no pinned row.
- No raw-input row for `head-pgo` (measurement A's raw sweep is head only, as in M1).
- The cause of the CLI's `--check` exit 1 on the two formatted stdin copies is not traced here;
  this record only states it.

## Appendix: `report.py` tables as written

Each block is the `report.md` of one results directory, unedited (`env.txt` header, then the
tables; `chloro` is the `--chloro` build of that sweep, `chloro-base` is `base`).

### `a1-head-base-formatted`

```text
date_utc=2026-10-14T20:22:05Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.14 2.01 1.16
cpu3_stat_start=16325 0 1934 24857 455 0 77 348 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.75 1.88 1.40
cpu3_stat_end=47425 0 4194 32999 458 0 84 858 0 0
cpu3_steal_ticks=510 cpu3_busy_ticks=33360 cpu3_idle_ticks=8142
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 982.3 ± 64.8 (870.6–1118.1) | 435.3 ± 29.8 (394.1–513.9) | 665.5 ± 56.7 (594.5–806.4) | 0.654 | below | 0.443 | 0.453 | below | 85 | 8 | 965.3 | 428.0 | 0.443 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 968.4 ± 62.3 (873.5–1108.9) | 126.4 ± 12.7 (109.1–151.7) | 232.6 ± 25.4 (195.1–294.6) | 0.543 | below | 0.130 | 0.125 | below | 85 | 21 | 955.6 | 468.5 | 0.490 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 315.2 ± 33.8 (282.2–432.1) | 138.1 ± 12.1 (125.2–170.3) | 227.5 ± 29.1 (191.5–293.5) | 0.607 | below | 0.438 | 0.444 | below | 57 | 9 | 310.7 | 136.2 | 0.438 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 322.0 ± 31.7 (279.3–394.9) | 49.8 ± 7.2 (40.1–68.2) | 104.9 ± 12.2 (90.5–136.1) | 0.475 | below | 0.155 | 0.144 | below | 57 | 18 | 316.3 | 162.2 | 0.513 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 348.0 ± 33.9 (299.5–429.8) | 166.9 ± 16.4 (143.5–214.9) | 249.8 ± 24.6 (214.8–309.8) | 0.668 | overlap | 0.480 | 0.479 | below | 58 | 10 | 343.0 | 163.7 | 0.477 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 352.3 ± 32.3 (306.3–425.8) | 70.8 ± 8.2 (56.7–87.5) | 140.2 ± 17.6 (102.6–186.7) | 0.505 | below | 0.201 | 0.185 | below | 58 | 19 | 343.0 | 185.9 | 0.542 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 79.8 ± 8.4 (72.8–104.6) | 44.5 ± 5.8 (38.8–61.8) | 75.7 ± 10.5 (64.7–102.2) | 0.588 | below | 0.558 | 0.533 | below | 47 | 5 | 79.0 | 43.7 | 0.553 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 85.8 ± 10.9 (74.3–115.9) | 17.2 ± 2.4 (14.7–23.7) | 29.8 ± 4.0 (23.2–39.9) | 0.578 | overlap | 0.201 | 0.198 | below | 47 | 6 | 82.9 | 53.4 | 0.643 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 150.6 ± 22.7 (128.0–212.9) | 77.8 ± 14.2 (66.0–121.5) | 146.3 ± 27.1 (116.9–208.2) | 0.532 | overlap | 0.516 | 0.516 | below | 50 | 6 | 147.4 | 76.6 | 0.520 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 148.5 ± 16.4 (128.3–220.8) | 24.5 ± 2.1 (21.6–29.5) | 51.4 ± 6.4 (42.8–71.5) | 0.476 | below | 0.165 | 0.168 | below | 50 | 8 | 145.8 | 82.6 | 0.566 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 217.8 ± 24.2 (192.5–286.6) | 112.7 ± 12.5 (99.1–144.6) | 190.6 ± 25.3 (166.0–268.3) | 0.591 | below | 0.517 | 0.515 | below | 55 | 6 | 214.2 | 110.5 | 0.516 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 227.5 ± 27.9 (195.6–314.5) | 38.2 ± 2.9 (32.4–43.1) | 70.2 ± 8.1 (57.3–85.4) | 0.544 | below | 0.168 | 0.165 | below | 55 | 12 | 224.3 | 128.2 | 0.572 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 506.3 ± 37.9 (459.3–626.9) | 308.6 ± 34.6 (267.7–400.3) | 469.9 ± 39.9 (414.4–573.1) | 0.657 | below | 0.609 | 0.583 | below | 66 | 14 | 500.0 | 303.8 | 0.608 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 520.5 ± 44.7 (451.2–633.7) | 99.2 ± 9.4 (83.2–118.9) | 160.1 ± 25.9 (125.4–235.0) | 0.620 | below | 0.191 | 0.184 | below | 66 | 31 | 511.1 | 340.1 | 0.665 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 44.8 ± 7.2 (39.3–65.5) | 21.2 ± 0.8 (19.9–24.4) | 37.7 ± 6.0 (33.5–54.3) | 0.562 | below | 0.473 | 0.507 | below | 43 | 4 | 44.3 | 20.7 | 0.468 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 48.3 ± 6.2 (41.2–64.1) | 9.6 ± 1.5 (8.1–15.3) | 19.7 ± 2.4 (15.4–25.6) | 0.486 | below | 0.198 | 0.196 | below | 43 | 5 | 46.8 | 26.3 | 0.563 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 199.1 ± 13.2 (181.1–233.3) | 121.7 ± 13.1 (107.0–153.9) | 213.0 ± 19.9 (188.5–268.1) | 0.571 | below | 0.611 | 0.591 | below | 59 | 7 | 195.6 | 118.9 | 0.608 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 207.3 ± 21.3 (180.3–272.6) | 37.2 ± 2.4 (33.6–42.0) | 76.8 ± 6.0 (66.4–90.4) | 0.484 | below | 0.179 | 0.186 | below | 59 | 11 | 203.7 | 130.9 | 0.642 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 10.8 ± 2.4 (8.9–17.4) | 2.0 ± 0.3 (1.7–2.5) | 2.0 ± 0.7 (1.7–5.4) | 1.033 | overlap | 0.188 | 0.187 | below | 34 | 4 | 10.2 | 1.8 | 0.180 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 12.2 ± 1.8 (9.4–15.6) | 2.5 ± 0.3 (2.1–3.6) | 2.5 ± 0.5 (1.9–4.5) | 0.996 | overlap | 0.206 | 0.224 | below | 34 | 4 | 11.7 | 2.3 | 0.196 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 54.5 ± 5.5 (48.5–72.3) | 16.1 ± 1.7 (14.7–22.9) | 26.0 ± 4.8 (22.2–38.0) | 0.619 | overlap | 0.295 | 0.304 | below | 44 | 4 | 53.7 | 15.8 | 0.294 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 60.2 ± 11.2 (49.4–92.4) | 10.2 ± 1.3 (8.6–14.1) | 15.3 ± 2.2 (12.6–22.2) | 0.663 | overlap | 0.169 | 0.174 | below | 44 | 4 | 59.0 | 20.4 | 0.346 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 48.3 ± 8.6 (41.9–69.2) | 20.9 ± 3.3 (18.4–33.4) | 28.9 ± 3.3 (25.9–41.1) | 0.722 | overlap | 0.433 | 0.438 | below | 44 | 5 | 47.5 | 20.2 | 0.425 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 102.7 ± 14.7 (90.9–156.1) | 49.0 ± 4.0 (44.4–64.2) | 75.8 ± 10.3 (65.4–99.2) | 0.646 | below | 0.477 | 0.489 | below | 47 | 8 | 101.2 | 47.3 | 0.467 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 33.8 ± 6.4 (28.7–49.0) | 13.9 ± 3.1 (11.7–20.4) | 19.1 ± 2.7 (17.6–32.8) | 0.728 | overlap | 0.411 | 0.410 | below | 40 | 4 | 33.4 | 13.5 | 0.405 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 158.4 ± 16.1 (139.0–204.5) | 79.6 ± 12.5 (69.4–116.5) | 92.3 ± 10.2 (84.0–126.6) | 0.862 | overlap | 0.502 | 0.499 | below | 43 | 10 | 155.3 | 78.4 | 0.505 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 94.4 ± 7.3 (87.1–112.9) | 54.5 ± 7.0 (49.7–72.7) | 91.3 ± 10.7 (78.8–116.1) | 0.597 | below | 0.577 | 0.571 | below | 53 | 14 | 92.9 | 53.9 | 0.581 | 1/1 | r[0] c[0] |

### `a2-pgo-base-formatted`

```text
date_utc=2026-10-14T20:29:07Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.75 1.88 1.40
cpu3_stat_start=47427 0 4195 33015 458 0 84 858 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=target/pgo/release/chloro sha256=711a973433c3bb01
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=2.11 1.90 1.54
cpu3_stat_end=73153 0 5956 41758 459 0 94 1254 0 0
cpu3_steal_ticks=396 cpu3_busy_ticks=27487 cpu3_idle_ticks=8743
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 983.8 ± 61.2 (871.2–1141.8) | 375.6 ± 24.5 (347.2–447.2) | 693.4 ± 61.3 (611.9–824.8) | 0.542 | below | 0.382 | 0.399 | below | 85 | 9 | 969.4 | 370.4 | 0.382 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 976.7 ± 55.3 (886.7–1099.8) | 101.9 ± 6.1 (93.5–121.1) | 217.8 ± 19.8 (180.0–264.2) | 0.468 | below | 0.104 | 0.105 | below | 85 | 22 | 957.4 | 376.4 | 0.393 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 308.1 ± 30.3 (276.5–373.6) | 121.6 ± 8.5 (111.8–142.3) | 211.5 ± 20.8 (189.6–273.0) | 0.575 | below | 0.395 | 0.404 | below | 57 | 10 | 304.8 | 120.3 | 0.394 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 325.1 ± 28.5 (282.7–409.3) | 44.4 ± 4.7 (38.5–55.3) | 112.8 ± 15.0 (90.4–142.3) | 0.394 | below | 0.137 | 0.136 | below | 56 | 17 | 320.7 | 148.2 | 0.462 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 327.8 ± 23.3 (303.7–386.5) | 120.4 ± 12.2 (110.7–175.8) | 248.6 ± 26.6 (218.5–314.4) | 0.484 | below | 0.367 | 0.365 | below | 58 | 11 | 325.1 | 118.4 | 0.364 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 344.8 ± 28.0 (317.5–452.3) | 50.9 ± 5.7 (41.3–65.9) | 139.9 ± 15.0 (121.3–191.8) | 0.364 | below | 0.148 | 0.130 | below | 58 | 20 | 338.1 | 145.9 | 0.431 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 86.2 ± 12.2 (73.2–116.4) | 36.2 ± 2.1 (34.2–43.8) | 72.7 ± 12.8 (65.4–123.5) | 0.497 | below | 0.420 | 0.467 | below | 47 | 5 | 84.5 | 35.5 | 0.420 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 83.4 ± 11.2 (73.9–122.7) | 14.1 ± 0.8 (12.4–16.1) | 31.0 ± 3.1 (25.4–38.4) | 0.453 | below | 0.169 | 0.168 | below | 47 | 7 | 82.0 | 43.3 | 0.528 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 141.7 ± 14.2 (126.7–182.5) | 60.6 ± 2.1 (57.8–66.9) | 118.5 ± 10.8 (109.7–155.2) | 0.511 | below | 0.428 | 0.456 | below | 50 | 6 | 139.3 | 60.0 | 0.431 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 147.7 ± 15.6 (131.3–196.5) | 22.7 ± 2.6 (19.1–27.9) | 47.5 ± 5.4 (37.8–60.0) | 0.477 | below | 0.154 | 0.146 | below | 50 | 9 | 144.0 | 74.5 | 0.517 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 225.0 ± 28.7 (196.4–327.1) | 94.4 ± 4.7 (89.6–111.0) | 180.8 ± 16.6 (167.3–239.9) | 0.522 | below | 0.420 | 0.456 | below | 55 | 7 | 220.5 | 93.3 | 0.423 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 219.6 ± 20.0 (192.9–277.8) | 35.8 ± 3.5 (29.3–44.4) | 66.8 ± 7.4 (56.4–82.3) | 0.536 | below | 0.163 | 0.152 | below | 55 | 13 | 215.5 | 117.8 | 0.547 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 487.6 ± 30.2 (453.4–569.4) | 220.4 ± 15.2 (207.3–271.5) | 454.7 ± 28.9 (418.0–560.4) | 0.485 | below | 0.452 | 0.457 | below | 66 | 15 | 482.4 | 217.6 | 0.451 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 482.4 ± 20.5 (448.9–538.3) | 75.7 ± 8.0 (66.4–104.3) | 142.5 ± 14.0 (121.9–197.6) | 0.531 | below | 0.157 | 0.148 | below | 66 | 33 | 475.9 | 256.6 | 0.539 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 42.1 ± 2.2 (39.6–50.9) | 21.0 ± 3.9 (17.7–29.3) | 36.5 ± 4.2 (33.9–53.8) | 0.575 | below | 0.499 | 0.447 | below | 43 | 5 | 41.6 | 20.6 | 0.494 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 47.3 ± 6.3 (41.8–65.5) | 8.9 ± 0.9 (7.7–10.9) | 19.9 ± 3.0 (14.8–29.0) | 0.446 | below | 0.188 | 0.185 | below | 43 | 6 | 46.2 | 23.8 | 0.515 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 197.5 ± 13.5 (179.8–227.9) | 101.3 ± 9.8 (92.7–125.1) | 212.9 ± 19.0 (186.5–260.7) | 0.476 | below | 0.513 | 0.516 | below | 59 | 8 | 193.1 | 99.4 | 0.515 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 206.0 ± 16.3 (185.1–250.3) | 32.0 ± 2.6 (28.9–38.3) | 72.9 ± 8.4 (60.2–91.2) | 0.439 | below | 0.155 | 0.156 | below | 59 | 12 | 201.8 | 111.4 | 0.552 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 10.4 ± 1.5 (9.0–14.8) | 1.9 ± 0.3 (1.7–2.6) | 2.4 ± 0.2 (1.7–2.6) | 0.814 | overlap | 0.186 | 0.190 | below | 34 | 4 | 10.1 | 1.8 | 0.176 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 10.8 ± 1.6 (9.1–17.5) | 2.2 ± 0.3 (1.9–3.1) | 2.1 ± 0.2 (1.9–2.7) | 1.001 | overlap | 0.199 | 0.213 | below | 34 | 4 | 10.2 | 2.0 | 0.191 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 52.3 ± 4.7 (48.5–71.1) | 14.6 ± 0.5 (14.1–16.0) | 24.2 ± 2.7 (21.8–36.3) | 0.605 | below | 0.280 | 0.290 | below | 44 | 5 | 51.6 | 14.3 | 0.278 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 55.6 ± 9.1 (49.4–86.2) | 9.0 ± 0.9 (7.8–11.2) | 14.3 ± 2.0 (11.5–19.8) | 0.625 | below | 0.161 | 0.158 | below | 44 | 5 | 54.8 | 18.0 | 0.328 | 287/287 | r[0] c[0] |

### `a3-head-raw`

```text
date_utc=2026-10-14T20:35:14Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=2.11 1.90 1.54
cpu3_stat_start=73154 0 5957 41773 459 0 94 1254 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.26 1.42 1.43
cpu3_stat_end=98611 0 10011 52977 461 0 96 1677 0 0
cpu3_steal_ticks=423 cpu3_busy_ticks=29511 cpu3_idle_ticks=11204
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1310.5 ± 49.9 (1217.7–1420.4) | 443.4 ± 33.5 (393.7–550.8) | 0.338 | 0.323 | below | 116 | 8 | 1290.6 | 432.7 | 0.335 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1296.8 ± 50.8 (1222.1–1424.0) | 115.1 ± 6.2 (107.3–130.5) | 0.089 | 0.088 | below | 116 | 22 | 1278.9 | 429.6 | 0.336 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 582.8 ± 36.3 (539.6–700.6) | 149.1 ± 34.2 (127.9–272.8) | 0.256 | 0.237 | below | 218 | 9 | 577.3 | 145.3 | 0.252 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 593.6 ± 20.7 (549.4–644.5) | 46.9 ± 4.1 (39.7–55.8) | 0.079 | 0.072 | below | 218 | 16 | 585.4 | 156.8 | 0.268 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 382.8 ± 25.2 (350.8–447.1) | 171.9 ± 30.3 (148.9–298.1) | 0.449 | 0.424 | below | 66 | 10 | 376.2 | 169.7 | 0.451 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 396.4 ± 32.5 (344.6–472.1) | 73.7 ± 8.4 (55.0–90.7) | 0.186 | 0.159 | below | 66 | 19 | 390.1 | 196.1 | 0.503 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 94.2 ± 8.1 (83.0–112.5) | 44.3 ± 5.0 (39.7–61.1) | 0.470 | 0.478 | below | 48 | 5 | 91.6 | 43.3 | 0.473 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 93.5 ± 10.8 (82.0–139.4) | 15.2 ± 1.4 (13.8–18.9) | 0.162 | 0.168 | below | 48 | 6 | 91.1 | 48.0 | 0.527 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 175.2 ± 14.3 (160.5–215.7) | 68.0 ± 2.2 (65.6–75.4) | 0.388 | 0.409 | below | 57 | 6 | 173.6 | 67.4 | 0.388 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 187.1 ± 21.9 (166.0–252.9) | 25.2 ± 2.3 (22.2–31.4) | 0.135 | 0.134 | below | 57 | 8 | 184.5 | 85.7 | 0.464 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 220.3 ± 31.5 (194.3–325.3) | 104.5 ± 6.0 (99.5–127.5) | 0.474 | 0.512 | below | 55 | 6 | 217.3 | 103.5 | 0.476 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 223.4 ± 25.5 (194.3–305.6) | 40.3 ± 7.7 (31.5–69.9) | 0.180 | 0.162 | below | 55 | 12 | 218.9 | 127.2 | 0.581 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1503.0 ± 70.5 (1391.7–1680.4) | 278.7 ± 17.1 (261.3–334.2) | 0.185 | 0.188 | below | 423 | 14 | 1485.3 | 275.9 | 0.186 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1491.6 ± 48.4 (1421.9–1616.6) | 95.3 ± 10.0 (82.1–124.4) | 0.064 | 0.058 | below | 423 | 32 | 1470.8 | 332.3 | 0.226 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 44.8 ± 6.0 (40.5–65.1) | 20.5 ± 0.6 (19.9–22.5) | 0.457 | 0.491 | below | 43 | 4 | 44.1 | 20.2 | 0.458 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 45.2 ± 2.8 (42.4–54.2) | 10.1 ± 1.1 (8.2–13.7) | 0.224 | 0.193 | below | 43 | 5 | 43.9 | 27.9 | 0.635 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 203.6 ± 22.2 (183.1–258.8) | 116.1 ± 7.7 (107.3–133.1) | 0.570 | 0.586 | below | 60 | 7 | 201.3 | 113.6 | 0.565 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 212.3 ± 17.1 (188.2–254.7) | 36.3 ± 2.9 (32.6–45.2) | 0.171 | 0.173 | below | 60 | 11 | 207.8 | 127.3 | 0.613 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 9.1 ± 1.8 (7.7–17.5) | 1.7 ± 0.1 (1.6–1.9) | 0.191 | 0.211 | below | 34 | 4 | 8.3 | 1.6 | 0.194 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 11.1 ± 2.1 (9.6–19.8) | 2.4 ± 0.2 (2.0–2.8) | 0.212 | 0.212 | below | 34 | 4 | 10.7 | 2.2 | 0.203 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 68.8 ± 10.7 (60.5–97.8) | 17.6 ± 2.0 (14.8–21.3) | 0.255 | 0.244 | below | 44 | 4 | 68.0 | 17.2 | 0.253 | 288/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 67.0 ± 13.9 (60.3–131.3) | 9.1 ± 0.8 (8.1–12.0) | 0.135 | 0.135 | below | 44 | 4 | 66.0 | 18.9 | 0.286 | 288/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 45.0 ± 1.6 (42.9–48.8) | 19.4 ± 0.5 (18.6–20.9) | 0.432 | 0.433 | below | 44 | 5 | 44.5 | 19.2 | 0.430 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 96.0 ± 11.7 (88.0–152.8) | 46.8 ± 2.5 (44.4–54.7) | 0.487 | 0.505 | below | 47 | 8 | 94.1 | 45.8 | 0.486 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 32.8 ± 5.1 (28.8–46.4) | 15.7 ± 4.8 (11.7–25.8) | 0.477 | 0.406 | below | 40 | 4 | 32.0 | 15.0 | 0.470 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 147.5 ± 12.3 (136.7–184.9) | 72.6 ± 6.9 (68.9–105.5) | 0.492 | 0.504 | below | 43 | 10 | 146.2 | 72.0 | 0.492 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 97.7 ± 11.8 (88.2–152.7) | 53.1 ± 5.2 (48.6–72.1) | 0.544 | 0.551 | below | 53 | 13 | 96.1 | 51.2 | 0.533 | 1/1 | r[0] c[0] |

### `d-inproc-harness`

```text
date_utc=2026-10-14T20:42:06Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.24 1.41 1.43
cpu3_stat_start=98612 0 10011 52994 461 0 96 1677 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.02 1.23 1.35
cpu3_stat_end=117884 0 10163 53006 461 0 96 1904 0 0
cpu3_steal_ticks=227 cpu3_busy_ticks=19424 cpu3_idle_ticks=12
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 3785.3 ± 135.8 (3580.5–3916.1) | 46 | 3741.6 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 5838.6 ± 160.4 (5493.8–6012.4) | 51 | 5768.9 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.781 s | 7.81 |
| 2 | chloro-base | 1238 | 13.9 | 2.735 s | 5.08 |
| 3 | chloro-base | 1238 | 13.9 | 2.651 s | 5.24 |
| 4 | chloro | 1238 | 13.9 | 1.833 s | 7.59 |
| 5 | chloro | 1238 | 13.9 | 1.807 s | 7.69 |
| 6 | chloro-base | 1238 | 13.9 | 2.848 s | 4.88 |

- chloro: 3 runs, 7.59–7.81 MB/s
- chloro-base: 3 runs, 4.88–5.24 MB/s

### `noise-hir-ty-1`

```text
date_utc=2026-10-14T20:50:48Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.15 1.60 1.55
cpu3_stat_start=146659 0 11091 55377 478 0 108 2340 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.54 1.62 1.56
cpu3_stat_end=154580 0 11372 58529 478 0 110 2489 0 0
cpu3_steal_ticks=149 cpu3_busy_ticks=8202 cpu3_idle_ticks=3152
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 929.0 ± 40.8 (874.0–1026.8) | 413.9 ± 26.6 (382.9–498.2) | 662.5 ± 34.4 (602.9–754.7) | 0.625 | below | 0.445 | 0.438 | below | 85 | 8 | 912.8 | 405.5 | 0.444 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 942.1 ± 38.8 (870.8–1037.4) | 117.5 ± 11.1 (105.0–151.6) | 217.0 ± 25.2 (186.8–296.6) | 0.542 | below | 0.125 | 0.121 | below | 85 | 20 | 925.5 | 434.9 | 0.470 | 122/122 | r[0] c[0] |

### `noise-hir-ty-2`

```text
date_utc=2026-10-14T20:52:44Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.54 1.62 1.56
cpu3_stat_start=154581 0 11374 58543 478 0 110 2489 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.83 1.70 1.59
cpu3_stat_end=162384 0 11680 61872 478 0 112 2620 0 0
cpu3_steal_ticks=131 cpu3_busy_ticks=8109 cpu3_idle_ticks=3329
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 944.6 ± 50.3 (865.8–1079.5) | 411.6 ± 22.3 (382.9–459.3) | 637.4 ± 34.0 (592.6–737.6) | 0.646 | below | 0.436 | 0.442 | below | 85 | 8 | 928.8 | 403.3 | 0.434 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 979.5 ± 60.0 (906.3–1160.0) | 114.1 ± 5.8 (106.3–135.2) | 210.6 ± 15.6 (182.7–239.8) | 0.542 | below | 0.117 | 0.117 | below | 85 | 20 | 961.2 | 426.8 | 0.444 | 122/122 | r[0] c[0] |

### `noise-hir-ty-3`

```text
date_utc=2026-10-14T20:54:40Z
commit=8d418eee2de08c56e1324bc424b1aa8e96929638
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.83 1.70 1.59
cpu3_stat_start=162384 0 11681 61886 478 0 112 2620 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=bda41f77264a3021
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=bc225b1e65ec9185
bench=/home/user/chloro/target/release/examples/bench sha256=cbcc51c4c2aa9bc9
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.95 1.87 1.67
cpu3_stat_end=170651 0 11975 64625 478 0 113 2749 0 0
cpu3_steal_ticks=129 cpu3_busy_ticks=8561 cpu3_idle_ticks=2739
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 931.2 ± 54.9 (852.2–1063.9) | 408.7 ± 26.4 (377.0–480.0) | 662.9 ± 44.6 (595.9–740.9) | 0.616 | below | 0.439 | 0.442 | below | 85 | 8 | 917.2 | 403.6 | 0.440 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 928.8 ± 38.1 (866.2–1003.2) | 113.5 ± 6.4 (104.4–126.8) | 213.7 ± 15.0 (188.6–239.0) | 0.531 | below | 0.122 | 0.121 | below | 85 | 20 | 913.9 | 422.1 | 0.462 | 122/122 | r[0] c[0] |
