# EXP-001: top-level items formatted in parallel

Experiment E1 of the [autoresearch programme](../../2026-10-14-autoresearch.md); procedure
and gates in [protocol.md](../protocol.md). Labels as in the journal: **measured**,
**read**, **hypothesis**.

## Identity

| field | value |
|---|---|
| branch | `20261014-parallel-items` |
| base | `8f6b2b0` (was `d797292`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`; formatter code as in the best-known-good `884f9c9`) |
| commits | `2f794d7` (was `d60c1fa`) (library, CLI, tests), `53a4f4b` (was `f847a87`) (this file) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`; release profile as in `Cargo.toml` (no LTO, default codegen units); the base and the candidate CLI built with `cargo build --release -p chloro --features cli` |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.10GHz" (`nproc` 4), Linux 6.18.44, hyperfine 2.0.0. This is not the 2.80GHz host of the programme entry: no number here is comparable with [experiments.md](../experiments.md) or [baseline.md](../baseline.md) |
| base binaries | a worktree of `8f6b2b0` (was `d797292`) at `/home/user/wt/base`, built with the same command; golden hashes recorded from it with `bench --record` |

## Hypothesis and falsification

**Hypothesis (E1).** Formatting the top-level items of one file on several threads, with
the sequential visitor keeping the gap handling (comments and blank lines between items,
reorder groups, `#[rustfmt::skip]`, inner attributes, file preamble), cuts the wall time of
a large file by close to the core count with byte-identical output.

**Falsified if** the output changes on any corpus, or the gain on the five largest fixtures
is under 1.5x on 4 cores.

**Evidence it rests on.** [X12b](../../2026-10-10-item-independence.md): on 13,650
fixture units and 965,193 registry units, every top-level item's in-file output equals its
standalone output once the text before it is set aside.

## Design (read; `chloro-core/src/formatter/parallel.rs`)

**API.** `format_source_with_config_parallel(source, &config, threads)`, additive.
`format_source` and `format_source_with_config` are untouched and single-threaded. With
`threads <= 1`, a file of fewer than two units, a parse error, `#![rustfmt::skip]` or
`disable_all_formatting`, no thread is spawned. The threads are spawned with
`std::thread::scope` for the call and joined before it returns; no new dependency, no
`unsafe`.

**Unit.** One top-level item, or one group of consecutive `use`, `mod x;` or
`extern crate` items that `reorder.rs` sorts together (same kind, no blank line between
them, the kind's reorder option on): the sequence `visit_items_with_reordering` walks.
`FmtVisitor::unit_ranges` computes it with the grouping predicate factored out of
`walk_reorderable_items` (`reorderable_group_len`), so the main thread and the workers
cut the item list at the same places. A worker formats a unit by calling
`visit_items_with_reordering` on the unit's slice, so a group is sorted and written by the
same code as before.

**Parse once, one red tree per thread.** `GreenNode` is `Send + Sync`; `SyntaxNode` is
not. The main thread parses (lexer and parser errors, `rustc_compat`, the file-level skip
check, all once), and each worker builds `SyntaxNode::new_root(green.clone())`, collects
the file's items and indexes them by position in the item list.

**Worker state.** Each worker has one `FmtVisitor` and one `RunState` (memo, macro
argument cache, padding tokens, line-start table) for all the units it takes, as the
sequential visitor has one for the file. Before a unit: `last_pos` at the unit's first
item (`item.span().lo()`, the rustc span, so at the first outer attribute or doc comment),
`block_indent` empty, `is_if_else_block` and `is_loop_block` false, `line_number` 0,
`macro_rewrite_failure` false, `lost_comment` false, `skipped_range` empty, and the buffer
holding a single `\n`. The newline stands for the state of the sequential buffer after the
gap before an item: not empty, and ending in a newline (`format_missing_indent` writes
`\n` and the (empty) top-level indent at the end of every gap, and `push_vertical_spaces`
clamps the run of trailing newlines to the configured upper bound). The unit's text is
what follows it. `config`, `snippet_provider` (the whole source) and `is_macro_def`
(false: this path formats whole files only; macro definition bodies are formatted inside
their item, sequentially, as before) are as in the sequential run.

**The gap before a unit** is written, in the sequential visitor, by the unit's own code:
its first `format_missing*` call, from `push_rewrite`, `visit_attrs`,
`push_skipped_with_span`, `visit_fn`, `visit_enum`, `format_mod`, `format_import`, …
That call reads the buffer (whether it is empty, how many newlines it ends with) and the
previous unit's end (`last_pos`), neither of which a worker has. So a worker does not
write the gap: `FmtVisitor::defer_gap`, called at the top of `format_missing`,
`format_missing_indent` and `format_missing_before_removed_import`, records the first
such call of the unit as a `GapOp` (`WithIndent(end)`, `WithoutIndent(end)`,
`NoIndent(end)`, `RemovedImport(lo)`), moves `last_pos` to `end` and returns without
writing. The stitching visitor on the main thread, which has the real buffer and
`last_pos`, replays exactly that call, appends the unit's text and sets `last_pos` to the
worker's final `last_pos`. A unit that never calls `format_missing*` (a
`#[rustfmt::skip] mod x;`, which `visit_item` leaves for the next gap to copy) records no
`GapOp`, writes nothing, and the main thread moves on with `last_pos` unchanged.
`RemovedImport` is the `Some("")` branch of `format_import` (a `use` whose rewrite is
empty), which pops the newline the gap left; the branch is reached by no file of either
corpus, and `format_missing_before_removed_import` carries the deferral so that it is
replayed in full if it ever is.

The main thread does what `format_text` and `format_separate_mod` do around the item
walk: `skip_empty_lines`, the file's inner attributes, the gaps and units in order, the
final `format_missing_with_indent(end_pos)`, the trailing newline, `apply_newline_style`.

**Shared state, merged in unit order.**
- `macro_rewrite_failure` (visitor) and `lost_comment` (run): or-ed. Both are consumed
  only by `format_snippet`, i.e. inside a macro definition body, never for a whole file.
- `skipped_range`: `push_skipped_with_span` records `(source line, output line)` with the
  output line taken from the visitor's `line_number`, which in a worker counts from the
  unit's start; `add_skipped_span` records source lines. A worker notes the indices of
  the former (`UnitRecord::output_line_ranges`); the main thread adds its `line_number` at
  the unit's start to those and copies the rest. The whole-file `skipped_range` is
  consumed by nothing (`format_source_with_config` drops it; only `format_snippet` reads
  it, for macro bodies), so the merge changes no output; it keeps `Formatted` truthful.
- The memo is per worker and is cleared at every top-level item anyway (`visit_item`).
  The macro-argument cache is keyed by source span, so a per-worker cache gives the same
  parses.

**Scheduling.** Units are sorted longest first (bytes) and taken from an atomic counter,
so the largest items start first and the tail is short. Each worker returns
`(unit index, output)` pairs; the main thread places them by index. No locks.

**CLI (`chloro/src/cli/orchestrate.rs`, `worker.rs`, `args.rs`).** Files are sorted
largest first (one `stat` each) and handed out from an atomic counter to
`--threads` workers (default: one per core), replacing the `threads × 4` static chunks of
the path-sorted list. A file of at least `PARALLEL_ITEMS_MIN_BYTES` (128 KiB, from the
crate measurements below) is formatted with the parallel API on `--threads` threads;
smaller files use `format_source_with_config`. When several large files are in flight the
item workers oversubscribe the cores; the alternatives measured are in the CLI section.

**What stays sequential.** Parsing, the file-level checks, the gap text, the sorting of
reorder groups (inside their unit, on one worker), everything below the top level, and the
final assembly. Amdahl's law with the parse at a quarter to a third of `format_source`
(phase-2 entries) bounds the gain on four cores at about 2–2.5x before thread start-up and
the sequential gaps are counted.

## Commands

```text
# base: a worktree of d797292, same build command as the candidate
git worktree add /home/user/wt/base d797292
(cd /home/user/wt/base && cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples)
/home/user/wt/base/target/release/examples/bench --record /home/user/wt/golden/fixtures-d797292.tsv
/home/user/wt/base/target/release/examples/bench --root ~/.cargo/registry/src --record /home/user/wt/golden/registry-d797292.tsv

# candidate
cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
target/release/examples/bench --check /home/user/wt/golden/fixtures-d797292.tsv                  # sequential path
target/release/examples/bench --threads N --check /home/user/wt/golden/fixtures-d797292.tsv      # N = 1, 2, 3, 4
target/release/examples/bench --threads N --root ~/.cargo/registry/src --check /home/user/wt/golden/registry-d797292.tsv
target/release/examples/conform -i
target/release/examples/bench -n 5 -w 1238 [--threads 2|4]                                       # per-file in-process times
target/release/examples/parse_floor rowan <dir with one file> -n 10                               # parse alone
hyperfine -N -i -w 2 -r 15 [taskset -c 0] {base,new,new --threads 1} --check <path>              # /home/user/wt/measure/run.sh
```

`bench --threads N` formats every file with `format_source_with_config_parallel` and
compares each output with `format_source`'s before hashing, so `--threads N --check`
checks the parallel path both against the sequential path of the same build and against
the base's recorded hashes.

## Gates 1–4 (measured)

| gate | result |
|---|---|
| `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace --all-features` | clean; 148 library tests (142 + 6 new in `src/tests/formatter/parallel.rs`), 17 conformance tests, 10 CLI tests |
| `bench --check` (sequential path) against the base's hashes, 1238 fixtures | 0 of 1238 changed |
| `bench --threads N --check`, fixtures, N = 1, 2, 3, 4 | 0 of 1238 differ from `format_source`; 0 of 1238 changed against the base, for each N |
| `bench --threads N --root ~/.cargo/registry/src --check`, N = 2, 3, 4 | 0 differ, 0 changed, over 21,245 files (N = 2) and 21,466 files (N = 3, 4; the workspace's `cargo test` fetched 221 more registry files between the two runs; files not in the record are reported, not counted) |
| `conform -i` | 1219/1220 identical, 0 not idempotent (unchanged) |
| new tests, debug build (the `debug_assert`s in `parallel.rs` and `defer_gap` active) | 6 passed: the mixed file (`#!` line, inner attributes, blank-line `use` groups, comments between items, a skipped fn, a skipped `mod x;`, `macro_rules!`, doc comments, a same-line trailing comment, a trailing comment at the end) with 1 and 4 threads against the sequential output, with `reorder_imports` on and off; files with fewer than two units; unparsable source; skipped items and removed imports at both edges of a file; CRLF source with `newline_style` Auto and Windows; every 20th rust-analyzer fixture (61 files) with 2 and 4 threads |

## In-process measurements (measured, `bench -n 5 -w 1238`, best of 5 per file)

Per-file best-of-5 time of `format_source` against `format_source_with_config_parallel`
with 2 and 4 threads, summed by file size. `par/seq` is the ratio of the sums; the last
column counts the files where the parallel call was faster than the sequential one. Whole
corpus: sequential 3.017 s (4.61 MB/s), 2 threads 2.638 s, 4 threads 2.336 s.

| size | files | seq ms | 2 threads ms | 2t/seq | faster | 4 threads ms | 4t/seq | faster |
|---|---|---|---|---|---|---|---|---|
| 0–1 KiB | 536 | 15.9 | 28.7 | 1.81 | 1/536 | 30.6 | 1.92 | 1/536 |
| 1–2 KiB | 43 | 16.5 | 26.3 | 1.60 | 0/43 | 25.1 | 1.52 | 0/43 |
| 2–4 KiB | 98 | 70.4 | 92.0 | 1.31 | 3/98 | 86.5 | 1.23 | 8/98 |
| 4–8 KiB | 157 | 202.6 | 218.0 | 1.08 | 42/157 | 202.1 | 1.00 | 63/157 |
| 8–16 KiB | 157 | 395.6 | 375.8 | 0.95 | 99/157 | 340.7 | 0.86 | 117/157 |
| 16–32 KiB | 143 | 614.9 | 536.7 | 0.87 | 116/143 | 469.3 | 0.76 | 124/143 |
| 32–64 KiB | 71 | 580.5 | 490.1 | 0.84 | 58/71 | 417.5 | 0.72 | 66/71 |
| 64–128 KiB | 25 | 425.5 | 347.2 | 0.82 | 22/25 | 306.9 | 0.72 | 22/25 |
| 128–256 KiB | 6 | 201.7 | 166.7 | 0.83 | 5/6 | 140.8 | 0.70 | 6/6 |
| 256–512 KiB | 1 | 64.8 | 48.5 | 0.75 | 1/1 | 38.2 | 0.59 | 1/1 |
| 512–1024 KiB | 1 | 76.4 | 52.0 | 0.68 | 1/1 | 49.4 | 0.65 | 1/1 |

The crossover is between 4 and 8 KiB with 4 threads; below 4 KiB the spawn and the extra
red trees cost more than the split saves (a 0–1 KiB file takes 30 µs sequentially and
57 µs with 4 threads). The CLI's threshold is not this crossover: the CLI already runs
one file per core, and a file-level worker only gains from splitting a file when the
other cores would otherwise idle, i.e. for the largest files of a crate. The threshold is
chosen from the crate measurements below.

### Where the time goes on the five largest fixtures (measured)

`parse_floor rowan` (lexer, parser, rowan tree; best of 10) next to the per-file
`format_source` time and the 4-thread time above, and the unit structure from a scratch
build of the candidate with a timer around every unit (`--threads 2`, so unit times
include some cache contention; not committed):

| file | bytes | parse ms | seq ms | parse share | 4 threads ms | seq / 4t | units | largest unit ms (bytes) | largest unit's share of unit work |
|---|---|---|---|---|---|---|---|---|---|
| `ide-db/src/generated/lints.rs` | 639,689 | 9 | 76.4 | 12% | 49.4 | 1.55x | 11 | 37.6 (287,600) | 49% |
| `syntax/src/ast/generated/nodes.rs` | 322,295 | 19 | 64.8 | 29% | 38.2 | 1.70x | 1,958 | 1.2 (5,492) | 2% |
| `hir/src/lib.rs` | 227,843 | 13 | 60.1 | 22% | 29.8 | 2.02x | 238 | 8.6 (36,831) | 16% |
| `ide/src/hover/tests.rs` | 210,028 | 3 | 14.0 | 21% | 8.5 | 1.64x | 259 | 0.8 (366) | 4% |
| `rust-analyzer/src/config.rs` | 144,317 | 6 | 22.2 | 27% | 12.9 | 1.72x | 104 | 15.3 (12,199) | 49% |

Stitching (gap replay and appending) takes 0.3–0.7 ms per file. The bound on the
4-thread time is the parse plus the longest unit plus a quarter of the rest:
`lints.rs` is two `const` arrays of 288 KB and 223 KB (one unit each), so its 4-thread
time is the parse plus the larger array; `config.rs` has one `impl` of 15 ms; `nodes.rs`
and `hir/src/lib.rs` split well and are bounded by the parse share.

## Process-level measurements (measured)

`hyperfine -N -i -w 2 -r 15`, `--check` mode, wall time in ms: min, mean ± σ, and the
mean CPU time (user + system, all threads) of the same runs. `base` is the `8f6b2b0` (was `d797292`)
CLI (one thread per core at file level, static chunks); `new` is the candidate with its
defaults (4 threads, largest-first queue, files of 128 KiB and more split over 4 threads);
`new --threads 1` is the candidate on one thread. "all cores": no pinning; "core 0":
every command under `taskset -c 0`, so the candidate's 4 threads share one core and its
per-core cost is visible. The host is a shared 4-vCPU VM; σ of 10–18% on a 100 ms run
is usual here, so the ratios are of minima.

### Single files (one file argument)

| file | cores | base min | base mean ± σ | base cpu | new min | new mean ± σ | new cpu | new `--threads 1` min | mean ± σ | cpu | base/new (min) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `ide-db/src/generated/lints.rs` (640 KB) | all | 93.7 | 103.5 ± 10.8 | 100.8 | 59.0 | 69.7 ± 8.5 | 108.2 | 88.5 | 101.6 ± 13.5 | 97.0 | 1.59x |
| | core 0 | 88.1 | 95.6 ± 7.8 | 93.8 | 88.3 | 108.0 ± 17.6 | 106.5 | 92.4 | 102.5 ± 9.0 | 98.4 | 1.00x |
| `syntax/src/ast/generated/nodes.rs` (322 KB) | all | 81.6 | 102.4 ± 16.9 | 99.8 | 52.7 | 59.2 ± 6.5 | 106.4 | 75.9 | 88.4 ± 16.0 | 87.4 | 1.55x |
| | core 0 | 77.2 | 101.0 ± 18.4 | 99.3 | 76.6 | 88.9 ± 15.1 | 85.4 | 74.5 | 85.3 ± 16.2 | 83.8 | 1.01x |
| `hir/src/lib.rs` (228 KB) | all | 68.1 | 78.6 ± 12.2 | 76.9 | 36.3 | 40.5 ± 3.6 | 80.3 | 66.2 | 76.1 ± 12.3 | 74.8 | 1.88x |
| | core 0 | 66.9 | 70.6 ± 3.2 | 69.6 | 66.7 | 75.4 ± 9.5 | 74.0 | 69.4 | 76.3 ± 7.4 | 73.5 | 1.00x |
| `ide/src/hover/tests.rs` (210 KB) | all | 18.8 | 22.5 ± 3.1 | 21.4 | 12.5 | 13.0 ± 0.4 | 25.8 | 18.5 | 20.2 ± 1.4 | 19.6 | 1.50x |
| | core 0 | 18.2 | 20.4 ± 3.6 | 20.1 | 18.2 | 19.4 ± 1.2 | 18.6 | 18.4 | 23.0 ± 4.0 | 21.9 | 1.00x |
| `rust-analyzer/src/config.rs` (144 KB) | all | 27.6 | 29.4 ± 1.5 | 28.6 | 17.3 | 19.8 ± 3.0 | 34.2 | 27.5 | 29.5 ± 2.4 | 28.7 | 1.60x |
| | core 0 | 28.5 | 30.0 ± 1.5 | 29.3 | 27.6 | 29.0 ± 1.0 | 28.3 | 30.5 | 39.4 ± 4.5 | 38.5 | 1.03x |

- All five files: 1.50x–1.88x on four cores (minima); on the means, 1.49x–1.94x.
- Pinned to one core, the candidate's minimum is within 1% of the base's on every file
  (its 4 threads then run one after the other); the means are within noise except
  `lints.rs` (108.0 against 95.6, σ 17.6).
- CPU time (all cores) rises with the split: +7% (`lints.rs`), +7% (`nodes.rs`), +4%
  (`hir/src/lib.rs`), +21% (`hover/tests.rs`), +20% (`config.rs`). This is the per-core
  cost: three more red trees and item lists, one `RunState` (line table, padding) per
  worker, cache traffic between cores, and the spawn. It is largest on the two files with
  the shortest run.
- `new --threads 1` is the base's time: the file-level changes cost nothing on one file.

### Crate directories (the CLI's own scheduling)

Two runs. The first (same session as the single-file table) compares `base`, `new` and
`new --threads 1`, where `new` here still had the 32 KiB threshold the first commit shipped
with. The second compares the scheduling alternatives, each a scratch build of the
candidate differing only in `PARALLEL_ITEMS_MIN_BYTES` and the item-thread count:

- `filesonly`: largest-first queue, no file split (threshold ∞);
- `t8k`, `t32k`, `t128k`: files of at least 8, 32, 128 KiB split over 4 threads;
- `t32k_half`: 32 KiB, split over 2 threads.

`t128k` is the committed configuration (`2f794d7` (was `d60c1fa`) shipped `t32k`; `53a4f4b` (was `f847a87`) moves it).

First run, all cores and core 0 (wall ms; min, mean ± σ, cpu):

| crate (files, bytes) | cores | base min | base mean ± σ | base cpu | new (32 KiB) min | mean ± σ | cpu | new `--threads 1` min | mean ± σ | cpu |
|---|---|---|---|---|---|---|---|---|---|---|
| `hir` (15, 624 KB) | all | 92.0 | 115.2 ± 18.2 | 240.2 | 60.9 | 72.2 ± 6.9 | 240.9 | 182.3 | 200.1 ± 19.6 | 197.5 |
| | core 0 | 186.1 | 201.2 ± 13.0 | 199.1 | 175.1 | 199.5 ± 23.1 | 197.2 | 182.0 | 218.8 ± 26.9 | 213.6 |
| `hir-ty` (122, 2.8 MB) | all | 190.5 | 203.6 ± 12.1 | 665.6 | 183.5 | 209.2 ± 13.9 | 760.7 | 593.1 | 674.3 ± 51.6 | 658.0 |
| | core 0 | 580.4 | 635.5 ± 37.8 | 626.8 | 583.1 | 631.1 ± 44.8 | 623.8 | 628.6 | 701.4 ± 62.6 | 689.0 |
| `ide` (89, 1.4 MB) | all | 112.6 | 125.1 ± 7.9 | 383.7 | 96.3 | 104.3 ± 6.6 | 363.3 | 301.9 | 325.5 ± 26.2 | 320.4 |
| | core 0 | 314.7 | 342.8 ± 23.6 | 335.4 | 289.6 | 323.0 ± 36.2 | 318.1 | 305.4 | 344.2 ± 27.0 | 333.1 |
| `ide-db` (49, 1.5 MB) | all | 124.0 | 138.0 ± 12.1 | 252.1 | 79.7 | 92.4 ± 11.5 | 283.6 | 214.3 | 249.2 ± 27.8 | 239.4 |
| | core 0 | 218.2 | 258.7 ± 36.9 | 254.8 | 215.0 | 230.3 ± 15.5 | 226.6 | 213.7 | 231.7 ± 19.9 | 228.8 |
| `syntax` (37, 1.0 MB) | all | 104.8 | 129.2 ± 20.1 | 230.0 | 66.1 | 76.3 ± 6.5 | 255.0 | 187.0 | 214.0 ± 23.9 | 208.4 |
| | core 0 | 184.1 | 207.5 ± 30.5 | 204.2 | 182.4 | 201.9 ± 29.5 | 201.1 | 181.4 | 193.6 ± 21.6 | 192.6 |

Second run, all cores (wall ms; min, mean ± σ, cpu):

| crate | base | `filesonly` | `t8k` | `t32k` | `t128k` (committed) | `t32k_half` |
|---|---|---|---|---|---|---|
| `hir` | 83.1, 96.1 ± 8.6, 210 | 67.1, 72.8 ± 5.8, 204 | 65.9, 72.7 ± 5.7, 242 | 56.8, 66.2 ± 6.8, 224 | **53.1**, 61.6 ± 7.5, 219 | 57.4, 70.3 ± 9.9, 228 |
| `hir-ty` | 203.8, 237.4 ± 32.1, 744 | 160.4, 170.4 ± 10.1, 654 | 194.2, 213.4 ± 12.0, 727 | 171.3, 187.1 ± 9.3, 674 | **154.6**, 175.2 ± 15.1, 664 | 162.8, 179.5 ± 12.3, 673 |
| `ide` | 98.9, 112.4 ± 8.9, 331 | **80.0**, 89.2 ± 9.3, 335 | 104.8, 123.8 ± 11.2, 411 | 98.4, 110.5 ± 9.1, 384 | 82.6, 91.5 ± 7.0, 341 | 99.8, 113.1 ± 7.3, 414 |
| `ide-db` | 129.9, 153.2 ± 18.9, 269 | 87.1, 102.6 ± 14.5, 245 | 80.5, 98.5 ± 8.7, 296 | 74.0, 83.7 ± 7.6, 256 | 79.1, 86.8 ± 7.0, 263 | **67.5**, 79.1 ± 8.9, 242 |
| `syntax` | 104.2, 116.5 ± 10.6, 212 | 76.6, 93.5 ± 13.0, 222 | 71.2, 80.1 ± 5.4, 249 | 62.4, 74.2 ± 7.2, 246 | **57.2**, 61.1 ± 4.0, 216 | 68.9, 76.6 ± 7.2, 232 |
| sum of minima | 619.9 | 471.2 | 516.6 | 462.9 | **426.6** | 456.4 |

- **The queue alone** (`filesonly`) takes 1.24x–1.49x off every crate: with static
  chunks of the path-sorted list, a chunk that holds `generated/lints.rs` or
  `hir/src/lib.rs` is one thread's whole share and the other threads finish early. The
  base's core-0 figures show the same work on one core (`hir` 186 ms against 92 ms on
  four: 2.0x on four cores), the queue's one-thread figure is the base's.
- **Splitting medium files hurts.** `t8k` and `t32k` are slower than `filesonly` on
  `hir-ty` and `ide`, the crates with many 8–100 KB files: four file workers each
  spawning four item workers oversubscribe the cores, and the CPU time rises 10–25%.
  `t32k_half` halves the oversubscription and is better than `t32k` on `ide-db` only.
- **Splitting the largest files helps** where a crate's wall time is one file's:
  `t128k` is the best on `hir`, `hir-ty`, `syntax`, within 3% of the best on `ide`,
  and 17% behind `t32k_half` on `ide-db` (whose largest file, `lints.rs`, is one unit
  of 288 KB plus one of 223 KB: splitting it 4 ways gains little over 2 ways, and the
  other 48 files run meanwhile).
- Against the base, `t128k` is 1.24x–1.82x on the five crates (minima); pinned to one
  core (first run, `new` with the 32 KiB threshold, more splitting than the committed
  one), the candidate is within 1–6% of the base on every crate.

**Alternatives not built.** A global item queue across files (every worker takes items
from any file) would remove the oversubscription and the threshold, at the price of
parsing every file before formatting any, a parsed tree held per file in flight, and the
gap replay for every file on the thread that finishes it; the measured gap between
`t128k` and the per-crate best (3% on `ide`, 17% on `ide-db`) bounds what it could gain
on these crates. Choosing the item-thread count from the number of idle file workers was
not tried: the count is fixed when the file starts, and with largest-first order the
largest files start while every worker is busy.

## Interpretation

- **Output.** Identical to the sequential path on every file of both corpora at every
  thread count tried, and to the base's hashes: the hypothesis's first clause holds. The
  gap-deferral design makes the stitching exact by construction for every item kind: the
  worker records what the sequential visitor's first gap call would have been and the
  main thread makes that call on the real buffer, so no per-kind analysis of where a
  unit's text starts is needed. The one residual assumption is that nothing in a unit
  reads the buffer's state before the unit's first write other than through that call
  (the primed `\n` covers `output_at_start`, the trailing-newline count and
  `last_line_width` for the cases found); the corpora and the debug assertions are the
  evidence for it, not a proof.
- **Gain.** 1.50x–1.88x on the five largest fixtures on four cores, not "close to the
  core count". The bound is the parse (12–29% of the file's time, sequential) plus the
  longest unit plus the sequential gaps and assembly: `lints.rs` has one unit of 49% of
  its item work, `config.rs` one of 49%, so their four-thread time is the parse plus that
  unit; `hir/src/lib.rs`, whose largest unit is 16%, gets 2.0x and is bounded by its
  parse. The falsification condition (under 1.5x) is not met, on minima or means, but the
  mechanism's ceiling on this corpus is about 2x, not 4x, and a file whose time is one
  item (a large `const` table, one `impl`) gains nothing from it.
- **Per-core cost.** Pinned to one core, the candidate's minima match the base's within
  1% on files and 1–6% on crates; CPU time on four cores rises 4–21% for a single file,
  more on the shorter runs. This is the price of the split and is reported, not hidden:
  on a loaded machine the gain shrinks by that much.
- **The scheduling fix is the larger half of the crate gain.** 1.24x–1.49x from the
  largest-first queue alone against 1.24x–1.82x with the split as well. It is independent
  of this experiment's hypothesis and would be kept on its own.

## Decision: kept, with the hypothesis revised

Kept: the output gates pass, the five largest fixtures gain 1.50x–1.88x, the crates
1.24x–1.82x, and the single-core cost is within noise. The hypothesis is revised rather
than confirmed: the gain is bounded by the parse and the longest item, about 2x on this
corpus on four cores, not the core count. The library default stays single-threaded; the
parallel path is an additive API the CLI uses for files of 128 KiB and more.

## Follow-up

- The parse is now the largest sequential part of a large file (E5 in the programme:
  `NodeCache`, `to_input`, `intersperse_trivia`); any gain there raises this ceiling.
- Units below the top level (the items of an `impl`, the statements of a large `fn`, the
  elements of a `const` array) are what `lints.rs` and `config.rs` need; X12b did not
  test their independence and this experiment does not assume it.
- The CPU overhead per split (4–21%): the per-worker item list and red root are the
  known part; a profile of the four-thread run against the one-thread run would say how
  much is contention.
- `t32k_half` on `ide-db` suggests 2 item threads for files between 128 KiB and, say,
  256 KiB when other files are in flight; not measured beyond that one crate.
- The programme's numbers in [experiments.md](../experiments.md) are from the 2.80GHz
  host; the integrator should re-measure on one host before promoting.
