# EXP-015: an item-level cache (formatted top-level units)

Experiment E15 of the [autoresearch programme](../../2026-10-14-autoresearch.md); procedure
and gates in [protocol.md](../protocol.md). It implements §1 "An item-level cache" of the
design study [../../2026-10-15-extensions-theory/incremental-and-process.md](../../2026-10-15-extensions-theory/incremental-and-process.md),
on the unit definition of [EXP-001](EXP-001-parallel-items.md) and the independence result of
[X12b](../../2026-10-10-item-independence.md). Labels as in the journal: **measured**,
**read**, **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-item-cache`, worktree `/home/user/wt/item-cache` |
| base | `2b3c761` (was `b69e00f`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`; formatter code of the best-known-good `bd9c768` (was `bdfb9d1`)) |
| commits | `fa71ec5` (was `07cab15`) (core), `a167552` (was `a4db417`) (CLI), `02ad66d` (was `8114d8b`), `5fb36c5` (was `c000c66`) and `7e64238` (was `2f14ffa`) (tests, harness), this record |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`; release profile of the workspace (fat LTO, one codegen unit); `cargo build --release -p chloro --features cli` and `-p chloro-core --example bench --example conform --example unit_cache` |
| base binaries | built in the same worktree at `2b3c761` (was `b69e00f`) before any change and copied aside (`chloro` 2,017,472 B; new `chloro` 2,088,808 B, +71 KB) |
| host | 4 vCPU, Linux 6.18.44, hyperfine 2.0.0, shared with other agents' builds and runs (load average 2.1–4.5 during the measurements). Ratios between binaries measured in one window are the claim; absolute times are not quiet-host figures and are not comparable with [M3](../measurements/m3-quiet-host.md) |

## Hypothesis and falsification

**Hypothesis.** A cache of per-unit outputs (a unit is one top-level item or one reorderable
group, as `parallel.rs` defines it), keyed by everything the unit's output depends on, lets a
re-format after an edit to one item do only the parse, a hash of each unit's text, the edited
unit and the gaps, with output byte-identical to `format_source_with_config`. The study's bound:
a one-item edit in `lints.rs` costs 0.23–0.63 of the file's time, `nodes.rs` 0.41; a cold run
gains nothing.

**Falsified if** any byte differs on either corpus (cold or warm), or if the warm re-format of a
file over 128 KiB after a one-item edit is not at least 1.5x faster than today's `--no-cache`
run of the same file.

## Design as built (read)

**Library.** `chloro_core::format_source_with_unit_cache(source, config, threads, &dyn
UnitCache) -> String`, a sibling of `format_source_with_config_parallel`; the trait
`UnitCache { get(&UnitKey) -> Option<CachedUnit>; insert(&UnitKey, CachedUnit) }`, the key
`UnitKey` (context bytes, unit text; `UnitKey::new` public for implementations' tests) and the
opaque value `CachedUnit` (`to_bytes`/`from_bytes`, `heap_size`). Off unless a caller passes a
cache: `format_source`, `format_source_with_config` and `format_source_with_config_parallel`
take the paths they took before (the last one through the refactored `format_units` with every
unit to do, which spawns the same threads).

With a cache, every file takes the units path, whatever `threads` is
(`parallel::format_file`): the main thread parses, runs the whole-file checks, computes the
units (`unit_ranges`), looks each unit up, formats only the units not found
(`format_units`, on the calling thread when `threads <= 1` or one unit is missing, else on
`threads` scoped threads, longest first), stores each formatted unit, and stitches the outputs
with the same `stitchable` check and the same replay (`stitch`) as E1.

**Key** (`unit_cache.rs` `key_context`, `parallel.rs` `format_file`):

| input | where it enters | where enforced |
|---|---|---|
| the unit's text after normalisation (no BOM, `\n` endings), from the first item's `span().lo()` (outer attributes and doc comments included) to the last item's `span().hi()`; a reorderable group is one unit, so the whole group's text | `UnitKey::text` | `format_units_cached` (`unit_span`) |
| the whole `Config` | `Config::key_bytes`, moved from the CLI's `config_key_bytes` (identical bytes: the CLI's pinned-key test passes unchanged; the destructuring without `..` still fails to compile on a new field) | `key_context`; `rustfmt_version`, `edition`, `style_edition`, `reorder_*` all in it |
| the file's `#![rustfmt::skip::macros(..)]`/`::attributes(..)` names, as every worker starts with them (`update_skip_context(file_attrs)`) | `SkipContext::write_key` (sorted, length-prefixed), appended to the context per file | `format_file` |
| cache format version and crate version | `key_context` | — |
| the build of chloro | not in the library key (the library cannot hash its own binary); the CLI selects the cache file by the fixed-point cache's file key `BLAKE3(tool key ‖ config key)` and checks the whole key in the header | `units.rs` `Units::new`, `parse` |

**Value** (`CachedUnit`, `unit_cache.rs`): E1's `UnitOutput` minus `broken`, with

- the `GapOp` variant and its position minus the unit's start (signed);
- `last_pos` minus the unit's start;
- the source from the unit's end to `last_pos` when the unit consumed text past its last item
  (`tail`), compared with the current source on a hit;
- the text, or nothing when it equals the source from the unit's start to `last_pos` (an
  already formatted unit; a hit reads it back from the source, which the key and the tail
  check make equal);
- `macro_failure`, `lost_comment`, `format_failure` (a hit with `format_failure` makes the file
  unchanged, as a fresh unit would);
- `skipped_range` and `output_line_ranges` as recorded, and the unit's 1-based source line when
  `skipped_range` is not empty: a hit on such a record requires the unit to start on the same
  line (see "What surprised").

Rebasing happens in `CachedUnit::from_output` (positions minus `lo`) and `to_output` (plus the
new `lo`, every position checked to lie inside the source, the tail compared, the line checked);
a record that fails a check is a miss and the unit is formatted. A `broken` output is never
stored.

**Whole-file, recomputed on every call** (read): `exceeds_offset_range`, lexer and parser
errors, `rejected_by_rustc`, `#![rustfmt::skip]` (`format_text_parallel`, before
`format_file`), `disable_all_formatting` (the entry point), the `stitchable` fallback (on the
assembled outputs, cached or fresh), the text between units (replayed by `stitch` from the
fresh parse), the trailing-newline fix-up and `newline_style` (on the assembled text).

**CLI** (`chloro/src/cli/units.rs`). A file the fixed-point cache does not answer is formatted
with `format_source_with_unit_cache` against `<cache-dir>/<file key>.chloro-units`: header
`CHLOROUC`, format version, generation, the 32-byte file key; records of a BLAKE3 digest of
`len(context) ‖ context ‖ text`, the generation last seen, the record length and
`CachedUnit::to_bytes`. The map is a `Mutex<HashMap<Digest, (u32, CachedUnit)>>` shared by the
file workers. The file is read on the first fixed-point miss (an unchanged tree never reads
it), written (temporary file, rename) only when a unit was stored and the executable's size
and modification time are those taken at open, bounded to 64 MiB of records (least recently
seen dropped), and swept with the other stale cache files after 30 days.

Flags (as of `d672456` (was `cafc1e8`)): the unit cache is **opt-in**. `--unit-cache` turns it on; `--no-cache`
turns both caches off and overrides `--unit-cache`. Reason: a run that finds nothing in it pays
for hashing and storing every unit and writing the file, +10–15% wall time and +4.6%
instructions on a cold `--check` of the fixture tree (gate 4 below, and again on the merged
branch in the follow-up section), which is not acceptable as a default for cold CI runs until a
resident process or a cheaper cold path amortises it. The first measurements below were taken
at `5fb36c5` (was `c000c66`), where the unit cache rode the fixed-point cache by default and `--no-unit-cache`
turned it off; that flag is gone. `-v` prints the units looked up, found and stored, the file read and
the record bytes held.

## Changes

| commit | change | test |
|---|---|---|
| `fa71ec5` (was `07cab15`) | `unit_cache.rs` (trait, key, value, rebasing, serialisation), `parallel.rs` (`CacheUse`, `format_units_cached`, `format_units` over a subset with the calling-thread case, `stitch`), `Config::key_bytes`, `SkipContext::write_key`, `format_source_with_unit_cache` | 6 unit tests: cold/warm at 1 and 4 threads; the key covers the configuration and the skip names; records are relative and rebased (a record from one position equals the record of the same unit 40 bytes and 3 lines further down, and replays at the new `lo`); a tail is checked, a gap before the source start refused; skipped ranges pin the record to its line; every record round-trips through bytes and every truncation, extension and unknown flag is refused |
| `a167552` (was `a4db417`) | the CLI's unit store, `--no-unit-cache`, the verbose report; `Cache::key`, `Cache::executable_unchanged`; the stale sweep covers `.chloro-units`; `cli_cache.rs`'s `cache_files` counts fixed-point files only | 3 unit tests in `units.rs`: stored units read back and found, another key reads nothing; a corrupt file is ignored; a save after the executable changed writes nothing |
| `02ad66d` (was `8114d8b`) | `chloro-core/tests/unit_cache_equivalence.rs` (every 11th fixture, default and no-reordering, 1 and 4 threads: cold, warm with no unit formatted, moved down a line); `chloro/tests/cli_unit_cache.rs` (a 167 KB fixture edited in one item: `--check` and `--write` equal the sequential formatter, `-v` reports `(104, 103, 1)`; the flags leave the unit cache alone; an unchanged tree never reads it); `examples/unit_cache.rs` | the tests |
| `5fb36c5` (was `c000c66`) | harness options `--every`, `--blank-max-bytes`, edit unit sizes | — |
| `7e64238` (was `2f14ffa`) | harness counts records with a tail or a moved gap | — |

## Gates

**1. fmt, clippy, tests** (on `02ad66d` (was `8114d8b`)'s tree; `5fb36c5` (was `c000c66`) and `7e64238` (was `2f14ffa`) change only the example, rechecked with `cargo fmt --all --check` and a release build):

| command | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | chloro: 17 unit, `cli_cache` 10, `cli_files` 1, `cli_unit_cache` 3, 1 doc test; chloro-core: 189 unit, `basic` 4, `conformance` 19, `nodes` 10, `parallel_equivalence` 1, `rustfmt_version` 10, `unit_cache_equivalence` 1; all pass |

**2. Output identity** (`target/release/examples/unit_cache`; each file formatted by
`format_source_with_config` and twice by `format_source_with_unit_cache`, cold then warm, with
an exact in-memory cache keyed by the whole key, shared across files; `--shift` also formats
the file with a comment line inserted at its start against the warm cache; `--blank` formats,
for each unit stored, the file with every byte outside the unit replaced by a space, newlines
kept, cached and uncached):

| corpus | command | result |
|---|---|---|
| fixtures, 1238 files | `unit_cache --shift --threads 1` | 13,644 units looked up cold, 364 found cold (equal units in other files), 13,280 stored, 13,644 found warm; shifted: 1227 files, 13,595 units, 13,203 used (392 line-pinned records refused); **0 differences** (cold, warm, shifted) |
| fixtures | `unit_cache --shift --threads 4` | the same counts; **0 differences** |
| fixtures | `unit_cache --blank` | 13,280 units checked with the rest of the file blanked, all found; **0 differences** |
| registry, 24,582 files (the corpus has grown since the golden's 20,850) | `unit_cache --root ~/.cargo/registry/src --shift --threads 1` | 1,895,787 units looked up cold, 1,135,986 found cold (equal units in other files and crate versions), 759,801 stored, 1,895,786 found warm (1 file with a warm miss, below); shifted: 21,309 files (files starting with `#!` or a BOM left out), 1,861,861 units, 1,780,353 used (81,508 line-pinned records refused); peak 482 MB held by the exact cache; **0 differences** (cold, warm, shifted); 137.9 s |
| registry | the same with `--threads 4` | the same counts; **0 differences**; 153.5 s |
| registry, every 4th file (6,146), blanked check on files ≤ 64 KiB | `unit_cache --root ~/.cargo/registry/src --blank --blank-max-bytes 65536 --every 4` | 483,004 units looked up, 483,004 found warm; 86,512 units checked with the rest of the file blanked (the units each file stored, in files of at most 64 KiB without `\r` or a BOM), all found; **0 differences** |
| registry, all files, record shapes | `unit_cache --root ~/.cargo/registry/src` (harness at `7e64238` (was `2f14ffa`)) | 0 of 759,442 stored records have a tail or a gap position other than the unit's start; the same on the fixtures (0 of 13,280); 0 differences |

The one registry file with a warm miss is `zerocopy-derive-0.8.63/tests/enum_to_bytes.rs`: it
holds `util_assert_impl_all!(HasData: imp::IntoBytes);` twice (lines 124 and 136), a macro
call left as written, so its record carries skipped ranges and is pinned to a line; the second
occurrence overwrites the first's record, and the warm pass formats the first again. Correct
by construction, measured here as one formatted unit.

Cache off, against the best-known-good goldens (`bench --check`, `--root` as recorded):

| command | result |
|---|---|
| `bench --root /home/user/chloro/chloro-core/tests/conformance/fixtures --check fixtures-e8d87c2.tsv` | 0 of 1238 outputs changed |
| the same with `--threads 4` | 0 of 1238 differ between 4 threads and `format_source`; 0 of 1238 changed |
| `bench --root /root/.cargo/registry/src --check registry-e8d87c2.tsv` | 0 of 24,582 changed (3,732 newer files not recorded; 0 of the 20,850 recorded) |
| the same with `--threads 4` | 0 of 24,582 differ between 4 threads and `format_source`; 0 changed |
| `conform -i` | 1220/1220 identical, 0 not idempotent, `rustfmt_version=1.9.0` |
| `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent |

**3. The edit scenario.** The five largest fixtures copied to scratch and formatted first (the
editor case: a formatted file), then edited by `flip.py`: one letter of the last string literal
that starts with a lowercase ASCII letter changed (`z`, or `a` for a `z`), which keeps the file
formatted. The edit lands in `CLIPPY_LINT_GROUPS` (a 57,403-byte unit) in `lints.rs`, a
217-byte `impl fmt::Debug` in `nodes.rs`, `impl<'db> Type<'db>` (37,633 bytes) in
`hir/src/lib.rs`, a 4,034-byte test function in `hover/tests.rs` and `mod tests` (7,254 bytes)
in `config.rs`. Caches warmed by one `--check` of the formatted file, snapshotted, and restored
before every timed run (`--prepare restore.sh SNAP DIR`), so each run is the first run after
the edit.

In process (`unit_cache --edit FILE FILE.edited -n 20`; the warm cache cloned per round; best
of 20; same window):

| file | bytes | units | found | `format_source_with_config` | parallel(4) | unit cache, 1 thread | ratio | unit cache, 4 threads | ratio |
|---|---|---|---|---|---|---|---|---|---|
| `lints.rs` | 639,771 | 11 | 10 | 26.62 ms | 15.91 ms | 8.14 ms | 0.306 | 7.78 ms | 0.297 |
| `nodes.rs` | 358,382 | 1958 | 1957 | 35.16 ms | 22.82 ms | 16.57 ms | 0.471 | 16.11 ms | 0.449 |
| `hir/src/lib.rs` | 236,066 | 238 | 237 | 31.87 ms | 16.38 ms | 13.89 ms | 0.436 | 14.48 ms | 0.436 |
| `hover/tests.rs` | 210,967 | 259 | 258 | 6.58 ms | 3.94 ms | 1.74 ms | 0.264 | 1.91 ms | 0.282 |
| `config.rs` | 169,196 | 104 | 103 | 12.32 ms | 7.46 ms | 4.60 ms | 0.373 | 4.68 ms | 0.373 |

Three repetitions, `taskset -c 3`, best of 10 each, 1 thread: `lints.rs` 0.312, 0.313,
0.304; `nodes.rs` 0.463, 0.444, 0.464; `hir/src/lib.rs` 0.448, 0.439, 0.443;
`hover/tests.rs` 0.271, 0.284, 0.286; `config.rs` 0.370, 0.378, 0.363: below the study's 0.5x
in 3 of 3 on every file.

Cold and warm passes of the identity harness on the unedited fixtures (`--threads 1`, one run
each, in the window of the registry runs): `lints.rs` reference 33.75 ms, cold 30.83, warm 8.11
(0.240); `nodes.rs` 49.35, 51.79, 20.80 (0.421); `hir/src/lib.rs` 39.20, 38.07, 11.26 (0.287);
`hover/tests.rs` 6.98, 8.58, 2.05 (0.293); `config.rs` 14.64, 14.60, 4.53 (0.310).

Process level: `hyperfine -N -i -w 2 -r 20`, the four variants in one invocation per file and
pinning (`taskset -c 3`, then unpinned), two windows. Cells are min / mean ± σ / peak RSS (MiB,
hyperfine's maximum), times in ms. Variants: `--no-cache`; fixed-point cache only
(`--no-unit-cache`, cache warmed on the unedited file: the edit makes it miss, so a full format
plus hashing); unit cache warm; the base binary with its own cache warmed the same way (today's
default).

Window 2 (load 2.7–3.1):

| file | pin | no-cache | fixed-point only | unit cache warm | base | no-cache / unit (min, mean) | base / unit (min, mean) |
|---|---|---|---|---|---|---|---|
| lints | pinned | 38.8 / 41.4 ± 4.0 / 10.1 | 39.6 / 43.9 ± 6.3 / 10.4 | 19.5 / 23.3 ± 5.5 / 9.7 | 39.7 / 41.0 ± 1.2 / 10.3 | 1.99x, 1.78x | 2.04x, 1.76x |
| lints | unpinned | 26.0 / 31.5 ± 5.1 / 15.0 | 28.8 / 33.2 ± 5.5 / 16.5 | 18.8 / 20.3 ± 2.6 / 9.6 | 29.1 / 33.6 ± 4.8 / 16.3 | 1.38x, 1.55x | 1.55x, 1.66x |
| nodes | pinned | 47.1 / 53.0 ± 9.7 / 14.1 | 51.0 / 54.6 ± 7.2 / 14.0 | 32.5 / 36.5 ± 3.2 / 15.1 | 50.8 / 52.7 ± 2.2 / 14.1 | 1.45x, 1.45x | 1.57x, 1.44x |
| nodes | unpinned | 39.8 / 56.4 ± 8.8 / 14.1 | 40.1 / 56.5 ± 13.1 / 14.0 | 30.4 / 43.8 ± 10.0 / 15.1 | 42.8 / 55.7 ± 11.3 / 14.0 | 1.31x, 1.29x | 1.41x, 1.27x |
| hir_lib | pinned | 42.0 / 44.0 ± 2.4 / 8.9 | 44.2 / 51.7 ± 8.2 / 9.6 | 26.2 / 27.5 ± 1.5 / 9.9 | 44.7 / 49.7 ± 6.4 / 9.7 | 1.60x, 1.60x | 1.71x, 1.81x |
| hir_lib | unpinned | 26.2 / 30.7 ± 3.1 / 8.9 | 29.1 / 32.8 ± 4.7 / 9.6 | 25.3 / 28.2 ± 2.9 / 9.9 | 28.2 / 31.2 ± 4.3 / 9.6 | 1.04x, 1.09x | 1.11x, 1.11x |
| hover_tests | pinned | 11.6 / 12.1 ± 0.3 / 4.8 | 13.5 / 14.5 ± 0.9 / 5.4 | 9.0 / 9.6 ± 0.4 / 5.4 | 13.6 / 14.1 ± 0.4 / 5.3 | 1.29x, 1.26x | 1.50x, 1.47x |
| hover_tests | unpinned | 8.0 / 9.0 ± 0.7 / 5.4 | 9.8 / 10.6 ± 1.2 / 5.9 | 8.1 / 9.9 ± 1.4 / 5.4 | 10.1 / 12.5 ± 1.1 / 5.9 | 0.99x, 0.90x | 1.25x, 1.26x |
| config | pinned | 18.7 / 19.8 ± 1.0 / 5.3 | 19.8 / 20.5 ± 0.5 / 6.0 | 10.9 / 14.8 ± 6.8 / 6.0 | 19.0 / 20.6 ± 2.3 / 6.0 | 1.72x, 1.34x | 1.74x, 1.39x |
| config | unpinned | 12.6 / 15.5 ± 2.6 / 6.2 | 15.5 / 17.9 ± 1.6 / 7.3 | 10.6 / 12.0 ± 1.3 / 6.0 | 14.5 / 17.5 ± 1.7 / 7.3 | 1.18x, 1.29x | 1.36x, 1.46x |

Window 1 (load 1.3–3.1) gave no-cache / unit (min, mean): pinned `lints` 1.97x, 1.71x;
`nodes` 1.57x, 1.61x; `hir_lib` 1.59x, 1.58x; `hover_tests` 1.31x, 1.35x; `config` 1.47x,
1.51x; unpinned 1.50x, 1.71x; 1.16x, 1.23x; 1.06x, 1.08x; 0.98x, 1.03x; 1.06x, 1.16x.

hyperfine runs all runs of one command before the next, so a load change between commands
moves a ratio. A third measurement interleaves every run (`interleave.py`: each round runs the
four variants once, in a rotating order, after their prepare step; 20 rounds after a warm-up
round; wall time from fork to exit). Minima in ms, no-cache / unit ratio of minima and of
means:

| file | pin | no-cache | fixed-point only | unit cache warm | base | min ratio | mean ratio |
|---|---|---|---|---|---|---|---|
| lints | pinned | 38.0 | 40.0 | 20.3 | 40.2 | 1.87x | 2.03x |
| lints | unpinned | 24.7 | 27.8 | 18.5 | 27.5 | 1.34x | 1.44x |
| nodes | pinned | 49.0 | 50.3 | 32.8 | 50.7 | 1.49x | 1.36x |
| nodes | unpinned | 37.8 | 39.4 | 31.4 | 41.0 | 1.20x | 1.29x |
| hir_lib | pinned | 41.1 | 43.4 | 24.0 | 42.4 | 1.71x | 1.58x |
| hir_lib | unpinned | 27.8 | 31.4 | 26.0 | 29.3 | 1.07x | 1.08x |
| hover_tests | pinned | 12.2 | 13.7 | 8.9 | 13.5 | 1.37x | 1.19x |
| hover_tests | unpinned | 9.4 | 11.3 | 8.6 | 11.6 | 1.09x | 1.21x |
| config | pinned | 18.6 | 20.2 | 12.0 | 20.0 | 1.55x | 1.47x |
| config | unpinned | 12.4 | 15.5 | 11.3 | 15.3 | 1.10x | 1.20x |

Where the process time goes (hyperfine, pinned, same window as window 2; min / mean ms):

| run | `lints.rs` | `hover/tests.rs` |
|---|---|---|
| `chloro --check --no-cache` on a 10-byte file (the process floor) | 3.3 / 3.6 | 3.0 / 3.1 |
| fixed-point cache hit on the edited file (floor of any cached run: executable hashed for the tool key, file read and hashed) | 6.0 / 8.3 | 5.1 / 5.4 |
| unit cache, every unit found (the snapshot warmed on the edited file, fixed-point file removed) | 17.3 / 18.7 | 8.2 / 9.7 |
| unit cache, one unit formatted | 19.4 / 20.6 | 8.9 / 9.8 |
| `--no-cache` | 35.6 / 39.2 | 11.7 / 12.2 |

Unit counts and sizes on the fixture tree (`chloro -v --check` over the 1238 fixtures, both
caches cold, then again): cold run 13,644 units looked up, 364 found, 13,280 stored; the unit
cache file 8,604,874 bytes (13,280 records; 703 of the files are not formatted, so their units
carry formatted text), the fixed-point file 19,056 bytes; in memory 13,280 entries holding
7,501,806 bytes of record strings (plus the map's 36-byte keys and the struct per entry). The
second run: 535 fixed-point hits, and for the 703 files that would change, 11,082 units looked
up and 11,082 found, 0 stored. On the five formatted edit files, the warm unit file holds 11
records in 950 bytes (`lints.rs`, every unit verbatim), 1958 in 161,900 (`nodes.rs`), 238 in
19,884, 259 in 21,286, 104 in 8,688.

**4. The cold path.**

| measurement | command | result |
|---|---|---|
| `Ir` on the 49-file sample, cache off | `benchmarks/profile.sh --bench <new bench> --core 3` | 312,735,600 against the best-known-good 312,734,032 (+0.0005%; `bench` takes `format_source`, which this change does not touch) |
| `bench -n 5`, 3 interleaved pairs, `taskset -c 3` | base then new, three times | base 1.471, 1.531, 1.515 s; new 1.465, 1.484, 1.488 s: ranges overlap, new not slower. A first triple in a loaded minute gave base 1.468–3.105 and new 1.432–3.187 and is not used |
| fixture tree, `--check`, interleaved (`interleave.py`, 6 rounds), pinned | no-cache; cold both caches; cold fixed-point only; cold base; warm both; warm fixed-point only; warm base | min / mean ms (peak RSS MiB): 1520 / 1584 (23.1); 1674 / 1820 (35.3); 1605 / 1699 (24.3); 1554 / 1680 (24.3); 451 / 487 (30.2); 1489 / 1618 (24.3); 1474 / 1557 (24.2) |
| the same, unpinned | | 646 / 871 (45.7); 721 / 963 (64.2); 610 / 853 (47.6); 636 / 878 (45.7); 193 / 262 (45.6); 551 / 753 (47.5); 606 / 821 (45.0) |
| instructions of the cold CLI run, one thread | `valgrind --tool=callgrind chloro --check --threads 1 [--no-cache]` over the fixtures | 9,436,106,920 without a cache, 9,871,943,573 with both caches cold (+4.6%); BLAKE3 update 188 M, `UnitStore::get` 109 M and `insert` 115 M inclusive (each missed unit is hashed twice, at the lookup and at the store), `memcpy` 295 M against 302 M |

The cache-on cold pass costs more than 10% over `--no-cache` on the fixture tree: +10.1% (min)
and +15.0% (mean) pinned, +11.7% and +10.6% unpinned; against today's default (the fixed-point
cache alone) +4.3% and +7.1% pinned. It costs 4.6% more instructions; the rest of the wall time
is memory: the cold run copies the formatted text of every unit that is not already formatted
into the map and writes an 8.6 MB file, and its peak RSS is 35.3 MiB against 24.3 (pinned).

## What surprised

- **Skipped line ranges cannot be rebased.** The study proposed storing the skipped ranges
  "with source lines relative to the unit's first line". The ranges a unit records are of three
  kinds that the record cannot tell apart: `(source line, source line)` from
  `add_skipped_span` (macro calls left as written), `(source line, output line of the unit)`
  from the unit's own `push_skipped_with_span` (the indices in `output_line_ranges`), and
  `(source line, output line of a nested visitor's buffer)` from `push_skipped_with_span` inside
  a block. Rebasing every first field by the unit's line would be right; rebasing the second
  field of the first kind would need provenance the visitor does not keep. The record keeps them
  as recorded and is used only where the unit starts on the same source line. 392 of 13,644
  fixture units carry such ranges; they miss after an edit that moves them. The ranges feed only
  `Formatted::skipped_range`, which no public entry point returns, so this guards an internal
  invariant, not an output byte.
- **`last_pos` past the unit's end, gaps before its start.** The tail check and the signed gap
  offset are in the record, but no unit on either corpus needs them: 0 of 13,280 fixture
  records and 0 of 759,442 registry records have a tail or a gap position other than the unit's
  start. Every refused record is a line-pinned one.
- **The process floor, not the formatter, bounds the CLI gain.** In process the edited files
  cost 0.26–0.47 of `format_source_with_config` (inside the study's bounds: `lints.rs`
  0.297–0.306 against 0.23–0.63, `nodes.rs` 0.449–0.471 against 0.41). At process level a
  cached run starts at 5–6 ms (3 ms process floor, about 2–3 ms hashing the 2 MB executable for
  the tool key, reading the file) and `--no-cache` on four cores already formats large files in
  parallel, so the edit run of `hover/tests.rs` (8.9 ms) cannot reach 1.5x of its `--no-cache`
  run (11.6 ms) even with a free unit path: its ceiling is 11.6 / 5.1 = 2.3x pinned and 8.0 /
  5.1 = 1.6x unpinned.
- **A repeated `--check` on an unformatted tree is the largest win.** The fixed-point cache
  never records a file that would change; the unit cache answers all its units. On the
  fixtures (703 of 1238 files unformatted) the warm run is 451 / 487 ms against 1489 / 1618 ms
  for the fixed-point cache alone (3.3x) pinned and 193 / 262 ms against 551 / 753 unpinned.
- **Equal units across files.** 364 fixture units and 1,135,986 registry units (the registry
  holds several versions of many crates) were found on their file's cold pass from an earlier
  file, with 0 differences: evidence that the output depends on the unit and not on its file.

## Decision

**Refuted as stated, at process level; confirmed in process.** Output identity holds: 0
differences on the fixtures (cold, warm, shifted, blanked, 1 and 4 threads) and on the registry
(cold, warm, shifted at 1 and 4 threads; blanked on a sample), and the default paths are
unchanged (goldens 0 changed, `conform -i` 1220/1220 at 1.9.0 and 1.8.0, `Ir` +0.0005%). The
in-process gain matches the study's bound on all five files (0.26–0.47 of the file's time, 3 of
3 repetitions below 0.5x). The process-level falsifier is met: the warm re-format after a
one-item edit is not 1.5x faster than `--no-cache` for every file over 128 KiB. Only `lints.rs`
and `hir/src/lib.rs` stay above 1.5x in every pinned window; `nodes.rs` and `config.rs` fall
below it in some windows, `hover/tests.rs` in all, and unpinned only `lints.rs` reaches it, in
some windows. Pinned it is
1.71–2.03x on `lints.rs`, 1.58–1.71x on `hir/src/lib.rs`, 1.36–1.61x on `nodes.rs`,
1.34–1.72x on `config.rs` and 1.19–1.37x on `hover/tests.rs` over the three windows; unpinned,
where `--no-cache` formats the items on four threads, 1.34–1.71x on `lints.rs` and 0.90–1.31x
on the others.

What the branch holds is exact and is faster than today's default (the base binary with its
fixed-point cache) in every edit cell (1.11x–2.04x on minima) and 3.3x on a repeated `--check`
of an unformatted tree, at +4–7% on a cold run against the fixed-point cache alone (+10–15%
against `--no-cache`) and an 8.6 MB cache file for the fixture tree. It stays on its branch for
the integrator. Following the review of this record, the CLI's unit cache is opt-in
(`--unit-cache`, `d672456` (was `cafc1e8`)), and the branch carries the integration head with E12 (the
executable's digest recorded under its identity instead of hashed every run); the re-measured
numbers are in the next section.

## Follow-up on the merged branch (`d672456` (was `cafc1e8`): integration head `21b91c5` (was `7b53917`) with E12 and E13 merged, unit cache opt-in)

Gates: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --all-features
-- -D warnings` clean; `cargo test --workspace --all-features` all pass (chloro 24 unit,
`cli_cache` 11, `cli_files` 5, `cli_unit_cache` 3, doc 1; chloro-core as before); identity
harness on the fixtures at 1 and 4 threads: 13,644 units, 13,644 found warm, 13,203 of 13,595
used moved, 0 differences (cold, warm, moved); `bench -n 1 --check fixtures-e8d87c2.tsv` with
the cache off 0 of 1238 changed; `profile.sh --core 3` 312,735,603 `Ir` (+0.0005% against
312,734,032).

The cached run's floor (hyperfine, pinned, under load 4.9–7.9): a fixed-point hit on
`hover/tests.rs` 3.8 ms min (5.1 before E12), the no-cache process floor 2.9 ms (3.0).

Edit scenario, the same five edited files and snapshots warmed by the merged binary, three
variants (`--no-cache`; fixed-point cache only, now the default; `--unit-cache` warm). The
interleaved runner (20 rounds; load 4.4–6.2), minima in ms, then `--no-cache` / unit cache on
minima and means, with the same ratios before the merge (interleaved, first window) beside
them:

| file | pin | no-cache | fixed-point only | `--unit-cache` warm | no-cache / unit, min | mean | before the merge (min, mean) | fixed-point / unit, min |
|---|---|---|---|---|---|---|---|---|
| lints | pinned | 38.2 | 39.0 | 18.3 | 2.09x | 2.10x | 1.87x, 2.03x | 2.14x |
| lints | unpinned | 25.4 | 26.7 | 16.8 | 1.52x | 1.60x | 1.34x, 1.44x | 1.59x |
| nodes | pinned | 48.7 | 49.3 | 30.2 | 1.61x | 1.50x | 1.49x, 1.36x | 1.63x |
| nodes | unpinned | 38.8 | 39.8 | 32.0 | 1.21x | 1.23x | 1.20x, 1.29x | 1.24x |
| hir_lib | pinned | 44.5 | 44.9 | 25.1 | 1.77x | 1.55x | 1.71x, 1.58x | 1.79x |
| hir_lib | unpinned | 27.1 | 27.3 | 24.0 | 1.13x | 1.12x | 1.07x, 1.08x | 1.14x |
| hover_tests | pinned | 12.1 | 12.4 | 7.7 | 1.56x | 1.62x | 1.37x, 1.19x | 1.61x |
| hover_tests | unpinned | 8.7 | 8.8 | 6.6 | 1.31x | 1.33x | 1.09x, 1.21x | 1.34x |
| config | pinned | 19.0 | 19.3 | 11.2 | 1.70x | 1.65x | 1.55x, 1.47x | 1.72x |
| config | unpinned | 12.0 | 13.0 | 9.9 | 1.21x | 1.34x | 1.10x, 1.20x | 1.31x |

hyperfine in the same window (`-N -i -w 2 -r 20`, the three variants per invocation; cells min /
mean ± σ / peak RSS MiB; noisier, load 7.0–7.9):

| file | pin | no-cache | fixed-point only | `--unit-cache` warm | no-cache / unit (min, mean) |
|---|---|---|---|---|---|
| lints | pinned | 35.5 / 44.6 ± 13.2 / 10.0 | 38.9 / 41.5 ± 3.8 / 10.0 | 17.8 / 20.6 ± 2.6 / 9.0 | 1.99x, 2.17x |
| lints | unpinned | 28.8 / 33.5 ± 4.4 / 15.1 | 26.1 / 32.8 ± 5.6 / 15.0 | 16.9 / 18.7 ± 2.6 / 9.0 | 1.70x, 1.79x |
| nodes | pinned | 48.8 / 53.2 ± 7.0 / 14.1 | 50.2 / 52.2 ± 2.8 / 14.1 | 31.7 / 34.1 ± 3.7 / 14.9 | 1.54x, 1.56x |
| nodes | unpinned | 43.3 / 50.2 ± 6.6 / 14.1 | 39.4 / 49.3 ± 6.8 / 14.1 | 29.1 / 32.0 ± 4.1 / 14.9 | 1.49x, 1.57x |
| hir_lib | pinned | 42.4 / 46.0 ± 6.0 / 9.0 | 41.9 / 47.8 ± 9.1 / 9.0 | 22.4 / 24.9 ± 4.9 / 9.3 | 1.89x, 1.85x |
| hir_lib | unpinned | 28.3 / 42.1 ± 12.1 / 9.0 | 31.4 / 44.9 ± 10.9 / 9.0 | 22.4 / 28.0 ± 12.0 / 9.3 | 1.26x, 1.50x |
| hover_tests | pinned | 11.6 / 13.4 ± 5.8 / 4.8 | 12.2 / 17.4 ± 6.0 / 4.9 | 7.7 / 12.2 ± 5.8 / 4.8 | 1.52x, 1.10x |
| hover_tests | unpinned | 9.4 / 14.3 ± 3.5 / 5.4 | 10.3 / 14.1 ± 2.8 / 5.3 | 6.5 / 8.0 ± 3.3 / 4.8 | 1.46x, 1.79x |
| config | pinned | 18.5 / 19.3 ± 0.8 / 5.4 | 18.9 / 20.3 ± 1.6 / 5.4 | 10.6 / 11.5 ± 1.5 / 5.6 | 1.74x, 1.69x |
| config | unpinned | 13.9 / 21.0 ± 6.0 / 6.1 | 13.1 / 18.9 ± 3.9 / 6.2 | 9.5 / 9.8 ± 0.2 / 5.5 | 1.47x, 2.14x |

Cold and warm `--check` of the 1238 fixtures (interleaved, 6 rounds; min / mean ms, peak RSS
MiB), against the same comparison before the merge:

| run | pinned | unpinned (load 4.9–5.9, means not usable) |
|---|---|---|
| `--no-cache` | 1498 / 1589 (23.1) | 843 / 1245 (42.5) |
| cold, fixed-point only | 1592 / 1674 (24.5) | 891 / 1244 (45.5) |
| cold, `--unit-cache` | 1705 / 1783 (36.1) | 932 / 1182 (68.3) |
| warm, fixed-point only | 1475 / 1528 (23.5) | 811 / 1108 (45.3) |
| warm, `--unit-cache` | 432 / 445 (29.9) | 263 / 418 (45.8) |
| cold `--unit-cache` against `--no-cache` | +13.8% min, +12.3% mean (before: +10.1%, +15.0%) | +10.6% min (before +11.7%) |
| cold `--unit-cache` against the fixed-point cache alone | +7.1% min, +6.5% mean (before: +4.3%, +7.1%) | +4.7% min (before +18.2% min, a noisy window) |

Reading: E12 lowered every cached run by about 1 ms, which moves the pinned edit ratios up by
0.06–0.27; pinned, all five files now clear 1.5x on minima and four of five on means
(`nodes.rs` 1.50x). Unpinned against the four-thread `--no-cache`, only `lints.rs` clears it
(1.52x, 1.60x); the others stay at 1.12–1.34x, bounded by the single-threaded parse. The cold
pass with `--unit-cache` still costs +10–14% against `--no-cache` and +5–7% against the
fixed-point cache alone, so the opt-in stands. The decision is unchanged: refuted as stated at
process level (the falsifier asks for 1.5x on every file over 128 KiB, and the unpinned cells
miss it), confirmed in process, exact.

## What is left

- The cached run's floor: done by E12 (the executable's digest recorded under its identity),
  merged here; a fixed-point hit is now about 1 ms above the process floor.
- The cold-run cost: each missed unit is hashed twice (lookup and store); a store that takes
  the lookup's digest would remove about 125 M of the 436 M extra instructions. Storing text
  only for `--write` and print mode (a `--check` run needs one bit per unit, the study's
  "entry without text") would shrink the 8.6 MB file of an unformatted tree to about 1 MB.
- Inline modules, `impl` blocks and large `const` arrays are single units: an edit inside one
  misses the whole item (`lints.rs`'s 288 KB array would cost about 0.62 of the file, by the
  study's model; not measured here because the scripted edit landed in a 57 KB unit).
- The `config_matrix` gate (31 option values) was not run on the cached path; the key holds
  the whole configuration and the equivalence test covers two configurations.

## Appendix: measurement scripts

Kept in the session scratchpad, reproduced here. `flip.py` (the scripted edit):

```python
# Writes <name>.edited.rs: one letter of the last string literal that starts with a lowercase
# ASCII letter changed (z, or a for a z). Prints the offset and the top-level line before it.
import sys, re
for path in sys.argv[1:]:
    src = open(path, encoding='utf-8').read()
    at = max(m.start() + 1 for m in re.finditer('"', src) if m.start() + 1 < len(src) and 'a' <= src[m.start() + 1] <= 'z')
    new = src[:at] + ('a' if src[at] == 'z' else 'z') + src[at + 1:]
    out = path[:-3] + '.edited.rs'
    open(out, 'w', encoding='utf-8').write(new)
    head = [l for l in src[:at].split('\n') if l and not l[0].isspace() and not l.startswith(('}', ']', ')', '//', '#'))]
    line = src[:at].count('\n') + 1
    print(f"{path}: {len(src)} bytes, edit at byte {at} (line {line}) in the item starting {head[-1][:60]!r}")
```

`hf-edit.sh FILE PIN` (hyperfine cell; `restore.sh SNAP DIR` is `rm -rf DIR && cp -r SNAP DIR`):

```bash
#!/bin/bash
# hf-edit.sh FILE PIN: the four variants of E15's edit scenario on FILE.edited.rs, interleaved
# by hyperfine (-N -i -w 2 -r 20); PIN is "pinned" (taskset -c 3) or "unpinned".
S=/tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/e15
f=$1; pin=$2
cd $S/edit
P=""; [ "$pin" = pinned ] && P="taskset -c 3 "
C=$S/caches
hyperfine -N -i -w 2 -r 20 --export-json $S/hf-$f-$pin.json \
  --prepare "true" \
  --prepare "$S/restore.sh $C/snap-$f-fp $C/run-fp" \
  --prepare "$S/restore.sh $C/snap-$f-unit $C/run-unit" \
  --prepare "$S/restore.sh $C/snap-$f-base $C/run-base" \
  -n "no-cache" "${P}$S/new/chloro --check --no-cache $f.edited.rs" \
  -n "fixed-point only" "${P}$S/new/chloro --check --no-unit-cache --cache-dir $C/run-fp $f.edited.rs" \
  -n "unit cache warm" "${P}$S/new/chloro --check --cache-dir $C/run-unit $f.edited.rs" \
  -n "base (integration head)" "${P}$S/base/chloro --check --cache-dir $C/run-base $f.edited.rs"
```

`interleave.py SPEC ROUNDS OUT` (the interleaved runner; its RSS column is not used for the
edit cells, since `wait4` reports the forked Python image there):

```python
# interleave.py SPEC.json ROUNDS OUT.json: runs every variant once per round, in a rotating
# order, with its prepare command first (untimed); records wall time (fork to exit) and peak
# RSS (wait4's ru_maxrss) of each run. SPEC: [{"name", "cmd": [...], "prepare": [...] or null}].
import json, os, subprocess, sys, time
spec = json.load(open(sys.argv[1])); rounds = int(sys.argv[2])
res = {v['name']: {'wall': [], 'rss_kb': []} for v in spec}
for r in range(rounds + 1):  # round 0 is the warm-up, not recorded
    order = spec[r % len(spec):] + spec[:r % len(spec)]
    for v in order:
        if v.get('prepare'):
            subprocess.run(v['prepare'], check=True)
        t = time.perf_counter()
        p = subprocess.Popen(v['cmd'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        _, status, ru = os.wait4(p.pid, 0)
        wall = time.perf_counter() - t
        if r:
            res[v['name']]['wall'].append(wall); res[v['name']]['rss_kb'].append(ru.ru_maxrss)
json.dump(res, open(sys.argv[3], 'w'), indent=1)
import statistics as st
for v in spec:
    w = res[v['name']]['wall']
    print(f"{v['name']:32s} min {min(w)*1e3:8.1f} ms  mean {st.mean(w)*1e3:8.1f} ± {st.stdev(w)*1e3:6.1f} ms  max rss {max(res[v['name']]['rss_kb'])/1024:6.1f} MiB")
```
