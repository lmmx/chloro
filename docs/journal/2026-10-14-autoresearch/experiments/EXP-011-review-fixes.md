# EXP-011: fixes for the ten findings of the cache, parallel-items and CST review

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). The review is
[review-cache-parallel-cst.md](review-cache-parallel-cst.md); its numbering is used below.
Branch `20261014-review-fixes` from `1eae9f6` (the integration head), worktree
`/home/user/wt/review-fixes`. Not a performance experiment: the claim is that every finding
is closed by a small change with a test, and that no output on either corpus moves.

## Changes

| commit | findings | change | test |
|---|---|---|---|
| `a640834` | 4 | files named on the command line are canonicalised, and the whole list is sorted and de-duplicated before formatting, so a file reachable twice (named twice, through a symlink, or both named and discovered) is formatted, counted and written once | `chloro/tests/cli_files.rs`: a directory with `b.rs` and a symlink `a.rs -> b.rs`, given as the directory, `b.rs` twice and `a.rs`; `--check` reports 1 file (5 before the fix), `--write -v` processes and writes once |
| `09339ea` | 1 | the temporary file of a save is `.<cache file name>.<pid>.<nanos since epoch>.tmp`, opened with `create_new`; a name that exists is retried with another suffix up to 16 times, then the save is skipped with `AlreadyExists`; on failure only a file this save created is removed | `cache::tests::temporary_file_names_its_target_and_is_never_shared`, `save_leaves_another_saves_temporary_file_alone` |
| `cd8ceea` | 6 | `format_text` and `format_text_parallel` return `None` (the input is echoed) for a source longer than `u32::MAX` bytes, instead of wrapping the tree's `u32` offsets | `formatting::tests::texts_longer_than_u32_offsets_are_refused` (the predicate; a 4 GiB input is not built in a test) |
| `0edf434` | 5, 7, 8 | every unit record is checked before anything is written to the main buffer (`parallel::stitchable`): a unit without a gap call must have written and consumed nothing, and no unit may have written text before its first gap call (`UnitRecord::broken`, set by `defer_gap`, which then writes the gap itself); a file whose records fail the check is formatted by the sequential walk, which gives the same output since the main visitor holds only what `format_separate_mod` writes before the items; debug builds still assert. A worker's panic is re-raised with `resume_unwind(payload)`. `format_missing_before_removed_import` saturates the line count at 0 | the equivalence test below runs the parallel path with the debug assertions on; no input reaches the fallback |
| `9ee9804` | 3 | `chloro-core/tests/parallel_equivalence.rs`: every eleventh fixture (113 files), under the default options and with `reorder_imports`/`reorder_modules` off, at 2 and 4 threads, equals `format_source_with_config` byte for byte (6.6 s in a debug build) | the test |
| `72bd4b1` | 9, 2, 10 | the header holds the whole 32-byte file key `BLAKE3(tool ‖ config)` (48-byte header, `FORMAT_VERSION` 2) and `parse` rejects a file whose key is not the one its name was derived from; the cache is opened before file discovery; Linux hashes the running image through `/proc/self/exe`; the executable's size and modification time are recorded at open and compared before save, and a save whose values differ returns an error of kind `Other` and writes nothing; the stale-file sweep also removes `.tmp` files of chloro's naming older than `STALE_FILE_AGE` | `cache::tests::parse_round_trip_and_rejects_corruption` (another key), `open_reads_the_file_its_key_names_and_rejects_a_copy` (a cache file copied under another configuration's name reads as empty), `save_is_skipped_when_the_executable_changed`, `stale_cache_and_temporary_files_are_removed`, `tool_key_is_the_running_executable`; `tests/cli_cache.rs` updated for the 48-byte header |

Behaviour changes a user can see:

- Print mode writes the files in sorted canonical-path order; before, in command-line order.
- Every cache file written before this change is ignored (the format version is part of the
  tool key, so the file name changes as well) and removed by the stale sweep after 30 days.
- A duplicate path or a symlink to a listed file counts once in `--check`'s report.
- `-v` logs "Cache: ..." before "Found N Rust file(s)".

What is still open from the review: finding 2's residual window (a binary replaced between
process start and `Cache::open`, now the first thing `main` does after parsing the
arguments) on macOS and Windows; on Linux there is none. Finding 3 is guarded by the test,
not removed: the cache key still does not say which path observed the fixed point.

## Gates (on `72bd4b1`, release build in the worktree, `bench` sha256 `f85a8f87c2b15621…`)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | chloro: 13 unit, `cli_cache` 10, `cli_files` 1, 1 doc test; chloro-core: 183 unit, `basic` 4, `conformance` 19, `nodes` 10, `parallel_equivalence` 1, `rustfmt_version` 10; all pass |
| `bench -n 1 --check fixtures-e8d87c2.tsv` (`--root` the main checkout's fixtures) | 0 of 1238 outputs changed |
| the same with `--threads 4` | 0 of 1238 differ between 4 threads and `format_source`; 0 of 1238 changed |
| `bench -n 1 --root ~/.cargo/registry/src --check registry-e8d87c2.tsv` | 0 of 20850 outputs changed |
| the same with `--threads 4` | 0 of 20850 differ between 4 threads and `format_source`; 0 of 20850 changed |
| `conform -i` (`taskset -c 3`) | 1220/1220 identical, 0 not idempotent, `rustfmt_version=1.9.0`, 13.8 MB in 1.529 s |
| `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, 13.8 MB in 1.448 s |
| `benchmarks/profile.sh --core 2 --runs 2` | `Ir` 312,734,679 and 312,732,924 against the best-known-good 312,734,032: within the run-to-run spread (about 2 K) |
| `config_matrix -n 1220` | not run: the diff touches no option-dependent path (the changed formatter code is the parallel stitching, which the thread-mode golden checks cover on both corpora, and the removed-import line count, on the default path of both corpora) |

Decision: kept; for the integrator. The best-known-good moves to the merge commit with the
same `Ir` and the same goldens.

## Process note

The fixes were started by an agent, which landed findings 4 and 1 and then stopped when the
container's disk allowance ran out while linking a test binary (31 MB free); it had removed
two other worktrees' `debug/incremental` directories (about 3.2 GB) and was then refused
further `cargo test` runs. The orchestrator freed the build caches of five merged
worktrees with `cargo clean` (about 10 GB; `/home/user/wt/{hot-paths,conformance,flat-port,
rustfmt-version}` and the agent's two), and wrote, tested and gated findings 6, 5, 7, 8, 3,
9, 2 and 10 itself. No evidence directory was touched.
