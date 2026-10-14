# EXP-002: CLI build profile, PGO, allocator, scheduler, fixed costs

Experiment E2 of the autoresearch programme ([../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md)),
run under [../protocol.md](../protocol.md). Levers that change the CLI's process-level
numbers without touching `format_source`: the release profile, profile-guided
optimisation, the CLI binary's allocator, the file scheduler, and the process's fixed costs.

Labels as in the journal: **measured**, **read**, **hypothesis**.

## Identity

| field | value |
|---|---|
| branch | `20261014-cli-build` |
| base | `d797292` (head of `claude/chloro-perf-research-swarm-nrscj8`; `884f9c9` plus the programme's journal entry) |
| commits | `0bb8e06` release profile, `d10c1e3` `benchmarks/pgo.sh`, plus this record (the scheduler commit `6c55740` was measured, then dropped by rebase; see E2d) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, LLVM 22.1.6, `cargo 1.97.0`, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, `llvm-tools-preview` for `llvm-profdata` |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.10GHz" (1 thread per core, L2 8 MiB, L3 260 MiB), 15.7 GB, Linux 6.18.44; a different host from the 2.80GHz one in the programme entry, so no figure here compares with figures there |
| hyperfine | 2.0.0 (`cargo install hyperfine`), `-N -i -w 2 -r 5`, three blocks per workload with the commands interleaved block by block, 15 runs per command per workload; peak RSS from hyperfine's `memory_peak_resident` |
| rustfmt binary | `~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` called directly; the `rustup` proxy on `PATH` adds a process exec and is measured separately on the startup workload |

## Variants and their builds

Every variant is built from the same source into its own `--target-dir` under
`/home/user/tgt/`, so the binaries coexist. The build profile is set through
`CARGO_PROFILE_RELEASE_*` for the measurement, and written into the workspace `Cargo.toml`
only for the kept variant.

| variant | source | build | binary size |
|---|---|---|---|
| `base` | `d797292` | `cargo build --release -p chloro --features cli` (no `[profile.release]`: `opt-level = 3`, 16 codegen units, thin local LTO, `panic = "unwind"`) | 2,516,048 B |
| `lto` | `d797292` | `CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` | 1,855,680 B |
| `lto-abort` | `d797292` | `lto` plus `CARGO_PROFILE_RELEASE_PANIC=abort` (headroom only; see E2a) | 1,655,456 B |
| `native` | `d797292` | `lto` plus `RUSTFLAGS="-C target-cpu=native"` (headroom only; not shippable, the release binaries run on any x86-64) | 1,862,064 B |
| `pgo` | `d797292` | `benchmarks/pgo.sh /home/user/tgt cli` on the default profile: instrumented build, training on `chloro --check chloro-core/tests/conformance/fixtures` and `bench -n 1`, `llvm-profdata merge`, `-Cprofile-use` build | 3,234,792 B |
| `mimalloc` | `d797292` plus the `cli-mimalloc` feature | `cargo build --release -p chloro --features cli-mimalloc` (default profile; `mimalloc 0.1.52`, `default-features = false`, so no `secure` mode) | 2,678,256 B |
| `sched` | `d797292` plus the work-queue scheduler | `cargo build --release -p chloro --features cli` (default profile) | 2,538,816 B |
| `combined` | the branch head | `cargo build --release -p chloro --features cli-mimalloc` with the committed `[profile.release]`, then `benchmarks/pgo.sh` on top | see E2 combined |

`opt-level` stays at 3 in every variant: the default release profile already uses 3, so a
variant setting it is the same build and is not measured.

## Workloads

Every workload gives both tools the same set of files. rustfmt is given the crate root and
follows its `mod` declarations; the files it reports under `-v` ("Formatting …") are
passed to chloro as arguments. The sets are recorded under `/home/user/wl/*.files`.

| workload | files | bytes | chloro command | rustfmt command |
|---|---|---|---|---|
| `hir` | 15 | 624,531 | `chloro --check <15 paths>` | `rustfmt --edition 2024 --check hir/src/lib.rs` |
| `hir-ty` | 122 | 2,816,000 | `chloro --check <122 paths>` | `rustfmt --edition 2024 --check hir-ty/src/lib.rs` |
| `ide` | 75 | 1,547,887 | `chloro --check <75 paths>` | `rustfmt --edition 2024 --check ide/src/lib.rs` |
| `ide-db` | 33 | 1,160,427 | `chloro --check <33 paths>` | `rustfmt --edition 2024 --check ide-db/src/lib.rs` |
| `syntax` | 33 of the 61 on disk (rustfmt does not reach `test_data/`) | 394,223 | `chloro --check <33 paths>` | `rustfmt --edition 2024 --check syntax/src/lib.rs` |
| `big-src-lints` | 1 (`ide-db/src/generated/lints.rs`) | 639,689 | `chloro --check big/src-lints.rs` | `rustfmt --edition 2024 --check big/src-lints.rs` |
| `big-ast-nodes` | 1 (`syntax/src/ast/generated/nodes.rs`) | 322,295 | same shape | same shape |
| `big-hir-lib` | 1 + 10 empty stubs (`hir/src/lib.rs` declares ten modules; empty files stand in so rustfmt resolves them) | 227,843 | `chloro --check <11 paths>` | `rustfmt --edition 2024 --check big/hir-lib.rs` |
| `big-src-tests` | 1 (`ide/src/hover/tests.rs`) | 210,028 | same shape | same shape |
| `big-rust-analyzer-config` | 1 + 1 empty stub (`rust-analyzer/src/config.rs`) | 167,117 | `chloro --check <2 paths>` | `rustfmt --edition 2024 --check big/rust-analyzer-config.rs` |
| `tiny` | the 200 smallest fixtures (0 to 30 bytes each) copied into one flat directory | 3,317 | `chloro --check tiny/` (directory discovery) | `rustfmt --edition 2024 --check tiny/*.rs` |
| `one` | one 12-byte file `fn main(){}` | 12 | `chloro --check one.rs` | `rustfmt --edition 2024 --check one.rs` |

The five big files are copies under `/home/user/wl/big/`, renamed so that a file called
`lib.rs` is not treated as a crate root by rustfmt's module resolution for the other
copies. For `hir/src/lib.rs` and `rust-analyzer/src/config.rs`, rustfmt refuses to format a
file whose `mod` declarations do not resolve, so empty stub files were created for the
modules they declare; both tools get the stubs.

Harness: `/home/user/wl/sweep.sh` runs, per workload, three `hyperfine -N -i -w 2 -r 5`
invocations with every variant's command in each (so variants alternate block by block),
exports JSON, and `/home/user/wl/agg.py` pools the 15 runs per command. Pinned runs
prefix every command with `taskset -c 0`. Each sweep ran with no other process of this
session running (builds and gates finished before it started).

## Gates common to every codegen variant (measured)

Each variant that changes code generation was checked for output equality and conformance
with its own build of the `bench` and `conform` examples (same `--target-dir`, same flags;
for `mimalloc`, a scratch copy of the two examples with the same `#[global_allocator]`
line, built in a worktree and not committed):

| variant | `bench --check /home/user/golden/golden.tsv` (golden recorded by the `base` build) | `conform -i` |
|---|---|---|
| `lto` | 0 of 1238 outputs changed | 1219/1220 identical, 0 not idempotent |
| `lto-abort` | 0 of 1238 outputs changed | 1219/1220 identical, 0 not idempotent |
| `native` | 0 of 1238 outputs changed | 1219/1220 identical, 0 not idempotent |
| `pgo` | 0 of 1238 outputs changed | 1219/1220 identical, 0 not idempotent |
| `mimalloc` | 0 of 1238 outputs changed | 1219/1220 identical, 0 not idempotent |

For the scheduler, `chloro --check -v` over the whole fixture tree (1238 files) was run
with the `base` and `sched` binaries, the per-file lines sorted, and the two reports
diffed: identical, 1252 lines each (1238 "Processing" lines, 702 changed, 536 unchanged).

## Sweep 1: every variant on its own (measured)

Cells are min / mean (σ) wall time in ms over 15 runs, three blocks of five with the commands interleaved block by block; the peak RSS tables give the maximum over the 15 runs. The ratio tables further down are computed from these numbers.

### Unpinned (all four cores available)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 77.7 / 100.5 (18.7) | 183.0 / 214.3 (28.5) | 88.7 / 104.4 (8.6) | 99.5 / 124.6 (16.2) | 35.1 / 45.3 (5.6) | 69.8 / 85.8 (10.4) | 55.5 / 75.8 (9.7) | 57.6 / 73.0 (11.7) | 15.2 / 22.0 (3.9) | 25.5 / 30.0 (3.3) | 4.9 / 6.4 (0.7) | 1.9 / 2.5 (0.5) |
| lto | 68.2 / 82.4 (8.7) | 184.8 / 214.0 (13.8) | 83.9 / 98.9 (6.9) | 91.4 / 120.5 (17.9) | 33.3 / 41.9 (5.8) | 72.8 / 87.7 (12.6) | 60.2 / 79.1 (16.6) | 45.1 / 63.9 (10.3) | 13.5 / 19.5 (2.4) | 22.6 / 27.7 (3.4) | 4.2 / 6.1 (1.2) | 1.8 / 2.5 (0.4) |
| lto-abort | 61.4 / 82.0 (12.9) | 156.0 / 198.9 (25.0) | 81.6 / 96.1 (12.9) | 94.2 / 122.3 (16.8) | 30.3 / 39.0 (5.5) | 75.4 / 88.0 (11.4) | 50.5 / 67.0 (10.4) | 51.9 / 64.6 (8.8) | 15.2 / 19.3 (4.0) | 22.0 / 26.2 (2.7) | 4.3 / 5.9 (1.0) | 1.6 / 2.3 (0.4) |
| native | 71.1 / 88.6 (13.1) | 178.7 / 200.7 (23.4) | 80.0 / 95.7 (9.5) | 104.6 / 121.7 (15.6) | 39.8 / 46.0 (4.2) | 61.0 / 83.3 (12.7) | 52.9 / 70.0 (10.4) | 47.1 / 63.3 (10.6) | 13.3 / 17.4 (2.8) | 22.9 / 28.0 (3.2) | 4.1 / 5.6 (1.4) | 1.7 / 2.1 (0.3) |
| pgo | 63.8 / 81.0 (8.5) | 157.1 / 187.9 (17.8) | 75.2 / 93.2 (9.1) | 73.7 / 95.8 (11.1) | 33.7 / 41.9 (5.7) | 51.4 / 66.8 (11.9) | 61.6 / 72.7 (12.1) | 55.1 / 62.3 (6.3) | 12.1 / 15.8 (2.6) | 19.1 / 26.5 (5.2) | 4.3 / 5.7 (1.1) | 2.0 / 2.4 (0.4) |
| mimalloc | 69.9 / 84.2 (13.0) | 155.4 / 187.8 (22.2) | 82.5 / 103.4 (12.8) | 86.9 / 120.8 (16.1) | 33.5 / 45.2 (9.1) | 66.3 / 101.0 (42.5) | 59.5 / 73.5 (9.7) | 57.9 / 81.8 (33.5) | 15.2 / 31.1 (21.2) | 23.5 / 35.9 (10.0) | 6.7 / 9.3 (2.5) | 2.4 / 3.1 (0.4) |
| sched | 56.3 / 76.6 (11.1) | 137.5 / 160.8 (16.8) | 62.1 / 78.3 (9.5) | 66.4 / 99.3 (46.9) | 29.1 / 31.8 (1.8) | 80.4 / 96.8 (18.9) | 58.0 / 79.7 (14.8) | 58.5 / 73.4 (16.8) | 15.1 / 19.8 (2.9) | 19.9 / 27.8 (6.4) | 3.3 / 4.8 (1.1) | 2.0 / 2.4 (0.3) |
| rustfmt | 430.9 / 565.6 (49.8) | 1064.0 / 1237.1 (152.4) | 704.2 / 843.4 (77.8) | 289.5 / 379.4 (45.9) | 152.9 / 168.4 (11.2) | 120.8 / 150.8 (26.0) | 655.9 / 832.0 (102.5) | 241.6 / 343.0 (52.9) | 221.2 / 263.6 (22.0) | 82.1 / 96.3 (8.4) | 9.1 / 12.2 (1.6) | 9.1 / 10.3 (0.9) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 23.5 | 34.8 | 21.6 | 20.2 | 12.4 | 10.4 | 13.8 | 9.3 | 5.7 | 5.9 | 4.0 | 3.6 |
| lto | 22.6 | 34.7 | 20.9 | 19.3 | 12.1 | 10.3 | 13.7 | 8.9 | 5.5 | 5.7 | 3.7 | 3.4 |
| lto-abort | 22.4 | 35.4 | 20.9 | 19.1 | 11.9 | 10.1 | 13.6 | 8.9 | 5.4 | 5.6 | 3.7 | 3.4 |
| native | 22.6 | 34.7 | 21.0 | 20.3 | 12.0 | 10.3 | 13.7 | 9.1 | 5.5 | 5.7 | 3.8 | 3.4 |
| pgo | 23.7 | 35.6 | 21.7 | 20.2 | 13.0 | 11.1 | 14.5 | 9.8 | 6.4 | 6.5 | 4.5 | 4.3 |
| mimalloc | 142.3 | 210.6 | 172.4 | 140.2 | 106.2 | 51.9 | 45.9 | 51.9 | 23.9 | 29.9 | 14.0 | 7.8 |
| sched | 16.5 | 20.1 | 12.4 | 15.6 | 9.1 | 10.5 | 13.8 | 9.4 | 5.8 | 6.0 | 3.8 | 3.7 |
| rustfmt | 217.3 | 116.1 | 249.3 | 65.6 | 56.5 | 43.3 | 544.5 | 208.5 | 218.6 | 82.2 | 38.8 | 34.1 |

### Pinned (`taskset -c 0`, one core)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 138.9 / 183.9 (34.0) | 414.6 / 565.7 (91.8) | 259.7 / 297.1 (30.5) | 155.6 / 232.1 (36.6) | 83.1 / 99.3 (10.3) | 68.5 / 86.8 (10.9) | 66.9 / 80.6 (10.4) | 51.9 / 64.0 (8.1) | 16.1 / 19.5 (1.9) | 24.1 / 30.6 (4.8) | 5.0 / 7.4 (1.7) | 2.2 / 3.6 (0.7) |
| lto | 126.8 / 171.1 (30.0) | 462.7 / 537.9 (39.6) | 204.6 / 264.7 (42.3) | 141.6 / 193.1 (37.4) | 71.0 / 88.7 (13.3) | 65.2 / 80.0 (9.1) | 62.4 / 84.2 (11.7) | 51.5 / 62.7 (9.4) | 13.4 / 17.8 (3.9) | 17.7 / 22.7 (3.3) | 5.4 / 7.4 (2.3) | 2.9 / 3.4 (0.3) |
| lto-abort | 127.3 / 184.1 (55.1) | 387.2 / 553.1 (128.2) | 231.9 / 293.4 (42.2) | 140.5 / 181.6 (22.8) | 74.6 / 113.8 (28.4) | 57.9 / 74.1 (9.2) | 51.3 / 73.3 (12.8) | 45.1 / 57.4 (9.0) | 12.5 / 16.3 (3.2) | 18.9 / 27.8 (8.8) | 6.0 / 7.4 (0.8) | 2.9 / 4.2 (1.2) |
| native | 152.6 / 194.7 (27.6) | 448.9 / 573.2 (76.8) | 195.5 / 254.0 (41.6) | 158.3 / 193.7 (20.5) | 75.0 / 99.2 (18.7) | 60.3 / 81.2 (14.4) | 48.4 / 66.0 (11.5) | 47.3 / 56.7 (6.3) | 15.0 / 17.2 (2.3) | 18.5 / 23.5 (5.4) | 6.6 / 8.2 (0.9) | 2.8 / 3.5 (0.5) |
| pgo | 117.1 / 194.0 (48.4) | 437.9 / 599.5 (83.3) | 241.0 / 288.5 (46.2) | 129.0 / 170.3 (23.4) | 72.0 / 89.6 (11.4) | 45.9 / 60.1 (11.7) | 49.8 / 59.5 (6.5) | 42.2 / 54.6 (5.8) | 11.1 / 14.8 (2.6) | 17.4 / 22.9 (5.7) | 5.2 / 7.5 (1.5) | 3.0 / 3.7 (0.7) |
| mimalloc | 153.0 / 208.3 (69.7) | 379.4 / 578.4 (118.4) | 252.8 / 305.6 (40.8) | 160.9 / 216.9 (42.1) | 86.4 / 117.2 (18.5) | 68.3 / 84.5 (10.2) | 50.7 / 72.7 (16.4) | 44.5 / 63.3 (15.6) | 15.3 / 18.7 (1.7) | 24.5 / 30.5 (5.1) | 7.3 / 8.9 (1.5) | 3.4 / 4.4 (1.1) |
| sched | 134.8 / 169.9 (26.3) | 467.3 / 596.7 (66.1) | 220.2 / 268.7 (36.7) | 170.3 / 197.6 (18.1) | 93.1 / 111.4 (13.4) | 77.1 / 97.1 (14.0) | 59.1 / 79.0 (11.4) | 45.4 / 67.5 (13.9) | 13.0 / 17.0 (2.8) | 20.2 / 28.0 (6.1) | 5.9 / 8.1 (1.8) | 2.8 / 4.1 (1.2) |
| rustfmt | 516.3 / 588.0 (43.8) | 1028.1 / 1299.5 (147.6) | 578.4 / 746.9 (88.4) | 290.5 / 350.2 (52.0) | 129.4 / 178.5 (34.1) | 125.6 / 135.0 (6.8) | 626.4 / 724.9 (89.0) | 245.3 / 307.7 (41.1) | 218.9 / 278.0 (30.5) | 81.0 / 102.0 (13.3) | 10.1 / 11.7 (0.9) | 8.6 / 11.5 (1.9) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 15.8 | 17.2 | 12.2 | 15.2 | 9.0 | 10.5 | 13.8 | 9.3 | 5.7 | 6.0 | 3.9 | 3.7 |
| lto | 15.5 | 17.0 | 11.9 | 15.0 | 8.8 | 10.2 | 13.7 | 9.0 | 5.5 | 5.8 | 3.6 | 3.4 |
| lto-abort | 15.2 | 16.8 | 11.9 | 14.9 | 8.7 | 10.2 | 13.7 | 9.0 | 5.4 | 5.7 | 3.5 | 3.4 |
| native | 15.5 | 17.1 | 12.0 | 14.9 | 8.8 | 10.3 | 13.6 | 9.2 | 5.5 | 5.8 | 3.6 | 3.5 |
| pgo | 16.3 | 17.8 | 12.9 | 15.9 | 9.7 | 11.1 | 14.5 | 9.9 | 6.4 | 6.7 | 4.5 | 4.1 |
| mimalloc | 108.0 | 100.1 | 98.0 | 104.0 | 86.0 | 51.9 | 45.9 | 46.0 | 23.9 | 29.9 | 9.9 | 7.8 |
| sched | 8.8 | 8.9 | 6.7 | 10.6 | 6.2 | 10.6 | 13.8 | 8.8 | 5.8 | 6.0 | 3.8 | 3.7 |
| rustfmt | 217.3 | 116.1 | 249.3 | 65.6 | 56.6 | 43.2 | 544.6 | 208.5 | 218.6 | 82.2 | 38.9 | 34.2 |

### Noise on this host (measured)

Within one binary and one workload, the 15 runs spread widely: pinned `base` on `hir-ty`
ranges from 414.6 to 735 ms (σ 91.8 ms, 16% of the mean), and `bench -n 5` (best of five
rounds, pinned to one core) on the unchanged `base` build gives 2.08–2.55 s across twelve
runs in one window. Steal time in `/proc/stat` is 1.8% of user time over the session. The
tables therefore report minima next to means, and a lever is kept only where its minima
and its means move the same way on most workloads and the in-process pairs agree.

Ratio of each variant to `base`, minimum / mean, from the tables above (values below 1
are faster than `base`; the last column counts workloads whose minimum is below `base`'s):

| variant (pinned) | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lto | 0.91/0.93 | 1.12/0.95 | 0.79/0.89 | 0.91/0.83 | 0.86/0.89 | 0.95/0.92 | 0.93/1.05 | 0.99/0.98 | 0.83/0.91 | 0.73/0.74 | 1.08/1.01 | 1.36/0.95 | 9/12 |
| lto-abort | 0.92/1.00 | 0.93/0.98 | 0.89/0.99 | 0.90/0.78 | 0.90/1.15 | 0.85/0.85 | 0.77/0.91 | 0.87/0.90 | 0.78/0.84 | 0.78/0.91 | 1.21/1.00 | 1.31/1.15 | 10/12 |
| native | 1.10/1.06 | 1.08/1.01 | 0.75/0.85 | 1.02/0.83 | 0.90/1.00 | 0.88/0.94 | 0.72/0.82 | 0.91/0.89 | 0.93/0.88 | 0.77/0.77 | 1.33/1.10 | 1.29/0.98 | 7/12 |
| pgo | 0.84/1.05 | 1.06/1.06 | 0.93/0.97 | 0.83/0.73 | 0.87/0.90 | 0.67/0.69 | 0.74/0.74 | 0.81/0.85 | 0.69/0.76 | 0.72/0.75 | 1.04/1.01 | 1.37/1.01 | 9/12 |
| mimalloc | 1.10/1.13 | 0.91/1.02 | 0.97/1.03 | 1.03/0.93 | 1.04/1.18 | 1.00/0.97 | 0.76/0.90 | 0.86/0.99 | 0.95/0.96 | 1.02/1.00 | 1.46/1.21 | 1.59/1.21 | 6/12 |
| sched | 0.97/0.92 | 1.13/1.05 | 0.85/0.90 | 1.09/0.85 | 1.12/1.12 | 1.13/1.12 | 0.88/0.98 | 0.88/1.06 | 0.81/0.87 | 0.84/0.91 | 1.18/1.09 | 1.31/1.12 | 6/12 |

| variant (unpinned) | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lto | 0.88/0.82 | 1.01/1.00 | 0.95/0.95 | 0.92/0.97 | 0.95/0.92 | 1.04/1.02 | 1.08/1.04 | 0.78/0.88 | 0.89/0.88 | 0.89/0.92 | 0.85/0.95 | 0.94/1.00 | 9/12 |
| lto-abort | 0.79/0.82 | 0.85/0.93 | 0.92/0.92 | 0.95/0.98 | 0.86/0.86 | 1.08/1.03 | 0.91/0.88 | 0.90/0.89 | 1.00/0.88 | 0.86/0.87 | 0.87/0.92 | 0.87/0.90 | 10/12 |
| native | 0.92/0.88 | 0.98/0.94 | 0.90/0.92 | 1.05/0.98 | 1.13/1.01 | 0.87/0.97 | 0.95/0.92 | 0.82/0.87 | 0.87/0.79 | 0.90/0.93 | 0.84/0.88 | 0.91/0.82 | 10/12 |
| pgo | 0.82/0.81 | 0.86/0.88 | 0.85/0.89 | 0.74/0.77 | 0.96/0.92 | 0.74/0.78 | 1.11/0.96 | 0.96/0.85 | 0.80/0.72 | 0.75/0.88 | 0.88/0.89 | 1.03/0.93 | 10/12 |
| mimalloc | 0.90/0.84 | 0.85/0.88 | 0.93/0.99 | 0.87/0.97 | 0.96/1.00 | 0.95/1.18 | 1.07/0.97 | 1.00/1.12 | 1.00/1.41 | 0.92/1.20 | 1.36/1.45 | 1.28/1.23 | 7/12 |
| sched | 0.72/0.76 | 0.75/0.75 | 0.70/0.75 | 0.67/0.80 | 0.83/0.70 | 1.15/1.13 | 1.04/1.05 | 1.02/1.01 | 1.00/0.90 | 0.78/0.93 | 0.68/0.75 | 1.05/0.97 | 8/12 |

### In-process `bench -n 5` pairs (measured, pinned to core 2, three interleaved pairs each)

Best of five rounds over the 1238 fixtures (13.9 MB); each pair runs `base` then the
variant.

| pair | base | lto | base | pgo | base | mimalloc (scratch bench) | base | native |
|---|---|---|---|---|---|---|---|---|
| 1 | 2.509 s (5.54 MB/s) | 2.044 s (6.80) | 2.404 s (5.78) | 2.004 s (6.94) | 2.278 s (6.10) | 2.418 s (5.75) | 2.425 s (5.73) | 2.284 s (6.09) |
| 2 | 2.122 s (6.55) | 1.948 s (7.14) | 2.361 s (5.89) | 1.822 s (7.63) | 2.076 s (6.70) | 1.771 s (7.85) | 2.450 s (5.67) | 2.328 s (5.97) |
| 3 | 2.437 s (5.71) | 2.205 s (6.30) | 2.184 s (6.36) | 2.170 s (6.41) | 2.546 s (5.46) | 2.049 s (6.79) | 2.483 s (5.60) | 2.086 s (6.67) |
| within-pair change | | −18.5%, −8.2%, −9.5% | | −16.6%, −22.8%, −0.6% | | +6.1%, −14.7%, −19.5% | | −5.8%, −5.0%, −16.0% |

The `base` range over the twelve runs (2.076–2.546 s) overlaps every variant's range
(`lto` 1.948–2.205, `pgo` 1.822–2.170, `mimalloc` 1.771–2.418, `native` 2.086–2.328), so
by the protocol's non-overlap rule none of the four is a single-threaded speedup on its
own; within pairs, `lto`, `pgo` and `native` are faster than their `base` three times
out of three and `mimalloc` two times out of three.

## E2a: build profile

**Hypothesis.** `lto = "fat"` with `codegen-units = 1` lets LLVM inline across the
`chloro-core` / `rowan` / `smol_str` boundaries that the default profile's thin local LTO
and 16 codegen units keep separate, lowering in-process throughput time and process wall
time by a measurable amount. Refuted if the interleaved `bench` pairs and the hyperfine
minima move in different directions or stay within the host's spread.

**Measured.** `lto` against `base`: `bench` pairs −18.5%, −8.2%, −9.5% (three of three
faster); hyperfine minima below `base` on 9 of 12 workloads pinned and 9 of 12 unpinned,
with the pinned minima at 0.73–0.99x on the crate and big-file workloads and the two
sub-10 ms workloads (`tiny`, `one`) inside their own spread (σ 0.3–2.3 ms). Binary size
2,516,048 → 1,855,680 B (−26%). Peak RSS unchanged within 0.3 MiB. Release build time of
the CLI from scratch on this host: 20.9 s (`base`) → 46.6 s (`lto`, one codegen unit).

`opt-level` is already 3 in the default release profile, so no variant changes it.

`panic = "abort"` (`lto-abort`) is measured as headroom only: `bench` is not built with it
(examples share the profile, and the equality gates pass), hyperfine minima are below
`base` on 10 of 12 workloads both pinned and unpinned, binary size 1,655,456 B. It is
not kept because it changes the CLI's behaviour on a formatter panic (**read**,
`chloro/src/cli/orchestrate.rs`): the worker threads run under `std::thread::scope` and
each handle is joined with `unwrap_or_default()`, so today a panic inside `format_source`
on one file prints the panic message, drops that thread's results, and the process goes on
to report the other files and exit with their status; with `panic = "abort"` the same
panic ends the process with SIGABRT before any report. There is no `catch_unwind` in the
workspace, so nothing recovers from a panic deliberately, but the join-and-continue path
is relied on for the report. Changing that is a CLI design decision, not a build setting.

`-C target-cpu=native` (`native`, on top of `lto`) is measured as headroom only: the
release binaries from `binaries.yml` run on any x86-64 and the flag would tie them to
this host's Xeon. Its minima are below `base` on 7 of 12 pinned and 10 of 12 unpinned
workloads and below `lto` on 6 of 12 pinned; `bench` pairs −5.8%, −5.0%, −16.0%. No
headroom beyond `lto` shows outside the host's spread.

**Decision.** Kept: `[profile.release] lto = "fat", codegen-units = 1` in the workspace
`Cargo.toml` (applies to the CLI binary and to every release build in the workspace,
examples included). `panic = "abort"` and `target-cpu=native` not kept, for the reasons
above.

## E2b: profile-guided optimisation

**Hypothesis.** A profile from the conformance fixtures lets LLVM lay out the formatter's
hot paths (the rewrite dispatch in `expr`, the comment scanner, `rowan` node creation)
by measured branch and call frequencies, lowering wall time beyond what LTO alone gives.
Refuted if the PGO binary's minima are not below `base`'s on most workloads.

**Commands** (`benchmarks/pgo.sh`, run here from a worktree at `d797292` with the
default profile, `benchmarks/pgo.sh /home/user/tgt cli`):

```text
RUSTFLAGS="-Cprofile-generate=$gen/profiles" cargo build --release -p chloro --features cli --target-dir $gen
RUSTFLAGS="-Cprofile-generate=$gen/profiles" cargo build --release -p chloro-core --example bench --target-dir $gen
$gen/release/chloro --check chloro-core/tests/conformance/fixtures
$gen/release/examples/bench -n 1
llvm-profdata merge -o $gen/profiles/merged.profdata $gen/profiles/*.profraw
RUSTFLAGS="-Cprofile-use=$gen/profiles/merged.profdata -Cllvm-args=-pgo-warn-missing-function" cargo build --release -p chloro --features cli --target-dir $root/pgo
```

`llvm-profdata` comes from the `llvm-tools-preview` rustup component
(`$(rustc --print sysroot)/lib/rustlib/<host>/bin/llvm-profdata`). The training run
writes 12 `.profraw` files (one per process and module signature). The instrumented
binaries print `LLVM Profile Warning: Unable to track new values: Running out of static
counters` ten times during training: value profiling of indirect calls saturates its
per-site counters; the warning names `-mllvm -vp-counters-per-site=<n>` as the knob,
which is not set here.

**Measured.** `pgo` (default profile plus PGO) against `base`: `bench` pairs −16.6%,
−22.8%, −0.6%; hyperfine minima below `base` on 9 of 12 pinned and 10 of 12 unpinned
workloads, with the largest moves on the single big files (`lints` 0.67/0.69 pinned,
`nodes` 0.74/0.74, `tests` 0.69/0.76, `config` 0.72/0.75, min/mean ratios). Binary size
3,234,792 B (+29% over `base`, from the profile-driven inlining and layout at 16
codegen units). Peak RSS +0.6–1.0 MiB. Equality gates pass (table above).

**Decision.** Kept as a reproducible script (`benchmarks/pgo.sh`); the combined section
below measures it on top of the committed profile and the scheduler. The release workflow
is not edited; the proposed change is in "Proposed `binaries.yml` change" below.

## E2c: allocator for the CLI binary

**Hypothesis.** `mimalloc` as the CLI binary's `#[global_allocator]` (the library never
sets one) lowers the 22% of instructions that the allocator family takes on the sample
([../profile-baseline.md](../profile-baseline.md)), and the per-thread glibc arena cost
seen in E2e. Refuted if wall time does not move outside the spread or if peak RSS grows
by more than the time it saves is worth.

**Change measured** (in a worktree at `d797292`, not committed):

```toml
# chloro/Cargo.toml
[features]
cli-mimalloc = ["cli", "dep:mimalloc"]
[dependencies]
mimalloc = { default-features = false, optional = true, version = "0.1" }   # resolves to 0.1.52
```

```rust
// chloro/src/cli.rs
#[cfg(feature = "cli-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

built with `cargo build --release -p chloro --features cli-mimalloc`. For the in-process
pairs and the equality gates, a scratch copy of `bench.rs` and `conform.rs` carried the
same two lines with `mimalloc` as a dev-dependency of `chloro-core`.

**Measured.** `bench` pairs +6.1%, −14.7%, −19.5% (two of three faster, spread 1.77–2.42
s against `base` 2.08–2.55 s). Hyperfine: minima below `base` on 6 of 12 pinned and 7 of
12 unpinned workloads; means above `base` on 8 of 12 unpinned. Peak RSS: `hir` 23.5 →
142.3 MiB unpinned (4 threads) and 15.8 → 108.0 MiB pinned; `hir-ty` 34.8 → 210.6 MiB;
`ide` 21.6 → 172.4 MiB; the single big files 2.5–4.2x; `one` 3.6 → 7.8 MiB. With
`MIMALLOC_VERBOSE=1` the allocator reports `reserved 1048576 KiB memory` at process init.
With `MIMALLOC_ARENA_EAGER_COMMIT=0 MIMALLOC_EAGER_COMMIT=0` on `hir` (10 runs each,
interleaved): wall time 92.3 ± 11.7 ms against `base` 90.7 ± 15.9 ms and plain
`mimalloc` 82.6 ± 13.4 ms, peak RSS 39.7 MiB (1.9x `base`); with `MIMALLOC_PURGE_DELAY=0`:
120.0 ± 15.7 ms, 56.5 MiB. So the eager commit of mimalloc's arenas is where the resident
memory goes, and switching it off takes the time gain with it.

**Decision.** Rejected, not committed: no wall-time gain outside the host's spread in
either the CLI or the in-process pairs, 4–10x peak RSS at the defaults, and 2x RSS with
no gain when the commit policy is relaxed. The feature is not added to the `chloro`
crate; the two snippets above reproduce the measurement.

### Instruction counts (measured, callgrind, deterministic)

`benchmarks/profile.sh` from the integration branch (the 49-file sample: every 25th
file of the bytewise-sorted fixture list, 650,213 bytes; `bench -n 1 --root <sample>`
under `valgrind --tool=callgrind`, fixed environment), run against each variant's own
`bench` build:

| bench build | Ir | against `base` |
|---|---|---|
| `base` (default profile) | 587,988,181 | — |
| `lto` (fat LTO, one codegen unit) | 554,256,067 | −5.7% |
| `pgo` (default profile plus PGO) | 492,983,621 | −16.2% |
| `mimalloc` (default profile, scratch bench with `mimalloc` as global allocator) | 515,309,648 | −12.4% |
| `lto-pgo` (the branch's profile plus PGO) | 448,458,885 | −23.7% |

The baseline profile ([../profile-baseline.md](../profile-baseline.md)) puts glibc's
`malloc`/`free`/`realloc` at 115.7 M Ir (19.7%) of the `base` count on this sample; the
`mimalloc` build removes 72.7 M of those, so the allocator's own cost under mimalloc is
about 43 M Ir (7.3% of `base`) for the same 628,959 blocks. The wall-time tables above
do not show that 12.4% fewer instructions: the process touches 4–10x the pages (eager
arena commit), and page faults are not instructions. Peak RSS of `bench -n 1` itself
(hyperfine, 3 runs): on the 49-file sample 6.7 MiB (`base`) against 37.5 MiB (`mimalloc`,
5.6x); on all 1238 fixtures 35.2 against 76.8 MiB (2.2x).

## E2d: scheduler

**Hypothesis.** A shared work queue over the files sorted by size descending, one worker
per core, lets a crate's wall time approach total work divided by cores instead of the
time of the chunk holding its largest files, and removes the `4 × cores` thread spawns for
small inputs. Refuted if the unpinned wall time of multi-file crates does not fall.

**Change** (`chloro/src/cli/orchestrate.rs`): `format_all` reads each file's size with
`std::fs::metadata`, sorts an index array by size descending (ties by position), spawns
`min(available_parallelism, files)` scoped threads that take the next index from an
`AtomicUsize`, collects `(index, result)` pairs and sorts them by index at the end, so the
result order equals the input order whatever the thread timing. `worker_threads` is
public for the verbose banner in `cli.rs`, which now prints the thread count and "largest
file first" instead of the chunk arithmetic. Output and exit status are unchanged (report
diff above).

**Measured.** Unpinned, `sched` against `base`, min/mean ratios: `hir` 0.72/0.76 (15
files, one of 228 KB), `hir-ty` 0.75/0.75 (122 files), `ide` 0.70/0.75, `ide-db`
0.67/0.80, `syntax` 0.83/0.70, `tiny` 0.68/0.75 (200 files: 4 threads instead of 16).
Single big files unpinned: `nodes` 1.04/1.05, `hir-lib` 1.02/1.01, `tests` 1.00/0.90,
`config` 0.78/0.93, `lints` 1.15/1.13 (one file is one thread under both schedulers;
`lints` is re-measured in sweep 2 below). Pinned to one core, the queue has nothing to
balance: 1.13/1.05 on `hir-ty`, 0.85/0.90 on `ide`, 1.12/1.12 on `syntax`, the rest
inside the spread. Peak RSS unpinned: `hir` 23.5 → 16.5 MiB, `hir-ty` 34.8 → 20.1,
`ide` 21.6 → 12.4, `ide-db` 20.2 → 15.6 (fewer concurrent threads, so fewer live trees
and fewer glibc arenas).

On `hir-ty`, 122 files summing 2.82 MB, the unpinned minimum goes from 183.0 ms to
137.5 ms; the pinned minimum (all work on one core) is 414.6 ms for `base`, so the queue
reaches 3.0x of single-core on four cores where the static chunks reached 2.3x.

**Decision.** Numbers kept as a cross-check; code dropped from this branch. The
orchestrator reports the same design (atomic index over files sorted by size descending,
one worker per core, plus a `--threads` flag) already implemented and measured on
`20261014-parallel-items` (E1) at 1.24–1.49x on the fixture crates, and the integration
takes E1's version, so the commit that carried this one (`6c55740`) was removed from the
branch by rebase to avoid two versions of `orchestrate.rs`. The figures above and the
`sched`, `lto-sched` and `combined` rows of sweep 2 are from that removed commit; the
"kept combination" section below measures what the branch carries (profile + PGO).

## E2e: fixed costs

**Measured, timing** (`one` workload, 12 bytes, unpinned, min / mean ms): `base` 1.9 / 2.5,
`combined` 1.8 / 2.2, rustfmt 9.1 / 10.3 (toolchain binary), rustfmt through the rustup
proxy 41.6 / 53.3. `tiny` (200 files of 0–30 bytes): `base` 4.9 / 6.4, `sched` 3.3 /
4.8, `combined` 3.4 / 4.3, rustfmt 9.1 / 12.2. Peak RSS on `one`: chloro 3.6 MiB,
rustfmt 34.2 MiB. The process cost of chloro on an empty input is about 2 ms, of which
the scheduler change removes nothing measurable on one file and about 1.5 ms on 200
files (12 fewer thread spawns).

**Measured, system calls** (`strace -f -c`):

`chloro --check one.rs` (12 bytes) makes 120 system calls (`strace -f -c`, 1 error):
dynamic loading of `libgcc_s` and `libc` (16 `mmap`, 8 `openat` in all), the Rust
runtime's stack-guard and signal setup (`/proc/self/maps`, `sigaltstack`), two calls of
`available_parallelism` (`sched_getaffinity` three times, `/proc/self/cgroup`,
`cpu.cfs_quota_us`, `cpu.cfs_period_us`), one `clone3` for the single worker thread (one
file gives one chunk under the base scheduler and one thread under the work queue), the
worker's 128 MiB `mmap`/`munmap` pair for its glibc malloc arena, one `openat` + `statx` +
two `read` for the file, and four `write` for the report. `rustfmt --edition 2024 --check
one.rs` makes 628 system calls (78 errors): seven shared libraries, an `openssl.cnf` probe,
four `clone3` for its thread pool, and the rustup proxy on `PATH` adds a second `execve`
with reads of `~/.rustup/settings.toml` and `rust-toolchain{,.toml}` probes up the
directory tree; the direct toolchain binary is used for every rustfmt figure here.

`chloro --check tiny/` (200 files through directory discovery) makes 2237 system calls:
209 `openat`, 406 `statx` (one `statx` per entry for `is_dir`, one per file for
`canonicalize`, one per file for the read), 16 `clone3` (the base scheduler's `4 ×
threads` chunks), 9 `futex`. rustfmt on the same 200 files makes 1315.

Nothing here is both obvious and cheap: the per-file calls are one open, one stat and one
read; the thread arena is glibc's per-thread behaviour (mimalloc changes it, see E2c); the
process cost is the dynamic loader and the Rust runtime. The scheduler change removes the
extra `clone3` calls for small inputs (one thread per file at most, one per core at most).

## Combined: profile + scheduler + PGO (measured after the individual variants)

Two builds from the branch head (profile and scheduler committed): `lto-sched` is
`cargo build --release -p chloro --features cli` (1,875,192 B), and `combined` is
`benchmarks/pgo.sh /home/user/tgt/combined cli` on the same tree (2,859,184 B; PGO on
top of fat LTO in one codegen unit). Gates: `bench --check` 0 of 1238 changed and
`conform -i` 1219/1220, 0 not idempotent, for both, with examples built under the same
flags; the sorted `--check -v` report of `combined` over the fixture tree is identical
to `base`'s.

### Sweep 2, unpinned (min / mean (σ) ms, 15 runs per command)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 75.3 / 95.6 (14.2) | 165.4 / 210.6 (20.6) | 80.2 / 106.6 (13.4) | 90.0 / 124.0 (19.5) | 37.0 / 48.7 (7.4) | 80.1 / 94.3 (13.2) | 52.3 / 71.9 (10.3) | 45.6 / 61.8 (8.5) | 13.3 / 17.1 (2.4) | 19.1 / 25.0 (3.5) | 5.0 / 5.9 (0.6) | 2.0 / 2.4 (0.3) |
| sched | 46.2 / 65.2 (11.1) | 134.7 / 156.2 (10.5) | 75.5 / 82.3 (4.8) | 75.4 / 96.9 (15.4) | 25.5 / 30.3 (3.6) | 81.9 / 100.4 (13.6) | 57.5 / 73.4 (12.1) | 47.2 / 60.6 (7.2) | 16.9 / 20.7 (3.2) | 21.6 / 25.8 (1.8) | 3.5 / 4.6 (0.6) | 2.0 / 2.4 (0.3) |
| lto-sched | 48.6 / 63.8 (10.8) | 117.4 / 147.2 (13.3) | 64.4 / 80.8 (10.7) | 60.7 / 87.8 (15.2) | 24.1 / 27.3 (2.6) | 62.1 / 89.0 (17.5) | 52.6 / 81.2 (15.8) | 41.9 / 53.7 (9.5) | 14.4 / 16.9 (1.5) | 17.5 / 22.6 (3.5) | 3.2 / 4.0 (0.6) | 1.8 / 2.3 (0.3) |
| combined | 43.8 / 59.9 (9.6) | 116.5 / 132.7 (9.2) | 59.5 / 71.0 (8.3) | 49.6 / 60.4 (9.6) | 20.4 / 26.7 (3.6) | 42.5 / 64.9 (9.9) | 45.7 / 65.3 (13.2) | 39.3 / 50.2 (7.5) | 13.6 / 17.6 (2.6) | 18.1 / 25.0 (5.1) | 3.4 / 4.3 (0.5) | 1.8 / 2.2 (0.4) |
| rustfmt | 490.2 / 594.7 (82.1) | 1016.0 / 1211.8 (131.5) | 627.1 / 823.6 (109.9) | 256.4 / 364.5 (50.4) | 143.0 / 166.1 (16.6) | 114.3 / 160.9 (26.0) | 673.0 / 891.3 (241.2) | 225.6 / 288.9 (35.0) | 222.8 / 257.3 (23.6) | 72.7 / 90.4 (7.6) | 9.4 / 11.1 (1.0) | 7.8 / 10.3 (1.5) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 22.6 | 35.1 | 21.3 | 20.2 | 12.4 | 10.4 | 13.7 | 9.3 | 5.7 | 5.9 | 3.9 | 3.6 |
| sched | 16.2 | 20.0 | 12.4 | 15.8 | 9.0 | 10.4 | 13.9 | 9.4 | 5.8 | 5.9 | 3.8 | 3.7 |
| lto-sched | 17.1 | 19.7 | 12.4 | 15.5 | 8.7 | 10.3 | 13.7 | 9.2 | 5.5 | 5.8 | 3.6 | 3.5 |
| combined | 18.0 | 20.5 | 13.5 | 16.4 | 9.6 | 11.2 | 14.1 | 9.6 | 6.5 | 6.7 | 4.6 | 4.1 |
| rustfmt | 217.4 | 116.4 | 249.5 | 65.9 | 56.8 | 43.3 | 544.9 | 208.7 | 218.8 | 82.6 | 39.1 | 34.2 |

### Sweep 2, pinned (`taskset -c 0`)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 138.2 / 186.1 (21.3) | 538.2 / 626.6 (58.1) | 208.7 / 308.5 (51.5) | 186.1 / 221.7 (28.5) | 81.4 / 109.0 (18.9) | 63.4 / 82.1 (9.9) | 54.6 / 68.4 (8.5) | 60.7 / 72.6 (9.7) | 14.7 / 18.6 (4.4) | 24.3 / 27.7 (3.0) | 6.1 / 7.5 (1.1) | 2.4 / 3.1 (0.5) |
| sched | 131.0 / 175.1 (21.9) | 482.3 / 584.7 (61.9) | 195.8 / 248.6 (43.8) | 155.3 / 197.6 (24.5) | 75.9 / 100.7 (16.0) | 61.9 / 76.1 (14.0) | 58.5 / 83.1 (13.0) | 54.3 / 67.6 (9.5) | 15.0 / 18.4 (2.5) | 19.6 / 28.1 (4.3) | 6.6 / 8.0 (0.6) | 2.5 / 3.6 (1.5) |
| lto-sched | 125.6 / 160.3 (24.1) | 400.2 / 557.8 (157.2) | 224.2 / 258.2 (27.2) | 148.4 / 209.2 (38.8) | 83.6 / 108.5 (12.8) | 75.9 / 91.2 (9.5) | 51.3 / 64.3 (13.6) | 42.3 / 60.2 (9.6) | 14.2 / 19.1 (3.8) | 20.2 / 25.9 (3.6) | 5.9 / 7.3 (0.7) | 2.5 / 2.9 (0.5) |
| combined | 106.6 / 151.9 (25.6) | 346.6 / 471.2 (60.8) | 171.6 / 236.3 (34.0) | 147.3 / 189.2 (35.8) | 78.0 / 92.3 (12.9) | 42.0 / 59.2 (10.2) | 44.1 / 57.6 (9.0) | 39.3 / 60.4 (15.0) | 13.3 / 17.7 (3.1) | 18.9 / 27.1 (5.0) | 4.8 / 6.5 (1.5) | 2.5 / 3.5 (1.5) |
| rustfmt | 509.9 / 608.3 (87.8) | 941.7 / 1163.7 (129.9) | 686.7 / 836.4 (89.2) | 240.1 / 331.0 (50.0) | 115.4 / 144.6 (18.7) | 110.6 / 147.9 (18.5) | 536.3 / 717.7 (102.6) | 237.4 / 299.5 (22.6) | 235.8 / 270.1 (17.8) | 86.9 / 98.4 (7.5) | 9.6 / 12.7 (3.6) | 8.0 / 9.6 (1.0) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 15.7 | 17.1 | 12.2 | 15.2 | 9.0 | 10.4 | 13.8 | 9.3 | 5.7 | 6.0 | 3.8 | 3.7 |
| sched | 8.8 | 8.9 | 6.7 | 10.6 | 6.2 | 10.5 | 13.8 | 8.8 | 5.8 | 6.1 | 3.8 | 3.7 |
| lto-sched | 8.7 | 8.6 | 6.5 | 10.3 | 6.0 | 10.3 | 13.7 | 8.7 | 5.6 | 5.8 | 3.6 | 3.5 |
| combined | 9.6 | 9.6 | 7.4 | 11.3 | 6.9 | 11.3 | 14.0 | 9.2 | 6.5 | 6.7 | 4.6 | 4.0 |
| rustfmt | 217.4 | 116.4 | 249.5 | 65.9 | 56.7 | 43.3 | 544.9 | 208.6 | 218.8 | 82.5 | 39.1 | 34.2 |

### Ratios to `base` and to rustfmt (from the tables above)

Each cell: variant minimum / `base` minimum, variant mean / `base` mean, and in brackets
rustfmt's mean divided by the variant's mean on the same file set.

| unpinned | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sched | 0.61/0.68 [9.1x] | 0.81/0.74 [7.8x] | 0.94/0.77 [10.0x] | 0.84/0.78 [3.8x] | 0.69/0.62 [5.5x] | 1.02/1.07 [1.6x] | 1.10/1.02 [12.1x] | 1.03/0.98 [4.8x] | 1.28/1.21 [12.4x] | 1.14/1.03 [3.5x] | 0.69/0.78 [2.4x] | 0.98/1.03 [4.2x] | 7/12 |
| lto-sched | 0.65/0.67 [9.3x] | 0.71/0.70 [8.2x] | 0.80/0.76 [10.2x] | 0.67/0.71 [4.2x] | 0.65/0.56 [6.1x] | 0.78/0.94 [1.8x] | 1.01/1.13 [11.0x] | 0.92/0.87 [5.4x] | 1.08/0.99 [15.2x] | 0.92/0.90 [4.0x] | 0.63/0.69 [2.7x] | 0.88/0.99 [4.4x] | 10/12 |
| combined | 0.58/0.63 [9.9x] | 0.70/0.63 [9.1x] | 0.74/0.67 [11.6x] | 0.55/0.49 [6.0x] | 0.55/0.55 [6.2x] | 0.53/0.69 [2.5x] | 0.87/0.91 [13.7x] | 0.86/0.81 [5.8x] | 1.03/1.03 [14.7x] | 0.95/1.00 [3.6x] | 0.68/0.72 [2.6x] | 0.87/0.94 [4.6x] | 11/12 |

| pinned | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sched | 0.95/0.94 [3.5x] | 0.90/0.93 [2.0x] | 0.94/0.81 [3.4x] | 0.83/0.89 [1.7x] | 0.93/0.92 [1.4x] | 0.98/0.93 [1.9x] | 1.07/1.22 [8.6x] | 0.89/0.93 [4.4x] | 1.02/0.99 [14.6x] | 0.81/1.01 [3.5x] | 1.08/1.06 [1.6x] | 1.07/1.16 [2.7x] | 8/12 |
| lto-sched | 0.91/0.86 [3.8x] | 0.74/0.89 [2.1x] | 1.07/0.84 [3.2x] | 0.80/0.94 [1.6x] | 1.03/1.00 [1.3x] | 1.20/1.11 [1.6x] | 0.94/0.94 [11.2x] | 0.70/0.83 [5.0x] | 0.97/1.03 [14.2x] | 0.83/0.93 [3.8x] | 0.98/0.97 [1.7x] | 1.04/0.92 [3.3x] | 8/12 |
| combined | 0.77/0.82 [4.0x] | 0.64/0.75 [2.5x] | 0.82/0.77 [3.5x] | 0.79/0.85 [1.7x] | 0.96/0.85 [1.6x] | 0.66/0.72 [2.5x] | 0.81/0.84 [12.5x] | 0.65/0.83 [5.0x] | 0.91/0.95 [15.2x] | 0.78/0.98 [3.6x] | 0.80/0.87 [2.0x] | 1.05/1.10 [2.8x] | 11/12 |

### In-process `bench -n 5` pairs, `base` against `combined` (pinned to core 2)

| pair | base | combined | change |
|---|---|---|---|
| 1 | 2.486 s (5.59 MB/s) | 1.764 s (7.88 MB/s) | −29.0% |
| 2 | 2.482 s (5.60 MB/s) | 2.060 s (6.75 MB/s) | −17.0% |
| 3 | 2.528 s (5.50 MB/s) | 2.032 s (6.84 MB/s) | −19.6% |

The ranges do not overlap (`base` 2.482–2.528 s, `combined` 1.764–2.060 s), which is
the protocol's condition for a single-threaded speedup; the individual `lto` and `pgo`
pairs above each overlap `base`.

### Interactions

- PGO on top of the profile and scheduler (`combined` against `lto-sched`, unpinned
  minima): `hir-ty` 117.4 → 116.5 ms, `ide` 64.4 → 59.5, `ide-db` 60.7 → 49.6, `syntax`
  24.1 → 20.4, `lints` 62.1 → 42.5, `nodes` 52.6 → 45.7, `hir-lib` 41.9 → 39.3; pinned
  minima `hir-ty` 400.2 → 346.6, `lints` 75.9 → 42.0, `hir` 125.6 → 106.6. No workload
  loses what `lto-sched` gained outside the spread (`tests` 14.4 → 13.6 unpinned,
  `config` 17.5 → 18.1 within σ 3.5–5.1).
- The scheduler on single files: sweep 1 had `sched` at 1.15/1.13 on `lints` unpinned;
  sweep 2 has 1.02/1.07 on `lints` and 1.28/1.21 on `tests` while the pinned figures are
  0.98/0.93 and 1.02/0.99. One file is one thread under both schedulers; the single-file
  differences change sign between sweeps and stay inside the host's spread.
- Peak RSS of `combined` is 0.5–0.8x `base` on the crate workloads (`hir-ty` 35.1 → 20.5
  MiB unpinned) from the scheduler's thread count, and +0.3–1.0 MiB over `lto-sched` on
  every workload from the PGO binary's size.
- The rustup proxy on the startup workload: `rustfmt --check one.rs` through
  `~/.cargo/bin/rustfmt` takes 41.6 / 53.3 ms (min / mean) against 8.3 / 9.6 ms for the
  toolchain binary; chloro `base` takes 1.6 / 2.2 ms on the same file. Every rustfmt
  figure in this record is from the toolchain binary.

### Decision

Kept at this point: the release profile (`0bb8e06`), the scheduler (`6c55740`, later
dropped, see E2d), the PGO script (`8efd333`, now `d10c1e3` after the rebase). Rejected: `mimalloc` (not committed), `panic = "abort"` and
`target-cpu=native` (headroom only). Against rustfmt on identical file sets with all
four cores, the `combined` build's mean wall time is 1/9.9 of rustfmt's on `hir`, 1/9.1
on `hir-ty`, 1/11.6 on `ide`, 1/6.0 on `ide-db`, 1/6.2 on `syntax`, and 1/2.5 to 1/14.7
on the single big files; on one core, 1/1.6 to 1/4.0 on the crates. Release binaries
from `binaries.yml` get the profile with no workflow change; PGO needs the proposed
step.

## Kept combination: profile + PGO (measured after the scheduler was dropped)

Built from the rebased branch (profile committed, no scheduler change):
`benchmarks/pgo.sh /home/user/tgt/lto-pgo cli` gives `lto-pgo`
(2,806,456 B; PGO on fat LTO in one codegen unit). Gates: `bench --check` 0 of 1238
changed, `conform -i` 1219/1220, 0 not idempotent, sorted `--check -v` report identical to
`base`'s; callgrind 448,458,885 Ir on the 49-file sample (−23.7% against `base`, −19.1%
against `lto`, −9.0% against `pgo` on the default profile).

### Sweep 3, unpinned (min / mean (σ) ms, 15 runs per command)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 75.0 / 90.9 (9.9) | 153.8 / 202.4 (26.1) | 76.1 / 114.9 (42.6) | 96.4 / 124.1 (19.6) | 32.8 / 41.8 (5.2) | 69.5 / 97.8 (20.1) | 54.6 / 68.3 (8.7) | 48.9 / 69.8 (13.4) | 13.5 / 16.7 (2.2) | 23.8 / 29.2 (4.8) | 4.1 / 5.4 (1.1) | 1.5 / 2.2 (0.4) |
| lto | 74.8 / 85.7 (9.4) | 142.4 / 190.5 (30.6) | 74.5 / 92.7 (9.3) | 98.9 / 118.8 (16.3) | 31.2 / 37.8 (4.0) | 73.9 / 85.1 (9.0) | 48.5 / 63.1 (6.3) | 44.9 / 61.5 (10.0) | 15.2 / 17.6 (2.3) | 17.5 / 25.7 (3.7) | 4.8 / 5.5 (0.6) | 1.7 / 2.2 (0.3) |
| lto-pgo | 61.9 / 80.8 (10.8) | 144.0 / 171.3 (20.3) | 71.4 / 85.5 (9.7) | 72.2 / 85.7 (10.9) | 33.3 / 37.4 (2.7) | 48.9 / 64.5 (13.0) | 45.4 / 59.7 (10.8) | 45.6 / 58.2 (7.9) | 11.4 / 16.3 (2.2) | 17.2 / 22.9 (4.5) | 4.4 / 5.3 (0.6) | 1.7 / 2.3 (0.4) |
| rustfmt | 456.0 / 587.6 (85.5) | 1038.9 / 1214.0 (92.1) | 705.9 / 808.5 (96.3) | 272.8 / 339.8 (32.1) | 133.1 / 165.9 (28.4) | 103.1 / 141.3 (21.6) | 606.2 / 825.4 (239.2) | 250.7 / 308.4 (28.3) | 217.1 / 263.3 (25.7) | 82.8 / 97.3 (11.2) | 9.1 / 10.3 (0.9) | 7.8 / 10.1 (1.7) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 22.5 | 35.0 | 21.1 | 20.3 | 12.2 | 10.4 | 13.8 | 9.3 | 5.7 | 5.9 | 4.0 | 3.7 |
| lto | 22.8 | 34.6 | 21.0 | 19.6 | 12.1 | 10.3 | 13.7 | 8.9 | 5.4 | 5.8 | 3.8 | 3.4 |
| lto-pgo | 23.9 | 36.9 | 21.9 | 20.3 | 13.0 | 11.2 | 14.3 | 9.6 | 6.4 | 6.6 | 4.9 | 4.1 |
| rustfmt | 217.5 | 116.4 | 249.5 | 65.8 | 56.8 | 43.4 | 544.9 | 208.7 | 218.8 | 82.5 | 39.1 | 34.3 |

### Sweep 3, pinned (`taskset -c 0`)

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 130.9 / 175.5 (28.4) | 438.2 / 547.0 (75.4) | 239.0 / 277.8 (24.1) | 178.2 / 211.4 (24.1) | 77.1 / 104.2 (11.9) | 80.6 / 105.3 (18.9) | 53.0 / 69.9 (13.0) | 48.9 / 73.7 (21.2) | 20.4 / 25.7 (5.6) | 31.6 / 42.5 (8.0) | 5.0 / 6.5 (1.0) | 2.6 / 3.1 (0.3) |
| lto | 126.2 / 173.3 (36.6) | 428.5 / 508.1 (51.0) | 201.0 / 262.0 (31.6) | 140.7 / 192.6 (28.6) | 79.3 / 108.4 (20.0) | 84.4 / 112.4 (19.0) | 49.1 / 57.3 (8.2) | 42.4 / 60.5 (16.3) | 13.7 / 19.2 (5.2) | 25.8 / 33.1 (5.9) | 5.2 / 6.6 (1.2) | 2.4 / 3.2 (0.6) |
| lto-pgo | 110.8 / 144.4 (24.1) | 433.0 / 503.9 (40.3) | 184.6 / 236.3 (35.3) | 123.4 / 182.9 (41.4) | 77.7 / 94.6 (13.5) | 58.3 / 74.5 (11.4) | 41.6 / 52.9 (9.2) | 47.1 / 60.0 (16.3) | 13.8 / 15.6 (1.6) | 24.6 / 32.9 (4.4) | 4.8 / 6.7 (0.9) | 2.9 / 3.2 (0.2) |
| rustfmt | 418.8 / 503.6 (62.0) | 1062.4 / 1215.1 (83.6) | 667.9 / 811.1 (86.7) | 276.3 / 338.7 (36.9) | 141.2 / 167.3 (20.3) | 143.4 / 189.1 (29.2) | 632.6 / 717.1 (71.6) | 265.8 / 365.7 (61.5) | 221.0 / 272.1 (33.6) | 102.0 / 127.5 (15.8) | 8.7 / 11.9 (1.6) | 9.2 / 10.9 (1.8) |

Peak RSS, MiB (max over runs):

| variant | hir | hir-ty | ide | ide-db | syntax | big-src-lints | big-ast-nodes | big-hir-lib | big-src-tests | big-rust-analyzer-config | tiny | one |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| base | 15.8 | 17.1 | 12.2 | 15.2 | 9.1 | 10.4 | 13.8 | 9.3 | 5.8 | 6.0 | 3.8 | 3.7 |
| lto | 14.8 | 17.0 | 12.0 | 15.0 | 8.9 | 10.2 | 13.7 | 9.0 | 5.5 | 5.8 | 3.7 | 3.4 |
| lto-pgo | 16.4 | 17.9 | 12.9 | 15.9 | 9.8 | 11.2 | 14.4 | 9.6 | 6.5 | 6.7 | 4.6 | 3.9 |
| rustfmt | 217.5 | 116.4 | 249.5 | 65.9 | 56.8 | 43.4 | 544.9 | 208.7 | 218.8 | 82.6 | 39.1 | 34.2 |

### Ratios to `base` and to rustfmt (from the tables above)

Each cell: variant minimum / `base` minimum, variant mean / `base` mean, and in brackets
rustfmt's mean divided by the variant's mean on the same file set.

| unpinned | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lto | 1.00/0.94 [6.9x] | 0.93/0.94 [6.4x] | 0.98/0.81 [8.7x] | 1.03/0.96 [2.9x] | 0.95/0.90 [4.4x] | 1.06/0.87 [1.7x] | 0.89/0.92 [13.1x] | 0.92/0.88 [5.0x] | 1.13/1.06 [14.9x] | 0.74/0.88 [3.8x] | 1.17/1.02 [1.9x] | 1.13/1.01 [4.6x] | 7/12 |
| lto-pgo | 0.83/0.89 [7.3x] | 0.94/0.85 [7.1x] | 0.94/0.74 [9.5x] | 0.75/0.69 [4.0x] | 1.02/0.89 [4.4x] | 0.70/0.66 [2.2x] | 0.83/0.87 [13.8x] | 0.93/0.83 [5.3x] | 0.85/0.98 [16.2x] | 0.72/0.79 [4.3x] | 1.06/0.98 [2.0x] | 1.12/1.04 [4.4x] | 9/12 |

| pinned | hir | hir-ty | ide | ide-db | syntax | lints | nodes | hir-lib | tests | config | tiny | one | min < base |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lto | 0.96/0.99 [2.9x] | 0.98/0.93 [2.4x] | 0.84/0.94 [3.1x] | 0.79/0.91 [1.8x] | 1.03/1.04 [1.5x] | 1.05/1.07 [1.7x] | 0.93/0.82 [12.5x] | 0.87/0.82 [6.0x] | 0.67/0.75 [14.2x] | 0.82/0.78 [3.8x] | 1.02/1.01 [1.8x] | 0.94/1.03 [3.5x] | 9/12 |
| lto-pgo | 0.85/0.82 [3.5x] | 0.99/0.92 [2.4x] | 0.77/0.85 [3.4x] | 0.69/0.87 [1.9x] | 1.01/0.91 [1.8x] | 0.72/0.71 [2.5x] | 0.78/0.76 [13.6x] | 0.96/0.81 [6.1x] | 0.68/0.61 [17.4x] | 0.78/0.77 [3.9x] | 0.96/1.03 [1.8x] | 1.11/1.04 [3.4x] | 10/12 |

### In-process `bench -n 5` pairs, `base` against `lto-pgo` (pinned to core 2)

| pair | base | lto-pgo | change |
|---|---|---|---|
| 1 | 2.539 s (5.47 MB/s) | 2.020 s (6.88 MB/s) | −20.4% |
| 2 | 2.156 s (6.45 MB/s) | 1.549 s (8.98 MB/s) | −28.2% |
| 3 | 2.291 s (6.07 MB/s) | 1.815 s (7.66 MB/s) | −20.8% |

Ranges do not overlap (`base` 2.156–2.539 s, `lto-pgo` 1.549–2.020 s), so the
profile-plus-PGO build meets the protocol's single-threaded speedup condition, as the
three-lever `combined` build did; `lto` alone and `pgo` alone did not. The multi-core
crate ratios here (0.69–0.94 of `base`'s means on `hir`, `hir-ty`, `ide`, `ide-db`,
`syntax`) are what the branch carries; the 0.49–0.67 of the `combined` row above
includes the scheduler that E1 supplies.

### Decision (final)

Kept and committed on the branch: the release profile (`0bb8e06`) and the PGO script
(`d10c1e3`). Dropped: the scheduler commit (E1 carries the same design). Rejected:
`mimalloc` (not committed), `panic = "abort"` and `target-cpu=native` (headroom only).
Release binaries from `binaries.yml` get the profile with no workflow change; PGO needs
the proposed step below.

## Proposed `binaries.yml` change for PGO (not applied; proposal only)

`taiki-e/upload-rust-binary-action` runs its own `cargo build --release` and honours
`RUSTFLAGS`, so the workflow only has to produce the merged profile before that step and
point `RUSTFLAGS` at it. Training needs to run the instrumented binary, so the step is
limited to targets that execute natively on their runner (`x86_64-unknown-linux-gnu` on
`ubuntu-22.04`, `x86_64-apple-darwin` on `macos-13`, `x86_64-pc-windows-msvc` on
`windows-2022`); the two `aarch64` targets keep the plain LTO build. The script's own
final build is then redundant in CI (about a minute on the `ubuntu` runner) and is left
in place so the script stays usable on its own.

```diff
       - name: Install Rust toolchain
         uses: dtolnay/rust-toolchain@stable
+        with:
+          components: llvm-tools-preview
 
       - uses: taiki-e/setup-cross-toolchain-action@v1
@@
       - run: echo "RUSTFLAGS=${RUSTFLAGS} -C target-feature=+crt-static" >> "${GITHUB_ENV}"
         if: endsWith(matrix.target, 'windows-msvc')
 
+      # Profile-guided optimisation: train the instrumented CLI on the conformance
+      # fixtures and build the release binary with the merged profile. Only where the
+      # instrumented binary runs on the runner itself.
+      - name: Collect PGO profile
+        if: startsWith(matrix.target, 'x86_64-')
+        run: |
+          benchmarks/pgo.sh target "${FEATURES}"
+          echo "RUSTFLAGS=${RUSTFLAGS} -Cprofile-use=$(pwd)/target/pgo-gen/profiles/merged.profdata -Cllvm-args=-pgo-warn-missing-function" >> "${GITHUB_ENV}"
+
       - name: Extract version from tag
```

On Windows the path in `-Cprofile-use` needs the runner's native form (`$(cygpath -w
"$(pwd)")/target/...` under the `bash` default shell); the diff above shows the Unix form.
`-Cllvm-args=-pgo-warn-missing-function` turns the "no profile data for function" case
into a warning, which `RUSTFLAGS: -D warnings` does not touch (it is an LLVM remark, not
a rustc lint). The profile is specific to the toolchain that produced it, so the job
regenerates it on every release rather than caching it.

## Follow-up

- `benchmarks/pgo.sh` trains on the fixtures with the default configuration only; a
  training run with `--config` values would cover the non-default paths that
  `config_matrix` exercises.
- The instrumented binary saturates its value-profile counters (`-vp-counters-per-site`);
  raising the limit may change the indirect-call promotion PGO does in the rewrite
  dispatch.
- The scheduler reads every file's size with one `statx` before any work; passing the
  `DirEntry` metadata out of `discover_rust_files` would remove that call for files found
  by discovery.
- The CLI's `unwrap_or_default()` on a worker's join drops that worker's results after a
  formatter panic without a report line; a per-file `catch_unwind` in the worker would
  turn the panic into a `ProcessResult::Error` for that file and make `panic = "abort"` a
  settled question either way.
- Host noise: σ of 10–20% of the mean on a 4 vCPU cloud host with 1.8% steal; a host
  with stable clocks, or `perf stat` instruction counts for the single-threaded levers,
  would tighten the ranges that the protocol's non-overlap rule needs.
