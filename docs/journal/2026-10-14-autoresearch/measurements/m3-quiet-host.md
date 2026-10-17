# M3: base, head and head-pgo against rustfmt on a quiet host (E3 + R2 integration head)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 3 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the E3 and R2 merges and on rustfmt
1.9.0, with the method of [m2-quiet-host.md](m2-quiet-host.md) and
[m1-quiet-host.md](m1-quiet-host.md). Every figure in this file is **measured** in the
windows named below; nothing is estimated. The host is the 2.80 GHz host class of
[../baseline.md](../baseline.md) and of M2 (protocol §3: figures from different hosts are not
compared; the "against M2" section sets within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `069f5a7` (was `4867896`) (the integration branch after the E3 and R2 merges: E6 cache, E1 parallel items, E2 `[profile.release]` `lto = "fat"`, `codegen-units = 1`, E4 green reads, E8 `rustfmt_version`, E7b flat substrate, E3 hot paths, R2 conformance fixes; one docs-only commit above the ledger's best-known-good `af52d66` (was `113367a`), same code) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `069f5a7f6441fb142de01a1dcc4de4ca85accbb0` (was `4867896e1b51a68afc9cdf3bf018bc80d8a00269`) (worktree clean,
  `worktree_dirty=0` in every `env.txt`; `git diff --stat 113367a HEAD` touches three
  Markdown files and no source); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git
  worktree add /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `165ad74afb5c45eebe8d035c6a66029f021f0c508cf841e7523bf9b030e69d23` | 2,010,696 |
| `head` bench | `target/release/examples/bench` | `f8ceb61944565753f99a45341c99f52c5549c0e4879640a46b89c2466acc5119` | 1,805,440 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `0d5a63a695d788c7cef5226218486123da994e68015527c5395b74f5b6e18a95` | 1,651,664 |
| `head` conform | `target/release/examples/conform` | `4a92f9af3bf267aa96d6887f4979cfcccd33f48f6688e9612807694a2152adc6` | 1,781,016 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `320b82ff3889e351562e4cac2a85bb6ad5a7ad8929413b1dfe24f4b323991fc9` | 2,714,112 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `ff00cc3c64c54db5b2f137738883aeac9a110db7e1043fad292d0eb2656f9eea` | 2,508,128 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes and the rustfmt hash equal the ones recorded in
  [m2-quiet-host.md](m2-quiet-host.md) and [m1-quiet-host.md](m1-quiet-host.md): the
  default-profile build of `884f9c9` reproduces byte for byte on this toolchain in a fresh
  container.
- The `head` chloro binary is 2,010,696 bytes against 1,928,728 for M2's `fe34bd2` (was `8d418ee`) build
  and 2,516,048 for `base` (0.799x base; 1.042x M2's head); `head-pgo` is 2,714,112 bytes
  (1.079x base, 1.047x M2's 2,592,152).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 17 lines for `head`,
  38 for `base` (the M2 counts).
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, installed for `pgo.sh`);
  hyperfine 2.0.0 (`cargo install hyperfine --locked`); valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. This is the 2.80 GHz
  host class of M2 and of [../baseline.md](../baseline.md), not the 2.10 GHz host of M1.
- **Load:** no other agent ran on the host. The 1-minute load average read 0.12 before the
  first process of the session; the builds (`base`, `head`, `pgo.sh` with its training runs),
  `rustup component add llvm-tools-preview`, `cargo fetch` of the four registry crates,
  `cargo install hyperfine` and a 1-run preparation pass of the harness (formatted and raw)
  preceded the sweeps, so the load at the first sweep's start is their tail. Ten one-shot
  `chloro --check` processes on the five stdin copies (the fixed-point check in Harness
  notes) ran during the first seconds of sweep 1. The load average at the start of every
  later sweep is the harness's own processes from the previous sweep (the `mc` rows run
  chloro with 4 threads). `cpu3` columns are `/proc/stat` ticks of core 3 (USER_HZ 100) over
  the sweep: steal / busy / idle.

| results directory | start (UTC) | load average at start | at end | cpu3 steal / busy / idle ticks |
|---|---|---|---|---|
| `a1-head-base-formatted` | 2026-10-14T23:37:06Z | 1.51 1.49 0.86 | 1.92 1.87 1.26 | 224 / 29132 / 7836 |
| `a2-pgo-base-formatted` | 2026-10-14T23:43:19Z | 1.92 1.87 1.26 | 1.88 1.83 1.43 | 263 / 25200 / 8175 |
| `a3-head-raw` | 2026-10-14T23:48:56Z | 1.88 1.83 1.43 | 1.02 1.29 1.31 | 241 / 25969 / 11948 |
| `d-inproc-harness` | 2026-10-14T23:55:19Z | 1.02 1.29 1.31 | 1.00 1.16 1.25 | 149 / 17440 / 8 |
| `c-cache` | 2026-10-14T23:58:15Z | 1.00 1.16 1.25 | 3.22 2.31 1.69 | 103 / 12223 / 548 |
| `d-inproc` | 2026-10-15T00:00:46Z | 2.31 2.16 1.65 | 1.17 1.77 1.57 | 81 / 11644 / 19 |
| `e-parallel-items` | 2026-10-15T00:02:44Z | 1.17 1.77 1.57 | 1.26 1.72 1.56 | 26 / 2744 / 1323 |
| `noise-hir-ty-1` | 2026-10-15T00:03:25Z | 1.26 1.72 1.56 | 1.83 1.83 1.62 | 65 / 8671 / 1931 |
| `noise-hir-ty-2` | 2026-10-15T00:05:12Z | 1.83 1.83 1.62 | 2.00 1.95 1.69 | 64 / 7922 / 2672 |
| `noise-hir-ty-3` | 2026-10-15T00:06:59Z | 2.00 1.95 1.69 | 1.68 1.85 1.68 | 87 / 8928 / 1866 |

- Steal on core 3 is 0.60–0.85% of the sweep's ticks (sweep 1: 224 of 37,192; sweep 2: 263 of
  33,638; sweep 3: 241 of 38,158; the cache sweep: 103 of 12,874; the noise runs: 64–87 of
  10,658–10,881), below M2's 1.0–1.4%. The pinned rows carry this steal.
- The sweeps ran in two chains: `a1`, `a2`, `a3`, `d-inproc-harness` and `c-cache` from
  23:37:06 to about 00:00 UTC in one chain, then `d-inproc`, `e-parallel-items` and the three
  noise runs from 00:00:46 to 00:08:48 UTC in a second chain (the first chain's own
  environment record aborted after the cache rounds; see Harness notes). The gap between the
  chains is 2.5 minutes with no timed command.

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
  has no cache and gets no flag. Verified before the sweeps: `head -v --check --no-cache`
  prints `Cache: disabled (--no-cache)`.
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found), made by
  a 1-run pass of the harness before the sweeps. Since M2 the harness formats the stdin
  copies through rustfmt's stdin, so the five stdin copies are fixed points for rustfmt,
  `fmt_stdin` and the CLI alike (`chloro --check <copy>` exits 0 for `base` and `head` on all
  five); `stdin-hir-lib` is 236,066 bytes and `stdin-config` 169,196 bytes in this record
  against 227,843 and 167,117 in M2, where the two were left as found. The registry crates
  `tokio-1.53.3`, `regex-syntax-0.8.11`, `winnow-0.7.15` and `clap_builder-4.6.7` were absent
  from this container's `~/.cargo/registry` and were fetched with a scratch `Cargo.toml`
  pinning the four versions (`cargo fetch`).
- The results directories (gitignored, under `benchmarks/results/`):

| directory | command |
|---|---|
| `a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `d-inproc` | by-hand `taskset -c 3 bench -n 5` triples, `benchmarks/profile.sh --runs 2 --core 3` per binary, `taskset -c 3 conform -i` per build |
| `e-parallel-items` | by-hand hyperfine, measurement E |
| `noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`); the `base` column shows sweep 1's base. The `ranges` column lists,
  in order, head vs base, head-pgo vs base, head vs rustfmt, head-pgo vs rustfmt: `below` =
  every run of the first command took less wall time than every run of the second; `overlap`
  = the min–max ranges intersect.
- The same `base` binary ran in sweeps 1 and 2: its crate means in sweep 2 lie within
  0.949x–1.085x of sweep 1 (`reg-clap-builder` mc 62.3 vs 65.6 ms; `fx-syntax` 1c 122.4 vs
  112.8 ms).
- Output agreement in every crate, files and stdin workload: `identical_outputs` equals the
  file count (15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57, 61/61, 287/287, 1/1
  ×5) on formatted input, and on raw input the same with `tiny-test-data` 289/289 (M2:
  288/289; the R2 fixes). Exit codes are 0 for every command on formatted input, 1 for both
  tools on raw input where files change (`reg-clap-builder` and the stdin rows exit 0).

## Crate level, formatted input, `--check` (measurement A)

### Crate level, all CPUs (`mc`), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 73.4 / 93.0 ± 9.0 / 25 | 32.7 / 35.3 ± 1.9 / 16 | 29.8 / 34.4 ± 2.5 / 18 | 273.5 / 294.9 ± 19.3 / 57 | 270.2 / 295.3 ± 18.2 / 56 | 0.380 | 0.357 | 0.120 | 0.116 | below; below; below; below | 203.2 | 121.0 | 112.9 | 291.7 | 0.596 | 0.543 | 0.415 | 0.387 | b[0]h[0]r[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 175.8 / 194.7 ± 13.4 / 35 | 88.5 / 95.3 ± 4.8 / 19 | 78.4 / 83.6 ± 4.4 / 20 | 843.1 / 906.0 ± 57.4 / 85 | 829.7 / 874.8 ± 32.1 / 85 | 0.490 | 0.428 | 0.105 | 0.096 | below; below; below; below | 627.7 | 356.0 | 312.1 | 898.7 | 0.567 | 0.491 | 0.396 | 0.360 | b[0]h[0]r[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 119.1 / 132.7 ± 9.2 / 20 | 33.6 / 39.6 ± 3.8 / 19 | 28.9 / 36.6 ± 4.2 / 20 | 301.8 / 327.9 ± 17.4 / 58 | 303.8 / 338.0 ± 26.1 / 58 | 0.299 | 0.255 | 0.121 | 0.108 | below; below; below; below | 242.8 | 126.0 | 110.6 | 323.5 | 0.519 | 0.437 | 0.389 | 0.334 | b[0]h[0]r[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 36.0 / 46.1 ± 6.2 / 13 | 18.7 / 20.3 ± 1.5 / 8 | 16.2 / 18.0 ± 1.6 / 9 | 128.3 / 143.6 ± 12.0 / 50 | 126.1 / 140.5 ± 14.1 / 50 | 0.441 | 0.402 | 0.141 | 0.128 | below; below; below; below | 123.3 | 68.8 | 59.6 | 141.7 | 0.558 | 0.503 | 0.486 | 0.429 | b[0]h[0]r[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 22.4 / 28.0 ± 2.7 / 10 | 12.9 / 14.9 ± 2.5 / 6 | 11.1 / 11.9 ± 0.7 / 7 | 73.0 / 80.0 ± 5.4 / 47 | 71.6 / 76.5 ± 4.3 / 47 | 0.531 | 0.441 | 0.186 | 0.156 | overlap; below; below; below | 78.1 | 45.0 | 37.0 | 79.0 | 0.577 | 0.502 | 0.570 | 0.492 | b[0]h[0]r[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 13.2 / 17.5 ± 2.9 / 7 | 7.6 / 8.8 ± 2.3 / 5 | 7.1 / 8.6 ± 1.8 / 6 | 39.0 / 43.4 ± 5.7 / 43 | 40.1 / 45.2 ± 4.4 / 43 | 0.505 | 0.457 | 0.203 | 0.190 | overlap; below; below; below | 39.2 | 23.0 | 21.5 | 43.0 | 0.586 | 0.527 | 0.534 | 0.498 | b[0]h[0]r[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 117.5 / 139.2 ± 16.5 / 38 | 73.4 / 81.0 ± 5.3 / 31 | 61.0 / 70.0 ± 7.9 / 32 | 438.5 / 476.4 ± 31.5 / 66 | 435.6 / 468.4 ± 28.1 / 66 | 0.582 | 0.504 | 0.170 | 0.149 | below; below; below; below | 427.6 | 281.0 | 231.4 | 472.6 | 0.657 | 0.535 | 0.595 | 0.500 | b[0]h[0]r[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 52.2 / 65.6 ± 8.4 / 18 | 28.2 / 31.4 ± 2.9 / 12 | 26.4 / 30.2 ± 3.1 / 13 | 187.1 / 207.1 ± 11.4 / 55 | 191.3 / 209.6 ± 13.6 / 55 | 0.479 | 0.485 | 0.152 | 0.144 | below; below; below; below | 187.1 | 105.0 | 98.0 | 205.0 | 0.561 | 0.541 | 0.512 | 0.475 | b[0]h[0]r[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 57.8 / 69.2 ± 6.3 / 18 | 29.4 / 33.1 ± 3.4 / 11 | 25.5 / 27.7 ± 2.5 / 12 | 174.7 / 186.3 ± 10.6 / 59 | 172.8 / 184.8 ± 10.2 / 59 | 0.478 | 0.413 | 0.178 | 0.150 | below; below; below; below | 211.3 | 114.5 | 94.4 | 184.9 | 0.542 | 0.465 | 0.619 | 0.517 | b[0]h[0]r[0] |

- `head/base` on all CPUs: 0.299 (`fx-ide-db`) to 0.582 (`reg-regex-syntax`); every head run is
  below every base run on seven crates; the ranges overlap on `fx-parser` (12.9–25.0 vs
  22.4–33.7 ms) and `reg-tokio` (7.6–20.1 vs 13.2–29.8 ms).
- `head/rustfmt` on all CPUs: 0.105 (`fx-hir-ty`) to 0.203 (`reg-tokio`); every head run and
  every head-pgo run is below every rustfmt run on all nine crates.
- `pgo/base` on all CPUs: 0.255 (`fx-ide-db`) to 0.504 (`reg-regex-syntax`), every head-pgo
  run below every base run on all nine crates; `pgo/rustfmt` 0.096 (`fx-hir-ty`) to 0.190
  (`reg-tokio`). `head-pgo` is below `head` in mean on all nine crates (the closest:
  `reg-tokio` 8.6 vs 8.8 ms, `fx-hir` 34.4 vs 35.3 ms).
- CPU `head/base` on all CPUs: 0.519 (`fx-ide-db`) to 0.657 (`reg-regex-syntax`); CPU
  `pgo/base` 0.437–0.543. The CPU ratio is above the wall ratio on every crate (the 4-thread
  rows pay the E1 split in CPU and get it back in wall).
- Peak RSS on all CPUs: head 5–31 MiB against base 7–38 MiB and rustfmt 43–85 MiB; head is
  below base on every crate; head-pgo 6–32 MiB, below sweep 2's base on eight crates and
  0.5 MiB above it on `fx-ide-db` (20.1 vs 19.6 MiB).

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 182.9 / 194.5 ± 12.0 / 16 | 107.1 / 113.8 ± 9.4 / 9 | 92.6 / 97.8 ± 5.7 / 10 | 269.2 / 282.5 ± 14.8 / 56 | 270.8 / 286.8 ± 15.9 / 57 | 0.585 | 0.512 | 0.403 | 0.341 | below; below; below; below | 192.9 | 112.7 | 96.5 | 280.9 | 0.584 | 0.508 | 0.401 | 0.342 | b[0]h[0]r[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 579.3 / 620.8 ± 22.5 / 18 | 317.0 / 343.0 ± 23.7 / 8 | 280.0 / 294.2 ± 11.5 / 9 | 838.0 / 905.5 ± 41.8 / 85 | 833.3 / 908.1 ± 52.2 / 85 | 0.552 | 0.477 | 0.379 | 0.324 | below; below; below; below | 615.5 | 340.1 | 292.2 | 895.0 | 0.553 | 0.478 | 0.380 | 0.325 | b[0]h[0]r[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 208.7 / 220.0 ± 10.1 / 15 | 97.2 / 102.0 ± 6.4 / 10 | 81.7 / 89.7 ± 9.3 / 11 | 296.9 / 314.3 ± 14.4 / 58 | 293.5 / 324.0 ± 26.8 / 58 | 0.464 | 0.402 | 0.324 | 0.277 | below; below; below; below | 218.4 | 101.4 | 88.5 | 311.6 | 0.464 | 0.400 | 0.325 | 0.276 | b[0]h[0]r[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 105.4 / 112.8 ± 8.7 / 9 | 59.0 / 67.8 ± 9.4 / 6 | 50.5 / 52.6 ± 3.5 / 6 | 125.6 / 136.4 ± 11.4 / 50 | 126.8 / 136.8 ± 9.9 / 50 | 0.601 | 0.430 | 0.497 | 0.385 | below; below; below; below | 112.1 | 66.6 | 52.1 | 135.2 | 0.594 | 0.432 | 0.492 | 0.384 | b[0]h[0]r[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 62.9 / 64.1 ± 0.8 / 7 | 34.7 / 37.2 ± 4.5 / 4 | 30.3 / 33.0 ± 3.2 / 5 | 71.6 / 75.3 ± 4.8 / 47 | 71.8 / 75.4 ± 4.1 / 47 | 0.580 | 0.477 | 0.493 | 0.437 | below; below; below; below | 63.7 | 36.9 | 32.3 | 74.6 | 0.579 | 0.474 | 0.494 | 0.434 | b[0]h[0]r[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 32.6 / 34.1 ± 1.6 / 6 | 18.4 / 19.8 ± 1.7 / 4 | 16.5 / 18.3 ± 2.7 / 5 | 37.8 / 42.0 ± 4.7 / 43 | 39.0 / 42.3 ± 5.2 / 43 | 0.580 | 0.510 | 0.471 | 0.431 | below; below; below; below | 33.8 | 19.3 | 18.0 | 41.7 | 0.573 | 0.508 | 0.464 | 0.432 | b[0]h[0]r[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 399.0 / 429.4 ± 25.6 / 29 | 235.9 / 253.0 ± 21.5 / 14 | 183.8 / 203.4 ± 20.2 / 15 | 437.3 / 471.7 ± 23.8 / 66 | 427.4 / 464.5 ± 27.8 / 66 | 0.589 | 0.474 | 0.536 | 0.438 | below; below; below; below | 426.5 | 250.0 | 201.4 | 467.4 | 0.586 | 0.474 | 0.535 | 0.438 | b[0]h[0]r[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 162.6 / 170.1 ± 9.3 / 12 | 89.2 / 93.3 ± 7.7 / 6 | 76.2 / 81.0 ± 7.7 / 7 | 184.4 / 191.6 ± 7.5 / 55 | 184.8 / 202.3 ± 16.9 / 55 | 0.548 | 0.468 | 0.487 | 0.400 | below; below; below; below | 167.9 | 92.5 | 79.5 | 190.2 | 0.551 | 0.463 | 0.486 | 0.396 | b[0]h[0]r[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 180.8 / 197.6 ± 15.3 / 11 | 93.7 / 100.8 ± 6.9 / 7 | 80.7 / 85.1 ± 5.6 / 8 | 167.3 / 186.5 ± 17.7 / 59 | 171.1 / 196.4 ± 30.3 / 59 | 0.510 | 0.428 | 0.540 | 0.433 | below; below; below; below | 195.8 | 99.8 | 83.7 | 185.1 | 0.510 | 0.426 | 0.539 | 0.432 | b[0]h[0]r[0] |

- Per core (both tools on core 3), `head/base` wall: 0.464 (`fx-ide-db`) to 0.601
  (`fx-syntax`); CPU `head/base` 0.464–0.594. Every head run is below every base run on all
  nine crates. The CPU ratio and the wall ratio agree to within 0.01 on every row (the pinned
  process is not waiting for its core).
- Per core, `head/rustfmt` wall: 0.324 (`fx-ide-db`) to 0.540 (`reg-winnow`); CPU
  0.325–0.539. Every head run is below every rustfmt run on all nine crates.
- Per core, `pgo/base` wall: 0.402 (`fx-ide-db`) to 0.512 (`fx-hir`), every head-pgo run
  below every base run on all nine crates; `pgo/rustfmt` 0.277 (`fx-ide-db`) to 0.438
  (`reg-regex-syntax`), every head-pgo run below every rustfmt run on all nine. The PGO
  build's mean wall and mean CPU are below head's on every crate (`reg-tokio` the closest:
  18.3 vs 19.8 ms wall, 18.0 vs 19.3 ms CPU).
- `head` min is below `base` min on every crate, pinned and unpinned; `head-pgo` min is below
  `base` min on every crate.
- Peak RSS per core: head 4–14 MiB against base 6–29 MiB and rustfmt 43–85 MiB; head-pgo
  5–15 MiB.

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 11.5 / 13.6 ± 1.5 / 5 | 7.6 / 8.2 ± 0.7 / 5 | 7.3 / 8.3 ± 1.0 / 5 | 47.0 / 53.1 ± 6.6 / 44 | 48.2 / 55.1 ± 8.9 / 44 | 0.605 | 0.585 | 0.155 | 0.151 | below; overlap; below; below | 27.9 | 17.2 | 16.6 | 52.3 | 0.619 | 0.569 | 0.330 | 0.308 | b[0]h[0]r[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 20.6 / 21.9 ± 2.7 / 5 | 13.3 / 14.7 ± 2.1 / 4 | 12.7 / 13.2 ± 0.3 / 5 | 45.9 / 48.2 ± 1.6 / 44 | 47.8 / 50.0 ± 3.6 / 44 | 0.672 | 0.592 | 0.306 | 0.264 | below; below; below; below | 21.7 | 14.6 | 13.0 | 47.9 | 0.672 | 0.590 | 0.304 | 0.263 | b[0]h[0]r[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 2.0 / 2.6 ± 0.4 / 4 | 2.0 / 2.4 ± 0.5 / 4 | 1.9 / 2.1 ± 0.2 / 4 | 8.3 / 11.1 ± 1.7 / 34 | 7.9 / 9.3 ± 0.9 / 34 | 0.916 | 0.950 | 0.215 | 0.229 | overlap; overlap; below; below | 2.4 | 2.3 | 2.0 | 10.8 | 0.949 | 0.990 | 0.209 | 0.221 | b[0]h[0]r[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.7 / 1.7 ± 0.1 / 4 | 1.6 / 1.7 ± 0.1 / 4 | 1.6 / 2.0 ± 0.3 / 4 | 8.1 / 9.8 ± 1.3 / 34 | 7.9 / 8.6 ± 0.5 / 34 | 0.999 | 0.878 | 0.178 | 0.237 | overlap; overlap; below; below | 1.6 | 1.6 | 1.9 | 9.5 | 0.991 | 0.927 | 0.169 | 0.224 | b[0]h[0]r[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt). CPU ratios are of mean user+sys.

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU pgo/base | CPU head/rustfmt | CPU pgo/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 81.4 / 85.7 ± 4.9 / 10 | 32.3 / 35.3 ± 2.1 / 10 | – | 134.2 / 146.1 ± 11.2 / 43 | – | 0.412 | – | 0.242 | – | below; –; below; – | 85.3 | 35.1 | – | 144.8 | 0.411 | – | 0.242 | – | b[0]h[0]r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 76.2 / 83.9 ± 7.9 / 14 | 44.3 / 47.7 ± 3.2 / 13 | – | 82.9 / 89.9 ± 6.1 / 53 | – | 0.568 | – | 0.531 | – | below; –; below; – | 82.4 | 47.0 | – | 89.1 | 0.570 | – | 0.528 | – | b[0]h[0]r[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 62.9 / 69.1 ± 8.8 / 9 | 39.3 / 42.8 ± 3.9 / 8 | – | 82.5 / 88.7 ± 8.5 / 47 | – | 0.619 | – | 0.482 | – | below; –; below; – | 68.3 | 42.2 | – | 87.9 | 0.618 | – | 0.480 | – | b[0]h[0]r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 16.7 / 18.0 ± 0.8 / 5 | 9.4 / 10.4 ± 1.6 / 4 | – | 27.4 / 29.2 ± 1.9 / 40 | – | 0.577 | – | 0.355 | – | overlap; –; below; – | 17.8 | 10.0 | – | 28.9 | 0.561 | – | 0.345 | – | b[0]h[0]r[0] |
| stdin-config | 1c (3) | 1 | 169 | 30 | 25.0 / 27.6 ± 3.5 / 6 | 15.6 / 16.3 ± 0.9 / 5 | – | 39.2 / 41.5 ± 2.7 / 44 | – | 0.591 | – | 0.393 | – | below; –; below; – | 27.2 | 16.1 | – | 41.1 | 0.593 | – | 0.393 | – | b[0]h[0]r[0] |

- `tiny-one` (one 13-byte file, process start-up): head 1.7 ms pinned and 2.4 ms unpinned
  against base 1.7 and 2.6 ms; head-pgo 2.0 and 2.1 ms; rustfmt 9.8 and 11.1 ms. `head/base`
  reads 0.916 unpinned and 0.999 pinned, `pgo/base` 0.950 and 0.878 (against sweep 2's base
  of 2.24 and 2.32 ms); every one of these has overlapping ranges. Every head and head-pgo
  run is below every rustfmt run.
- `tiny-test-data` (287 files): head/base 0.605 unpinned and 0.672 pinned, every head run
  below every base run in both; pgo/base 0.585 (ranges overlap, 7.3–12.9 vs 11.2–19.5 ms)
  and 0.592 (below); head/rustfmt 0.155 and 0.306, every head run below every rustfmt run.
- stdin (`examples/fmt_stdin`, no threads, no file I/O, no cache code, no PGO build):
  head/base 0.412 (`stdin-lints`) to 0.619 (`stdin-hir-lib`); every head run is below every
  base run on four files, `stdin-hover-tests` overlaps (9.4–17.7 vs 16.7–20.3 ms).
  head/rustfmt 0.242 (`stdin-lints`) to 0.531 (`stdin-nodes`), every head run below every
  rustfmt run on all five files. CPU ratios equal the wall ratios to within 0.02.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 625 | 30 | 537.2 / 578.6 ± 31.3 / 218 | 33.9 / 36.8 ± 2.6 / 17 | 0.064 | below | 571.6 | 125.5 | 0.220 | 15/15 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 537.1 / 564.1 ± 28.2 / 218 | 106.5 / 113.9 ± 9.1 / 9 | 0.202 | below | 559.6 | 113.0 | 0.202 | 15/15 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1175.2 / 1258.0 ± 36.3 / 116 | 89.9 / 95.4 ± 5.5 / 19 | 0.076 | below | 1245.6 | 355.5 | 0.285 | 122/122 | r[1] h[1] |
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1168.1 / 1233.8 ± 38.1 / 116 | 323.1 / 351.7 ± 22.1 / 8 | 0.285 | below | 1223.0 | 347.0 | 0.284 | 122/122 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 337.8 / 362.5 ± 22.7 / 66 | 34.5 / 37.8 ± 2.9 / 19 | 0.104 | below | 357.8 | 117.2 | 0.328 | 33/33 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 335.8 / 353.0 ± 17.1 / 66 | 99.1 / 111.9 ± 16.9 / 10 | 0.317 | below | 347.8 | 110.0 | 0.316 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 158.3 / 167.9 ± 11.6 / 57 | 19.3 / 21.1 ± 1.8 / 8 | 0.126 | below | 165.5 | 69.8 | 0.422 | 33/33 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 158.3 / 167.0 ± 9.5 / 57 | 58.9 / 59.9 ± 1.0 / 5 | 0.359 | below | 165.7 | 59.6 | 0.360 | 33/33 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 81.4 / 91.1 ± 12.9 / 48 | 12.2 / 13.7 ± 1.5 / 6 | 0.151 | below | 89.0 | 43.6 | 0.490 | 28/28 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 80.0 / 86.6 ± 9.1 / 48 | 34.8 / 35.4 ± 0.9 / 4 | 0.409 | below | 85.5 | 35.1 | 0.410 | 28/28 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 40.0 / 45.9 ± 7.4 / 43 | 7.9 / 9.4 ± 0.9 / 5 | 0.205 | below | 44.4 | 24.9 | 0.561 | 49/49 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 38.4 / 41.5 ± 3.9 / 43 | 18.7 / 19.2 ± 0.4 / 4 | 0.462 | below | 41.2 | 19.0 | 0.460 | 49/49 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1360.2 / 1441.7 ± 49.3 / 423 | 74.9 / 86.7 ± 6.6 / 30 | 0.060 | below | 1429.1 | 290.4 | 0.203 | 33/33 | r[1] h[1] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1348.2 / 1434.3 ± 36.0 / 423 | 234.8 / 249.5 ± 14.1 / 14 | 0.174 | below | 1424.2 | 246.9 | 0.173 | 33/33 | r[1] h[1] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 188.9 / 210.1 ± 17.8 / 55 | 29.1 / 34.2 ± 4.3 / 13 | 0.163 | below | 207.9 | 111.9 | 0.538 | 57/57 | r[0] h[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 187.2 / 195.8 ± 9.4 / 55 | 88.9 / 97.5 ± 8.4 / 6 | 0.498 | below | 195.1 | 96.2 | 0.493 | 57/57 | r[0] h[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 180.2 / 193.8 ± 10.8 / 60 | 29.1 / 31.0 ± 1.9 / 11 | 0.160 | below | 191.7 | 108.0 | 0.564 | 61/61 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 181.4 / 199.6 ± 20.0 / 60 | 94.9 / 101.4 ± 5.9 / 7 | 0.508 | below | 197.9 | 100.6 | 0.508 | 61/61 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 60.1 / 67.5 ± 11.7 / 44 | 7.5 / 8.9 ± 1.7 / 5 | 0.131 | below | 65.7 | 18.2 | 0.278 | 289/289 | r[1] h[1] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 59.1 / 64.4 ± 6.1 / 44 | 14.2 / 14.9 ± 0.7 / 4 | 0.231 | below | 63.9 | 14.6 | 0.229 | 289/289 | r[1] h[1] |
| tiny-one | mc (none) | 1 | 0 | 30 | 9.3 / 11.6 ± 2.3 / 34 | 1.9 / 2.5 ± 0.4 / 4 | 0.212 | below | 10.9 | 2.3 | 0.207 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 9.0 / 11.2 ± 1.9 / 34 | 1.6 / 1.8 ± 0.1 / 4 | 0.161 | below | 10.6 | 1.6 | 0.154 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 136.3 / 147.2 ± 11.8 / 43 | 34.7 / 38.9 ± 4.1 / 10 | 0.264 | below | 145.3 | 38.0 | 0.262 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 87.9 / 95.1 ± 8.5 / 53 | 44.8 / 47.6 ± 2.8 / 13 | 0.500 | below | 94.0 | 46.6 | 0.496 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 87.2 / 89.9 ± 3.2 / 47 | 39.8 / 44.5 ± 5.8 / 8 | 0.495 | below | 89.4 | 43.6 | 0.488 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 28.0 / 28.8 ± 0.5 / 40 | 9.4 / 9.7 ± 0.2 / 4 | 0.337 | below | 28.5 | 9.5 | 0.333 | 1/1 | r[0] h[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 42.0 / 44.1 ± 4.6 / 44 | 15.7 / 16.7 ± 1.0 / 5 | 0.379 | below | 43.5 | 16.2 | 0.374 | 1/1 | r[0] h[0] |

- rustfmt `--check` on unformatted input includes the line diff of every changed file:
  `reg-regex-syntax` 1434.3 ms and 423 MiB pinned against 471.7 ms and 66 MiB on formatted
  input (sweep 1); `fx-hir` 564.1 ms and 218 MiB against 282.5 ms and 56 MiB. head's figures
  on the two inputs differ by less than 12% on every row (the raw/formatted mean ratio lies
  between 0.884 on `fx-syntax` `1c` and 1.099 on `stdin-lints`).
- Every head run is below every rustfmt run on every raw row (27 of 27 `below`).
- The ratios in this table (0.060–0.508) are not like-for-like and are not the ones to quote;
  the formatted-input tables above are.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 13,901,559 bytes)
  copied to a scratch directory and rewritten once by `head --write --no-cache`; `head --check
  --no-cache` on the result exits 0. `CHLORO_CACHE_DIR` points at a scratch directory, which
  holds one file `56497adc96513e4f417307216ac2c810.chloro-cache` of 44,152 bytes after a run
  (the file name is the key of this binary; M2's `fe34bd2` (was `8d418ee`) build wrote `083aac67…`, the same
  size). `head -v --check` on the copy reports `(1226 entries)` for the file; the 1238
  fixtures hold 1230 distinct contents by sha256.
- Commands: `hyperfine -N -i -w 1 -r 15 --metrics … -n cold --prepare "rm -rf $C" "head
  --check $T" -n warm --prepare true "head --check $T" -n no-cache --prepare true "head --check
  --no-cache $T" -n base --prepare true "base --check $T"`, two rounds with the order reversed
  (n = 30); then one round under `taskset -c 3` for `warm`, `no-cache`, `base` (n = 15).
  `cold` is not in the pinned round.

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 430.2 / 446.3 ± 14.1 / 46 | 25.5 / 28.9 ± 2.4 / 5 | 412.2 / 434.3 ± 16.7 / 46 | 740.8 / 838.8 ± 50.7 / 53 | 15.0 | 29.0 | 0.518 | 1656.6 | 40.1 | 1617.2 | 2924.1 |
| 1c | 15 | – | 33.0 / 37.1 ± 3.8 / 5 | 1493.5 / 1563.5 ± 42.7 / 23 | 2826.3 / 2916.9 ± 57.1 / 33 | 42.1 | 78.6 | 0.536 | – | 36.2 | 1551.4 | 2893.5 |

- Warm `--check` of the 1238 formatted files takes 28.9 ms unpinned (15.0x below `--no-cache`,
  29.0x below base) and 37.1 ms on one core (42.1x below `--no-cache`, 78.6x below base); CPU
  40.1 and 36.2 ms; peak RSS 5 MiB against 46 MiB without the cache.
- `cold` (cache directory removed before every run) reads 446.3 ms against 434.3 ms for
  `no-cache` in mean (1.028x; ranges 430.2–483.3 vs 412.2–478.3 ms overlap): hashing and the
  cache write add 12 ms in mean over the formatter on 4 threads in this window.
- `no-cache` head against base on the same copy: 0.518 unpinned (434.3 vs 838.8 ms) and 0.536
  pinned (1563.5 vs 2916.9 ms); CPU 0.553 and 0.536. Peak RSS of `cold` (46 MiB) and
  `no-cache` (46 MiB) is below base's 53 MiB unpinned; pinned, 23 MiB against 33 MiB.
- M2 measured, on this host class, warm 30.4 ms against 523.4 ms `--no-cache` (17.2x) and
  917.2 ms base (30.2x) unpinned, and 38.5 against 1952.4 and 3171.2 ms pinned (50.7x,
  82.3x); the `--no-cache` figure here is below M2's by the E3 and R2 work and the warm figure
  is within 2 ms of M2's, so the `no-cache/warm` ratio reads lower.

## In process and instructions (measurement D)

`taskset -c 3 bench -n 5` (best of 5 in-process formats of the 1238 fixtures, 13.9 MB), three interleaved triples in the order base, head, head-pgo:

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 2.636 | 5.27 |
| 1 | head | 1238 | 13.9 | 1.477 | 9.41 |
| 1 | head-pgo | 1238 | 13.9 | 1.232 | 11.28 |
| 2 | base | 1238 | 13.9 | 2.664 | 5.22 |
| 2 | head | 1238 | 13.9 | 1.480 | 9.39 |
| 2 | head-pgo | 1238 | 13.9 | 1.227 | 11.33 |
| 3 | base | 1238 | 13.9 | 2.637 | 5.27 |
| 3 | head | 1238 | 13.9 | 1.457 | 9.54 |
| 3 | head-pgo | 1238 | 13.9 | 1.200 | 11.59 |

- base: 3 runs, best time 2.636–2.664 s, 5.22–5.27 MB/s
- head: 3 runs, best time 1.457–1.480 s, 9.39–9.54 MB/s
- head-pgo: 3 runs, best time 1.200–1.232 s, 11.28–11.59 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, `chloro` = head, `chloro-base` = base:

- chloro: 1238 files, 13.9 MB: 1.514s, 9.18 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.636s, 5.27 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.671s, 5.20 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.482s, 9.38 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 1.448s, 9.60 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.658s, 5.23 MB/s (best of 5)

### Instructions on the 49-file sample (`benchmarks/profile.sh --runs 2 --core 3`)

| binary | run 1 Ir | run 2 Ir | Ir / base |
|---|---|---|---|
| base | 587,989,199 | 587,990,986 | 1.0000 |
| head | 312,734,041 | 312,733,931 | 0.5319 |
| head-pgo | 244,712,537 | 244,710,917 | 0.4162 |

- `bench -n 5`, three interleaved triples: base 5.22–5.27 MB/s, head 9.39–9.54 MB/s, head-pgo
  11.28–11.59 MB/s. The three ranges are pairwise disjoint (protocol §3 rule for a
  single-threaded change: 3 of 3 pairs, non-overlapping); head/base 1.79x–1.81x on the
  per-round MB/s, head-pgo/head 1.20x–1.22x. The harness's own three pairs (same core, 23:55
  UTC) give base 5.20–5.27 and head 9.18–9.60 MB/s, non-overlapping.
- `conform -i` (hyperfine, 9 runs each, core 3): head 3052.5 ± 59.6 ms (2988.8–3136.3)
  against base 5491.9 ± 104.3 ms (5316.9–5602.6), 0.556, every head run below every base
  run; CPU 3017.7 vs 5450.1 ms; peak RSS 47 vs 50 MiB; exit 0 on every run. By hand in the
  second chain (`taskset -c 3 conform -i`, one run each): head **1220/1220 identical**, 0 not
  idempotent, `rustfmt_version=1.9.0`, diff lines +0 −0, 13.8 MB in 1.454 s (9.5 MB/s), 2.963 s
  wall; base 1219/1220 identical, 0 not idempotent, diff lines +2 −2, 13.8 MB in 2.698 s (5.1
  MB/s), 5.419 s wall.
- `Ir` on the 49-file sample (650,213 bytes, `env -i`, `setarch -R`): base 587,989,199, head
  312,733,931 (−46.81%), head-pgo 244,710,917 (−58.38%; 0.7825x head); two runs of each
  agree to within 1,787 instructions (base), 110 (head) and 1,620 (head-pgo). The base count
  is 44 below M2's 587,989,243 for the same binary hash; the head count's run 1 is 9 above
  the 312,734,032 the ledger records for `af52d66` (was `113367a`) in [../experiments.md](../experiments.md).

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), head with `--no-cache`, `hyperfine -N -i
-w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no `--threads`
option; on one file its static chunking gives the file to one thread, so the `base` column is
the sequential figure with the base's codegen.

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 84.4 / 94.7 ± 10.3 / 10 | 34.7 / 39.0 ± 4.5 / 10 | 23.5 / 27.6 ± 3.6 / 15 | 0.292 | 0.412 | below | 92.9 | 38.4 | 42.9 | 0.462 | b[0] t[0] h[0] |
| stdin-lints | 640 | 1c | 30 | 85.4 / 89.3 ± 9.2 / 10 | 32.3 / 35.5 ± 5.2 / 10 | 32.1 / 34.8 ± 2.6 / 10 | 0.390 | 0.398 | below | 88.5 | 35.3 | 34.6 | 0.391 | b[0] t[0] h[0] |
| stdin-nodes | 358 | mc | 30 | 80.7 / 93.1 ± 14.6 / 14 | 46.2 / 51.8 ± 7.4 / 14 | 31.9 / 36.0 ± 3.1 / 14 | 0.386 | 0.556 | below | 92.0 | 51.0 | 58.0 | 0.630 | b[0] t[0] h[0] |
| stdin-nodes | 358 | 1c | 30 | 77.6 / 87.1 ± 10.4 / 14 | 45.6 / 47.9 ± 3.2 / 14 | 45.4 / 49.4 ± 5.8 / 14 | 0.568 | 0.549 | below | 86.2 | 47.6 | 48.9 | 0.568 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | mc | 30 | 65.9 / 70.6 ± 6.7 / 9 | 40.6 / 46.7 ± 7.0 / 9 | 23.1 / 26.0 ± 2.6 / 9 | 0.368 | 0.662 | below | 69.2 | 46.0 | 47.6 | 0.687 | b[0] t[0] h[0] |
| stdin-hir-lib | 236 | 1c | 30 | 63.0 / 66.5 ± 7.3 / 9 | 38.7 / 42.9 ± 4.7 / 9 | 38.5 / 41.4 ± 5.1 / 9 | 0.622 | 0.646 | overlap | 66.2 | 42.1 | 40.7 | 0.615 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | mc | 30 | 18.3 / 20.3 ± 2.8 / 6 | 10.3 / 11.0 ± 1.0 / 5 | 6.7 / 7.3 ± 0.7 / 5 | 0.360 | 0.541 | below | 20.0 | 10.7 | 12.8 | 0.636 | b[0] t[0] h[0] |
| stdin-hover-tests | 211 | 1c | 30 | 17.7 / 20.1 ± 3.1 / 6 | 9.6 / 10.9 ± 1.6 / 5 | 9.7 / 9.9 ± 0.2 / 5 | 0.494 | 0.540 | below | 19.1 | 10.0 | 9.7 | 0.509 | b[0] t[0] h[0] |
| stdin-config | 169 | mc | 30 | 26.7 / 29.1 ± 2.6 / 6 | 17.0 / 19.5 ± 2.9 / 5 | 11.8 / 13.5 ± 1.3 / 6 | 0.465 | 0.670 | below | 28.7 | 19.0 | 21.9 | 0.764 | b[0] t[0] h[0] |
| stdin-config | 169 | 1c | 30 | 25.6 / 27.0 ± 1.5 / 6 | 15.9 / 16.8 ± 0.9 / 5 | 16.3 / 18.1 ± 2.6 / 5 | 0.668 | 0.621 | below | 26.6 | 16.5 | 17.7 | 0.663 | b[0] t[0] h[0] |

- Unpinned, `head/base` in mean: 0.292 (`stdin-lints`), 0.360 (`stdin-hover-tests`), 0.368
  (`stdin-hir-lib`), 0.386 (`stdin-nodes`), 0.465 (`stdin-config`); on the minima 2.27x–3.59x
  (base min / head min: 84.4/23.5, 80.7/31.9, 65.9/23.1, 18.3/6.7, 26.7/11.8). Every head run
  is below every base run on all five files (M2: four of five, `stdin-config` overlapping).
- `head --threads 1` against base, unpinned: 0.412–0.670 (the codegen and the E4, E7b and E3
  substrate on one thread); pinned: 0.398–0.646.
- CPU of the split, unpinned `head` against base: 0.462 (`stdin-lints`) to 0.764
  (`stdin-config`); every unpinned head row is below base in CPU. Against `head --threads 1`
  the split costs 1.03x (`stdin-hir-lib`, 47.6 vs 46.0 ms) to 1.19x (`stdin-hover-tests`,
  12.8 vs 10.7 ms) CPU. Peak RSS head vs base: 1.44x on `stdin-lints` (15.0 vs 10.4 MiB),
  0.94x–1.05x on the other four.
- Pinned (4 threads on one core), `head/base` in mean: 0.390–0.668; `head` against `head
  --threads 1` pinned: 0.91 (`stdin-hover-tests`) to 1.08 (`stdin-config`, 18.1 vs 16.8 ms),
  ranges overlapping on all five.
- Exit codes: `base`, `head-t1` and `head` exit 0 on every run of every file. M2's exit 1 on
  `stdin-hir-lib` and `stdin-config` came from copies the harness had left as found; the
  harness now formats the stdin copies through stdin (see Method), and `stdin-hir-lib` here
  is 236 KB against M2's 228 KB.

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload.

| run | load at start | `1c` rustfmt | `1c` head | `1c` base | `1c` head/base | `1c` head/rustfmt | `mc` rustfmt | `mc` head | `mc` base | `mc` head/base | `mc` head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 1.51 1.49 0.86 | 838 / 905 ± 42 | 317 / 343 ± 24 | 579 / 621 ± 23 | 0.552 | 0.379 | 843 / 906 ± 57 | 89 / 95 ± 5 | 176 / 195 ± 13 | 0.490 | 0.105 |
| 1 | 1.26 1.72 1.56 | 829 / 891 ± 41 | 317 / 341 ± 22 | 582 / 621 ± 27 | 0.549 | 0.383 | 839 / 886 ± 40 | 88 / 97 ± 7 | 185 / 204 ± 13 | 0.474 | 0.109 |
| 2 | 1.83 1.83 1.62 | 829 / 883 ± 33 | 322 / 347 ± 18 | 597 / 632 ± 29 | 0.549 | 0.393 | 851 / 900 ± 38 | 89 / 94 ± 3 | 173 / 191 ± 11 | 0.492 | 0.104 |
| 3 | 2.00 1.95 1.69 | 841 / 904 ± 39 | 320 / 349 ± 17 | 584 / 665 ± 63 | 0.524 | 0.386 | 835 / 895 ± 43 | 87 / 93 ± 5 | 174 / 197 ± 13 | 0.471 | 0.104 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 341–349 ms
  (2%), the `1c` base mean 621–665 ms (7%), the `1c` rustfmt mean 883–905 ms (2%); the `mc`
  head mean 93–97 ms (4%) and the `mc` base mean 191–204 ms (7%). The `1c` head/base ratio
  reads 0.524–0.552 and the `mc` ratio 0.471–0.492; head/rustfmt reads 0.379–0.393 (`1c`)
  and 0.104–0.109 (`mc`). Every head run is below every base run and every rustfmt run in all
  four runs, pinned and unpinned.
- Within one run of 30, max/min on this host reaches 1.48x for rustfmt (`reg-winnow` `1c`
  167.3–247.0 ms, sweep 1), 1.43x for base (`fx-syntax` `1c` 105.4–150.9 ms) and 1.64x for
  head (`fx-parser` `1c` 34.7–56.9 ms); σ/mean lies between 1.2% and 13.8% on the pinned
  crate rows of the three commands (`fx-parser` base the narrowest at 1.2%, `fx-syntax` head
  the widest at 13.8%) and between 5.1% and 25.7% on the unpinned head crate rows
  (`reg-tokio` the widest).
- Instruction counts move by at most 1,787 of 588 M (0.0003%) between runs.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir` 1c: head vs base: below (107.1–154.1 vs 182.9–228.8 ms); head vs rustfmt: below (107.1–154.1 vs 269.2–340.2 ms); head-pgo vs base: below (92.6–116.4 vs 179.3–232.6 ms); head-pgo vs rustfmt: below (92.6–116.4 vs 270.8–358.1 ms).
- `fx-hir` mc: head vs base: below (32.7–41.1 vs 73.4–116.0 ms); head vs rustfmt: below (32.7–41.1 vs 273.5–346.4 ms); head-pgo vs base: below (29.8–40.5 vs 78.1–124.2 ms); head-pgo vs rustfmt: below (29.8–40.5 vs 270.2–332.9 ms).
- `fx-hir-ty` 1c: head vs base: below (317.0–432.1 vs 579.3–658.8 ms); head vs rustfmt: below (317.0–432.1 vs 838.0–1006.8 ms); head-pgo vs base: below (280.0–322.3 vs 585.5–687.7 ms); head-pgo vs rustfmt: below (280.0–322.3 vs 833.3–1075.1 ms).
- `fx-hir-ty` mc: head vs base: below (88.5–107.7 vs 175.8–229.3 ms); head vs rustfmt: below (88.5–107.7 vs 843.1–1141.0 ms); head-pgo vs base: below (78.4–98.1 vs 169.7–223.6 ms); head-pgo vs rustfmt: below (78.4–98.1 vs 829.7–928.9 ms).
- `fx-ide-db` 1c: head vs base: below (97.2–128.2 vs 208.7–257.4 ms); head vs rustfmt: below (97.2–128.2 vs 296.9–354.0 ms); head-pgo vs base: below (81.7–124.7 vs 207.0–267.2 ms); head-pgo vs rustfmt: below (81.7–124.7 vs 293.5–402.8 ms).
- `fx-ide-db` mc: head vs base: below (33.6–52.2 vs 119.1–158.5 ms); head vs rustfmt: below (33.6–52.2 vs 301.8–363.9 ms); head-pgo vs base: below (28.9–48.2 vs 124.0–213.2 ms); head-pgo vs rustfmt: below (28.9–48.2 vs 303.8–399.8 ms).
- `fx-parser` 1c: head vs base: below (34.7–56.9 vs 62.9–66.2 ms); head vs rustfmt: below (34.7–56.9 vs 71.6–92.0 ms); head-pgo vs base: below (30.3–44.2 vs 63.1–96.7 ms); head-pgo vs rustfmt: below (30.3–44.2 vs 71.8–88.7 ms).
- `fx-parser` mc: head vs base: overlap (12.9–25.0 vs 22.4–33.7 ms); head vs rustfmt: below (12.9–25.0 vs 73.0–91.4 ms); head-pgo vs base: below (11.1–14.1 vs 22.1–46.6 ms); head-pgo vs rustfmt: below (11.1–14.1 vs 71.6–95.1 ms).
- `fx-syntax` 1c: head vs base: below (59.0–90.1 vs 105.4–150.9 ms); head vs rustfmt: below (59.0–90.1 vs 125.6–163.5 ms); head-pgo vs base: below (50.5–69.1 vs 110.2–156.9 ms); head-pgo vs rustfmt: below (50.5–69.1 vs 126.8–163.1 ms).
- `fx-syntax` mc: head vs base: below (18.7–23.3 vs 36.0–68.0 ms); head vs rustfmt: below (18.7–23.3 vs 128.3–184.3 ms); head-pgo vs base: below (16.2–21.3 vs 38.5–53.7 ms); head-pgo vs rustfmt: below (16.2–21.3 vs 126.1–175.0 ms).
- `reg-clap-builder` 1c: head vs base: below (89.2–131.4 vs 162.6–197.1 ms); head vs rustfmt: below (89.2–131.4 vs 184.4–220.8 ms); head-pgo vs base: below (76.2–117.0 vs 163.5–209.2 ms); head-pgo vs rustfmt: below (76.2–117.0 vs 184.8–255.8 ms).
- `reg-clap-builder` mc: head vs base: below (28.2–39.9 vs 52.2–87.1 ms); head vs rustfmt: below (28.2–39.9 vs 187.1–235.1 ms); head-pgo vs base: below (26.4–37.4 vs 50.5–77.3 ms); head-pgo vs rustfmt: below (26.4–37.4 vs 191.3–244.1 ms).
- `reg-regex-syntax` 1c: head vs base: below (235.9–322.1 vs 399.0–486.4 ms); head vs rustfmt: below (235.9–322.1 vs 437.3–518.5 ms); head-pgo vs base: below (183.8–252.9 vs 400.7–481.1 ms); head-pgo vs rustfmt: below (183.8–252.9 vs 427.4–526.7 ms).
- `reg-regex-syntax` mc: head vs base: below (73.4–93.3 vs 117.5–188.6 ms); head vs rustfmt: below (73.4–93.3 vs 438.5–582.8 ms); head-pgo vs base: below (61.0–90.7 vs 112.1–185.1 ms); head-pgo vs rustfmt: below (61.0–90.7 vs 435.6–535.1 ms).
- `reg-tokio` 1c: head vs base: below (18.4–24.5 vs 32.6–40.6 ms); head vs rustfmt: below (18.4–24.5 vs 37.8–55.7 ms); head-pgo vs base: below (16.5–26.0 vs 32.6–50.6 ms); head-pgo vs rustfmt: below (16.5–26.0 vs 39.0–63.2 ms).
- `reg-tokio` mc: head vs base: overlap (7.6–20.1 vs 13.2–29.8 ms); head vs rustfmt: below (7.6–20.1 vs 39.0–64.5 ms); head-pgo vs base: below (7.1–13.2 vs 14.3–23.8 ms); head-pgo vs rustfmt: below (7.1–13.2 vs 40.1–57.0 ms).
- `reg-winnow` 1c: head vs base: below (93.7–123.1 vs 180.8–241.5 ms); head vs rustfmt: below (93.7–123.1 vs 167.3–247.0 ms); head-pgo vs base: below (80.7–101.4 vs 181.4–235.8 ms); head-pgo vs rustfmt: below (80.7–101.4 vs 171.1–289.1 ms).
- `reg-winnow` mc: head vs base: below (29.4–41.2 vs 57.8–80.9 ms); head vs rustfmt: below (29.4–41.2 vs 174.7–216.4 ms); head-pgo vs base: below (25.5–36.4 vs 56.7–80.6 ms); head-pgo vs rustfmt: below (25.5–36.4 vs 172.8–207.9 ms).
- `stdin-config` 1c: head vs base: below (15.6–19.8 vs 25.0–41.6 ms); head vs rustfmt: below (15.6–19.8 vs 39.2–51.6 ms).
- `stdin-hir-lib` 1c: head vs base: below (39.3–54.4 vs 62.9–98.1 ms); head vs rustfmt: below (39.3–54.4 vs 82.5–120.1 ms).
- `stdin-hover-tests` 1c: head vs base: overlap (9.4–17.7 vs 16.7–20.3 ms); head vs rustfmt: below (9.4–17.7 vs 27.4–37.2 ms).
- `stdin-lints` 1c: head vs base: below (32.3–39.1 vs 81.4–104.3 ms); head vs rustfmt: below (32.3–39.1 vs 134.2–181.2 ms).
- `stdin-nodes` 1c: head vs base: below (44.3–59.9 vs 76.2–112.6 ms); head vs rustfmt: below (44.3–59.9 vs 82.9–105.1 ms).
- `tiny-one` 1c: head vs base: overlap (1.6–2.1 vs 1.7–1.9 ms); head vs rustfmt: below (1.6–2.1 vs 8.1–12.6 ms); head-pgo vs base: overlap (1.6–2.6 vs 1.6–6.2 ms); head-pgo vs rustfmt: below (1.6–2.6 vs 7.9–10.0 ms).
- `tiny-one` mc: head vs base: overlap (2.0–4.2 vs 2.0–3.2 ms); head vs rustfmt: below (2.0–4.2 vs 8.3–14.0 ms); head-pgo vs base: overlap (1.9–2.5 vs 1.9–4.0 ms); head-pgo vs rustfmt: below (1.9–2.5 vs 7.9–11.5 ms).
- `tiny-test-data` 1c: head vs base: below (13.3–20.5 vs 20.6–33.0 ms); head vs rustfmt: below (13.3–20.5 vs 45.9–51.6 ms); head-pgo vs base: below (12.7–13.7 vs 21.7–26.0 ms); head-pgo vs rustfmt: below (12.7–13.7 vs 47.8–65.7 ms).
- `tiny-test-data` mc: head vs base: below (7.6–10.2 vs 11.5–17.3 ms); head vs rustfmt: below (7.6–10.2 vs 47.0–72.1 ms); head-pgo vs base: overlap (7.3–12.9 vs 11.2–19.5 ms); head-pgo vs rustfmt: below (7.3–12.9 vs 48.2–90.4 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS (`head-pgo` against the
`base` runs of sweep 2, its own window). No chloro build reads above rustfmt in mean,
minimum, CPU or peak RSS on any workload, formatted or raw; no cell of any table reads head
or head-pgo above base in mean wall, minimum wall or CPU; the list is peak RSS only:

- `fx-ide-db` mc, formatted, head-pgo vs base, rss: 20.1 vs 19.6 (1.025); wall ranges below.
- `tiny-test-data` mc, formatted, head-pgo vs base, rss: 5.3 vs 5.1 (1.032); wall ranges overlap.
- `tiny-test-data` 1c, formatted, head-pgo vs base, rss: 5.0 vs 4.8 (1.048); wall ranges below.
- `tiny-one` mc, formatted, head-pgo vs base, rss: 4.1 vs 3.6 (1.136); wall ranges overlap.
- `tiny-one` 1c, formatted, head-pgo vs base, rss: 4.2 vs 3.6 (1.162); wall ranges overlap.
- `stdin-lints` mc, parallel items, head vs base, rss: 15.0 vs 10.4 (1.436); wall ranges below.
- `stdin-nodes` mc, parallel items, head vs base, rss: 13.9 vs 13.9 (1.000); wall ranges below.
- `stdin-nodes` 1c, parallel items, head vs base, rss: 14.1 vs 13.9 (1.012); wall ranges below.
- `stdin-hir-lib` mc, parallel items, head vs base, rss: 8.9 vs 8.8 (1.006); wall ranges below.
- `stdin-hir-lib` 1c, parallel items, head vs base, rss: 9.0 vs 8.8 (1.018); wall ranges overlap.
- `stdin-config` mc, parallel items, head vs base, rss: 6.2 vs 5.9 (1.046); wall ranges below.

- Wall-time means above base: none. `tiny-one` reads head/base 0.916 unpinned and 0.999
  pinned, pgo/base 0.950 and 0.878, every one with overlapping ranges (a 13-byte file,
  process start-up). M2 read `tiny-one` head above base in mean on the pinned row (1.033) and
  head-pgo above on the unpinned row (1.001); neither cell is above base here.
- CPU above base: none. M2 read `tiny-one` head CPU above base (1.032 unpinned, 1.090
  pinned); here 0.949 and 0.991.
- Peak RSS above base: `stdin-lints` unpinned (15.0 vs 10.4 MiB, the E1 split, the same
  1.44x as M2), five split rows within 0.3 MiB of base, head-pgo on `fx-ide-db` unpinned
  (+0.5 MiB), `tiny-one` (+0.5–0.6 MiB) and `tiny-test-data` (+0.2 MiB). No crate row reads
  `head` above base in peak RSS; no cache row and no in-process row reads above base.

## Against M2

M2 ([m2-quiet-host.md](m2-quiet-host.md)) measured `fe34bd2` (was `8d418ee`) (E6, E1, E2, E4, E8, E7b) on
the 2.80 GHz host class; this record measures `069f5a7` (was `4867896`) (M2's head plus E3 and R2) on the
same host class (same `model name`, `nproc`, `MemTotal` and kernel). Protocol §3 keeps
absolute figures within one window, so the two records' ratios are set side by side and the
absolute figures are not; `base` is the same binary (same sha256) in both:

| ratio (means) | M2, `fe34bd2` (was `8d418ee`) | M3, `069f5a7` (was `4867896`) |
|---|---|---|
| crates, `mc`, head/base | 0.48–0.62 | 0.30–0.58 |
| crates, `1c`, head/base (wall; CPU) | 0.53–0.67; 0.54–0.67 | 0.46–0.60; 0.46–0.59 |
| crates, `mc`, pgo/base | 0.36–0.54 | 0.26–0.50 |
| crates, `1c`, pgo/base | 0.48–0.58 | 0.40–0.51 |
| crates, `mc`, head/rustfmt | 0.130–0.201 | 0.105–0.203 |
| crates, `1c`, head/rustfmt | 0.44–0.61 | 0.32–0.54 |
| crates, `1c`, pgo/rustfmt | 0.37–0.51 | 0.28–0.44 |
| crates, `1c`, head below base in range | 7 of 9 | 9 of 9 |
| crates, `1c`, head below rustfmt in range | 9 of 9 | 9 of 9 |
| stdin, head/base | 0.60–0.86 | 0.41–0.62 |
| split files, `mc`, head/base (wall; CPU) | 0.39–0.56; 0.68–0.92 | 0.29–0.47; 0.46–0.76 |
| `bench -n 5`, head/base on the triples' MB/s | 7.25–7.73 vs 4.94–5.03 (1.44x–1.56x) | 9.39–9.54 vs 5.22–5.27 (1.79x–1.81x) |
| `bench -n 5`, head-pgo MB/s | 8.91–9.27 | 11.28–11.59 |
| `Ir`, head/base; pgo/base | 0.6645; 0.5219 | 0.5319; 0.4162 |
| cache, warm vs no-cache vs base, `mc` | 30.4 / 523.4 / 917.2 ms (17.2x, 30.2x) | 28.9 / 434.3 / 838.8 ms (15.0x, 29.0x) |
| `conform -i`, head/base | 0.65 | 0.56 |
| `conform -i`, head identical | 1219/1220 | 1220/1220 |

- The M2 head/base ratios on the pinned crates (0.53–0.67) and this record's (0.46–0.60)
  differ by the E3 and R2 work between the two heads; the two records' `Ir` ratios (0.6645
  and 0.5319) move in the same direction (−33.55% and −46.81% against base).
- In M2 the pinned crate ranges of head and base overlapped on two crates (`fx-ide-db`,
  `fx-syntax`); here none of the nine overlap. The `tiny-one` cells M2 listed above base in
  mean and CPU read below base here.
- The `stdin-lints` row reads head/base 0.412 here against 0.862 in M2 and `stdin-hir-lib`
  0.619 against 0.646; the `stdin-hir-lib` and `stdin-config` copies differ between the two
  records (see Method), so those two rows compare different bytes.

## Harness notes

- `compare_rustfmt.sh` passes `--no-cache` to the head builds on its own (`chloro_cache_flag=
  --no-cache` in `env.txt`) and no flag to `base`, which rejects it. The `--rustfmt` and
  `--registry` defaults resolved without being passed.
- `benchmarks/pgo.sh "$PWD/target" cli` ran to completion: instrumented build, training (13
  `.profraw` files merged), optimised build of `chloro` and `examples/bench` under
  `target/pgo/release/`.
- The harness's stdin preparation (since M2: the copy is rustfmt's stdin output) leaves the
  five stdin copies as fixed points for the CLI: `chloro --check <copy>` exits 0 for `base`
  and `head` on all five, so the measurement E rows time a `--check` that finds no change in
  every build, and no row carries exit 1.
- The first by-hand chain stopped after the cache rounds with exit 141: its environment
  record piped `head -v --check` into `head -3` under `set -o pipefail`, and the SIGPIPE on
  the writer ended the script. The three cache exports (`cache.mc.r1.json`, `cache.mc.r2.json`,
  `cache.1c.r1.json`) were complete; the remaining steps (`d-inproc`, `e-parallel-items`,
  the noise runs) ran in a second chain started 2.5 minutes later, with the same binaries
  and copies.
- `/usr/bin/time` is not in the container; the by-hand `conform -i` runs record wall, user
  and sys through bash's `time`, and the peak RSS of `conform -i` comes from the harness's
  hyperfine rows in `d-inproc-harness`.
- `--baseline` requires all four baseline binaries and `--chloro` requires `fmt_stdin`, `bench`
  and `conform` to exist even for groups that do not run them; the PGO sweep used head's three
  examples (the harness defaults) to satisfy the check.
- The three registry crates other than `clap_builder` are not in a fresh container's cargo
  registry; the script exits 2 at `prepare` until they are fetched.
- The harness's `conform -i` validation log records only the exit status, not the agreement
  counts; the counts in measurement D come from the by-hand run in the second chain.
- hyperfine is not in a fresh container; `cargo install hyperfine --locked` built 2.0.0.

## Current State

- `benchmarks/compare_rustfmt.sh`, `report.py`, `profile.sh` and `pgo.sh` ran to completion on
  every sweep (exit 0) on a host with no other agent; seven harness results directories and
  three by-hand directories hold the JSON exports, `commands.txt`, `env.txt` and `report.md`.
- On formatted input with all 4 CPUs, `head` takes 0.105–0.203x rustfmt's wall time on the
  nine crates and 0.30–0.58x base's; on one core 0.32–0.54x rustfmt's and 0.46–0.60x base's
  (CPU 0.46–0.59x). Every head run is below every rustfmt run and every base run on every
  crate pinned; unpinned, below every rustfmt run on every crate and below every base run on
  seven of nine.
- `head-pgo` takes 0.096–0.190x rustfmt's wall time unpinned and 0.28–0.44x pinned; 0.26–0.50x
  base's unpinned and 0.40–0.51x pinned; `Ir` 0.4162x base's and 0.7825x head's.
- The warm cache answers `--check` on 1238 formatted files in 28.9 ms against 434.3 ms without
  it (15.0x) and 838.8 ms for base (29.0x); 42.1x and 78.6x on one core.
- In process, head formats the fixtures at 9.39–9.54 MB/s against base's 5.22–5.27 MB/s
  (disjoint over 3 interleaved triples), head-pgo at 11.28–11.59 MB/s; `conform -i` reports
  1220/1220 identical for head and 1219/1220 for base.
- The parallel-items split gives 0.29–0.47x base's wall time on the five largest files with
  4 threads, at 0.46–0.76x the CPU and up to 1.44x the peak RSS.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure.
- The `cold` cache variant has no pinned row.
- No raw-input row for `head-pgo` (measurement A's raw sweep is head only, as in M1 and M2).
- The by-hand `conform -i` runs have no peak RSS of their own (no `/usr/bin/time` in the
  container).

## Appendix: `report.py` tables as written

Each block is the `report.md` of one results directory, unedited (`env.txt` header, then the
tables; `chloro` is the `--chloro` build of that sweep, `chloro-base` is `base`).

### `a1-head-base-formatted`

```text
date_utc=2026-10-14T23:37:06Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.51 1.49 0.86
cpu3_stat_start=18310 0 2246 34827 736 0 41 213 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.92 1.87 1.26
cpu3_stat_end=45454 0 4234 42663 738 0 46 437 0 0
cpu3_steal_ticks=224 cpu3_busy_ticks=29132 cpu3_idle_ticks=7836
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 905.5 ± 41.8 (838.0–1006.8) | 343.0 ± 23.7 (317.0–432.1) | 620.8 ± 22.5 (579.3–658.8) | 0.552 | below | 0.379 | 0.378 | below | 85 | 8 | 895.0 | 340.1 | 0.380 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 906.0 ± 57.4 (843.1–1141.0) | 95.3 ± 4.8 (88.5–107.7) | 194.7 ± 13.4 (175.8–229.3) | 0.490 | below | 0.105 | 0.105 | below | 85 | 19 | 898.7 | 356.0 | 0.396 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 282.5 ± 14.8 (269.2–340.2) | 113.8 ± 9.4 (107.1–154.1) | 194.5 ± 12.0 (182.9–228.8) | 0.585 | below | 0.403 | 0.398 | below | 56 | 9 | 280.9 | 112.7 | 0.401 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 294.9 ± 19.3 (273.5–346.4) | 35.3 ± 1.9 (32.7–41.1) | 93.0 ± 9.0 (73.4–116.0) | 0.380 | below | 0.120 | 0.120 | below | 57 | 16 | 291.7 | 121.0 | 0.415 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 314.3 ± 14.4 (296.9–354.0) | 102.0 ± 6.4 (97.2–128.2) | 220.0 ± 10.1 (208.7–257.4) | 0.464 | below | 0.324 | 0.327 | below | 58 | 10 | 311.6 | 101.4 | 0.325 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 327.9 ± 17.4 (301.8–363.9) | 39.6 ± 3.8 (33.6–52.2) | 132.7 ± 9.2 (119.1–158.5) | 0.299 | below | 0.121 | 0.111 | below | 58 | 19 | 323.5 | 126.0 | 0.389 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 75.3 ± 4.8 (71.6–92.0) | 37.2 ± 4.5 (34.7–56.9) | 64.1 ± 0.8 (62.9–66.2) | 0.580 | below | 0.493 | 0.484 | below | 47 | 4 | 74.6 | 36.9 | 0.494 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 80.0 ± 5.4 (73.0–91.4) | 14.9 ± 2.5 (12.9–25.0) | 28.0 ± 2.7 (22.4–33.7) | 0.531 | overlap | 0.186 | 0.177 | below | 47 | 6 | 79.0 | 45.0 | 0.570 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 136.4 ± 11.4 (125.6–163.5) | 67.8 ± 9.4 (59.0–90.1) | 112.8 ± 8.7 (105.4–150.9) | 0.601 | below | 0.497 | 0.469 | below | 50 | 6 | 135.2 | 66.6 | 0.492 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 143.6 ± 12.0 (128.3–184.3) | 20.3 ± 1.5 (18.7–23.3) | 46.1 ± 6.2 (36.0–68.0) | 0.441 | below | 0.141 | 0.146 | below | 50 | 8 | 141.7 | 68.8 | 0.486 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 191.6 ± 7.5 (184.4–220.8) | 93.3 ± 7.7 (89.2–131.4) | 170.1 ± 9.3 (162.6–197.1) | 0.548 | below | 0.487 | 0.484 | below | 55 | 6 | 190.2 | 92.5 | 0.486 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 207.1 ± 11.4 (187.1–235.1) | 31.4 ± 2.9 (28.2–39.9) | 65.6 ± 8.4 (52.2–87.1) | 0.479 | below | 0.152 | 0.151 | below | 55 | 12 | 205.0 | 105.0 | 0.512 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 471.7 ± 23.8 (437.3–518.5) | 253.0 ± 21.5 (235.9–322.1) | 429.4 ± 25.6 (399.0–486.4) | 0.589 | below | 0.536 | 0.539 | below | 66 | 14 | 467.4 | 250.0 | 0.535 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 476.4 ± 31.5 (438.5–582.8) | 81.0 ± 5.3 (73.4–93.3) | 139.2 ± 16.5 (117.5–188.6) | 0.582 | below | 0.170 | 0.167 | below | 66 | 31 | 472.6 | 281.0 | 0.595 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 42.0 ± 4.7 (37.8–55.7) | 19.8 ± 1.7 (18.4–24.5) | 34.1 ± 1.6 (32.6–40.6) | 0.580 | below | 0.471 | 0.487 | below | 43 | 4 | 41.7 | 19.3 | 0.464 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 43.4 ± 5.7 (39.0–64.5) | 8.8 ± 2.3 (7.6–20.1) | 17.5 ± 2.9 (13.2–29.8) | 0.505 | overlap | 0.203 | 0.195 | below | 43 | 5 | 43.0 | 23.0 | 0.534 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 186.5 ± 17.7 (167.3–247.0) | 100.8 ± 6.9 (93.7–123.1) | 197.6 ± 15.3 (180.8–241.5) | 0.510 | below | 0.540 | 0.560 | below | 59 | 7 | 185.1 | 99.8 | 0.539 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 186.3 ± 10.6 (174.7–216.4) | 33.1 ± 3.4 (29.4–41.2) | 69.2 ± 6.3 (57.8–80.9) | 0.478 | below | 0.178 | 0.168 | below | 59 | 11 | 184.9 | 114.5 | 0.619 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 9.8 ± 1.3 (8.1–12.6) | 1.7 ± 0.1 (1.6–2.1) | 1.7 ± 0.1 (1.7–1.9) | 0.999 | overlap | 0.178 | 0.203 | below | 34 | 4 | 9.5 | 1.6 | 0.169 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 11.1 ± 1.7 (8.3–14.0) | 2.4 ± 0.5 (2.0–4.2) | 2.6 ± 0.4 (2.0–3.2) | 0.916 | overlap | 0.215 | 0.234 | below | 34 | 4 | 10.8 | 2.3 | 0.209 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 48.2 ± 1.6 (45.9–51.6) | 14.7 ± 2.1 (13.3–20.5) | 21.9 ± 2.7 (20.6–33.0) | 0.672 | below | 0.306 | 0.289 | below | 44 | 4 | 47.9 | 14.6 | 0.304 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 53.1 ± 6.6 (47.0–72.1) | 8.2 ± 0.7 (7.6–10.2) | 13.6 ± 1.5 (11.5–17.3) | 0.605 | below | 0.155 | 0.161 | below | 44 | 5 | 52.3 | 17.2 | 0.330 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 169 | 30 | 41.5 ± 2.7 (39.2–51.6) | 16.3 ± 0.9 (15.6–19.8) | 27.6 ± 3.5 (25.0–41.6) | 0.591 | below | 0.393 | 0.399 | below | 44 | 5 | 41.1 | 16.1 | 0.393 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 236 | 30 | 88.7 ± 8.5 (82.5–120.1) | 42.8 ± 3.9 (39.3–54.4) | 69.1 ± 8.8 (62.9–98.1) | 0.619 | below | 0.482 | 0.477 | below | 47 | 8 | 87.9 | 42.2 | 0.480 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 29.2 ± 1.9 (27.4–37.2) | 10.4 ± 1.6 (9.4–17.7) | 18.0 ± 0.8 (16.7–20.3) | 0.577 | overlap | 0.355 | 0.342 | below | 40 | 4 | 28.9 | 10.0 | 0.345 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 146.1 ± 11.2 (134.2–181.2) | 35.3 ± 2.1 (32.3–39.1) | 85.7 ± 4.9 (81.4–104.3) | 0.412 | below | 0.242 | 0.241 | below | 43 | 10 | 144.8 | 35.1 | 0.242 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 89.9 ± 6.1 (82.9–105.1) | 47.7 ± 3.2 (44.3–59.9) | 83.9 ± 7.9 (76.2–112.6) | 0.568 | below | 0.531 | 0.534 | below | 53 | 13 | 89.1 | 47.0 | 0.528 | 1/1 | r[0] c[0] |


### `a2-pgo-base-formatted`

```text
date_utc=2026-10-14T23:43:19Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.92 1.87 1.26
cpu3_stat_start=45461 0 4235 42672 738 0 46 437 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/pgo/release/chloro sha256=320b82ff3889e351
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=1.88 1.83 1.43
cpu3_stat_end=69063 0 5833 50847 740 0 51 700 0 0
cpu3_steal_ticks=263 cpu3_busy_ticks=25200 cpu3_idle_ticks=8175
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 908.1 ± 52.2 (833.3–1075.1) | 294.2 ± 11.5 (280.0–322.3) | 617.3 ± 26.0 (585.5–687.7) | 0.477 | below | 0.324 | 0.336 | below | 85 | 9 | 898.2 | 292.2 | 0.325 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 874.8 ± 32.1 (829.7–928.9) | 83.6 ± 4.4 (78.4–98.1) | 195.4 ± 12.5 (169.7–223.6) | 0.428 | below | 0.096 | 0.094 | below | 85 | 20 | 867.7 | 312.1 | 0.360 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 286.8 ± 15.9 (270.8–358.1) | 97.8 ± 5.7 (92.6–116.4) | 191.1 ± 13.8 (179.3–232.6) | 0.512 | below | 0.341 | 0.342 | below | 57 | 10 | 282.6 | 96.5 | 0.342 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 295.3 ± 18.2 (270.2–332.9) | 34.4 ± 2.5 (29.8–40.5) | 96.3 ± 10.0 (78.1–124.2) | 0.357 | below | 0.116 | 0.110 | below | 56 | 18 | 291.6 | 112.9 | 0.387 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 324.0 ± 26.8 (293.5–402.8) | 89.7 ± 9.3 (81.7–124.7) | 223.2 ± 15.5 (207.0–267.2) | 0.402 | below | 0.277 | 0.278 | below | 58 | 11 | 320.7 | 88.5 | 0.276 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 338.0 ± 26.1 (303.8–399.8) | 36.6 ± 4.2 (28.9–48.2) | 143.9 ± 23.1 (124.0–213.2) | 0.255 | below | 0.108 | 0.095 | below | 58 | 20 | 331.5 | 110.6 | 0.334 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 75.4 ± 4.1 (71.8–88.7) | 33.0 ± 3.2 (30.3–44.2) | 69.1 ± 6.4 (63.1–96.7) | 0.477 | below | 0.437 | 0.422 | below | 47 | 5 | 74.4 | 32.3 | 0.434 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 76.5 ± 4.3 (71.6–95.1) | 11.9 ± 0.7 (11.1–14.1) | 27.1 ± 5.0 (22.1–46.6) | 0.441 | below | 0.156 | 0.154 | below | 47 | 7 | 75.2 | 37.0 | 0.492 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 136.8 ± 9.9 (126.8–163.1) | 52.6 ± 3.5 (50.5–69.1) | 122.4 ± 12.6 (110.2–156.9) | 0.430 | below | 0.385 | 0.398 | below | 50 | 6 | 135.5 | 52.1 | 0.384 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 140.5 ± 14.1 (126.1–175.0) | 18.0 ± 1.6 (16.2–21.3) | 44.7 ± 4.2 (38.5–53.7) | 0.402 | below | 0.128 | 0.128 | below | 50 | 9 | 139.0 | 59.6 | 0.429 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 202.3 ± 16.9 (184.8–255.8) | 81.0 ± 7.7 (76.2–117.0) | 173.1 ± 12.4 (163.5–209.2) | 0.468 | below | 0.400 | 0.412 | below | 55 | 7 | 200.5 | 79.5 | 0.396 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 209.6 ± 13.6 (191.3–244.1) | 30.2 ± 3.1 (26.4–37.4) | 62.3 ± 6.9 (50.5–77.3) | 0.485 | below | 0.144 | 0.138 | below | 55 | 13 | 206.3 | 98.0 | 0.475 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 464.5 ± 27.8 (427.4–526.7) | 203.4 ± 20.2 (183.8–252.9) | 429.5 ± 22.7 (400.7–481.1) | 0.474 | below | 0.438 | 0.430 | below | 66 | 15 | 459.8 | 201.4 | 0.438 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 468.4 ± 28.1 (435.6–535.1) | 70.0 ± 7.9 (61.0–90.7) | 138.8 ± 16.6 (112.1–185.1) | 0.504 | below | 0.149 | 0.140 | below | 66 | 32 | 463.0 | 231.4 | 0.500 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 42.3 ± 5.2 (39.0–63.2) | 18.3 ± 2.7 (16.5–26.0) | 35.8 ± 4.7 (32.6–50.6) | 0.510 | below | 0.431 | 0.423 | below | 43 | 5 | 41.6 | 18.0 | 0.432 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 45.2 ± 4.4 (40.1–57.0) | 8.6 ± 1.8 (7.1–13.2) | 18.8 ± 2.9 (14.3–23.8) | 0.457 | below | 0.190 | 0.178 | below | 43 | 6 | 43.3 | 21.5 | 0.498 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 196.4 ± 30.3 (171.1–289.1) | 85.1 ± 5.6 (80.7–101.4) | 198.5 ± 14.1 (181.4–235.8) | 0.428 | below | 0.433 | 0.471 | below | 59 | 8 | 193.9 | 83.7 | 0.432 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 184.8 ± 10.2 (172.8–207.9) | 27.7 ± 2.5 (25.5–36.4) | 67.2 ± 6.2 (56.7–80.6) | 0.413 | below | 0.150 | 0.148 | below | 59 | 12 | 182.6 | 94.4 | 0.517 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 8.6 ± 0.5 (7.9–10.0) | 2.0 ± 0.3 (1.6–2.6) | 2.3 ± 0.8 (1.6–6.2) | 0.878 | overlap | 0.237 | 0.206 | below | 34 | 4 | 8.4 | 1.9 | 0.224 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 9.3 ± 0.9 (7.9–11.5) | 2.1 ± 0.2 (1.9–2.5) | 2.2 ± 0.4 (1.9–4.0) | 0.950 | overlap | 0.229 | 0.245 | below | 34 | 4 | 9.0 | 2.0 | 0.221 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 50.0 ± 3.6 (47.8–65.7) | 13.2 ± 0.3 (12.7–13.7) | 22.3 ± 0.8 (21.7–26.0) | 0.592 | below | 0.264 | 0.266 | below | 44 | 5 | 49.3 | 13.0 | 0.263 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 55.1 ± 8.9 (48.2–90.4) | 8.3 ± 1.0 (7.3–12.9) | 14.2 ± 1.9 (11.2–19.5) | 0.585 | overlap | 0.151 | 0.152 | below | 44 | 5 | 53.7 | 16.6 | 0.308 | 287/287 | r[0] c[0] |


### `a3-head-raw`

```text
date_utc=2026-10-14T23:48:56Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.88 1.83 1.43
cpu3_stat_start=69072 0 5836 50852 740 0 51 700 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.02 1.29 1.31
cpu3_stat_end=91467 0 9410 62800 741 0 53 941 0 0
cpu3_steal_ticks=241 cpu3_busy_ticks=25969 cpu3_idle_ticks=11948
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1233.8 ± 38.1 (1168.1–1326.1) | 351.7 ± 22.1 (323.1–395.9) | 0.285 | 0.277 | below | 116 | 8 | 1223.0 | 347.0 | 0.284 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1258.0 ± 36.3 (1175.2–1327.3) | 95.4 ± 5.5 (89.9–112.6) | 0.076 | 0.076 | below | 116 | 19 | 1245.6 | 355.5 | 0.285 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 564.1 ± 28.2 (537.1–658.2) | 113.9 ± 9.1 (106.5–146.9) | 0.202 | 0.198 | below | 218 | 9 | 559.6 | 113.0 | 0.202 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 578.6 ± 31.3 (537.2–647.7) | 36.8 ± 2.6 (33.9–46.5) | 0.064 | 0.063 | below | 218 | 17 | 571.6 | 125.5 | 0.220 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 353.0 ± 17.1 (335.8–405.2) | 111.9 ± 16.9 (99.1–169.3) | 0.317 | 0.295 | below | 66 | 10 | 347.8 | 110.0 | 0.316 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 362.5 ± 22.7 (337.8–414.2) | 37.8 ± 2.9 (34.5–47.9) | 0.104 | 0.102 | below | 66 | 19 | 357.8 | 117.2 | 0.328 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 86.6 ± 9.1 (80.0–122.6) | 35.4 ± 0.9 (34.8–39.2) | 0.409 | 0.435 | below | 48 | 4 | 85.5 | 35.1 | 0.410 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 91.1 ± 12.9 (81.4–132.8) | 13.7 ± 1.5 (12.2–19.8) | 0.151 | 0.150 | below | 48 | 6 | 89.0 | 43.6 | 0.490 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 167.0 ± 9.5 (158.3–202.7) | 59.9 ± 1.0 (58.9–62.5) | 0.359 | 0.372 | below | 57 | 5 | 165.7 | 59.6 | 0.360 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 167.9 ± 11.6 (158.3–212.6) | 21.1 ± 1.8 (19.3–26.6) | 0.126 | 0.122 | below | 57 | 8 | 165.5 | 69.8 | 0.422 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 195.8 ± 9.4 (187.2–231.4) | 97.5 ± 8.4 (88.9–121.6) | 0.498 | 0.475 | below | 55 | 6 | 195.1 | 96.2 | 0.493 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 210.1 ± 17.8 (188.9–243.1) | 34.2 ± 4.3 (29.1–45.4) | 0.163 | 0.154 | below | 55 | 13 | 207.9 | 111.9 | 0.538 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1434.3 ± 36.0 (1348.2–1522.5) | 249.5 ± 14.1 (234.8–288.3) | 0.174 | 0.174 | below | 423 | 14 | 1424.2 | 246.9 | 0.173 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1441.7 ± 49.3 (1360.2–1615.2) | 86.7 ± 6.6 (74.9–101.7) | 0.060 | 0.055 | below | 423 | 30 | 1429.1 | 290.4 | 0.203 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 41.5 ± 3.9 (38.4–54.9) | 19.2 ± 0.4 (18.7–20.3) | 0.462 | 0.486 | below | 43 | 4 | 41.2 | 19.0 | 0.460 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 45.9 ± 7.4 (40.0–68.2) | 9.4 ± 0.9 (7.9–11.7) | 0.205 | 0.198 | below | 43 | 5 | 44.4 | 24.9 | 0.561 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 199.6 ± 20.0 (181.4–255.5) | 101.4 ± 5.9 (94.9–122.6) | 0.508 | 0.523 | below | 60 | 7 | 197.9 | 100.6 | 0.508 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 193.8 ± 10.8 (180.2–219.1) | 31.0 ± 1.9 (29.1–36.5) | 0.160 | 0.162 | below | 60 | 11 | 191.7 | 108.0 | 0.564 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 11.2 ± 1.9 (9.0–14.0) | 1.8 ± 0.1 (1.6–2.2) | 0.161 | 0.181 | below | 34 | 4 | 10.6 | 1.6 | 0.154 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 11.6 ± 2.3 (9.3–19.3) | 2.5 ± 0.4 (1.9–3.6) | 0.212 | 0.206 | below | 34 | 4 | 10.9 | 2.3 | 0.207 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 64.4 ± 6.1 (59.1–82.6) | 14.9 ± 0.7 (14.2–17.7) | 0.231 | 0.240 | below | 44 | 4 | 63.9 | 14.6 | 0.229 | 289/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 67.5 ± 11.7 (60.1–102.1) | 8.9 ± 1.7 (7.5–16.6) | 0.131 | 0.126 | below | 44 | 5 | 65.7 | 18.2 | 0.278 | 289/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 44.1 ± 4.6 (42.0–67.0) | 16.7 ± 1.0 (15.7–21.5) | 0.379 | 0.375 | below | 44 | 5 | 43.5 | 16.2 | 0.374 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 89.9 ± 3.2 (87.2–104.0) | 44.5 ± 5.8 (39.8–62.7) | 0.495 | 0.456 | below | 47 | 8 | 89.4 | 43.6 | 0.488 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 28.8 ± 0.5 (28.0–29.5) | 9.7 ± 0.2 (9.4–10.5) | 0.337 | 0.337 | below | 40 | 4 | 28.5 | 9.5 | 0.333 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 147.2 ± 11.8 (136.3–194.6) | 38.9 ± 4.1 (34.7–51.5) | 0.264 | 0.255 | below | 43 | 10 | 145.3 | 38.0 | 0.262 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 95.1 ± 8.5 (87.9–124.5) | 47.6 ± 2.8 (44.8–54.4) | 0.500 | 0.510 | below | 53 | 13 | 94.0 | 46.6 | 0.496 | 1/1 | r[0] c[0] |


### `d-inproc-harness`

```text
date_utc=2026-10-14T23:55:19Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.02 1.29 1.31
cpu3_stat_start=91472 0 9413 62809 741 0 53 941 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.00 1.16 1.25
cpu3_stat_end=108755 0 9570 62817 741 0 53 1090 0 0
cpu3_steal_ticks=149 cpu3_busy_ticks=17440 cpu3_idle_ticks=8
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 3052.5 ± 59.6 (2988.8–3136.3) | 47 | 3017.7 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 5491.9 ± 104.3 (5316.9–5602.6) | 50 | 5450.1 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.514 s | 9.18 |
| 2 | chloro-base | 1238 | 13.9 | 2.636 s | 5.27 |
| 3 | chloro-base | 1238 | 13.9 | 2.671 s | 5.20 |
| 4 | chloro | 1238 | 13.9 | 1.482 s | 9.38 |
| 5 | chloro | 1238 | 13.9 | 1.448 s | 9.60 |
| 6 | chloro-base | 1238 | 13.9 | 2.658 s | 5.23 |

- chloro: 3 runs, 9.18–9.60 MB/s
- chloro-base: 3 runs, 5.20–5.27 MB/s


### `noise-hir-ty-1`

```text
date_utc=2026-10-15T00:03:25Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.26 1.72 1.56
cpu3_stat_start=134647 0 10425 66795 770 0 65 1301 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.83 1.83 1.62
cpu3_stat_end=143004 0 10739 68726 771 0 65 1366 0 0
cpu3_steal_ticks=65 cpu3_busy_ticks=8671 cpu3_idle_ticks=1931
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 891.1 ± 40.6 (829.1–1013.6) | 341.2 ± 22.4 (316.6–401.5) | 621.0 ± 27.5 (581.5–695.2) | 0.549 | below | 0.383 | 0.382 | below | 85 | 8 | 884.9 | 339.3 | 0.383 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 886.1 ± 40.0 (838.9–1041.4) | 96.9 ± 7.3 (88.2–116.0) | 204.3 ± 12.6 (184.7–235.7) | 0.474 | below | 0.109 | 0.105 | below | 85 | 20 | 878.4 | 357.2 | 0.407 | 122/122 | r[0] c[0] |


### `noise-hir-ty-2`

```text
date_utc=2026-10-15T00:05:12Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.83 1.83 1.62
cpu3_stat_start=143004 0 10740 68738 771 0 65 1366 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=2.00 1.95 1.69
cpu3_stat_end=150656 0 11010 71410 771 0 65 1430 0 0
cpu3_steal_ticks=64 cpu3_busy_ticks=7922 cpu3_idle_ticks=2672
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 883.4 ± 33.2 (829.1–947.1) | 346.8 ± 18.0 (322.1–384.9) | 632.1 ± 29.3 (596.7–716.0) | 0.549 | below | 0.393 | 0.388 | below | 85 | 8 | 877.0 | 343.1 | 0.391 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 899.8 ± 38.4 (851.2–1003.4) | 93.7 ± 3.3 (89.1–102.5) | 190.6 ± 11.3 (173.0–209.8) | 0.492 | below | 0.104 | 0.105 | below | 85 | 19 | 891.6 | 350.6 | 0.393 | 122/122 | r[0] c[0] |


### `noise-hir-ty-3`

```text
date_utc=2026-10-15T00:06:59Z
commit=4867896e1b51a68afc9cdf3bf018bc80d8a00269
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.80GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=2.00 1.95 1.69
cpu3_stat_start=150661 0 11012 71419 771 0 65 1430 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=165ad74afb5c45ee
chloro_cache_flag=--no-cache
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0d5a63a695d788c7
bench=/home/user/chloro/target/release/examples/bench sha256=f8ceb61944565753
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.68 1.85 1.68
cpu3_stat_end=159274 0 11327 73285 771 0 66 1517 0 0
cpu3_steal_ticks=87 cpu3_busy_ticks=8928 cpu3_idle_ticks=1866
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 904.3 ± 38.8 (840.6–990.0) | 348.9 ± 17.2 (319.8–385.2) | 665.3 ± 63.4 (584.3–829.7) | 0.524 | below | 0.386 | 0.380 | below | 85 | 8 | 896.1 | 345.5 | 0.386 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 895.2 ± 42.7 (835.1–971.2) | 92.9 ± 5.2 (86.8–108.4) | 197.1 ± 12.7 (174.3–223.1) | 0.471 | below | 0.104 | 0.104 | below | 85 | 19 | 885.0 | 346.4 | 0.391 | 122/122 | r[0] c[0] |
