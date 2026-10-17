# EXP-014: the allocator's own time, and other allocators' time and peak RSS under the CLI

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md), run under
[../protocol.md](../protocol.md). E2 measured `mimalloc` at −12.4% instructions with wall
time inside the spread and 2–10x peak RSS ([EXP-002](EXP-002-cli-build.md), E2c); E9 closed
glibc's tunables ([EXP-009](EXP-009-glibc-tunables.md)); the extensions record derived the
acceptance rule applied here
([../../2026-10-15-extensions-theory/incremental-and-process.md](../../2026-10-15-extensions-theory/incremental-and-process.md)
§ 4). This experiment measures what the instruction counts could not: the share of *time*
spent inside the allocator, and what three replacement allocators do to wall time, CPU time
and peak RSS under the CLI's real thread pattern (one worker per core, item-level threads
for files of 128 KiB and more). Labels: **measured** (run here), **read** (from source or
documentation), **hypothesis**.

## Identity

| field | value |
|---|---|
| branch | `20261015-allocators` from `ff5a369` (was `a36b5e8`) (integration branch at the start), worktree `/home/user/wt/allocators` |
| base BKG | `397159b` (was `92fdef0`); `Ir` 312,734,032 on the sample |
| commits | `bdf8d70` (was `0d78c92`) (`perf(bench)`: the `alloc-timing` wrapper and the `alloc-*` features), the following commit (this file, the ledger row) |
| crates | `mimalloc` 0.1.52 with `libmimalloc-sys` 0.1.49 (default features: mimalloc C **v3**, `MI_MALLOC_VERSION 30302`; `v2` is a feature); `tikv-jemallocator` 0.7.0 with `tikv-jemalloc-sys` 0.7.1+5.3.1 (jemalloc 5.3.1, symbols prefixed `_rjem_`, so the environment variable is `_RJEM_MALLOC_CONF`); `snmalloc-rs` 0.7.6 with `snmalloc-sys` 0.7.6 (default features: `build_cmake`, `usewait-on-address`). All three fetched through the proxy and built offline afterwards; each built in under 2 minutes |
| glibc | 2.39 (Ubuntu 2.39-0ubuntu8.9) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`; release profile of the workspace (fat LTO, one codegen unit) for every binary |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.80GHz" (`constant_tsc`, `nonstop_tsc`; TSC measured at 2.79–2.80 GHz), Linux 6.18.44, THP `enabled` = `madvise`, `defrag` = `madvise`, `overcommit_memory` = 0. Shared with other agents' builds and tests: load average 3.4–8.6 during the timed windows. Only ratios between binaries interleaved in one window are used; absolute times are **provisional** (protocol § 6) |
| tools | hyperfine 2.0.0, valgrind 3.22.0 (`benchmarks/profile.sh`), `taskset` |

Binaries (copied out of the worktree's `target/` to the session scratchpad `e14/<variant>/`;
the `bench-timing` builds of the four variants predate a clippy-only rewrite of the
report loop in `bdf8d70` (was `0d78c92`); `glibc/bench-timing-final` is the committed code and prints the
same call and byte counts):

| binary | bytes | sha256 (first 16) |
|---|---|---|
| `glibc/chloro` | 2,043,368 | `e593833c175b93f7` |
| `glibc/bench` | 1,805,104 | `f3dc9fa998e34a28` |
| `glibc/bench-timing` | 1,992,640 | `84f88e56252fc37e` |
| `glibc/bench-default-final` | 1,805,104 | `c9a41c81e46b7971` |
| `glibc/bench-timing-final` | 1,992,816 | `dd1f989bddb67ed3` |
| `mimalloc/chloro` | 2,210,968 | `b3889b219bd227d0` |
| `mimalloc/bench` | 1,969,168 | `55ba76e65ea5f38f` |
| `mimalloc/bench-timing` | 2,192,176 | `ff6ca0630450fe11` |
| `jemalloc/chloro` | 2,597,304 | `6ed80375e000f7a2` |
| `jemalloc/bench` | 2,355,240 | `96586850e1028f35` |
| `jemalloc/bench-timing` | 2,581,888 | `15c9fadd5cf6c522` |
| `snmalloc/chloro` | 2,185,184 | `de944ad97535c646` |
| `snmalloc/bench` | 1,943,104 | `392681d44a87eddb` |
| `snmalloc/bench-timing` | 2,181,936 | `50ccd7ecf57ef29c` |

## Hypothesis and decision rule

1. **Allocator time.** At the final head the glibc malloc family is 15.1% of instructions on
   the 49-file sample (47.0 M of 312.7 M). Hypothesis: its share of *time* is of the same
   order, since `malloc.c` carries 20.8% of the D1 read misses
   ([../profile-flat.md](../profile-flat.md)) and an instruction count is blind to page
   faults and cache misses. Measured directly here with `rdtsc` around every call.
2. **Replacement allocators.** A faster allocator lowers wall time on most workloads; the
   cost is peak RSS through thread-local heaps and eager commit.

Decision rule, from the extensions record § 4, applied unchanged: an allocator is a
candidate for the CLI only if its peak RSS is within 1.2x of glibc's on **every**
workload and its wall time is below glibc's with **non-overlapping** ranges on **most**
workloads. Otherwise the result is a measurement, not a change. Whatever the outcome the
default stays glibc: every feature added here is off by default.

## What was built (`bdf8d70` (was `0d78c92`))

- `chloro-core` features `alloc-timing`, `alloc-mimalloc`, `alloc-jemalloc`,
  `alloc-snmalloc` (with the three crates as optional dependencies), used by
  `examples/bench.rs` only; the library sets no allocator. `chloro` features
  `alloc-mimalloc`, `alloc-jemalloc`, `alloc-snmalloc` (each implies `cli`) set the CLI
  binary's `#[global_allocator]` in `chloro/src/cli.rs`. The first enabled of mimalloc,
  jemalloc, snmalloc wins, so `--all-features` builds and tests.
- `alloc-timing` wraps the chosen allocator (`Timed<Inner>`, `Inner` = `System` without an
  `alloc-*` feature): every `alloc`, `dealloc`, `realloc` and `alloc_zeroed` reads `rdtsc`
  before and after the inner call, then adds calls, requested bytes (`dealloc` sizes kept
  apart) and the tick difference to relaxed atomics, outside the timed window. `bench`
  snapshots the counters and `rdtsc` around its timed rounds and prints the totals per
  operation, the share of the rounds' ticks, and the median of 10,001 back-to-back
  `rdtsc` pairs (the floor each timed call carries). `CHLORO_ALLOC_TIMING=count` keeps the
  counters and skips `rdtsc`; `=off` passes through.
- **Default build unchanged (measured):** the default `bench` has no `#[global_allocator]`,
  0 `rdtsc` instructions and no `alloc_timing` symbol (`objdump -d`, `nm`), against 2,950
  `rdtsc` in the timing build; `benchmarks/profile.sh --core 3` gives **312,732,773** `Ir`
  against 312,734,032 (−0.0004%).
- Commands: `cargo build --release -p chloro --features "cli alloc-<x>"`, `cargo build
  --release -p chloro-core --example bench --features "alloc-<x>"` and the same with
  `alloc-timing` added (the scratchpad script `e14/build.sh`).

## 1. The allocator's own time (measured)

`CHLORO_ALLOC_TIMING=<time|count> taskset -c 3 bench-timing -n 1 --root <root>` and the
plain `bench` of the same allocator, five process repetitions of each, interleaved
(repetition × root × allocator × mode); medians. Window 02:55:54–02:57:42 UTC, load
5.30 → 3.60. "Net" subtracts calls × the `rdtsc` pair floor (28 ticks) from the ticks
measured inside the calls; "share of plain" divides it by the plain build's round time ×
the TSC rate. The `sample` root is `benchmarks/profile.sh`'s 49 files (650,213 bytes), the
`full` root the 1238 fixtures (14.2 MB).

| root | allocator | calls | requested bytes | ticks inside calls | share of rounds (time mode) | net of floor | net, share of plain round | rounds s: plain / count / time |
|---|---|---|---|---|---|---|---|---|
| sample | glibc | 447,859 | 120,940,437 | 38.5 M | 17.81% | 25.9 M | **16.3%** | 0.057 / 0.055 / 0.073 |
| sample | mimalloc | 447,859 | 120,940,437 | 461.4 M | 74.64% | 448.9 M | (bimodal, see below) | 0.049 / 0.073 / 0.217 |
| sample | jemalloc | 447,859 | 120,940,437 | 26.7 M | 15.64% | 14.2 M | 10.6% | 0.048 / 0.051 / 0.061 |
| sample | snmalloc | 447,859 | 120,940,437 | 26.0 M | 15.05% | 13.5 M | 9.5% | 0.051 / 0.053 / 0.062 |
| full | glibc | 13,198,372 | 5,469,791,650 | 951.2 M | 17.79% | 581.7 M | **13.9%** | 1.495 / 1.572 / 1.903 |
| full | mimalloc | 13,198,372 | 5,469,791,650 | 1321.7 M | 21.85% | 952.1 M | 22.3% | 1.527 / 1.677 / 2.223 |
| full | jemalloc | 13,198,372 | 5,469,791,650 | 788.0 M | 15.33% | 418.5 M | 10.8% | 1.382 / 1.495 / 1.836 |
| full | snmalloc | 13,198,372 | 5,469,791,650 | 765.3 M | 14.50% | 395.7 M | 9.9% | 1.428 / 1.534 / 1.885 |

Per operation on the full tree (ticks per call, median run): glibc `alloc` 59, `dealloc`
54, `realloc` 305; jemalloc 47 / 42 / 308; snmalloc 41 / 48 / 259; mimalloc 58 / 39 / 911.
`alloc_zeroed` is called 11 times. The calls equal `alloc_count`'s 237,431 allocations on
the sample (210,428 `alloc` + 27,003 `realloc`) and its 120.9 MB.

A second window (03:33:31–03:34:37, load 4.51 → 5.43) ran glibc and mimalloc with
`MIMALLOC_ALLOW_THP=0`, five repetitions each:

| root | allocator | ticks inside calls | share of rounds (time mode) | net, share of count / plain round | `realloc` ticks per call | rounds s: plain / count / time |
|---|---|---|---|---|---|---|
| sample | glibc | 33.6 M | 18.34% | 13.5% / 13.7% | 335 | 0.055 / 0.056 / 0.065 |
| sample | mimalloc, THP off | 34.4 M | 15.82% | 14.9% / 15.9% | 349 | 0.049 / 0.052 / 0.067 |
| full | glibc | 995.4 M | 17.81% | 13.6% / 12.7% | 308 | 1.759 / 1.645 / 1.992 |
| full | mimalloc, THP off | 799.7 M | 14.44% | 9.5% / 10.9% | 303 | 1.405 / 1.620 / 1.998 |

**Reading.**

- **glibc's allocator is 13–16% of the formatter's time** in a single-threaded run: 16.3%
  and 13.7% of the plain round on the sample in the two windows, 13.9% and 12.7% on the
  full tree; 17.8% before the floor is taken off. Its instruction share on the sample is
  15.1%. The instruction share is a fair proxy for the time spent inside the calls; the
  cache misses `malloc.c` takes (profile-flat) do not make its time share larger than its
  instruction share. What the in-call ticks do not see is the caller's cost of touching
  fresh memory (first-touch faults and misses in the caller's own code), so 13–16% is the
  allocator's direct share, not everything allocation costs.
- **The wrapper's overhead** (measured): counting alone (two relaxed atomic adds per call)
  costs 0–5% of the round (sample 0.055 against 0.057 s plain, inside the noise; full 1.572
  against 1.495 s); timing adds 21–33% more, about 70 ticks per call on the full tree
  ((1.903 − 1.572) s × 2.80 GHz / 13.2 M calls), of which the floor of 28 ticks falls inside
  the window and the rest (the second `rdtsc`, the third atomic) outside it. Hence the
  "net" column; the true in-call share lies between it and the raw share.
- **jemalloc and snmalloc spend 3–7 points less** of the round inside the allocator than
  glibc (10.8% and 9.9% against 13.9% on the full tree, 10.6% and 9.5% against 16.3% on
  the sample), from cheaper `alloc`/`dealloc` (41–48 ticks against 54–59). That bounds a
  single-threaded gain from swapping the allocator at about 3–7% of the formatter's time.
- **mimalloc with its defaults is slower inside the allocator than glibc on this host**,
  because of `realloc`: 911 ticks per call on the full tree against 305, and on the sample
  a bimodal run time (time mode 0.157–0.217 s in some processes, 0.064–0.065 s in others;
  `realloc` 7,700–14,100 ticks per call in the slow ones). With `MIMALLOC_ALLOW_THP=0`
  `realloc` is 303–349 ticks and mimalloc's share drops to 9.5–10.9% on the full tree, like
  jemalloc's. **Read** (`libmimalloc-sys` 0.1.49, `c_src/mimalloc/v3/src/prim/unix/prim.c`
  lines 412–421): with `allow_thp` on (the default) mimalloc calls
  `madvise(MADV_HUGEPAGE)` on the OS memory it maps for its arenas; this host's THP
  `defrag` setting is `madvise`, under which (kernel documentation, **read**) a fault in a
  `MADV_HUGEPAGE` region enters direct reclaim and compaction to get a 2 MiB page.
  **Hypothesis**, consistent with the bimodality and with the THP-off result: the slow
  `realloc`s are first touches of fresh arena memory that stall in compaction, so their cost
  depends on the host's memory fragmentation, which other agents' builds change.

## 2. Variants under the CLI and `bench` (measured)

| label | binary | environment |
|---|---|---|
| `glibc` | `glibc/chloro`, `glibc/bench` (today's build) | — |
| `mi` | `mimalloc/*` | defaults (v3: `arena_eager_commit` 2 = on with overcommit, `allow_thp` 1, `purge_delay` 1000 ms, `arena_reserve` 1 GiB; read in `options.c` and confirmed by `MIMALLOC_VERBOSE=1`) |
| `mi-bounded` | `mimalloc/*` | `MIMALLOC_ALLOW_THP=0 MIMALLOC_ARENA_EAGER_COMMIT=0 MIMALLOC_ALLOW_LARGE_OS_PAGES=0` (the memory options the extensions record lists, names as in v3's `mimalloc.h` and readme; `MIMALLOC_VERBOSE=1` prints `arena_eager_commit: 0`, `allow_large_os_pages: 0`) |
| `mi-purge0` | `mimalloc/*` | `MIMALLOC_ALLOW_THP=0 MIMALLOC_PURGE_DELAY=0` (the only mimalloc setting that came near glibc's RSS in a probe of six, below) |
| `je` | `jemalloc/*` | defaults (`narenas` = 4 × CPUs, `dirty_decay_ms` 10 s, background threads off; jemalloc(3), read) |
| `je-bounded` | `jemalloc/*` | `_RJEM_MALLOC_CONF=background_thread:false,dirty_decay_ms:0,muzzy_decay_ms:0` |
| `sn` | `snmalloc/*` | defaults (snmalloc has no run-time memory options to set) |

**Probe** (hyperfine `-r 8`, `fx-hir-ty` unpinned, 02:55, load 3.0 → 4.1), peak RSS max in
MiB with mean wall ms: glibc 19.5 (136); mi 132 (155); `ALLOW_THP=0` 52.6 (136);
`ARENA_EAGER_COMMIT=0` 50.3 (134); both 53.7 (150); both + `PAGE_COMMIT_ON_DEMAND=1` 56.1
(135); `ALLOW_THP=0 PURGE_DELAY=0` 18.3 (200); je 26.3 (202); je decay 0 16.8 (290); je
`narenas:1` 21.4 (192); sn 34.6 (189).

**Output (measured):** `bench -n 1 --check fixtures-e8d87c2.tsv --root
chloro-core/tests/conformance/fixtures` reports **0 of 1238 outputs changed** for every
build (`bench` and `bench-timing` of glibc, mimalloc, jemalloc, snmalloc), for
`mi-bounded` and `je-bounded`, and with `--threads 4` for the four `bench` builds. Every
CLI command below exits 0 on every run (the inputs are fixed points; `--check --no-cache`
with the glibc build exits 0 on each).

### Harness

`e14/hf.sh`: per workload, `hyperfine -N -i -w 2 -r 15 --metrics
time_wall_clock,time_user,time_system,memory_peak_resident --export-json`, all seven
labels in one invocation, two rounds with the order reversed in the second (n = 30 per
cell; `bench-n3` `-r 5`, n = 10). Every command runs through `env` (`env E14=1` for the
labels without settings) so each pays the same exec. Pinned rows prefix `taskset -c 3`.
Peak RSS is hyperfine's `memory_peak_resident` (`getrusage` of the child, all threads),
as in M3 and EXP-002.

| id | command (after `env ...`) | pinning | input |
|---|---|---|---|
| `hirty-mc` | `chloro --check --no-cache <hir-ty>` (4 workers) | none | `fx-hir-ty`'s rustfmt-formatted copy from `compare_rustfmt.sh` (122 files, 2,876,032 bytes) |
| `hirty-1c` | same | core 3 (1 worker) | same |
| `lints-t4` | `chloro --check --no-cache --threads 4 lints.rs` (item-level threads) | none | `stdin-lints`' formatted copy (639,771 bytes) |
| `lints-t1` | `chloro --check --no-cache --threads 1 lints.rs` | core 3 | same |
| `tiny-mc` | `chloro --check --no-cache <tiny>` | none | `tiny-test-data`'s formatted copy (287 files) |
| `fx-warm` | `chloro --check --cache-dir e14/cache/<label> <fx>` (warmups fill the cache; every timed run is all hits and formats nothing) | none | the 1238 fixtures written once by the glibc build with `--write --no-cache` (14.2 MB) |
| `bench-n3` | `bench -n 3 --root chloro-core/tests/conformance/fixtures` (in process, sequential) | core 3 | the 1238 fixtures as tracked |

Windows (UTC, load average at start → end): window 1 02:58:45–03:12:37, 3.61 → 5.35 (peak
8.55); window 2 03:21:28–03:33:13, 5.13 → 4.92 (low 3.44). The pinned rows show wall
time far above CPU time in places (window 2 `hirty-1c` glibc 503 ms wall against 362 ms
CPU, σ 183 ms): core 3 was shared with other agents' processes, so for the pinned rows the
CPU column is the steadier comparison.

### Ratios to glibc in both windows (window 1 / window 2; means; RSS of the maxima)

Wall time:

| allocator | hirty-mc | hirty-1c | lints-t4 | lints-t1 | tiny-mc | fx-warm | bench-n3 | workloads ≤ 1.2x RSS (both windows) | wall ranges below glibc |
|---|---|---|---|---|---|---|---|---|---|
| mi | 1.16 / 0.85 | 0.90 / 1.08 | 1.48 / 1.19 | 0.91 / 1.25 | 1.54 / 1.36 | 1.26 / 1.26 | 0.87 / 0.95 | 0/7 | 0/7 |
| mi-bounded | 1.21 / 0.92 | 1.35 / 1.09 | 1.46 / 1.22 | 0.98 / 1.69 | 1.20 / 1.22 | 1.16 / 1.26 | 1.02 / 0.96 | 0/7 | 0/7 |
| mi-purge0 | 1.40 / 1.35 | 1.21 / 1.25 | 1.40 / 1.33 | 1.06 / 1.79 | 1.41 / 1.39 | 1.24 / 1.49 | 1.14 / 1.31 | 5/7 | 0/7 |
| je | 0.84 / 0.89 | 1.00 / 1.02 | 1.26 / 1.17 | 1.30 / 1.54 | 1.23 / 1.17 | 1.12 / 1.34 | 1.13 / 0.92 | 0/7 | 0/7 |
| je-bounded | 1.17 / 1.42 | 1.10 / 1.24 | 1.49 / 1.45 | 2.24 / 1.68 | 1.31 / 1.38 | 1.31 / 1.85 | 1.17 / 1.10 | 5/7 | 0/7 |
| sn | 0.99 / 1.01 | 0.97 / 1.32 | 1.28 / 1.18 | 2.23 / 1.06 | 1.09 / 1.06 | 1.13 / 1.39 | 0.92 / 0.88 | 0/7 | 0/7 |

CPU time (user + sys):

| allocator | hirty-mc | hirty-1c | lints-t4 | lints-t1 | tiny-mc | fx-warm | bench-n3 | workloads ≤ 1.2x RSS (both windows) | wall ranges below glibc |
|---|---|---|---|---|---|---|---|---|---|
| mi | 1.10 / 0.93 | 0.92 / 0.96 | 1.51 / 1.26 | 0.91 / 1.01 | 1.56 / 1.33 | 1.34 / 1.23 | 1.01 / 0.96 | 0/7 | 0/7 |
| mi-bounded | 1.01 / 0.94 | 1.02 / 0.98 | 1.17 / 1.12 | 0.98 / 1.10 | 1.05 / 1.09 | 1.10 / 1.13 | 1.04 / 0.95 | 0/7 | 0/7 |
| mi-purge0 | 1.31 / 1.42 | 1.20 / 1.12 | 1.33 / 1.39 | 1.06 / 1.21 | 1.56 / 1.50 | 1.51 / 1.34 | 1.15 / 1.11 | 5/7 | 0/7 |
| je | 0.95 / 0.93 | 0.96 / 0.92 | 1.23 / 1.19 | 1.00 / 1.00 | 1.09 / 1.14 | 1.22 / 1.17 | 0.93 / 0.92 | 0/7 | 0/7 |
| je-bounded | 1.35 / 1.45 | 1.10 / 1.11 | 1.48 / 1.66 | 1.12 / 1.17 | 1.48 / 1.31 | 1.83 / 1.64 | 1.12 / 1.08 | 5/7 | 0/7 |
| sn | 0.97 / 0.95 | 0.95 / 0.95 | 1.19 / 1.09 | 1.07 / 1.06 | 1.04 / 1.06 | 1.30 / 1.25 | 0.95 / 0.90 | 0/7 | 0/7 |

Peak RSS:

| allocator | hirty-mc | hirty-1c | lints-t4 | lints-t1 | tiny-mc | fx-warm | bench-n3 | workloads ≤ 1.2x RSS (both windows) | wall ranges below glibc |
|---|---|---|---|---|---|---|---|---|---|
| mi | 6.84 / 6.45 | 6.74 / 6.73 | 4.31 / 4.42 | 4.42 / 4.40 | 6.54 / 6.42 | 9.72 / 10.24 | 2.31 / 2.31 | 0/7 | 0/7 |
| mi-bounded | 2.68 / 2.94 | 3.54 / 3.54 | 1.69 / 1.68 | 2.10 / 2.09 | 1.45 / 1.43 | 1.76 / 1.76 | 1.74 / 1.74 | 0/7 | 0/7 |
| mi-purge0 | 0.87 / 0.89 | 1.05 / 1.05 | 0.96 / 0.97 | 1.20 / 1.19 | 1.23 / 1.22 | 1.25 / 1.27 | 1.13 / 1.13 | 5/7 | 0/7 |
| je | 1.26 / 1.30 | 1.51 / 1.50 | 1.59 / 1.64 | 1.68 / 1.67 | 1.39 / 1.39 | 1.48 / 1.42 | 1.38 / 1.29 | 0/7 | 0/7 |
| je-bounded | 0.81 / 0.82 | 1.02 / 1.01 | 0.92 / 0.96 | 1.15 / 1.14 | 1.34 / 1.32 | 1.29 / 1.28 | 0.98 / 0.99 | 5/7 | 0/7 |
| sn | 1.72 / 1.75 | 2.01 / 2.01 | 1.78 / 1.81 | 2.39 / 2.38 | 2.18 / 2.14 | 3.02 / 3.11 | 1.90 / 1.90 | 0/7 | 0/7 |

The last two columns count workloads where the condition holds in both windows; "below"
means every run of the allocator took less wall time than every glibc run.

### Window 2 (the lower load), per cell

Cells: min / mean ± sd wall ms, max wall, mean CPU (user + sys) ms, peak RSS MiB (median and
max over the runs), ratios to glibc, and whether the min–max wall ranges overlap.

#### `hirty-mc`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 124.7 / 159.2 ± 26.6 | 253.9 | 385.6 | 19.3, 20.5 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 117.1 / 134.9 ± 14.1 | 182.2 | 357.6 | 111.1, 132.3 | 0.847 | 0.927 | 6.446 | overlap | [0] |
| mi-bounded | 30 | 126.7 / 145.7 ± 15.8 | 187.9 | 361.5 | 48.8, 60.4 | 0.915 | 0.937 | 2.944 | overlap | [0] |
| mi-purge0 | 30 | 183.2 / 214.5 ± 18.7 | 267.8 | 549.3 | 15.9, 18.4 | 1.347 | 1.425 | 0.894 | overlap | [0] |
| je | 30 | 122.9 / 141.2 ± 16.3 | 190.3 | 356.8 | 25.8, 26.7 | 0.887 | 0.925 | 1.299 | overlap | [0] |
| je-bounded | 30 | 184.1 / 226.5 ± 26.0 | 301.3 | 558.4 | 15.6, 16.8 | 1.423 | 1.448 | 0.821 | overlap | [0] |
| sn | 30 | 135.0 / 161.4 ± 15.9 | 198.5 | 364.5 | 33.1, 35.8 | 1.014 | 0.945 | 1.746 | overlap | [0] |

#### `hirty-1c`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 336.8 / 503.1 ± 182.8 | 836.7 | 361.5 | 7.9, 8.0 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 318.1 / 544.3 ± 212.3 | 1176.2 | 348.6 | 53.8, 53.9 | 1.082 | 0.965 | 6.733 | overlap | [0] |
| mi-bounded | 30 | 328.8 / 547.2 ± 186.3 | 795.9 | 352.5 | 28.2, 28.4 | 1.088 | 0.975 | 3.541 | overlap | [0] |
| mi-purge0 | 30 | 371.4 / 629.3 ± 246.7 | 1007.9 | 403.0 | 8.3, 8.4 | 1.251 | 1.115 | 1.047 | overlap | [0] |
| je | 30 | 313.5 / 512.8 ± 177.8 | 786.4 | 332.9 | 10.8, 12.0 | 1.019 | 0.921 | 1.498 | overlap | [0] |
| je-bounded | 30 | 368.5 / 625.2 ± 235.1 | 944.6 | 402.3 | 7.8, 8.1 | 1.243 | 1.113 | 1.012 | overlap | [0] |
| sn | 30 | 315.9 / 666.3 ± 90.9 | 777.8 | 344.4 | 16.0, 16.1 | 1.324 | 0.953 | 2.013 | overlap | [0] |

#### `lints-t4`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 25.9 / 30.5 ± 3.7 | 44.0 | 44.8 | 14.2, 14.9 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 25.9 / 36.2 ± 11.7 | 63.2 | 56.3 | 60.7, 65.7 | 1.185 | 1.257 | 4.417 | overlap | [0] |
| mi-bounded | 30 | 29.5 / 37.1 ± 4.7 | 49.0 | 50.4 | 24.5, 24.9 | 1.218 | 1.124 | 1.676 | overlap | [0] |
| mi-purge0 | 30 | 35.0 / 40.6 ± 3.6 | 51.6 | 62.3 | 13.4, 14.4 | 1.333 | 1.389 | 0.967 | overlap | [0] |
| je | 30 | 31.5 / 35.5 ± 4.2 | 45.8 | 53.5 | 22.7, 24.4 | 1.165 | 1.195 | 1.644 | overlap | [0] |
| je-bounded | 30 | 36.8 / 44.4 ± 9.4 | 87.0 | 74.4 | 13.4, 14.2 | 1.455 | 1.661 | 0.958 | overlap | [0] |
| sn | 30 | 30.0 / 36.0 ± 5.3 | 53.0 | 48.8 | 25.7, 26.9 | 1.181 | 1.090 | 1.810 | overlap | [0] |

#### `lints-t1`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 36.9 / 43.4 ± 7.2 | 65.4 | 42.6 | 9.8, 10.0 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 35.9 / 54.3 ± 21.2 | 101.1 | 43.2 | 43.7, 43.8 | 1.252 | 1.014 | 4.396 | overlap | [0] |
| mi-bounded | 30 | 40.5 / 73.1 ± 28.1 | 114.4 | 47.0 | 20.7, 20.9 | 1.686 | 1.101 | 2.094 | overlap | [0] |
| mi-purge0 | 30 | 44.6 / 77.8 ± 25.4 | 115.9 | 51.4 | 11.8, 11.8 | 1.794 | 1.206 | 1.189 | overlap | [0] |
| je | 30 | 39.1 / 66.9 ± 25.9 | 107.5 | 42.6 | 16.5, 16.6 | 1.543 | 1.000 | 1.669 | overlap | [0] |
| je-bounded | 30 | 45.6 / 72.8 ± 26.5 | 130.3 | 50.0 | 11.2, 11.4 | 1.678 | 1.172 | 1.142 | overlap | [0] |
| sn | 30 | 41.8 / 46.0 ± 6.4 | 72.4 | 45.3 | 19.7, 23.7 | 1.062 | 1.062 | 2.384 | overlap | [0] |

#### `tiny-mc`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 9.8 / 12.0 ± 1.4 | 16.3 | 17.5 | 4.5, 4.7 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 12.4 / 16.4 ± 13.0 | 84.4 | 23.2 | 25.8, 29.9 | 1.363 | 1.329 | 6.421 | overlap | [0] |
| mi-bounded | 30 | 10.7 / 14.6 ± 2.7 | 24.3 | 19.1 | 6.2, 6.7 | 1.216 | 1.090 | 1.433 | overlap | [0] |
| mi-purge0 | 30 | 13.2 / 16.7 ± 2.9 | 24.5 | 26.2 | 5.2, 5.7 | 1.393 | 1.498 | 1.219 | overlap | [0] |
| je | 30 | 10.8 / 14.0 ± 2.2 | 19.6 | 19.9 | 6.2, 6.5 | 1.167 | 1.137 | 1.394 | overlap | [0] |
| je-bounded | 30 | 12.7 / 16.6 ± 3.3 | 26.2 | 22.9 | 5.8, 6.1 | 1.385 | 1.308 | 1.315 | overlap | [0] |
| sn | 30 | 10.6 / 12.8 ± 2.1 | 20.5 | 18.5 | 9.6, 10.0 | 1.063 | 1.056 | 2.140 | overlap | [0] |

#### `fx-warm`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 16.1 / 20.6 ± 4.5 | 34.3 | 25.5 | 4.3, 4.6 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 18.6 / 26.0 ± 4.8 | 35.4 | 31.3 | 34.1, 47.2 | 1.260 | 1.226 | 10.236 | overlap | [0] |
| mi-bounded | 30 | 18.6 / 26.0 ± 5.2 | 37.3 | 28.9 | 6.8, 8.1 | 1.257 | 1.134 | 1.757 | overlap | [0] |
| mi-purge0 | 30 | 21.4 / 30.7 ± 7.7 | 62.1 | 34.1 | 4.8, 5.9 | 1.488 | 1.336 | 1.273 | overlap | [0] |
| je | 30 | 20.2 / 27.6 ± 4.7 | 37.7 | 29.9 | 6.3, 6.6 | 1.336 | 1.174 | 1.425 | overlap | [0] |
| je-bounded | 30 | 27.7 / 38.2 ± 8.4 | 74.5 | 41.7 | 5.7, 5.9 | 1.849 | 1.636 | 1.285 | overlap | [0] |
| sn | 30 | 21.9 / 28.6 ± 5.1 | 42.4 | 31.8 | 12.4, 14.3 | 1.386 | 1.246 | 3.105 | overlap | [0] |

#### `bench-n3`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 10 | 4550.1 / 4778.6 ± 163.9 | 5024.2 | 4657.4 | 34.1, 34.1 | 1 | 1 | 1 | — | [0] |
| mi | 10 | 4346.8 / 4516.3 ± 109.2 | 4691.2 | 4453.0 | 78.6, 78.7 | 0.945 | 0.956 | 2.306 | overlap | [0] |
| mi-bounded | 10 | 4307.5 / 4593.0 ± 467.9 | 5889.7 | 4407.7 | 59.1, 59.2 | 0.961 | 0.946 | 1.736 | overlap | [0] |
| mi-purge0 | 10 | 4743.9 / 6265.7 ± 2549.1 | 11469.1 | 5151.3 | 38.5, 38.5 | 1.311 | 1.106 | 1.129 | overlap | [0] |
| je | 10 | 4186.1 / 4393.6 ± 179.0 | 4657.5 | 4282.0 | 42.5, 44.0 | 0.919 | 0.919 | 1.289 | overlap | [0] |
| je-bounded | 10 | 4808.6 / 5254.2 ± 335.9 | 5807.4 | 5038.0 | 33.6, 33.7 | 1.100 | 1.082 | 0.989 | overlap | [0] |
| sn | 10 | 4051.7 / 4226.2 ± 154.3 | 4617.0 | 4183.3 | 55.0, 64.9 | 0.884 | 0.898 | 1.900 | overlap | [0] |

### Window 1, per cell

#### `hirty-mc`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 163.1 / 278.1 ± 42.0 | 373.2 | 377.8 | 19.3, 20.8 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 228.9 / 322.3 ± 74.8 | 575.5 | 413.7 | 116.1, 142.0 | 1.159 | 1.095 | 6.837 | overlap | [0] |
| mi-bounded | 30 | 227.7 / 336.4 ± 65.8 | 503.8 | 381.0 | 46.9, 55.7 | 1.210 | 1.009 | 2.684 | overlap | [0] |
| mi-purge0 | 30 | 290.1 / 389.3 ± 55.5 | 521.7 | 494.1 | 16.0, 18.1 | 1.400 | 1.308 | 0.871 | overlap | [0] |
| je | 30 | 172.5 / 233.2 ± 48.9 | 324.3 | 359.1 | 24.6, 26.2 | 0.839 | 0.951 | 1.261 | overlap | [0] |
| je-bounded | 30 | 248.2 / 326.1 ± 63.9 | 460.9 | 509.8 | 15.4, 16.7 | 1.173 | 1.349 | 0.806 | overlap | [0] |
| sn | 30 | 203.4 / 275.0 ± 36.0 | 343.4 | 365.3 | 33.4, 35.7 | 0.989 | 0.967 | 1.717 | overlap | [0] |

#### `hirty-1c`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 330.0 / 364.7 ± 46.3 | 578.9 | 346.4 | 7.9, 8.0 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 300.2 / 327.0 ± 25.8 | 383.8 | 319.6 | 53.8, 53.9 | 0.897 | 0.923 | 6.736 | overlap | [0] |
| mi-bounded | 30 | 328.1 / 493.1 ± 195.8 | 844.3 | 353.7 | 28.3, 28.4 | 1.352 | 1.021 | 3.543 | overlap | [0] |
| mi-purge0 | 30 | 360.8 / 443.1 ± 87.3 | 751.8 | 414.9 | 8.3, 8.4 | 1.215 | 1.198 | 1.047 | overlap | [0] |
| je | 30 | 305.0 / 364.6 ± 43.4 | 485.1 | 333.9 | 10.8, 12.1 | 1.000 | 0.964 | 1.508 | overlap | [0] |
| je-bounded | 30 | 357.9 / 401.3 ± 43.2 | 553.7 | 379.7 | 7.9, 8.1 | 1.100 | 1.096 | 1.016 | overlap | [0] |
| sn | 30 | 307.1 / 352.0 ± 44.6 | 519.1 | 330.4 | 16.0, 16.1 | 0.965 | 0.954 | 2.014 | overlap | [0] |

#### `lints-t4`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 35.3 / 45.4 ± 7.0 | 61.6 | 44.2 | 14.3, 14.8 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 36.5 / 67.4 ± 52.5 | 288.8 | 66.6 | 59.7, 63.8 | 1.485 | 1.506 | 4.313 | overlap | [0] |
| mi-bounded | 30 | 41.5 / 66.5 ± 12.1 | 96.6 | 51.8 | 24.4, 24.9 | 1.464 | 1.172 | 1.686 | overlap | [0] |
| mi-purge0 | 30 | 44.0 / 63.8 ± 14.7 | 97.9 | 59.0 | 12.8, 14.1 | 1.404 | 1.335 | 0.956 | overlap | [0] |
| je | 30 | 41.8 / 57.3 ± 9.6 | 81.6 | 54.4 | 22.4, 23.4 | 1.261 | 1.230 | 1.585 | overlap | [0] |
| je-bounded | 30 | 51.8 / 67.7 ± 10.6 | 95.0 | 65.5 | 13.1, 13.6 | 1.491 | 1.483 | 0.921 | overlap | [0] |
| sn | 30 | 38.7 / 58.2 ± 11.1 | 79.2 | 52.8 | 25.3, 26.3 | 1.281 | 1.195 | 1.780 | overlap | [0] |

#### `lints-t1`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 40.1 / 43.7 ± 3.8 | 54.9 | 43.0 | 9.9, 9.9 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 36.6 / 39.8 ± 3.8 | 54.8 | 39.0 | 43.7, 43.8 | 0.911 | 0.907 | 4.422 | overlap | [0] |
| mi-bounded | 30 | 40.8 / 42.8 ± 1.8 | 51.0 | 42.4 | 20.7, 20.9 | 0.981 | 0.985 | 2.104 | overlap | [0] |
| mi-purge0 | 30 | 43.0 / 46.4 ± 3.2 | 56.4 | 45.7 | 11.8, 11.9 | 1.061 | 1.061 | 1.195 | overlap | [0] |
| je | 30 | 37.6 / 56.7 ± 21.9 | 100.5 | 43.0 | 16.5, 16.6 | 1.297 | 1.000 | 1.677 | overlap | [0] |
| je-bounded | 30 | 45.4 / 97.8 ± 19.7 | 140.6 | 48.3 | 11.2, 11.4 | 2.238 | 1.122 | 1.145 | overlap | [0] |
| sn | 30 | 91.4 / 97.6 ± 4.3 | 108.0 | 46.0 | 23.6, 23.7 | 2.234 | 1.068 | 2.395 | above | [0] |

#### `tiny-mc`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 9.9 / 11.3 ± 1.1 | 14.3 | 18.5 | 4.4, 4.6 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 11.7 / 17.4 ± 8.4 | 42.3 | 28.9 | 25.8, 29.9 | 1.537 | 1.556 | 6.545 | overlap | [0] |
| mi-bounded | 30 | 11.2 / 13.5 ± 1.5 | 17.5 | 19.5 | 6.2, 6.6 | 1.197 | 1.054 | 1.448 | overlap | [0] |
| mi-purge0 | 30 | 13.8 / 15.9 ± 1.6 | 19.4 | 28.9 | 5.4, 5.6 | 1.406 | 1.560 | 1.234 | overlap | [0] |
| je | 30 | 10.2 / 13.9 ± 3.3 | 24.3 | 20.2 | 6.1, 6.4 | 1.226 | 1.088 | 1.391 | overlap | [0] |
| je-bounded | 30 | 12.3 / 14.9 ± 2.2 | 20.0 | 27.4 | 5.8, 6.1 | 1.313 | 1.477 | 1.342 | overlap | [0] |
| sn | 30 | 10.4 / 12.3 ± 1.9 | 18.9 | 19.2 | 9.6, 10.0 | 1.090 | 1.035 | 2.180 | overlap | [0] |

#### `fx-warm`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 30 | 14.9 / 17.8 ± 1.8 | 22.9 | 25.4 | 4.3, 4.7 | 1 | 1 | 1 | — | [0] |
| mi | 30 | 17.8 / 22.4 ± 3.6 | 32.7 | 34.0 | 40.1, 45.3 | 1.259 | 1.337 | 9.723 | overlap | [0] |
| mi-bounded | 30 | 16.3 / 20.7 ± 2.8 | 26.4 | 28.0 | 7.3, 8.2 | 1.162 | 1.101 | 1.755 | overlap | [0] |
| mi-purge0 | 30 | 18.8 / 22.1 ± 2.9 | 31.3 | 38.3 | 5.2, 5.8 | 1.243 | 1.507 | 1.246 | overlap | [0] |
| je | 30 | 16.0 / 20.0 ± 4.1 | 39.0 | 31.0 | 6.6, 6.9 | 1.120 | 1.220 | 1.476 | overlap | [0] |
| je-bounded | 30 | 21.4 / 23.4 ± 1.5 | 29.1 | 46.4 | 5.8, 6.0 | 1.315 | 1.826 | 1.290 | overlap | [0] |
| sn | 30 | 17.6 / 20.1 ± 2.3 | 25.4 | 33.0 | 13.3, 14.1 | 1.130 | 1.297 | 3.018 | overlap | [0] |

#### `bench-n3`

| allocator | n | wall min / mean ± sd ms | wall max | CPU mean ms | peak RSS MiB (median, max) | wall mean / glibc | CPU / glibc | RSS max / glibc | wall range vs glibc | exits |
|---|---|---|---|---|---|---|---|---|---|---|
| glibc | 10 | 4480.6 / 6138.6 ± 1783.7 | 10089.6 | 4755.1 | 34.1, 34.1 | 1 | 1 | 1 | — | [0] |
| mi | 10 | 4705.6 / 5330.4 ± 346.3 | 5840.2 | 4819.5 | 78.5, 78.7 | 0.868 | 1.014 | 2.306 | overlap | [0] |
| mi-bounded | 10 | 4619.9 / 6256.3 ± 1510.6 | 9600.9 | 4936.8 | 57.5, 59.3 | 1.019 | 1.038 | 1.737 | overlap | [0] |
| mi-purge0 | 10 | 5043.9 / 6978.6 ± 2346.1 | 11763.3 | 5460.6 | 38.5, 38.5 | 1.137 | 1.148 | 1.129 | overlap | [0] |
| je | 10 | 4444.5 / 6911.4 ± 2366.0 | 9486.0 | 4442.8 | 43.4, 46.9 | 1.126 | 0.934 | 1.375 | overlap | [0] |
| je-bounded | 10 | 5086.8 / 7177.6 ± 2550.3 | 11896.6 | 5343.9 | 33.4, 33.6 | 1.169 | 1.124 | 0.985 | overlap | [0] |
| sn | 10 | 4441.5 / 5627.6 ± 1890.8 | 9330.8 | 4523.6 | 57.0, 64.9 | 0.917 | 0.951 | 1.900 | overlap | [0] |

## 3. Where the RSS goes, per allocator

Peak RSS reproduces between the two windows to within 0.1x on every cell, unlike wall
time, so these multiples are firm (**measured**); the mechanisms are labelled.

| allocator | largest multiple (workload) | range over the 7 workloads | mechanism |
|---|---|---|---|
| `mi` | **9.7x / 10.2x** (`fx-warm`: 45–47 MiB against 4.6, a run that formats nothing) | 2.3–10.2x | Transparent huge pages. **Measured:** `MIMALLOC_ALLOW_THP=0` alone takes `fx-hir-ty` from 132 to 52.6 MiB (probe) and `mi-bounded` takes `fx-warm` to 8.1 MiB. **Read:** v3 `madvise(MADV_HUGEPAGE)`s the OS memory it maps when `allow_thp` is on (`prim/unix/prim.c` 412–421); the readme says eager arena commit "only increases the commit but not the actual peak resident set". **Hypothesis:** every 2 MiB region of an arena that is touched at all becomes resident as a whole huge page, and the heaps of several threads and the pages of many size classes spread the touches over many regions; so the cost follows the regions touched, not the bytes formatted: largest where four workers do little (`fx-warm`, `tiny-mc` 6.4–6.5x), smallest on `bench-n3` (2.3x), whose 34 MiB baseline is mostly the sources it holds and whose 4–11 s runs outlast the purge delay. Switching eager commit off alone (50.3 MiB in the probe) helps as much as THP off; **hypothesis:** the huge-page advice applies only to memory committed when it is mapped |
| `mi-bounded` | **3.54x** (`hirty-1c`: 28.3 MiB against 8.0, one thread) | 1.4–3.5x | Freed memory not purged within the process's life. **Read:** `purge_delay` is 1000 ms in v3 (readme, `options.c`), longer than the CLI runs here (mean wall 12–630 ms; only `bench-n3`, 4–11 s, outlasts it). **Measured:** with `PURGE_DELAY=0` the same workload is 1.05x. That the largest multiple is on one thread rules out per-thread heaps as its cause; **hypothesis:** freed pages of one size class are not reused for another class before fresh pages are touched. Per-thread heaps add to it with 4 workers (`hirty-mc` 2.7–2.9x) |
| `mi-purge0` | **1.25x / 1.27x** (`fx-warm`), `tiny-mc` 1.22–1.23x | 0.87–1.27x | Fixed per-process footprint on the smallest runs (**hypothesis**); the price is the immediate decommit on every freed page (readme: purging at once "decreases performance"): wall 1.06–1.79x, CPU 1.06–1.56x of glibc |
| `je` | **1.67x / 1.68x** (`lints-t1`), `lints-t4` 1.59–1.64x | 1.26–1.68x | Dirty pages kept for `dirty_decay_ms` = 10 s (**read**, jemalloc(3) in `tikv-jemalloc-sys`), longer than the process lives, so nothing freed is returned; per-thread arenas (`narenas` = 4 × CPUs = 16, threads assigned to arenas round-robin, **read**) add more with threads (**measured**, probe: `narenas:1` 21.4 MiB against 26.3 on `fx-hir-ty`) |
| `je-bounded` | **1.34x / 1.32x** (`tiny-mc`), `fx-warm` 1.28–1.29x | 0.81–1.34x | Decay 0 returns freed pages at once and is below glibc on the large workloads (0.81–1.02x); the small runs keep a fixed footprint (**hypothesis**: arena and base metadata mapped at start). Wall 1.10–2.24x, CPU 1.08–1.83x of glibc (a `madvise` per freed run) |
| `sn` | **3.0x / 3.1x** (`fx-warm`: 14 MiB against 4.6), `lints-t1` 2.4x | 1.7–3.1x | No run-time option bounds it. **Hypothesis**, not read in documentation (the crate ships none): each thread's allocator takes its own chunks for every size class it uses (minimum chunk 16 KiB, `sizeclassconfig.h`), and nothing is returned within the run; again largest where the work is smallest |

So EXP-002's 2–10x was mimalloc's huge-page backing of the arena regions its heaps touch
(there the crate rows with 4 workers were 6.0–8.6x unpinned and the one-file, one-worker
`one` 2.2x); here the largest multiple of every default configuration but jemalloc's is on
the run that formats nothing, the warm `--check` of the fixture tree.

## Decision

Under the rule: **no allocator is a candidate.**

- Peak RSS within 1.2x of glibc on every workload: none. The closest, `mi-purge0` and
  `je-bounded`, hold it on 5 of 7 workloads and fail on `tiny-mc` and `fx-warm`
  (1.22–1.34x); every default configuration fails on all 7 (`je` 1.26–1.68x, `sn`
  1.7–3.1x, `mi` 2.3–10.2x), and `mi-bounded` on all 7 (1.4–3.5x).
- Wall time below glibc with non-overlapping ranges: 0 of 7 workloads for every allocator
  in both windows. The two configurations that come near glibc's RSS, `mi-purge0` and
  `je-bounded`, are slower than glibc in mean on every workload in both windows (wall
  1.06–2.24x).

The result is a measurement, not a change. The default allocator stays glibc; the branch
adds only off-by-default features (`alloc-timing`, `alloc-mimalloc`, `alloc-jemalloc`,
`alloc-snmalloc`) and this record, and is mergeable as instrumentation.

What the measurement does say: glibc's allocator is 13–16% of the formatter's own time,
about its 15.1% instruction share; jemalloc and snmalloc would take 3–7 points of it back
single-threaded (in-call share 10–11%; `bench-n3` CPU 0.92–0.93x for `je` and 0.90–0.95x
for `sn`, ranges overlapping) at 1.3–3.1x the peak RSS. Together with E9 this closes the
allocator lever at the process level for this formatter; what remains is fewer allocations
in the formatter itself (the `String`-creation and growth sites of
[../profile-flat.md](../profile-flat.md) § 3, 63.4% of blocks).

## Gates (measured, on `bdf8d70` (was `0d78c92`))

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean (also without `--all-features`, and per feature: `chloro-core` with none, `alloc-timing`, `alloc-jemalloc`, `alloc-snmalloc alloc-timing`; `chloro` with `cli`, `alloc-snmalloc`, `alloc-jemalloc`) |
| `cargo test --workspace --all-features` | 264 passed, 1 ignored, 0 failed (the CLI tests ran on the mimalloc build, the first enabled) |
| `benchmarks/profile.sh --core 3`, default features | 312,732,773 `Ir` against 312,734,032 (−0.0004%; the bound was 0.1%) |
| `bench -n 1 --check fixtures-e8d87c2.tsv` | 0 of 1238 changed for every allocator build and setting, sequential and `--threads 4` (section 2) |

## What is left

- **Quiet host.** Every wall-time figure here is provisional (load 3.4–8.6, core 3
  shared). The 5–10% in-process CPU difference of `je` and `sn` would need protocol § 3's
  three interleaved `bench -n 5` pairs on a quiet host to become a claim; it cannot change
  the decision, which the RSS column settles.
- **Not measured:** mimalloc v2 (the `v2` feature), `rpmalloc`, jemalloc with `narenas:1`
  and decay 0 together under the full harness (probe only), the caller-side cost of fresh
  memory (first-touch faults and cache misses outside the allocator's calls; `perf` is not
  installed, `valgrind --cache-sim=yes` would give the misses), per-thread `alloc-timing`
  for the 4-thread runs (the counters are global atomics, so the share is only meaningful
  for the sequential `bench`), and the cross-thread frees of E1's units in isolation.
