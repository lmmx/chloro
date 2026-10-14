# M1: base, head and head-pgo against rustfmt on a quiet host

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 1 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds the programme has produced so far and on rustfmt 1.9.0. Every
figure in this file is **measured** in the windows named below; nothing is estimated. The
provisional figures of [../baseline.md](../baseline.md) (load average 6–10) are not compared
with these (protocol §3: figures from different windows are not compared).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `dc20cd7` (the integration branch at the start of this session: E6 cache, E1 parallel items with the largest-first queue and the 128 KiB split threshold, E2 `[profile.release]` `lto = "fat"`, `codegen-units = 1`) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `dc20cd7c652784466484d1196092109feaed6601` (worktree clean, `worktree_dirty=0`
  in every `env.txt`); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git worktree add
  /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `522fcb6f5f690d0044c20c327efc8dc4dc0f2cfef1dc38c8fce56e498a6f01f4` | 2,048,304 |
| `head` bench | `target/release/examples/bench` | `d5fb58db74e7ebfe9d67892d4b595794ce8651b0dc99d0d96eecc1fe6b4550d2` | 1,843,520 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `0aecaf61e604db85a69e723e008f9a4dcebcd0c1287f613a353882aad7cc1033` | 1,683,920 |
| `head` conform | `target/release/examples/conform` | `c058543ce8c8fe8f81333238123859a8e28579d687f9e075621fc379d9c18483` | 1,809,976 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `9e908f4193cf31468815621fe49522c916be11d10fab6fd999e7e7bbff586997` | 3,047,112 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `fce2ad5f59f7e0d534de4ccfa754c8effe73686497a7c5eb9f94f7c3b65376a1` | 2,841,864 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | – |

- The `base` chloro hash `26d25ca997cecebe…` equals the one recorded in
  [../baseline.md](../baseline.md) for the same commit — the default-profile build of
  `884f9c9` reproduces byte for byte on this toolchain.
- The `head-pgo` bench example is `examples/bench` built with the same merged profile
  (`RUSTFLAGS="-Cprofile-use=<repo>/target/pgo-gen/profiles/merged.profdata
  -Cllvm-args=-pgo-warn-missing-function" cargo build --release -p chloro-core --example bench
  --target-dir target/pgo`), a one-command variation of `pgo.sh`'s last step.
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`); hyperfine 2.0.0 (`cargo
  install hyperfine --locked`); valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.10GHz" (`cpu MHz` 2100.000
  in `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. This is the 2.10 GHz
  host of [../experiments/EXP-001-parallel-items.md](../experiments/EXP-001-parallel-items.md),
  not the 2.80 GHz host of [../baseline.md](../baseline.md).
- **Load:** no other agent ran on the host. The load average at the start of each sweep is the
  harness's own processes from the previous sweep (the `mc` rows run chloro with 4 threads);
  before the first process of the day the 1-minute load average read 0.15, before the cache
  sweep 0.07. `cpu3` columns are `/proc/stat` ticks of core 3 (USER_HZ 100) over the sweep:
  steal / busy / idle.

| results directory | start (UTC) | load average at start | at end | cpu3 steal / busy / idle ticks |
|---|---|---|---|---|
| `a1-head-base-formatted` | 2026-10-14T16:19:45Z | 0.77 1.29 0.85 | 1.45 1.56 1.15 | 487 / 31530 / 9949 |
| `a2-pgo-base-formatted` | 2026-10-14T16:27:07Z | 0.95 1.43 1.12 | 1.62 1.57 1.28 | 384 / 26408 / 7454 |
| `a3-head-raw` | 2026-10-14T16:32:50Z | 1.62 1.57 1.28 | 0.96 1.20 1.21 | 434 / 29972 / 11660 |
| `d-inproc-harness` | 2026-10-14T16:39:51Z | 0.96 1.20 1.21 | 1.00 1.09 1.16 | 231 / 19922 / 9 |
| `noise-hir-ty-1` | 2026-10-14T18:58:26Z | 1.07 1.05 0.55 | 1.65 1.34 0.72 | 136 / 7999 / 2318 |
| `noise-hir-ty-2` | 2026-10-14T19:00:11Z | 1.68 1.35 0.72 | 1.51 1.39 0.81 | 128 / 8506 / 1945 |
| `noise-hir-ty-3` | 2026-10-14T19:01:57Z | 1.51 1.39 0.81 | 1.77 1.52 0.93 | 143 / 8762 / 1730 |
| `c-cache` | 2026-10-14T18:52:42Z | 0.07 0.02 0.00 | 2.06 1.10 0.44 | – (by-hand hyperfine) |
| `d-inproc` | 2026-10-14T18:55:17Z | 2.06 1.10 0.44 | 1.10 1.06 0.52 | – |
| `e-parallel-items` | 2026-10-14T18:57:37Z | 1.10 1.06 0.52 | 1.07 1.05 0.55 | – |

- Steal on core 3 is 1.0–1.3% of the sweep's ticks (sweep 1: 487 of 41,966; sweep 2: 384 of
  34,246; the noise runs: 136–143 of 10,453–10,635). The pinned rows carry this steal.
- The session was suspended between the in-process harness run (ended 16:43 UTC) and the
  cache sweep (started 18:52 UTC); the builds, the workload copies under `/tmp/chloro-bench`
  and the results of the first four sweeps were unchanged across the suspension (same
  hashes, same files). The first cache run, interrupted by the suspension, was discarded and
  rerun in full.

## Method

- All process-level figures: `benchmarks/compare_rustfmt.sh` with hyperfine 2.0 `-N -i`,
  `--metrics time_wall_clock,time_user,time_system,memory_peak_resident`, 2 warmup runs and
  **15 timed runs per command per round, 2 rounds with the command order reversed** (n = 30
  per cell), pinned rows on core 3, `mc` rows unpinned, `--mode check`. `report.py` merges
  the rounds; the tables below are built from the same JSON exports (min / mean ± σ wall ms
  / peak RSS MiB; CPU is mean user+sys ms over all threads).
- **The cache is disabled in every sweep except measurement C.** `head` reads and writes a
  fixed-point cache by default (E6), keyed on the binary and the content; on formatted input
  every file is a fixed point, so from the second run on the CLI skips the formatter. The
  harness has no option for `--no-cache`, and `base` rejects the flag (`Could not parse CLI
  arguments`), so the sweeps run under `env -u HOME -u XDG_CACHE_HOME -u CHLORO_CACHE_DIR`:
  `cache::resolve_dir` returns `None` and the CLI logs `Cache: disabled (no cache directory)`
  under `-v` (`chloro/src/cli/cache.rs:220-240`). `--rustfmt` and `--registry` were passed
  explicitly because the script resolves both through `$HOME`. Verified before the sweeps:
  with the three variables unset, `head -v --check` prints the disabled line and the CPU
  column of every `head` row equals the formatter's work (for example `fx-hir-ty` `mc`
  527 ms CPU for 139 ms wall).
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found). The
  registry crates `tokio-1.53.3`, `regex-syntax-0.8.11` and `winnow-0.7.15` were absent from
  this container's `~/.cargo/registry` and were fetched with a scratch `Cargo.toml` pinning
  the four versions (`cargo fetch`); `clap_builder-4.6.7` was present.
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

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt).

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU head/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | mc (none) | 15 | 646 | 30 | 79.2 / 96.3 ± 15.6 / 24 | 41.4 / 51.7 ± 4.6 / 19 | 39.6 / 47.0 ± 3.9 / 20 | 240.6 / 304.1 ± 39.3 / 57 | 242.8 / 290.8 ± 28.7 / 57 | 0.537 | 0.520 | 0.170 | 0.162 | below; below; below; below | 202.3 | 178.3 | 163.2 | 298.5 | 0.882 | 0.597 | b[0] h[0] r[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 174.3 / 197.2 ± 16.4 / 35 | 112.1 / 139.5 ± 15.5 / 23 | 124.9 / 143.1 ± 15.2 / 24 | 749.3 / 934.5 ± 119.9 / 86 | 738.8 / 877.1 ± 90.9 / 86 | 0.707 | 0.772 | 0.149 | 0.163 | below; overlap; below; below | 602.3 | 527.0 | 541.6 | 915.6 | 0.875 | 0.576 | b[0] h[0] r[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 98.9 / 122.8 ± 17.0 / 20 | 61.7 / 74.7 ± 8.2 / 21 | 52.3 / 59.1 ± 4.7 / 22 | 226.8 / 314.1 ± 38.1 / 58 | 257.5 / 305.3 ± 27.8 / 58 | 0.608 | 0.479 | 0.238 | 0.194 | below; below; below; below | 219.1 | 208.1 | 185.9 | 306.1 | 0.950 | 0.680 | b[0] h[0] r[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 35.9 / 49.5 ± 5.6 / 13 | 23.5 / 28.4 ± 3.8 / 9 | 21.1 / 26.3 ± 3.2 / 10 | 110.8 / 136.5 ± 19.8 / 51 | 97.0 / 135.5 ± 19.3 / 51 | 0.573 | 0.615 | 0.208 | 0.194 | overlap; overlap; below; below | 123.8 | 98.8 | 89.4 | 132.8 | 0.798 | 0.744 | b[0] h[0] r[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 20.7 / 27.9 ± 3.7 / 10 | 17.5 / 20.3 ± 2.2 / 7 | 14.1 / 17.6 ± 1.8 / 8 | 54.8 / 82.2 ± 12.3 / 47 | 55.4 / 77.2 ± 13.7 / 47 | 0.728 | 0.676 | 0.247 | 0.228 | overlap; overlap; below; below | 75.2 | 67.4 | 57.9 | 80.1 | 0.896 | 0.841 | b[0] h[0] r[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 14.2 / 20.3 ± 3.4 / 7 | 8.5 / 10.6 ± 1.5 / 6 | 8.4 / 10.0 ± 1.6 / 6 | 31.7 / 43.7 ± 6.7 / 44 | 36.5 / 43.1 ± 4.2 / 44 | 0.522 | 0.550 | 0.242 | 0.231 | below; overlap; below; below | 44.1 | 32.8 | 30.3 | 41.7 | 0.745 | 0.787 | b[0] h[0] r[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 121.5 / 143.7 ± 12.2 / 39 | 106.5 / 126.3 ± 11.7 / 38 | 75.2 / 92.9 ± 9.3 / 39 | 331.3 / 445.7 ± 64.2 / 66 | 361.5 / 434.5 ± 40.6 / 66 | 0.878 | 0.700 | 0.283 | 0.214 | overlap; overlap; below; below | 426.8 | 438.4 | 312.5 | 439.0 | 1.027 | 0.999 | b[0] h[0] r[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 50.1 / 65.6 ± 9.5 / 18 | 43.7 / 48.7 ± 4.3 / 16 | 38.4 / 47.8 ± 6.1 / 17 | 172.2 / 205.2 ± 27.9 / 55 | 143.3 / 200.1 ± 30.2 / 56 | 0.741 | 0.711 | 0.237 | 0.239 | overlap; overlap; below; below | 182.6 | 165.8 | 158.7 | 201.4 | 0.908 | 0.823 | b[0] h[0] r[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 46.7 / 59.0 ± 7.8 / 18 | 37.6 / 44.2 ± 3.9 / 14 | 36.0 / 44.0 ± 5.3 / 15 | 146.0 / 191.9 ± 26.1 / 59 | 128.8 / 162.3 ± 17.7 / 59 | 0.748 | 0.710 | 0.230 | 0.271 | overlap; overlap; below; below | 175.9 | 160.4 | 156.2 | 188.3 | 0.912 | 0.852 | b[0] h[0] r[0] |

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt).

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU head/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 136.3 / 185.6 ± 25.7 / 16 | 118.0 / 152.7 ± 19.0 / 9 | 123.9 / 145.2 ± 13.6 / 10 | 209.5 / 269.8 ± 42.7 / 57 | 195.7 / 291.3 ± 60.6 / 57 | 0.823 | 0.714 | 0.566 | 0.498 | overlap; overlap; below; below | 182.2 | 148.3 | 142.5 | 264.0 | 0.814 | 0.562 | b[0] h[0] r[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 423.2 / 575.8 ± 73.0 / 18 | 462.2 / 544.6 ± 61.5 / 9 | 353.0 / 476.6 ± 52.3 / 10 | 720.5 / 907.2 ± 116.0 / 86 | 608.5 / 891.1 ± 125.2 / 86 | 0.946 | 0.866 | 0.600 | 0.535 | overlap; overlap; below; below | 569.1 | 539.5 | 470.0 | 899.3 | 0.948 | 0.600 | b[0] h[0] r[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 158.8 / 208.6 ± 25.2 / 15 | 134.7 / 204.8 ± 45.4 / 11 | 118.4 / 152.6 ± 22.3 / 12 | 235.8 / 296.7 ± 35.4 / 58 | 232.2 / 289.6 ± 30.5 / 58 | 0.982 | 0.669 | 0.690 | 0.527 | overlap; overlap; overlap; below | 204.4 | 203.1 | 150.2 | 292.8 | 0.993 | 0.694 | b[0] h[0] r[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 95.3 / 117.0 ± 12.6 / 9 | 83.0 / 99.9 ± 15.0 / 6 | 64.2 / 80.8 ± 9.6 / 7 | 95.0 / 128.4 ± 22.1 / 51 | 92.7 / 134.4 ± 21.8 / 51 | 0.854 | 0.804 | 0.778 | 0.601 | overlap; overlap; overlap; overlap | 114.2 | 98.2 | 79.2 | 126.4 | 0.860 | 0.777 | b[0] h[0] r[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 44.3 / 52.8 ± 4.7 / 7 | 46.1 / 50.1 ± 4.7 / 5 | 37.8 / 50.9 ± 5.9 / 6 | 53.5 / 74.1 ± 14.7 / 47 | 53.1 / 68.4 ± 10.3 / 47 | 0.948 | 0.805 | 0.675 | 0.745 | overlap; overlap; overlap; overlap | 52.4 | 49.3 | 50.0 | 72.7 | 0.940 | 0.677 | b[0] h[0] r[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 27.6 / 42.0 ± 11.4 / 6 | 21.2 / 36.3 ± 7.5 / 4 | 23.8 / 29.7 ± 4.3 / 5 | 35.5 / 48.1 ± 10.3 / 44 | 31.9 / 39.5 ± 6.7 / 44 | 0.865 | 0.960 | 0.756 | 0.753 | overlap; overlap; overlap; overlap | 41.6 | 35.7 | 29.2 | 47.4 | 0.859 | 0.754 | b[0] h[0] r[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 312.7 / 402.4 ± 57.2 / 29 | 292.2 / 354.0 ± 41.6 / 16 | 229.5 / 280.0 ± 40.3 / 17 | 371.3 / 434.6 ± 55.1 / 66 | 304.0 / 407.0 ± 55.9 / 66 | 0.880 | 0.707 | 0.814 | 0.688 | overlap; overlap; overlap; overlap | 391.5 | 348.1 | 275.6 | 429.1 | 0.889 | 0.811 | b[0] h[0] r[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 121.4 / 168.4 ± 25.2 / 12 | 102.2 / 128.6 ± 15.3 / 7 | 97.2 / 118.6 ± 14.7 / 8 | 166.2 / 189.4 ± 30.3 / 55 | 138.3 / 179.4 ± 18.7 / 56 | 0.764 | 0.859 | 0.679 | 0.661 | overlap; overlap; overlap; overlap | 165.9 | 126.1 | 117.7 | 188.2 | 0.760 | 0.670 | b[0] h[0] r[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 138.2 / 184.0 ± 27.2 / 11 | 110.6 / 153.9 ± 22.5 / 8 | 106.0 / 146.4 ± 16.0 / 9 | 127.6 / 172.9 ± 23.2 / 59 | 126.9 / 175.5 ± 27.2 / 59 | 0.836 | 0.847 | 0.890 | 0.834 | overlap; overlap; overlap; overlap | 180.0 | 153.0 | 141.9 | 170.6 | 0.850 | 0.897 | b[0] h[0] r[0] |

- `head/base` on all CPUs: 0.52 (`reg-tokio`) to 0.88 (`reg-regex-syntax`); every head run is
  below every base run on `fx-hir`, `fx-hir-ty`, `fx-ide-db` and `reg-tokio`; the ranges
  overlap on the other five crates.
- `head/rustfmt` on all CPUs: 0.149 (`fx-hir-ty`) to 0.283 (`reg-regex-syntax`); every head
  run and every head-pgo run is below every rustfmt run on all nine crates.
- `pgo/base` on all CPUs: 0.48 (`fx-ide-db`) to 0.77 (`fx-hir-ty`). `head-pgo` is below
  `head` in mean on eight crates and above it on `fx-hir-ty` (143.1 vs 139.5 ms, less than
  σ); the single-core table below separates codegen from scheduling.
- Peak RSS on all CPUs: head 6–38 MiB against base 7–39 MiB and rustfmt 44–86 MiB;
  `fx-ide-db` is the one crate where head's peak (21.2 MiB) is above base's (19.7 MiB).

### Crate level, one core (`1c`, core 3), formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt).

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU head/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c (3) | 15 | 646 | 30 | 136.3 / 185.6 ± 25.7 / 16 | 118.0 / 152.7 ± 19.0 / 9 | 123.9 / 145.2 ± 13.6 / 10 | 209.5 / 269.8 ± 42.7 / 57 | 195.7 / 291.3 ± 60.6 / 57 | 0.823 | 0.714 | 0.566 | 0.498 | overlap; overlap; below; below | 182.2 | 148.3 | 142.5 | 264.0 | 0.814 | 0.562 | b[0] h[0] r[0] |
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 423.2 / 575.8 ± 73.0 / 18 | 462.2 / 544.6 ± 61.5 / 9 | 353.0 / 476.6 ± 52.3 / 10 | 720.5 / 907.2 ± 116.0 / 86 | 608.5 / 891.1 ± 125.2 / 86 | 0.946 | 0.866 | 0.600 | 0.535 | overlap; overlap; below; below | 569.1 | 539.5 | 470.0 | 899.3 | 0.948 | 0.600 | b[0] h[0] r[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 158.8 / 208.6 ± 25.2 / 15 | 134.7 / 204.8 ± 45.4 / 11 | 118.4 / 152.6 ± 22.3 / 12 | 235.8 / 296.7 ± 35.4 / 58 | 232.2 / 289.6 ± 30.5 / 58 | 0.982 | 0.669 | 0.690 | 0.527 | overlap; overlap; overlap; below | 204.4 | 203.1 | 150.2 | 292.8 | 0.993 | 0.694 | b[0] h[0] r[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 95.3 / 117.0 ± 12.6 / 9 | 83.0 / 99.9 ± 15.0 / 6 | 64.2 / 80.8 ± 9.6 / 7 | 95.0 / 128.4 ± 22.1 / 51 | 92.7 / 134.4 ± 21.8 / 51 | 0.854 | 0.804 | 0.778 | 0.601 | overlap; overlap; overlap; overlap | 114.2 | 98.2 | 79.2 | 126.4 | 0.860 | 0.777 | b[0] h[0] r[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 44.3 / 52.8 ± 4.7 / 7 | 46.1 / 50.1 ± 4.7 / 5 | 37.8 / 50.9 ± 5.9 / 6 | 53.5 / 74.1 ± 14.7 / 47 | 53.1 / 68.4 ± 10.3 / 47 | 0.948 | 0.805 | 0.675 | 0.745 | overlap; overlap; overlap; overlap | 52.4 | 49.3 | 50.0 | 72.7 | 0.940 | 0.677 | b[0] h[0] r[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 27.6 / 42.0 ± 11.4 / 6 | 21.2 / 36.3 ± 7.5 / 4 | 23.8 / 29.7 ± 4.3 / 5 | 35.5 / 48.1 ± 10.3 / 44 | 31.9 / 39.5 ± 6.7 / 44 | 0.865 | 0.960 | 0.756 | 0.753 | overlap; overlap; overlap; overlap | 41.6 | 35.7 | 29.2 | 47.4 | 0.859 | 0.754 | b[0] h[0] r[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 312.7 / 402.4 ± 57.2 / 29 | 292.2 / 354.0 ± 41.6 / 16 | 229.5 / 280.0 ± 40.3 / 17 | 371.3 / 434.6 ± 55.1 / 66 | 304.0 / 407.0 ± 55.9 / 66 | 0.880 | 0.707 | 0.814 | 0.688 | overlap; overlap; overlap; overlap | 391.5 | 348.1 | 275.6 | 429.1 | 0.889 | 0.811 | b[0] h[0] r[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 121.4 / 168.4 ± 25.2 / 12 | 102.2 / 128.6 ± 15.3 / 7 | 97.2 / 118.6 ± 14.7 / 8 | 166.2 / 189.4 ± 30.3 / 55 | 138.3 / 179.4 ± 18.7 / 56 | 0.764 | 0.859 | 0.679 | 0.661 | overlap; overlap; overlap; overlap | 165.9 | 126.1 | 117.7 | 188.2 | 0.760 | 0.670 | b[0] h[0] r[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 138.2 / 184.0 ± 27.2 / 11 | 110.6 / 153.9 ± 22.5 / 8 | 106.0 / 146.4 ± 16.0 / 9 | 127.6 / 172.9 ± 23.2 / 59 | 126.9 / 175.5 ± 27.2 / 59 | 0.836 | 0.847 | 0.890 | 0.834 | overlap; overlap; overlap; overlap | 180.0 | 153.0 | 141.9 | 170.6 | 0.850 | 0.897 | b[0] h[0] r[0] |

- Per core (both tools on core 3), `head/base` wall: 0.76 (`reg-clap-builder`) to 0.98
  (`fx-ide-db`); CPU `head/base` 0.76–0.99. No crate has head below base in range; all nine
  overlap. The CPU ratio and the wall ratio agree to within 0.02 on every row (the pinned
  process is not waiting for its core).
- Per core, `head/rustfmt` wall: 0.57 (`fx-hir`) to 0.89 (`reg-winnow`); CPU 0.56–0.90.
  Every head run is below every rustfmt run on `fx-hir` and `fx-hir-ty`; the other seven
  overlap.
- Per core, `pgo/base` wall: 0.67 (`fx-ide-db`) to 0.96 (`reg-tokio`); `pgo/rustfmt`
  0.50–0.83. The PGO build's CPU is below head's on every crate except `fx-parser` (50.0 vs
  49.3 ms).
- `head` min is above `base` min on `fx-hir-ty` (462.2 vs 423.2 ms) and `fx-parser` (46.1 vs
  44.3 ms) while the means are below; one run of 30 decides a minimum.

## Many independent files, start-up and one file on stdin (measurement A)

### Many independent files and start-up, formatted input, `--check`

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt).

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU head/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-test-data | mc (none) | 287 | 67 | 30 | 10.1 / 13.6 ± 1.6 / 5 | 7.6 / 9.6 ± 2.4 / 5 | 6.9 / 9.0 ± 1.2 / 6 | 36.8 / 49.1 ± 5.6 / 44 | 36.8 / 49.8 ± 6.0 / 44 | 0.703 | 0.651 | 0.195 | 0.181 | overlap; overlap; below; below | 26.5 | 20.9 | 20.6 | 47.3 | 0.789 | 0.442 | b[0] h[0] r[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 21.7 / 24.4 ± 0.9 / 5 | 17.6 / 21.6 ± 1.0 / 4 | 13.1 / 16.5 ± 4.7 / 5 | 36.5 / 49.6 ± 9.3 / 44 | 37.4 / 47.2 ± 5.8 / 44 | 0.887 | 0.946 | 0.436 | 0.350 | overlap; overlap; below; below | 24.1 | 21.4 | 15.8 | 48.0 | 0.889 | 0.446 | b[0] h[0] r[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 1.6 / 2.0 ± 0.3 / 4 | 1.7 / 2.2 ± 0.5 / 4 | 1.8 / 2.3 ± 0.5 / 4 | 8.9 / 10.2 ± 1.2 / 35 | 8.2 / 10.2 ± 1.1 / 34 | 1.091 | 1.108 | 0.216 | 0.222 | overlap; overlap; below; below | 1.8 | 1.9 | 2.0 | 9.6 | 1.043 | 0.197 | b[0] h[0] r[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 1.3 / 1.5 ± 0.2 / 4 | 1.2 / 1.4 ± 0.1 / 4 | 1.4 / 1.7 ± 0.2 / 4 | 7.4 / 9.3 ± 1.1 / 35 | 6.8 / 7.9 ± 0.8 / 34 | 0.911 | 0.899 | 0.147 | 0.210 | overlap; overlap; below; below | 1.3 | 1.2 | 1.5 | 9.1 | 0.926 | 0.136 | b[0] h[0] r[0] |

### One file on stdin (rustfmt vs `examples/fmt_stdin`), core 3

Cells: min / mean ± σ wall ms / peak RSS MiB. Ratios of means. `ranges` compares min–max wall ranges (head vs base; head-pgo vs base; head vs rustfmt; head-pgo vs rustfmt).

| workload | variant | files | KB | n | base | head | head-pgo | rustfmt (sweep 1) | rustfmt (sweep 2) | head/base | pgo/base | head/rustfmt | pgo/rustfmt | ranges | CPU base | CPU head | CPU pgo | CPU rustfmt | CPU head/base | CPU head/rustfmt | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 1c (3) | 1 | 640 | 30 | 66.1 / 86.6 ± 10.4 / 10 | 69.1 / 83.1 ± 13.3 / 10 | – | 121.5 / 139.2 ± 18.4 / 43 | – | 0.960 | – | 0.597 | – | overlap; –; below; – | 82.5 | 81.1 | – | 137.1 | 0.984 | 0.592 | b[0] h[0] r[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 61.3 / 69.7 ± 7.3 / 14 | 56.5 / 61.4 ± 6.3 / 14 | – | 71.5 / 87.7 ± 12.4 / 53 | – | 0.882 | – | 0.700 | – | overlap; –; overlap; – | 68.3 | 60.2 | – | 85.6 | 0.882 | 0.703 | b[0] h[0] r[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 45.0 / 57.1 ± 9.0 / 8 | 42.0 / 56.9 ± 13.8 / 8 | – | 64.1 / 92.4 ± 22.8 / 47 | – | 0.997 | – | 0.616 | – | overlap; –; overlap; – | 56.2 | 56.2 | – | 90.9 | 0.999 | 0.618 | b[0] h[0] r[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 14.2 / 16.1 ± 2.0 / 5 | 13.0 / 14.5 ± 2.1 / 5 | – | 24.3 / 26.1 ± 1.3 / 40 | – | 0.901 | – | 0.555 | – | overlap; –; below; – | 15.4 | 14.2 | – | 25.6 | 0.923 | 0.554 | b[0] h[0] r[0] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 21.6 / 25.4 ± 6.5 / 6 | 17.6 / 22.9 ± 2.6 / 5 | – | 38.0 / 47.9 ± 7.3 / 44 | – | 0.900 | – | 0.477 | – | overlap; –; below; – | 24.8 | 22.2 | – | 47.2 | 0.895 | 0.470 | b[0] h[0] r[0] |

- `tiny-one` (one 13-byte file, process start-up): head 1.4 ms pinned and 2.2 ms unpinned
  against base 1.5 and 2.0 ms; rustfmt 9.3 and 10.2 ms. Unpinned, head and head-pgo are above
  base in mean (1.09, 1.11) and the ranges overlap; the binary is 2.0 MB (head), 3.0 MB
  (head-pgo), 2.5 MB (base).
- `tiny-test-data` (287 files): head/base 0.70 unpinned and 0.89 pinned; head/rustfmt 0.195
  and 0.436, every head run below every rustfmt run.
- stdin (`examples/fmt_stdin`, no threads, no file I/O, no cache code): head/base 0.88–1.00
  — `stdin-hir-lib` 0.997 and `stdin-lints` 0.960 with overlapping ranges; head/rustfmt
  0.48–0.70; CPU ratios equal the wall ratios to within 0.03.

## Input as found (`--input raw`), head only — the inflated comparison

rustfmt builds and prints a diff for every changed file; chloro compares two strings. Cells: min / mean ± σ wall ms / peak RSS MiB.

| workload | variant | files | KB | n | rustfmt | head | head/rustfmt | ranges | CPU rustfmt | CPU head | CPU head/rustfmt | identical outputs | exits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1023.0 / 1273.8 ± 109.3 / 117 | 432.6 / 536.1 ± 64.7 / 9 | 0.421 | below | 1254.0 | 530.6 | 0.423 | 122/122 | r[1] h[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 991.4 / 1200.2 ± 93.9 / 117 | 119.2 / 135.4 ± 9.6 / 23 | 0.113 | below | 1180.9 | 508.4 | 0.431 | 122/122 | r[1] h[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 434.4 / 552.2 ± 45.8 / 218 | 117.3 / 156.1 ± 25.9 / 9 | 0.283 | below | 545.5 | 154.4 | 0.283 | 15/15 | r[1] h[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 474.5 / 553.8 ± 33.8 / 218 | 44.1 / 51.6 ± 5.0 / 19 | 0.093 | below | 542.2 | 174.0 | 0.321 | 15/15 | r[1] h[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 250.5 / 324.5 ± 59.7 / 66 | 143.7 / 180.0 ± 21.3 / 10 | 0.555 | below | 319.4 | 178.0 | 0.557 | 33/33 | r[1] h[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 268.5 / 344.4 ± 54.5 / 66 | 57.6 / 74.8 ± 7.9 / 21 | 0.217 | below | 339.1 | 207.1 | 0.611 | 33/33 | r[1] h[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 72.0 / 86.4 ± 7.6 / 48 | 39.9 / 49.9 ± 7.6 / 5 | 0.577 | below | 85.1 | 49.2 | 0.578 | 28/28 | r[1] h[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 60.0 / 81.6 ± 11.4 / 48 | 16.7 / 18.9 ± 1.8 / 7 | 0.232 | below | 80.1 | 63.2 | 0.789 | 28/28 | r[1] h[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 143.1 / 163.2 ± 18.6 / 57 | 69.4 / 91.5 ± 13.3 / 6 | 0.561 | below | 160.1 | 89.4 | 0.558 | 33/33 | r[1] h[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 131.8 / 167.4 ± 20.2 / 57 | 23.6 / 27.1 ± 3.2 / 9 | 0.162 | below | 162.8 | 93.3 | 0.573 | 33/33 | r[1] h[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 138.3 / 180.8 ± 23.9 / 56 | 98.6 / 123.2 ± 20.1 / 7 | 0.681 | overlap | 178.5 | 122.2 | 0.685 | 57/57 | r[0] h[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 154.0 / 193.6 ± 24.2 / 55 | 38.3 / 48.1 ± 5.0 / 16 | 0.249 | below | 190.7 | 164.4 | 0.862 | 57/57 | r[0] h[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1059.7 / 1378.8 ± 155.8 / 423 | 291.2 / 363.8 ± 52.7 / 16 | 0.264 | below | 1360.6 | 356.0 | 0.262 | 33/33 | r[1] h[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1147.2 / 1451.9 ± 137.1 / 423 | 94.2 / 107.1 ± 8.7 / 38 | 0.074 | below | 1431.0 | 369.3 | 0.258 | 33/33 | r[1] h[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 35.0 / 41.7 ± 6.3 / 44 | 21.9 / 29.7 ± 4.8 / 4 | 0.711 | overlap | 41.1 | 28.8 | 0.702 | 49/49 | r[1] h[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 36.3 / 47.8 ± 10.8 / 44 | 8.7 / 10.4 ± 1.3 / 5 | 0.217 | below | 46.0 | 31.9 | 0.693 | 49/49 | r[1] h[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 154.9 / 182.2 ± 20.0 / 60 | 128.9 / 153.3 ± 17.6 / 8 | 0.842 | overlap | 179.2 | 150.2 | 0.838 | 61/61 | r[1] h[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 159.7 / 206.4 ± 28.9 / 60 | 41.0 / 48.8 ± 4.4 / 14 | 0.236 | below | 202.0 | 173.5 | 0.859 | 61/61 | r[1] h[1] |
| stdin-config | 1c (3) | 1 | 167 | 30 | 35.5 / 39.8 ± 6.0 / 44 | 18.8 / 19.6 ± 0.9 / 5 | 0.493 | below | 39.5 | 19.4 | 0.492 | 1/1 | r[0] h[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 68.0 / 95.4 ± 24.4 / 47 | 49.2 / 62.4 ± 13.1 / 8 | 0.654 | overlap | 94.1 | 62.1 | 0.660 | 1/1 | r[0] h[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 22.6 / 30.6 ± 6.1 / 40 | 11.3 / 15.4 ± 3.7 / 5 | 0.501 | below | 29.9 | 15.0 | 0.503 | 1/1 | r[0] h[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 103.9 / 135.2 ± 16.6 / 43 | 67.0 / 85.6 ± 11.8 / 10 | 0.633 | overlap | 133.5 | 83.4 | 0.624 | 1/1 | r[0] h[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 73.6 / 83.4 ± 10.8 / 54 | 55.6 / 58.3 ± 2.4 / 14 | 0.699 | below | 82.7 | 57.7 | 0.698 | 1/1 | r[0] h[0] |
| tiny-one | 1c (3) | 1 | 0 | 30 | 6.7 / 8.8 ± 1.4 / 35 | 1.4 / 1.6 ± 0.2 / 4 | 0.181 | below | 8.4 | 1.4 | 0.170 | 1/1 | r[0] h[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 8.3 / 9.9 ± 1.0 / 35 | 1.7 / 2.0 ± 0.2 / 4 | 0.201 | below | 9.7 | 1.8 | 0.187 | 1/1 | r[0] h[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 53.5 / 61.3 ± 9.0 / 44 | 16.7 / 19.3 ± 3.1 / 4 | 0.315 | below | 60.5 | 19.0 | 0.314 | 288/289 | r[1] h[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 49.7 / 60.6 ± 7.0 / 44 | 7.5 / 9.8 ± 1.6 / 5 | 0.161 | below | 59.1 | 23.0 | 0.389 | 288/289 | r[1] h[1] |

- rustfmt `--check` on unformatted input includes the line diff of every changed file:
  `reg-regex-syntax` 1378.8 ms and 423 MiB pinned against 434.6 ms and 66 MiB on formatted
  input (sweep 1); `fx-hir` 552.2 ms and 218 MiB against 269.8 ms and 57 MiB. head's figures
  on the two inputs differ by less than 20% on every crate row (`fx-hir-ty` `1c` 536.1 raw vs
  544.6 formatted; `reg-regex-syntax` `1c` 363.8 vs 354.0; the largest differences are
  `reg-tokio` `1c` 29.7 vs 36.3 and `reg-regex-syntax` `mc` 107.1 vs 126.3).
- The ratios in this table (0.074–0.842) are not like-for-like and are not the ones to quote;
  the formatted-input tables above are.

## Cache (measurement C)

- The copy: `chloro-core/tests/conformance/fixtures` (1238 `.rs` files, 14,190,149 bytes)
  copied to a scratch directory and rewritten once by `head --write --no-cache`; `head --check
  --no-cache` on the result exits 0. `CHLORO_CACHE_DIR` points at a scratch directory, which
  holds one file `eb8c3d0ce12f85559305271f2c580d59.chloro-cache` of 44,152 bytes after a run.
- Commands: `hyperfine -N -i -w 1 -r 15 --metrics … -n cold --prepare "rm -rf $C" "head
  --check $T" -n warm --prepare "" "head --check $T" -n no-cache --prepare "" "head --check
  --no-cache $T" -n base --prepare "" "base --check $T"`, two rounds with the order reversed
  (n = 30); then one round under `taskset -c 3` for `warm`, `no-cache`, `base` (n = 15).
  `cold` is not in the pinned round.

| variant | n | cold | warm | no-cache | base | no-cache/warm | base/warm | no-cache/base | CPU cold | CPU warm | CPU no-cache | CPU base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mc | 30 | 526.7 / 641.3 ± 52.5 / 57 | 19.8 / 24.5 ± 2.9 / 5 | 567.8 / 650.8 ± 44.4 / 54 | 658.9 / 773.4 ± 60.0 / 53 | 26.6 | 31.6 | 0.841 | 2391.4 | 34.7 | 2449.7 | 2691.0 |
| 1c | 15 | – | 21.3 / 24.9 ± 2.3 / 5 | 2019.5 / 2342.3 ± 170.0 / 23 | 2591.6 / 2815.2 ± 170.3 / 33 | 94.1 | 113.2 | 0.832 | – | 24.3 | 2306.0 | 2774.2 |

- Warm `--check` of the 1238 formatted files takes 24.5 ms unpinned (26.6x below `--no-cache`,
  31.6x below base) and 24.9 ms on one core (94x below `--no-cache`, 113x below base); CPU
  34.7 and 24.3 ms; peak RSS 5 MiB against 54 MiB without the cache.
- `cold` (cache directory removed before every run) equals `no-cache` in mean (641.3 vs
  650.8 ms, ranges overlap): hashing and the cache write add nothing visible over the
  formatter on 4 threads.
- `no-cache` head against base on the same copy: 0.84 unpinned (650.8 vs 773.4 ms) and 0.83
  pinned (2342.3 vs 2815.2 ms); CPU 0.91 and 0.83. Peak RSS of `cold` (56.8 MiB) and
  `no-cache` (54.2 MiB) is above base's 53.0 MiB unpinned; pinned, 23 MiB against 33 MiB.
- The E6 record ([../experiments/EXP-006-cli-cache.md](../experiments/EXP-006-cli-cache.md))
  measured warm 32.7 ms against 819.4 ms no-cache (25.1x) on the same host class under load
  0.5; the 1c figures there were 28.9 ms against 2904.6 ms (100.7x).

## In process and instructions (measurement D)

| round | binary | files | MB | best time s | MB/s |
|---|---|---|---|---|---|
| 1 | base | 1238 | 13.9 | 2.333 | 5.96 |
| 1 | head | 1238 | 13.9 | 2.002 | 6.94 |
| 1 | head-pgo | 1238 | 13.9 | 2.010 | 6.91 |
| 2 | head-pgo | 1238 | 13.9 | 1.993 | 6.98 |
| 2 | head | 1238 | 13.9 | 2.122 | 6.55 |
| 2 | base | 1238 | 13.9 | 2.320 | 5.99 |
| 3 | base | 1238 | 13.9 | 2.556 | 5.44 |
| 3 | head | 1238 | 13.9 | 2.129 | 6.53 |
| 3 | head-pgo | 1238 | 13.9 | 1.708 | 8.14 |

- base: 3 runs, best time 2.320–2.556 s, 5.44–5.99 MB/s
- head: 3 runs, best time 2.002–2.129 s, 6.53–6.94 MB/s
- head-pgo: 3 runs, best time 1.708–2.010 s, 6.91–8.14 MB/s

Harness run (`compare_rustfmt.sh --groups e --rounds 3 --baseline`), same core, `chloro` = head, `chloro-base` = base:

- chloro: 1238 files, 13.9 MB: 1.938s, 7.17 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.241s, 6.20 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.346s, 5.93 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 2.239s, 6.21 MB/s (best of 5)
- chloro: 1238 files, 13.9 MB: 2.131s, 6.52 MB/s (best of 5)
- chloro-base: 1238 files, 13.9 MB: 2.279s, 6.10 MB/s (best of 5)

### Instructions on the 49-file sample (`benchmarks/profile.sh --runs 2 --core 3`)

| binary | run 1 Ir | run 2 Ir | Ir / base |
|---|---|---|---|
| base | 587,989,174 | 587,989,205 | 1.0000 |
| head | 554,187,095 | 554,185,312 | 0.9425 |
| head-pgo | 448,212,267 | 448,212,439 | 0.7623 |

- `bench -n 5` (best of 5 in-process formats of the 1238 fixtures, 13.9 MB, core 3), three
  interleaved triples: base 5.44–5.99 MB/s, head 6.53–6.94 MB/s, head-pgo 6.91–8.14 MB/s. The
  head range lies entirely above the base range (3 pairs, non-overlapping, protocol §3 rule
  for a single-threaded change); the head-pgo range lies above base and intersects head's at
  6.91–6.94. The harness's own three pairs (same core, 16:40 UTC) give base 5.93–6.20 and head
  6.21–7.17 MB/s, non-overlapping by 0.01 MB/s.
- `conform -i` (hyperfine, 9 runs each, core 3): head 4762.6 ± 190.5 ms against base
  5135.1 ± 324.4 ms (0.93; ranges 4460.6–5044.5 vs 4638.3–5715.4 overlap); CPU 4710.8 vs
  5072.6 ms; 50 MiB both; 1219/1220 identical, 0 not idempotent, exit 0 on every run.
- `Ir` on the 49-file sample (650,213 bytes, `env -i`, `setarch -R`): base 587,989,174, head
  554,185,312 (−5.75%), head-pgo 448,212,267 (−23.77%); two runs of each agree to within
  1,783 instructions. The base count is 598 above the 587,988,576 recorded in
  [../baseline.md](../baseline.md) for the same binary hash on the other host; the head count
  is 76 below the 554,185,388 in [../experiments.md](../experiments.md).

## Parallel items (measurement E)

Five single files, the formatted copies the harness made for the stdin workloads, run through
the CLI (`chloro --check <file>`, not `fmt_stdin`), cache disabled by the environment as above,
`hyperfine -N -i -w 2 -r 15`, two rounds with reversed order (n = 30 per cell). `base` has no
`--threads` option; on one file its static chunking gives the file to one thread, so the
`base` column is the sequential figure with the base's codegen.

| file | KB | variant | n | base | head-t1 | head | head/base | head-t1/base | ranges head vs base | CPU base | CPU head-t1 | CPU head | CPU head/base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-lints | 640 | mc | 30 | 66.7 / 81.9 ± 8.9 / 10 | 59.4 / 78.7 ± 11.8 / 10 | 41.8 / 56.7 ± 5.9 / 16 | 0.692 | 0.961 | overlap | 80.0 | 77.1 | 88.3 | 1.104 |
| stdin-lints | 640 | 1c | 30 | 60.3 / 85.4 ± 13.9 / 10 | 55.9 / 72.1 ± 7.9 / 10 | 58.2 / 79.7 ± 10.7 / 10 | 0.933 | 0.845 | overlap | 84.1 | 71.0 | 77.9 | 0.927 |
| stdin-nodes | 358 | mc | 30 | 64.8 / 78.2 ± 10.1 / 14 | 54.7 / 71.6 ± 11.0 / 14 | 39.4 / 45.3 ± 4.4 / 14 | 0.579 | 0.915 | below | 75.6 | 69.5 | 75.7 | 1.000 |
| stdin-nodes | 358 | 1c | 30 | 52.5 / 70.8 ± 8.2 / 14 | 48.3 / 66.9 ± 11.7 / 14 | 48.9 / 75.1 ± 12.4 / 14 | 1.060 | 0.944 | overlap | 69.7 | 64.1 | 74.0 | 1.061 |
| stdin-hir-lib | 228 | mc | 30 | 47.4 / 60.7 ± 7.6 / 9 | 44.1 / 58.1 ± 7.4 / 9 | 27.1 / 34.7 ± 3.3 / 10 | 0.571 | 0.957 | below | 59.7 | 56.4 | 65.0 | 1.089 |
| stdin-hir-lib | 228 | 1c | 30 | 44.6 / 56.1 ± 11.2 / 9 | 49.5 / 57.8 ± 6.9 / 9 | 45.3 / 55.0 ± 6.0 / 9 | 0.979 | 1.030 | overlap | 55.2 | 56.9 | 53.4 | 0.968 |
| stdin-hover-tests | 211 | mc | 30 | 13.3 / 17.7 ± 2.2 / 6 | 13.7 / 16.7 ± 2.3 / 6 | 8.9 / 11.5 ± 1.2 / 7 | 0.651 | 0.944 | overlap | 17.2 | 16.2 | 21.2 | 1.232 |
| stdin-hover-tests | 211 | 1c | 30 | 12.2 / 16.0 ± 2.1 / 6 | 12.8 / 14.7 ± 2.0 / 6 | 11.2 / 14.0 ± 1.1 / 6 | 0.874 | 0.916 | overlap | 15.4 | 14.4 | 13.6 | 0.882 |
| stdin-config | 167 | mc | 30 | 19.9 / 26.3 ± 3.9 / 6 | 17.7 / 23.4 ± 2.8 / 6 | 12.8 / 16.6 ± 1.9 / 8 | 0.631 | 0.888 | overlap | 25.6 | 23.0 | 28.3 | 1.106 |
| stdin-config | 167 | 1c | 30 | 21.4 / 25.6 ± 3.7 / 6 | 19.5 / 22.7 ± 2.7 / 6 | 16.8 / 22.7 ± 4.6 / 6 | 0.890 | 0.888 | overlap | 25.1 | 21.7 | 22.1 | 0.880 |

- Unpinned, `head/base` in mean: 0.57 (`stdin-hir-lib`), 0.58 (`stdin-nodes`), 0.63
  (`stdin-config`), 0.65 (`stdin-hover-tests`), 0.69 (`stdin-lints`); on the minima
  1.49x–1.75x (base min / head min: 66.7/41.8, 64.8/39.4, 47.4/27.1, 13.3/8.9, 19.9/12.8).
  Every head run is below every base run on `stdin-nodes` and `stdin-hir-lib`; the other three
  overlap. [EXP-001](../experiments/EXP-001-parallel-items.md) reported 1.50x–1.88x on the
  minima of 15 runs on this host class.
- `head --threads 1` against base, unpinned: 0.89–0.96 (the fat-LTO codegen on one thread);
  pinned: 0.85–1.03, with `stdin-hir-lib` at 1.030 (57.8 vs 56.1 ms, ranges overlap).
- CPU cost of the split, unpinned `head` against base: 1.00 (`stdin-nodes`) to 1.23
  (`stdin-hover-tests`); peak RSS 1.01x–1.52x (`stdin-lints` 15.7 vs 10.4 MiB).
- Pinned (4 threads on one core), `head/base` in mean: 0.87–1.06; `stdin-nodes` 1.060 (75.1 vs
  70.8 ms, ranges overlap, CPU 1.061).

## Noise

Cells: min / mean ± σ wall ms. Run 0 is sweep 1's row for the same workload.

| run | load at start | `1c` rustfmt | `1c` head | `1c` base | `1c` head/base | `1c` head/rustfmt | `mc` rustfmt | `mc` head | `mc` base | `mc` head/base | `mc` head/rustfmt |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.77 1.29 0.85 | 721 / 907 ± 116 | 462 / 545 ± 61 | 423 / 576 ± 73 | 0.946 | 0.600 | 749 / 934 ± 120 | 112 / 139 ± 15 | 174 / 197 ± 16 | 0.707 | 0.149 |
| 1 | 1.07 1.05 0.55 | 693 / 812 ± 65 | 394 / 482 ± 32 | 438 / 535 ± 56 | 0.901 | 0.594 | 673 / 826 ± 84 | 113 / 131 ± 11 | 151 / 172 ± 18 | 0.758 | 0.158 |
| 2 | 1.68 1.35 0.72 | 691 / 825 ± 79 | 364 / 509 ± 58 | 457 / 564 ± 53 | 0.904 | 0.617 | 692 / 813 ± 75 | 116 / 131 ± 11 | 157 / 185 ± 17 | 0.707 | 0.161 |
| 3 | 1.51 1.39 0.81 | 658 / 845 ± 85 | 385 / 485 ± 40 | 413 / 567 ± 74 | 0.855 | 0.574 | 686 / 845 ± 81 | 107 / 129 ± 12 | 158 / 178 ± 13 | 0.729 | 0.153 |

- Over the four runs of identical binaries and bytes, the `1c` head mean moves 482–545 ms
  (13%), the `1c` base mean 535–576 ms (8%), the `1c` rustfmt mean 812–907 ms (12%); the `mc`
  head mean 129–139 ms (8%). The `1c` head/base ratio reads 0.855–0.946 and the `mc` ratio
  0.707–0.758; head/rustfmt reads 0.574–0.617 (`1c`) and 0.149–0.161 (`mc`).
- Within one run of 30, max/min on this host reaches 1.5x for rustfmt (`fx-hir-ty` `1c`
  720.5–1103.6 ms, sweep 1) and 1.7x for base (`1c` 423.2–729.0 ms); σ/mean lies between
  8% and 27% on the pinned crate rows of the three commands (`reg-tokio` the widest: 21%,
  21%, 27%) and between 9% and 15% on the unpinned head crate rows.
- Instruction counts move by at most 1,783 of 554 M (0.0003%) between runs.

## Range statements, one per workload and variant (formatted input)

Format: comparison: `below` or `overlap` (range of the first vs range of the second, ms).

- `fx-hir-ty` 1c: head vs base: overlap (462.2–702.2 vs 423.2–729.0 ms); head vs rustfmt: below (462.2–702.2 vs 720.5–1103.6 ms); head-pgo vs base: overlap (353.0–587.1 vs 452.9–669.4 ms); head-pgo vs rustfmt: below (353.0–587.1 vs 608.5–1120.9 ms).
- `fx-hir-ty` mc: head vs base: below (112.1–168.9 vs 174.3–233.7 ms); head vs rustfmt: below (112.1–168.9 vs 749.3–1213.1 ms); head-pgo vs base: overlap (124.9–193.4 vs 156.3–243.0 ms); head-pgo vs rustfmt: below (124.9–193.4 vs 738.8–1112.5 ms).
- `fx-hir` 1c: head vs base: overlap (118.0–190.4 vs 136.3–246.1 ms); head vs rustfmt: below (118.0–190.4 vs 209.5–368.0 ms); head-pgo vs base: overlap (123.9–175.7 vs 128.9–350.7 ms); head-pgo vs rustfmt: below (123.9–175.7 vs 195.7–495.2 ms).
- `fx-hir` mc: head vs base: below (41.4–59.4 vs 79.2–143.9 ms); head vs rustfmt: below (41.4–59.4 vs 240.6–379.3 ms); head-pgo vs base: below (39.6–53.2 vs 65.9–107.6 ms); head-pgo vs rustfmt: below (39.6–53.2 vs 242.8–346.7 ms).
- `fx-ide-db` 1c: head vs base: overlap (134.7–264.4 vs 158.8–266.7 ms); head vs rustfmt: overlap (134.7–264.4 vs 235.8–382.7 ms); head-pgo vs base: overlap (118.4–200.7 vs 185.8–297.4 ms); head-pgo vs rustfmt: below (118.4–200.7 vs 232.2–360.7 ms).
- `fx-ide-db` mc: head vs base: below (61.7–91.0 vs 98.9–161.8 ms); head vs rustfmt: below (61.7–91.0 vs 226.8–404.3 ms); head-pgo vs base: below (52.3–68.5 vs 90.2–165.6 ms); head-pgo vs rustfmt: below (52.3–68.5 vs 257.5–392.7 ms).
- `fx-parser` 1c: head vs base: overlap (46.1–67.8 vs 44.3–57.2 ms); head vs rustfmt: overlap (46.1–67.8 vs 53.5–104.0 ms); head-pgo vs base: overlap (37.8–65.7 vs 48.8–85.2 ms); head-pgo vs rustfmt: overlap (37.8–65.7 vs 53.1–89.7 ms).
- `fx-parser` mc: head vs base: overlap (17.5–27.6 vs 20.7–36.4 ms); head vs rustfmt: below (17.5–27.6 vs 54.8–101.4 ms); head-pgo vs base: overlap (14.1–21.4 vs 21.1–30.7 ms); head-pgo vs rustfmt: below (14.1–21.4 vs 55.4–107.7 ms).
- `fx-syntax` 1c: head vs base: overlap (83.0–127.4 vs 95.3–136.2 ms); head vs rustfmt: overlap (83.0–127.4 vs 95.0–192.0 ms); head-pgo vs base: overlap (64.2–110.3 vs 78.2–128.9 ms); head-pgo vs rustfmt: overlap (64.2–110.3 vs 92.7–181.6 ms).
- `fx-syntax` mc: head vs base: overlap (23.5–41.1 vs 35.9–58.7 ms); head vs rustfmt: below (23.5–41.1 vs 110.8–182.4 ms); head-pgo vs base: overlap (21.1–35.9 vs 31.4–52.8 ms); head-pgo vs rustfmt: below (21.1–35.9 vs 97.0–175.5 ms).
- `reg-clap-builder` 1c: head vs base: overlap (102.2–168.5 vs 121.4–226.8 ms); head vs rustfmt: overlap (102.2–168.5 vs 166.2–273.5 ms); head-pgo vs base: overlap (97.2–159.3 vs 109.5–184.4 ms); head-pgo vs rustfmt: overlap (97.2–159.3 vs 138.3–230.8 ms).
- `reg-clap-builder` mc: head vs base: overlap (43.7–64.2 vs 50.1–87.3 ms); head vs rustfmt: below (43.7–64.2 vs 172.2–261.0 ms); head-pgo vs base: overlap (38.4–64.6 vs 49.9–100.5 ms); head-pgo vs rustfmt: below (38.4–64.6 vs 143.3–275.3 ms).
- `reg-regex-syntax` 1c: head vs base: overlap (292.2–444.7 vs 312.7–528.5 ms); head vs rustfmt: overlap (292.2–444.7 vs 371.3–563.6 ms); head-pgo vs base: overlap (229.5–377.4 vs 282.2–516.2 ms); head-pgo vs rustfmt: overlap (229.5–377.4 vs 304.0–531.3 ms).
- `reg-regex-syntax` mc: head vs base: overlap (106.5–156.0 vs 121.5–165.4 ms); head vs rustfmt: below (106.5–156.0 vs 331.3–585.6 ms); head-pgo vs base: overlap (75.2–119.0 vs 109.6–167.2 ms); head-pgo vs rustfmt: below (75.2–119.0 vs 361.5–529.6 ms).
- `reg-tokio` 1c: head vs base: overlap (21.2–49.0 vs 27.6–66.2 ms); head vs rustfmt: overlap (21.2–49.0 vs 35.5–62.6 ms); head-pgo vs base: overlap (23.8–37.7 vs 23.2–41.7 ms); head-pgo vs rustfmt: overlap (23.8–37.7 vs 31.9–52.4 ms).
- `reg-tokio` mc: head vs base: below (8.5–13.4 vs 14.2–30.5 ms); head vs rustfmt: below (8.5–13.4 vs 31.7–64.9 ms); head-pgo vs base: overlap (8.4–16.2 vs 12.6–26.0 ms); head-pgo vs rustfmt: below (8.4–16.2 vs 36.5–53.9 ms).
- `reg-winnow` 1c: head vs base: overlap (110.6–203.4 vs 138.2–246.6 ms); head vs rustfmt: overlap (110.6–203.4 vs 127.6–225.3 ms); head-pgo vs base: overlap (106.0–188.7 vs 150.5–232.4 ms); head-pgo vs rustfmt: overlap (106.0–188.7 vs 126.9–233.4 ms).
- `reg-winnow` mc: head vs base: overlap (37.6–51.3 vs 46.7–77.8 ms); head vs rustfmt: below (37.6–51.3 vs 146.0–251.4 ms); head-pgo vs base: overlap (36.0–61.6 vs 53.3–74.7 ms); head-pgo vs rustfmt: below (36.0–61.6 vs 128.8–209.5 ms).
- `stdin-config` 1c: head vs base: overlap (17.6–28.1 vs 21.6–43.5 ms); head vs rustfmt: below (17.6–28.1 vs 38.0–57.8 ms).
- `stdin-hir-lib` 1c: head vs base: overlap (42.0–96.6 vs 45.0–86.9 ms); head vs rustfmt: overlap (42.0–96.6 vs 64.1–134.8 ms).
- `stdin-hover-tests` 1c: head vs base: overlap (13.0–18.8 vs 14.2–21.8 ms); head vs rustfmt: below (13.0–18.8 vs 24.3–29.7 ms).
- `stdin-lints` 1c: head vs base: overlap (69.1–117.6 vs 66.1–106.0 ms); head vs rustfmt: below (69.1–117.6 vs 121.5–180.6 ms).
- `stdin-nodes` 1c: head vs base: overlap (56.5–80.5 vs 61.3–90.0 ms); head vs rustfmt: overlap (56.5–80.5 vs 71.5–116.3 ms).
- `tiny-one` 1c: head vs base: overlap (1.2–1.7 vs 1.3–2.4 ms); head vs rustfmt: below (1.2–1.7 vs 7.4–10.8 ms); head-pgo vs base: overlap (1.4–1.9 vs 1.4–2.3 ms); head-pgo vs rustfmt: below (1.4–1.9 vs 6.8–9.7 ms).
- `tiny-one` mc: head vs base: overlap (1.7–4.0 vs 1.6–3.1 ms); head vs rustfmt: below (1.7–4.0 vs 8.9–13.0 ms); head-pgo vs base: overlap (1.8–4.6 vs 1.8–2.4 ms); head-pgo vs rustfmt: below (1.8–4.6 vs 8.2–13.2 ms).
- `tiny-test-data` 1c: head vs base: overlap (17.6–22.5 vs 21.7–26.0 ms); head vs rustfmt: below (17.6–22.5 vs 36.5–68.7 ms); head-pgo vs base: overlap (13.1–37.0 vs 15.4–21.5 ms); head-pgo vs rustfmt: below (13.1–37.0 vs 37.4–57.9 ms).
- `tiny-test-data` mc: head vs base: overlap (7.6–19.7 vs 10.1–17.1 ms); head vs rustfmt: below (7.6–19.7 vs 36.8–59.6 ms); head-pgo vs base: overlap (6.9–12.6 vs 11.0–17.1 ms); head-pgo vs rustfmt: below (6.9–12.6 vs 36.8–62.2 ms).

## Regressions

Every cell, over every table above, where `head` or `head-pgo` reads above `base`, or where a
chloro build reads above rustfmt, in mean, minimum, CPU or peak RSS. No chloro build reads
above rustfmt in mean, minimum or CPU on any workload, formatted or raw; the list is head and
head-pgo against base.

- `fx-hir-ty` 1c, formatted, head vs base, min: 462.2 vs 423.2 (1.092); wall ranges overlap.
- `fx-ide-db` mc, formatted, head vs base, rss: 21.2 vs 19.7 (1.076); wall ranges below.
- `fx-ide-db` mc, formatted, head-pgo vs base, rss: 22.2 vs 19.6 (1.128); wall ranges below.
- `fx-parser` 1c, formatted, head vs base, min: 46.1 vs 44.3 (1.040); wall ranges overlap.
- `reg-regex-syntax` mc, formatted, head vs base, cpu: 438.4 vs 426.8 (1.027); wall ranges overlap.
- `reg-tokio` 1c, formatted, head-pgo vs base, min: 23.8 vs 23.2 (1.025); wall ranges overlap.
- `stdin-lints` 1c, formatted, head vs base, min: 69.1 vs 66.1 (1.045); wall ranges overlap.
- `tiny-one` 1c, formatted, head-pgo vs base, rss: 4.2 vs 3.6 (1.162); wall ranges overlap.
- `tiny-one` mc, formatted, head vs base, mean: 2.2 vs 2.0 (1.091); wall ranges overlap.
- `tiny-one` mc, formatted, head vs base, min: 1.7 vs 1.6 (1.121); wall ranges overlap.
- `tiny-one` mc, formatted, head vs base, cpu: 1.9 vs 1.8 (1.043); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, mean: 2.3 vs 2.0 (1.108); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, min: 1.8 vs 1.8 (1.017); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, cpu: 2.0 vs 1.8 (1.109); wall ranges overlap.
- `tiny-one` mc, formatted, head-pgo vs base, rss: 4.2 vs 3.7 (1.138); wall ranges overlap.
- `tiny-test-data` 1c, formatted, head-pgo vs base, rss: 5.3 vs 4.8 (1.112); wall ranges overlap.
- `tiny-test-data` mc, formatted, head-pgo vs base, rss: 5.7 vs 5.2 (1.101); wall ranges overlap.
- `stdin-config` mc, parallel items, head vs base, cpu: 28.3 vs 25.6 (1.106); wall ranges overlap.
- `stdin-config` mc, parallel items, head vs base, rss: 8.1 vs 5.9 (1.384); wall ranges overlap.
- `stdin-hir-lib` 1c, parallel items, head vs base, min: 45.3 vs 44.6 (1.016); wall ranges overlap.
- `stdin-hir-lib` 1c, parallel items, head vs base, rss: 8.9 vs 8.8 (1.012); wall ranges overlap.
- `stdin-hir-lib` 1c, parallel items, head-t1 vs base, mean: 57.8 vs 56.1 (1.030); wall ranges overlap.
- `stdin-hir-lib` 1c, parallel items, head-t1 vs base, min: 49.5 vs 44.6 (1.110); wall ranges overlap.
- `stdin-hir-lib` 1c, parallel items, head-t1 vs base, cpu: 56.9 vs 55.2 (1.031); wall ranges overlap.
- `stdin-hir-lib` mc, parallel items, head vs base, cpu: 65.0 vs 59.7 (1.089); wall ranges below.
- `stdin-hir-lib` mc, parallel items, head vs base, rss: 10.0 vs 8.8 (1.139); wall ranges below.
- `stdin-hover-tests` 1c, parallel items, head-t1 vs base, min: 12.8 vs 12.2 (1.055); wall ranges overlap.
- `stdin-hover-tests` mc, parallel items, head vs base, cpu: 21.2 vs 17.2 (1.232); wall ranges overlap.
- `stdin-hover-tests` mc, parallel items, head vs base, rss: 7.3 vs 5.6 (1.300); wall ranges overlap.
- `stdin-hover-tests` mc, parallel items, head-t1 vs base, min: 13.7 vs 13.3 (1.034); wall ranges overlap.
- `stdin-lints` mc, parallel items, head vs base, cpu: 88.3 vs 80.0 (1.104); wall ranges overlap.
- `stdin-lints` mc, parallel items, head vs base, rss: 15.7 vs 10.4 (1.516); wall ranges overlap.
- `stdin-nodes` 1c, parallel items, head vs base, mean: 75.1 vs 70.8 (1.060); wall ranges overlap.
- `stdin-nodes` 1c, parallel items, head vs base, cpu: 74.0 vs 69.7 (1.061); wall ranges overlap.
- `stdin-nodes` 1c, parallel items, head vs base, rss: 14.0 vs 13.9 (1.004); wall ranges overlap.
- `stdin-nodes` mc, parallel items, head vs base, cpu: 75.7 vs 75.6 (1.000); wall ranges below.
- `stdin-nodes` mc, parallel items, head vs base, rss: 13.8 vs 13.7 (1.008); wall ranges below.
- `stdin-nodes` mc, parallel items, head-t1 vs base, rss: 13.7 vs 13.7 (1.003); wall ranges overlap.
- cache mc, head cold vs base, rss: 56.8 vs 53.0 (1.071).
- cache mc, head no-cache vs base, rss: 54.2 vs 53.0 (1.023).

- Wall-time means above base: `tiny-one` unpinned (head 1.09, head-pgo 1.11; a 13-byte file,
  process start-up), `stdin-nodes` pinned with 4 threads on one core (head 1.06), and
  `stdin-hir-lib` pinned `--threads 1` (1.03). Every one of these has overlapping ranges.
- CPU above base: `reg-regex-syntax` unpinned (1.027, wall ranges overlap) and the five
  split files unpinned (1.00–1.23) — the cost of the E1 split, as EXP-001 recorded (+4–21%).
- Peak RSS above base: `fx-ide-db` unpinned (head 21.2 vs 19.7 MiB, head-pgo 22.2 MiB), the
  five split files unpinned (up to 1.52x), the cache sweep's `cold` and `no-cache` unpinned
  (56.8 and 54.2 vs 53.0 MiB), and head-pgo on `tiny-one` and `tiny-test-data` (+0.5 MiB).

## Harness notes

- `benchmarks/pgo.sh` with its default root fails at the optimised build: `-Cprofile-use`
  receives the relative path `target/pgo-gen/profiles/merged.profdata`, which rustc resolves
  from a different working directory (`error: file … passed to -C profile-use does not
  exist`, after the instrumented build, training and merge succeeded). `benchmarks/pgo.sh
  /home/user/chloro/target cli` (an absolute root) completes; the script rebuilds the
  instrumented binaries and retrains on the rerun.
- `compare_rustfmt.sh` has no option to pass flags to the chloro CLI, so `head`'s cache cannot
  be turned off from the harness; on `--input formatted` with a cache directory resolvable,
  every timed run after the first warmup is a cache hit and the `chloro` column measures the
  cache, not the formatter. The environment workaround above is what this record used; the
  `--baseline` build (`884f9c9`) rejects `--no-cache` and `--threads`.
- `--baseline` requires all four baseline binaries and `--chloro` requires `fmt_stdin`, `bench`
  and `conform` to exist even for groups that do not run them; the PGO sweep passed head's
  three examples to satisfy the check.
- The three registry crates other than `clap_builder` are not in a fresh container's cargo
  registry; the script exits 2 at `prepare` until they are fetched.
- `pgo.sh` builds an instrumented `bench` for training but no optimised `bench`; the
  `head-pgo` in-process figure needed the extra build command recorded under Identity.
- The harness's `inproc-bench` rounds (`--rounds 3 --baseline`) give three interleaved pairs;
  the by-hand triples add the PGO binary. Both are in the in-process table.

## Current State

- `benchmarks/compare_rustfmt.sh`, `report.py` and `profile.sh` ran to completion on every
  sweep (exit 0) on a host with no other agent; seven harness results directories and three
  by-hand directories hold the JSON exports, `commands.txt`, `env.txt` and `report.md`.
- On formatted input with all 4 CPUs, `head` takes 0.149–0.283x rustfmt's wall time on the
  nine crates and 0.52–0.88x base's; on one core 0.57–0.89x rustfmt's and 0.76–0.98x base's
  (CPU 0.76–0.99x).
- `head-pgo` takes 0.162–0.271x rustfmt's wall time unpinned and 0.50–0.83x pinned; 0.48–0.77x
  base's unpinned and 0.67–0.96x pinned; `Ir` 0.762x base's.
- The warm cache answers `--check` on 1238 formatted files in 24.5 ms against 650.8 ms without
  it (26.6x) and 773.4 ms for base (31.6x); 94x and 113x on one core.
- In process, head formats the fixtures at 6.53–6.94 MB/s against base's 5.44–5.99 MB/s
  (non-overlapping over 3 interleaved pairs), head-pgo at 6.91–8.14 MB/s.
- The parallel-items split gives 0.57–0.69x base's wall time on the five largest files with
  4 threads, at 1.00–1.23x the CPU and up to 1.52x the peak RSS.

## Missing

- No PGO build of `fmt_stdin` or `conform`: the stdin rows and the `conform -i` row have no
  `head-pgo` figure.
- No quiet-host figure for the newer integration head (commits after `dc20cd7`); this record
  is milestone 1 and stays on `dc20cd7`.
- The `cold` cache variant has no pinned row.

## Appendix: `report.py` tables as written

Each block is the `report.md` of one results directory, unedited (`env.txt` header, then the
tables; `chloro` is the `--chloro` build of that sweep, `chloro-base` is `base`).

### `a1-head-base-formatted`

```text
date_utc=2026-10-14T16:19:45Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=0.77 1.29 0.85
cpu3_stat_start=19913 0 1735 40053 1285 0 91 336 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=1.45 1.56 1.15
cpu3_stat_end=49166 0 4012 50002 1290 0 117 823 0 0
cpu3_steal_ticks=487 cpu3_busy_ticks=31530 cpu3_idle_ticks=9949
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 907.2 ± 116.0 (720.5–1103.6) | 544.6 ± 61.5 (462.2–702.2) | 575.8 ± 73.0 (423.2–729.0) | 0.946 | overlap | 0.600 | 0.641 | below | 86 | 9 | 899.3 | 539.5 | 0.600 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 934.5 ± 119.9 (749.3–1213.1) | 139.5 ± 15.5 (112.1–168.9) | 197.2 ± 16.4 (174.3–233.7) | 0.707 | below | 0.149 | 0.150 | below | 86 | 23 | 915.6 | 527.0 | 0.576 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 269.8 ± 42.7 (209.5–368.0) | 152.7 ± 19.0 (118.0–190.4) | 185.6 ± 25.7 (136.3–246.1) | 0.823 | overlap | 0.566 | 0.563 | below | 57 | 9 | 264.0 | 148.3 | 0.562 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 304.1 ± 39.3 (240.6–379.3) | 51.7 ± 4.6 (41.4–59.4) | 96.3 ± 15.6 (79.2–143.9) | 0.537 | below | 0.170 | 0.172 | below | 57 | 19 | 298.5 | 178.3 | 0.597 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 296.7 ± 35.4 (235.8–382.7) | 204.8 ± 45.4 (134.7–264.4) | 208.6 ± 25.2 (158.8–266.7) | 0.982 | overlap | 0.690 | 0.571 | overlap | 58 | 11 | 292.8 | 203.1 | 0.694 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 314.1 ± 38.1 (226.8–404.3) | 74.7 ± 8.2 (61.7–91.0) | 122.8 ± 17.0 (98.9–161.8) | 0.608 | below | 0.238 | 0.272 | below | 58 | 21 | 306.1 | 208.1 | 0.680 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 74.1 ± 14.7 (53.5–104.0) | 50.1 ± 4.7 (46.1–67.8) | 52.8 ± 4.7 (44.3–57.2) | 0.948 | overlap | 0.675 | 0.862 | overlap | 47 | 5 | 72.7 | 49.3 | 0.677 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 82.2 ± 12.3 (54.8–101.4) | 20.3 ± 2.2 (17.5–27.6) | 27.9 ± 3.7 (20.7–36.4) | 0.728 | overlap | 0.247 | 0.320 | below | 47 | 7 | 80.1 | 67.4 | 0.841 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 128.4 ± 22.1 (95.0–192.0) | 99.9 ± 15.0 (83.0–127.4) | 117.0 ± 12.6 (95.3–136.2) | 0.854 | overlap | 0.778 | 0.874 | overlap | 51 | 6 | 126.4 | 98.2 | 0.777 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 136.5 ± 19.8 (110.8–182.4) | 28.4 ± 3.8 (23.5–41.1) | 49.5 ± 5.6 (35.9–58.7) | 0.573 | overlap | 0.208 | 0.212 | below | 51 | 9 | 132.8 | 98.8 | 0.744 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 189.4 ± 30.3 (166.2–273.5) | 128.6 ± 15.3 (102.2–168.5) | 168.4 ± 25.2 (121.4–226.8) | 0.764 | overlap | 0.679 | 0.615 | overlap | 55 | 7 | 188.2 | 126.1 | 0.670 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 205.2 ± 27.9 (172.2–261.0) | 48.7 ± 4.3 (43.7–64.2) | 65.6 ± 9.5 (50.1–87.3) | 0.741 | overlap | 0.237 | 0.253 | below | 55 | 16 | 201.4 | 165.8 | 0.823 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 434.6 ± 55.1 (371.3–563.6) | 354.0 ± 41.6 (292.2–444.7) | 402.4 ± 57.2 (312.7–528.5) | 0.880 | overlap | 0.814 | 0.787 | overlap | 66 | 16 | 429.1 | 348.1 | 0.811 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 445.7 ± 64.2 (331.3–585.6) | 126.3 ± 11.7 (106.5–156.0) | 143.7 ± 12.2 (121.5–165.4) | 0.878 | overlap | 0.283 | 0.321 | below | 66 | 38 | 439.0 | 438.4 | 0.999 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 48.1 ± 10.3 (35.5–62.6) | 36.3 ± 7.5 (21.2–49.0) | 42.0 ± 11.4 (27.6–66.2) | 0.865 | overlap | 0.756 | 0.597 | overlap | 44 | 4 | 47.4 | 35.7 | 0.754 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 43.7 ± 6.7 (31.7–64.9) | 10.6 ± 1.5 (8.5–13.4) | 20.3 ± 3.4 (14.2–30.5) | 0.522 | below | 0.242 | 0.268 | below | 44 | 6 | 41.7 | 32.8 | 0.787 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 172.9 ± 23.2 (127.6–225.3) | 153.9 ± 22.5 (110.6–203.4) | 184.0 ± 27.2 (138.2–246.6) | 0.836 | overlap | 0.890 | 0.867 | overlap | 59 | 8 | 170.6 | 153.0 | 0.897 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 191.9 ± 26.1 (146.0–251.4) | 44.2 ± 3.9 (37.6–51.3) | 59.0 ± 7.8 (46.7–77.8) | 0.748 | overlap | 0.230 | 0.258 | below | 59 | 14 | 188.3 | 160.4 | 0.852 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 9.3 ± 1.1 (7.4–10.8) | 1.4 ± 0.1 (1.2–1.7) | 1.5 ± 0.2 (1.3–2.4) | 0.911 | overlap | 0.147 | 0.158 | below | 35 | 4 | 9.1 | 1.2 | 0.136 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 10.2 ± 1.2 (8.9–13.0) | 2.2 ± 0.5 (1.7–4.0) | 2.0 ± 0.3 (1.6–3.1) | 1.091 | overlap | 0.216 | 0.196 | below | 35 | 4 | 9.6 | 1.9 | 0.197 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 49.6 ± 9.3 (36.5–68.7) | 21.6 ± 1.0 (17.6–22.5) | 24.4 ± 0.9 (21.7–26.0) | 0.887 | overlap | 0.436 | 0.482 | below | 44 | 4 | 48.0 | 21.4 | 0.446 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 49.1 ± 5.6 (36.8–59.6) | 9.6 ± 2.4 (7.6–19.7) | 13.6 ± 1.6 (10.1–17.1) | 0.703 | overlap | 0.195 | 0.207 | below | 44 | 5 | 47.3 | 20.9 | 0.442 | 287/287 | r[0] c[0] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 47.9 ± 7.3 (38.0–57.8) | 22.9 ± 2.6 (17.6–28.1) | 25.4 ± 6.5 (21.6–43.5) | 0.900 | overlap | 0.477 | 0.462 | below | 44 | 5 | 47.2 | 22.2 | 0.470 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 92.4 ± 22.8 (64.1–134.8) | 56.9 ± 13.8 (42.0–96.6) | 57.1 ± 9.0 (45.0–86.9) | 0.997 | overlap | 0.616 | 0.655 | overlap | 47 | 8 | 90.9 | 56.2 | 0.618 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 211 | 30 | 26.1 ± 1.3 (24.3–29.7) | 14.5 ± 2.1 (13.0–18.8) | 16.1 ± 2.0 (14.2–21.8) | 0.901 | overlap | 0.555 | 0.534 | below | 40 | 5 | 25.6 | 14.2 | 0.554 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 139.2 ± 18.4 (121.5–180.6) | 83.1 ± 13.3 (69.1–117.6) | 86.6 ± 10.4 (66.1–106.0) | 0.960 | overlap | 0.597 | 0.569 | below | 43 | 10 | 137.1 | 81.1 | 0.592 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 358 | 30 | 87.7 ± 12.4 (71.5–116.3) | 61.4 ± 6.3 (56.5–80.5) | 69.7 ± 7.3 (61.3–90.0) | 0.882 | overlap | 0.700 | 0.790 | overlap | 53 | 14 | 85.6 | 60.2 | 0.703 | 1/1 | r[0] c[0] |


### `a2-pgo-base-formatted`

```text
date_utc=2026-10-14T16:27:07Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=0.95 1.43 1.12
cpu3_stat_start=49176 0 4019 52077 1293 0 117 824 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/pgo/release/chloro sha256=9e908f4193cf3146
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b,d
work=/tmp/chloro-bench
loadavg_end=1.62 1.57 1.28
cpu3_stat_end=73869 0 5734 59531 1295 0 136 1208 0 0
cpu3_steal_ticks=384 cpu3_busy_ticks=26408 cpu3_idle_ticks=7454
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 891.1 ± 125.2 (608.5–1120.9) | 476.6 ± 52.3 (353.0–587.1) | 550.5 ± 54.3 (452.9–669.4) | 0.866 | overlap | 0.535 | 0.580 | below | 86 | 10 | 881.7 | 470.0 | 0.533 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 877.1 ± 90.9 (738.8–1112.5) | 143.1 ± 15.2 (124.9–193.4) | 185.4 ± 16.9 (156.3–243.0) | 0.772 | overlap | 0.163 | 0.169 | below | 86 | 24 | 863.9 | 541.6 | 0.627 | 122/122 | r[0] c[0] |
| fx-hir | 1c (3) | 15 | 646 | 30 | 291.3 ± 60.6 (195.7–495.2) | 145.2 ± 13.6 (123.9–175.7) | 203.2 ± 45.1 (128.9–350.7) | 0.714 | overlap | 0.498 | 0.633 | below | 57 | 10 | 287.9 | 142.5 | 0.495 | 15/15 | r[0] c[0] |
| fx-hir | mc (none) | 15 | 646 | 30 | 290.8 ± 28.7 (242.8–346.7) | 47.0 ± 3.9 (39.6–53.2) | 90.4 ± 13.0 (65.9–107.6) | 0.520 | below | 0.162 | 0.163 | below | 57 | 20 | 283.8 | 163.2 | 0.575 | 15/15 | r[0] c[0] |
| fx-ide-db | 1c (3) | 33 | 1169 | 30 | 289.6 ± 30.5 (232.2–360.7) | 152.6 ± 22.3 (118.4–200.7) | 228.2 ± 31.2 (185.8–297.4) | 0.669 | overlap | 0.527 | 0.510 | below | 58 | 12 | 284.8 | 150.2 | 0.527 | 33/33 | r[0] c[0] |
| fx-ide-db | mc (none) | 33 | 1169 | 30 | 305.3 ± 27.8 (257.5–392.7) | 59.1 ± 4.7 (52.3–68.5) | 123.4 ± 17.2 (90.2–165.6) | 0.479 | below | 0.194 | 0.203 | below | 58 | 22 | 300.5 | 185.9 | 0.619 | 33/33 | r[0] c[0] |
| fx-parser | 1c (3) | 28 | 235 | 30 | 68.4 ± 10.3 (53.1–89.7) | 50.9 ± 5.9 (37.8–65.7) | 63.3 ± 8.1 (48.8–85.2) | 0.805 | overlap | 0.745 | 0.712 | overlap | 47 | 6 | 66.8 | 50.0 | 0.749 | 28/28 | r[0] c[0] |
| fx-parser | mc (none) | 28 | 235 | 30 | 77.2 ± 13.7 (55.4–107.7) | 17.6 ± 1.8 (14.1–21.4) | 26.1 ± 2.6 (21.1–30.7) | 0.676 | overlap | 0.228 | 0.254 | below | 47 | 8 | 75.7 | 57.9 | 0.765 | 28/28 | r[0] c[0] |
| fx-syntax | 1c (3) | 33 | 405 | 30 | 134.4 ± 21.8 (92.7–181.6) | 80.8 ± 9.6 (64.2–110.3) | 100.6 ± 12.1 (78.2–128.9) | 0.804 | overlap | 0.601 | 0.692 | overlap | 51 | 7 | 131.5 | 79.2 | 0.603 | 33/33 | r[0] c[0] |
| fx-syntax | mc (none) | 33 | 405 | 30 | 135.5 ± 19.3 (97.0–175.5) | 26.3 ± 3.2 (21.1–35.9) | 42.7 ± 6.2 (31.4–52.8) | 0.615 | overlap | 0.194 | 0.217 | below | 51 | 10 | 131.1 | 89.4 | 0.682 | 33/33 | r[0] c[0] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 179.4 ± 18.7 (138.3–230.8) | 118.6 ± 14.7 (97.2–159.3) | 138.2 ± 15.3 (109.5–184.4) | 0.859 | overlap | 0.661 | 0.703 | overlap | 56 | 8 | 177.7 | 117.7 | 0.662 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 200.1 ± 30.2 (143.3–275.3) | 47.8 ± 6.1 (38.4–64.6) | 67.3 ± 12.8 (49.9–100.5) | 0.711 | overlap | 0.239 | 0.268 | below | 56 | 17 | 194.6 | 158.7 | 0.816 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1647 | 30 | 407.0 ± 55.9 (304.0–531.3) | 280.0 ± 40.3 (229.5–377.4) | 396.1 ± 49.9 (282.2–516.2) | 0.707 | overlap | 0.688 | 0.755 | overlap | 66 | 17 | 401.1 | 275.6 | 0.687 | 33/33 | r[0] c[0] |
| reg-regex-syntax | mc (none) | 33 | 1647 | 30 | 434.5 ± 40.6 (361.5–529.6) | 92.9 ± 9.3 (75.2–119.0) | 132.7 ± 14.6 (109.6–167.2) | 0.700 | overlap | 0.214 | 0.208 | below | 66 | 39 | 426.4 | 312.5 | 0.733 | 33/33 | r[0] c[0] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 39.5 ± 6.7 (31.9–52.4) | 29.7 ± 4.3 (23.8–37.7) | 31.0 ± 4.8 (23.2–41.7) | 0.960 | overlap | 0.753 | 0.745 | overlap | 44 | 5 | 38.9 | 29.2 | 0.750 | 49/49 | r[0] c[0] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 43.1 ± 4.2 (36.5–53.9) | 10.0 ± 1.6 (8.4–16.2) | 18.2 ± 2.7 (12.6–26.0) | 0.550 | overlap | 0.231 | 0.229 | below | 44 | 6 | 41.9 | 30.3 | 0.723 | 49/49 | r[0] c[0] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 175.5 ± 27.2 (126.9–233.4) | 146.4 ± 16.0 (106.0–188.7) | 172.9 ± 19.2 (150.5–232.4) | 0.847 | overlap | 0.834 | 0.835 | overlap | 59 | 9 | 172.8 | 141.9 | 0.821 | 61/61 | r[0] c[0] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 162.3 ± 17.7 (128.8–209.5) | 44.0 ± 5.3 (36.0–61.6) | 62.0 ± 6.1 (53.3–74.7) | 0.710 | overlap | 0.271 | 0.280 | below | 59 | 15 | 159.4 | 156.2 | 0.980 | 61/61 | r[0] c[0] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 7.9 ± 0.8 (6.8–9.7) | 1.7 ± 0.2 (1.4–1.9) | 1.8 ± 0.3 (1.4–2.3) | 0.899 | overlap | 0.210 | 0.207 | below | 34 | 4 | 7.6 | 1.5 | 0.197 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 10.2 ± 1.1 (8.2–13.2) | 2.3 ± 0.5 (1.8–4.6) | 2.0 ± 0.2 (1.8–2.4) | 1.108 | overlap | 0.222 | 0.220 | below | 34 | 4 | 9.6 | 2.0 | 0.214 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 287 | 67 | 30 | 47.2 ± 5.8 (37.4–57.9) | 16.5 ± 4.7 (13.1–37.0) | 17.5 ± 1.6 (15.4–21.5) | 0.946 | overlap | 0.350 | 0.351 | below | 44 | 5 | 45.9 | 15.8 | 0.344 | 287/287 | r[0] c[0] |
| tiny-test-data | mc (none) | 287 | 67 | 30 | 49.8 ± 6.0 (36.8–62.2) | 9.0 ± 1.2 (6.9–12.6) | 13.8 ± 1.6 (11.0–17.1) | 0.651 | overlap | 0.181 | 0.187 | below | 44 | 6 | 48.4 | 20.6 | 0.426 | 287/287 | r[0] c[0] |


### `a3-head-raw`

```text
date_utc=2026-10-14T16:32:50Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.62 1.57 1.28
cpu3_stat_start=73874 0 5737 59535 1295 0 136 1209 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=none
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=raw groups=a,b,c,d
work=/tmp/chloro-bench
loadavg_end=0.96 1.20 1.21
cpu3_stat_end=99737 0 9846 71195 1300 0 155 1643 0 0
cpu3_steal_ticks=434 cpu3_busy_ticks=29972 cpu3_idle_ticks=11660
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2816 | 30 | 1273.8 ± 109.3 (1023.0–1554.0) | 536.1 ± 64.7 (432.6–663.3) | 0.421 | 0.423 | below | 117 | 9 | 1254.0 | 530.6 | 0.423 | 122/122 | r[1] c[1] |
| fx-hir-ty | mc (none) | 122 | 2816 | 30 | 1200.2 ± 93.9 (991.4–1371.8) | 135.4 ± 9.6 (119.2–158.9) | 0.113 | 0.120 | below | 117 | 23 | 1180.9 | 508.4 | 0.431 | 122/122 | r[1] c[1] |
| fx-hir | 1c (3) | 15 | 625 | 30 | 552.2 ± 45.8 (434.4–638.3) | 156.1 ± 25.9 (117.3–221.2) | 0.283 | 0.270 | below | 218 | 9 | 545.5 | 154.4 | 0.283 | 15/15 | r[1] c[1] |
| fx-hir | mc (none) | 15 | 625 | 30 | 553.8 ± 33.8 (474.5–611.1) | 51.6 ± 5.0 (44.1–67.1) | 0.093 | 0.093 | below | 218 | 19 | 542.2 | 174.0 | 0.321 | 15/15 | r[1] c[1] |
| fx-ide-db | 1c (3) | 33 | 1160 | 30 | 324.5 ± 59.7 (250.5–572.6) | 180.0 ± 21.3 (143.7–221.5) | 0.555 | 0.574 | below | 66 | 10 | 319.4 | 178.0 | 0.557 | 33/33 | r[1] c[1] |
| fx-ide-db | mc (none) | 33 | 1160 | 30 | 344.4 ± 54.5 (268.5–555.4) | 74.8 ± 7.9 (57.6–90.2) | 0.217 | 0.215 | below | 66 | 21 | 339.1 | 207.1 | 0.611 | 33/33 | r[1] c[1] |
| fx-parser | 1c (3) | 28 | 232 | 30 | 86.4 ± 7.6 (72.0–98.1) | 49.9 ± 7.6 (39.9–67.4) | 0.577 | 0.554 | below | 48 | 5 | 85.1 | 49.2 | 0.578 | 28/28 | r[1] c[1] |
| fx-parser | mc (none) | 28 | 232 | 30 | 81.6 ± 11.4 (60.0–111.3) | 18.9 ± 1.8 (16.7–22.9) | 0.232 | 0.278 | below | 48 | 7 | 80.1 | 63.2 | 0.789 | 28/28 | r[1] c[1] |
| fx-syntax | 1c (3) | 33 | 394 | 30 | 163.2 ± 18.6 (143.1–218.8) | 91.5 ± 13.3 (69.4–125.6) | 0.561 | 0.485 | below | 57 | 6 | 160.1 | 89.4 | 0.558 | 33/33 | r[1] c[1] |
| fx-syntax | mc (none) | 33 | 394 | 30 | 167.4 ± 20.2 (131.8–219.6) | 27.1 ± 3.2 (23.6–39.9) | 0.162 | 0.179 | below | 57 | 9 | 162.8 | 93.3 | 0.573 | 33/33 | r[1] c[1] |
| reg-clap-builder | 1c (3) | 57 | 944 | 30 | 180.8 ± 23.9 (138.3–250.3) | 123.2 ± 20.1 (98.6–171.5) | 0.681 | 0.713 | overlap | 56 | 7 | 178.5 | 122.2 | 0.685 | 57/57 | r[0] c[0] |
| reg-clap-builder | mc (none) | 57 | 944 | 30 | 193.6 ± 24.2 (154.0–246.9) | 48.1 ± 5.0 (38.3–60.1) | 0.249 | 0.249 | below | 55 | 16 | 190.7 | 164.4 | 0.862 | 57/57 | r[0] c[0] |
| reg-regex-syntax | 1c (3) | 33 | 1657 | 30 | 1378.8 ± 155.8 (1059.7–1696.4) | 363.8 ± 52.7 (291.2–494.7) | 0.264 | 0.275 | below | 423 | 16 | 1360.6 | 356.0 | 0.262 | 33/33 | r[1] c[1] |
| reg-regex-syntax | mc (none) | 33 | 1657 | 30 | 1451.9 ± 137.1 (1147.2–1944.0) | 107.1 ± 8.7 (94.2–126.4) | 0.074 | 0.082 | below | 423 | 38 | 1431.0 | 369.3 | 0.258 | 33/33 | r[1] c[1] |
| reg-tokio | 1c (3) | 49 | 215 | 30 | 41.7 ± 6.3 (35.0–52.9) | 29.7 ± 4.8 (21.9–36.8) | 0.711 | 0.626 | overlap | 44 | 4 | 41.1 | 28.8 | 0.702 | 49/49 | r[1] c[1] |
| reg-tokio | mc (none) | 49 | 215 | 30 | 47.8 ± 10.8 (36.3–93.3) | 10.4 ± 1.3 (8.7–14.5) | 0.217 | 0.240 | below | 44 | 5 | 46.0 | 31.9 | 0.693 | 49/49 | r[1] c[1] |
| reg-winnow | 1c (3) | 61 | 1005 | 30 | 182.2 ± 20.0 (154.9–244.0) | 153.3 ± 17.6 (128.9–207.7) | 0.842 | 0.832 | overlap | 60 | 8 | 179.2 | 150.2 | 0.838 | 61/61 | r[1] c[1] |
| reg-winnow | mc (none) | 61 | 1005 | 30 | 206.4 ± 28.9 (159.7–253.2) | 48.8 ± 4.4 (41.0–57.1) | 0.236 | 0.257 | below | 60 | 14 | 202.0 | 173.5 | 0.859 | 61/61 | r[1] c[1] |

#### Many independent files

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiny-one | 1c (3) | 1 | 0 | 30 | 8.8 ± 1.4 (6.7–12.2) | 1.6 ± 0.2 (1.4–1.9) | 0.181 | 0.206 | below | 35 | 4 | 8.4 | 1.4 | 0.170 | 1/1 | r[0] c[0] |
| tiny-one | mc (none) | 1 | 0 | 30 | 9.9 ± 1.0 (8.3–12.2) | 2.0 ± 0.2 (1.7–2.4) | 0.201 | 0.207 | below | 35 | 4 | 9.7 | 1.8 | 0.187 | 1/1 | r[0] c[0] |
| tiny-test-data | 1c (3) | 289 | 65 | 30 | 61.3 ± 9.0 (53.5–83.6) | 19.3 ± 3.1 (16.7–26.9) | 0.315 | 0.313 | below | 44 | 4 | 60.5 | 19.0 | 0.314 | 288/289 | r[1] c[1] |
| tiny-test-data | mc (none) | 289 | 65 | 30 | 60.6 ± 7.0 (49.7–78.3) | 9.8 ± 1.6 (7.5–16.2) | 0.161 | 0.150 | below | 44 | 5 | 59.1 | 23.0 | 0.389 | 288/289 | r[1] c[1] |

#### One file on stdin (rustfmt vs examples/fmt_stdin)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| stdin-config | 1c (3) | 1 | 167 | 30 | 39.8 ± 6.0 (35.5–53.4) | 19.6 ± 0.9 (18.8–23.5) | 0.493 | 0.529 | below | 44 | 5 | 39.5 | 19.4 | 0.492 | 1/1 | r[0] c[0] |
| stdin-hir-lib | 1c (3) | 1 | 228 | 30 | 95.4 ± 24.4 (68.0–168.6) | 62.4 ± 13.1 (49.2–93.0) | 0.654 | 0.723 | overlap | 47 | 8 | 94.1 | 62.1 | 0.660 | 1/1 | r[0] c[0] |
| stdin-hover-tests | 1c (3) | 1 | 210 | 30 | 30.6 ± 6.1 (22.6–42.3) | 15.4 ± 3.7 (11.3–19.8) | 0.501 | 0.498 | below | 40 | 5 | 29.9 | 15.0 | 0.503 | 1/1 | r[0] c[0] |
| stdin-lints | 1c (3) | 1 | 640 | 30 | 135.2 ± 16.6 (103.9–171.1) | 85.6 ± 11.8 (67.0–108.6) | 0.633 | 0.645 | overlap | 43 | 10 | 133.5 | 83.4 | 0.624 | 1/1 | r[0] c[0] |
| stdin-nodes | 1c (3) | 1 | 322 | 30 | 83.4 ± 10.8 (73.6–114.4) | 58.3 ± 2.4 (55.6–67.4) | 0.699 | 0.755 | below | 54 | 14 | 82.7 | 57.7 | 0.698 | 1/1 | r[0] c[0] |


### `d-inproc-harness`

```text
date_utc=2026-10-14T16:39:51Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=0.96 1.20 1.21
cpu3_stat_start=99738 0 9847 71206 1300 0 155 1643 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=3 warmup=1 rounds=3 core=3 cpus=unpinned mode=check input=formatted groups=e
work=/tmp/chloro-bench
loadavg_end=1.00 1.09 1.16
cpu3_stat_end=119505 0 10002 71215 1300 0 155 1874 0 0
cpu3_steal_ticks=231 cpu3_busy_ticks=19922 cpu3_idle_ticks=9
```

#### In process (examples/conform -i, wall time of the process)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | n | command | wall | RSS | CPU | exit codes |
|---|---|---|---|---|---|---|---|
| inproc-conform | 1c | 9 | chloro | 4762.6 ± 190.5 (4460.6–5044.5) | 50 | 4710.8 | [0] |
| inproc-conform | 1c | 9 | chloro-base | 5135.1 ± 324.4 (4638.3–5715.4) | 50 | 5072.6 | [0] |

#### In process: inproc-bench (examples/bench, best of -n per run)

| run | binary | files | MB | best time | MB/s |
|---|---|---|---|---|---|
| 1 | chloro | 1238 | 13.9 | 1.938 s | 7.17 |
| 2 | chloro-base | 1238 | 13.9 | 2.241 s | 6.20 |
| 3 | chloro-base | 1238 | 13.9 | 2.346 s | 5.93 |
| 4 | chloro | 1238 | 13.9 | 2.239 s | 6.21 |
| 5 | chloro | 1238 | 13.9 | 2.131 s | 6.52 |
| 6 | chloro-base | 1238 | 13.9 | 2.279 s | 6.10 |

- chloro: 3 runs, 6.21–7.17 MB/s
- chloro-base: 3 runs, 5.93–6.20 MB/s


### `noise-hir-ty-1`

```text
date_utc=2026-10-14T18:58:26Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.07 1.05 0.55
cpu3_stat_start=30424 0 960 3686 405 0 29 520 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.65 1.34 0.72
cpu3_stat_end=38124 0 1259 6004 440 0 32 656 0 0
cpu3_steal_ticks=136 cpu3_busy_ticks=7999 cpu3_idle_ticks=2318
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 811.6 ± 64.7 (692.8–976.7) | 482.0 ± 32.3 (394.3–565.8) | 534.8 ± 56.4 (437.8–722.6) | 0.901 | overlap | 0.594 | 0.569 | below | 82 | 9 | 800.2 | 473.4 | 0.592 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 825.6 ± 84.0 (673.3–1055.7) | 130.8 ± 10.8 (113.0–160.6) | 172.5 ± 17.9 (151.2–223.3) | 0.758 | overlap | 0.158 | 0.168 | below | 82 | 23 | 810.3 | 497.1 | 0.613 | 122/122 | r[0] c[0] |


### `noise-hir-ty-2`

```text
date_utc=2026-10-14T19:00:11Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.68 1.35 0.72
cpu3_stat_start=38125 0 1260 6012 440 0 32 656 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.51 1.39 0.81
cpu3_stat_end=46349 0 1542 7957 440 0 37 784 0 0
cpu3_steal_ticks=128 cpu3_busy_ticks=8506 cpu3_idle_ticks=1945
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 825.3 ± 79.4 (690.7–1015.6) | 509.4 ± 58.1 (364.1–607.0) | 563.6 ± 52.9 (457.3–682.3) | 0.904 | overlap | 0.617 | 0.527 | below | 82 | 9 | 812.2 | 501.7 | 0.618 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 813.3 ± 74.9 (692.1–1060.0) | 130.7 ± 10.5 (115.7–150.1) | 184.9 ± 16.8 (156.8–227.5) | 0.707 | below | 0.161 | 0.167 | below | 82 | 23 | 799.6 | 491.9 | 0.615 | 122/122 | r[0] c[0] |


### `noise-hir-ty-3`

```text
date_utc=2026-10-14T19:01:57Z
commit=dc20cd7c652784466484d1196092109feaed6601
worktree_dirty=0
host_cpu=Intel(R) Xeon(R) Processor @ 2.10GHz
nproc=4
kernel=6.18.44-fc-v114
loadavg_start=1.51 1.39 0.81
cpu3_stat_start=46351 0 1543 7965 440 0 37 784 0 0
rustfmt=rustfmt 1.9.0-stable (2d8144b788 2026-07-07) path=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
rustc=rustc 1.97.0 (2d8144b78 2026-07-07)
hyperfine=hyperfine 2.0.0
chloro=/home/user/chloro/target/release/chloro sha256=522fcb6f5f690d00
fmt_stdin=/home/user/chloro/target/release/examples/fmt_stdin sha256=0aecaf61e604db85
bench=/home/user/chloro/target/release/examples/bench sha256=d5fb58db74e7ebfe
baseline=/home/user/wt/base/target/release
runs=15 warmup=2 rounds=2 core=3 cpus=unpinned mode=check input=formatted groups=a,b
work=/tmp/chloro-bench
loadavg_end=1.77 1.52 0.93
cpu3_stat_end=54820 0 1836 9695 440 0 39 927 0 0
cpu3_steal_ticks=143 cpu3_busy_ticks=8762 cpu3_idle_ticks=1730
```

#### Crate level (rustfmt on the crate root, chloro on the same file set)

Wall time in ms: mean ± sd (min–max); RSS: peak resident MiB, max over runs; CPU: mean user+sys ms.

| workload | variant | files | KB | n | rustfmt | chloro | chloro-base | chloro/base mean | ranges vs base | chloro/rustfmt mean | chloro/rustfmt min | ranges | RSS rustfmt | RSS chloro | CPU rustfmt | CPU chloro | CPU chloro/rustfmt | identical outputs | exit codes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir-ty | 1c (3) | 122 | 2876 | 30 | 845.0 ± 84.5 (657.9–1013.6) | 484.8 ± 40.5 (385.2–569.6) | 566.7 ± 73.7 (412.7–708.0) | 0.855 | overlap | 0.574 | 0.586 | below | 82 | 9 | 831.0 | 479.6 | 0.577 | 122/122 | r[0] c[0] |
| fx-hir-ty | mc (none) | 122 | 2876 | 30 | 844.6 ± 81.2 (685.7–1010.9) | 129.5 ± 12.0 (106.8–152.9) | 177.7 ± 12.5 (158.5–207.5) | 0.729 | below | 0.153 | 0.156 | below | 82 | 23 | 826.3 | 489.3 | 0.592 | 122/122 | r[0] c[0] |
