# 2026-10-15: Incremental formatting, a resident process, and process-level levers (theory)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels: measured, read, derived, hypothesis. No code was changed.

## Question

How far below today's M3 figures each of the following can take chloro, as an upper bound with the
arithmetic shown, the exactness risk, the cost, and the smallest experiment that would refute it:
an item-level output cache (X12b's open "cache" half), a resident process, `--check` early exit
and the CLI's input/output costs, the global allocator, build-time levers (PGO in the release
workflow, BOLT, link flags, static linking, huge pages), and file/unit scheduling.

## Method

Code read at head `31ce20e`; every code claim carries file:line. Numbers from the record are cited
by file. New measurements ran on copies under the scratchpad
(`.../scratchpad/theory-incremental/`), with `CHLORO_CACHE_DIR` pointed there, using the existing
release binaries (`target/release/chloro`, 2,017,472 bytes; `examples/{bench,parse_floor,dump_tree,alloc_count}`).
The host is the 2.80 GHz, 4-vCPU class of M3 (`model name` "Intel(R) Xeon(R) Processor @ 2.80GHz",
AVX-512). Load average was 0.49–0.73 for the start-up and warm-cache runs (01:17–01:19 UTC) and
reached 3.18 at 01:22 UTC while another session's `rustc` ran. Wall times from this window are
**measured, provisional** (protocol §6) and are not compared with M3's absolute figures, only
with each other. Instruction counts are not affected by load.

Commands (`$B` = `target/release`, `$S` = the scratch directory):

```text
cp -r chloro-core/tests/conformance/fixtures $S/fx; $B/chloro --write --no-cache $S/fx   # 1238 files, then --check exits 0
for f in $S/fx/**/*.rs: $B/examples/dump_tree $f | awk ...      # elements and top-level items per file ($S/tree-stats.txt)
hyperfine -N -i -w 3 -r 30..40  "$B/chloro --check one.rs" "$B/chloro --check --no-cache one.rs"   # cache fixed cost, two cache dirs, two rounds
hyperfine -N -i -w 3 -r 30  "$B/chloro --check $S/fx" "$B/chloro --check --threads 1 $S/fx"         # warm cache
hyperfine -N -w 3 -r 30  "xargs -a files-abs.txt realpath|echo|stat -c %s"                         # canonicalize and stat floors
strace -f -c $B/chloro --check $S/fx ; strace -f -tt -T $B/chloro --check one.rs                    # syscalls
valgrind --tool=callgrind $B/chloro --check $S/fx | --check one.rs | --check --no-cache one.rs      # Ir of the CLI paths
$B/examples/parse_floor {flat,lex,events} <dir with lints.rs> -n 20 ; ... <dir with nodes.rs> ; ... $S/fx -n 3
$B/examples/bench -n 10 --root <lints.rs dir|nodes.rs dir> ; bench -n 2 --root $S/fx
hyperfine ... "$B/chloro --check --no-cache [--threads 1] lints.rs"; ... hir-ty and fx with --no-cache (user/sys split)
$B/examples/alloc_count /tmp/chloro-bench/sample49
callgrind_annotate profile-final/callgrind.1.out   # final head, 312,733,055 Ir, 49-file sample
size -A $B/chloro; readelf -p .comment $B/chloro; ldd $B/chloro
```

Total compute: under 6 minutes.

## 1. An item-level cache

### What a unit's output depends on (read)

The parallel path already isolates a unit: a worker formats it in a fresh visitor state and the
main thread stitches it (`chloro-core/src/formatter/parallel.rs:93-181`). The dependency set of a
unit's `UnitOutput` (`parallel.rs:77-88`), path by path:

| path into the unit's output | where | consequence for a cache |
|---|---|---|
| the items' source text, from the first item's `lo` to the last item's `hi` (a reorder group is one unit) | `parallel.rs:195-197`, `reorder.rs:280-294` | key: BLAKE3 of the normalised text of `[lo, hi)` |
| the whole `Config` (edition, `style_edition`, `rustfmt_version`, `reorder_imports`/`reorder_modules`, which also decide unit boundaries) | `cli/cache.rs:91-200`, `reorder.rs:285` | key: `config_key_bytes` |
| the binary | `cli/cache.rs:204-217` | key: `tool_key` (or the cache file name, as today) |
| the file's inner attributes, as the skip context (`#![rustfmt::skip::macros(..)]`, `::attributes(..)`) | `parallel.rs:109-110, 212`; `visitor.rs:424-433` | key: the serialised `SkipContext` (empty for nearly every file) |
| the text between the previous unit and this one | deferred as a `GapOp` and replayed on the main buffer (`parallel.rs:51-61, 134-139`; `visitor.rs:121`) | not cached; recomputed every run from the fresh parse |
| absolute byte positions: `GapOp`'s end, `last_pos` | `parallel.rs:51-61, 81, 142` | stored relative to the unit's `lo`, rebased on a hit |
| absolute source lines in `skipped_range.lo` (`line_of`) and output lines shifted by `line_base` | `visitor.rs:673-697`, `parallel.rs:140-157` | stored relative to the unit's first line; feeds only `Formatted.skipped_range`, which `format_source_with_config_parallel` discards (`formatter.rs:103-117`) |
| the run's memo, block cache and macro-argument cache, shared across units on one worker | `context.rs:82-106`; memo cleared at each top-level item (`visitor.rs:411-414`) | keys are (tree id, element index) and (`BytePos`, `BytePos`, flag) (`context.rs:19-20, 333-339`): no key is shared by two units, so no output path crosses units (**derived** from the key types) |
| `format_failure` of any unit returns the whole file unchanged | `parallel.rs:147-149, 168-170`; `context.rs:86-91` (only under `rustfmt_version = "1.8.0"`) | stored; a hit with it set forces the whole-file fallback |
| `broken` (text written before the first gap call) forces the sequential walk | `parallel.rs:124-161, 288-294` | never stored |
| reads of the source outside `[lo, hi)` through the shared `SnippetProvider` | `parallel.rs:209` | **hypothesis:** none change the text; X12b found every one of 978,843 units equal standalone (where nothing lies outside) and in-file, gap text aside (`2026-10-10-item-independence.md`). A `last_pos` past `hi` is stored with a digest of the bytes `[hi, last_pos)` and checked on a hit |
| `rustc_compat` rules that look at ancestors | `rustc_compat.rs:629-645`: the file-level pass runs on the whole tree before any unit; inside units, `subtree_rejected_by_rustc` sees ancestors only within the walked subtree (macro arguments, `macro_args.rs:30`) | file-level decision recomputed every run from the parse; unit-level decision is a function of the unit's text |
| file-level gates: length over `u32::MAX`, lexer/parser errors, `rejected_by_rustc`, `#![rustfmt::skip]`, `disable_all_formatting` | `formatting.rs:45-58, 132-145`; `formatter.rs:103-107` | run before any lookup: a lookup needs the full parse |
| trailing-newline fix-up and `newline_style` | `parallel.rs:171-172`; `formatter.rs:109-114` | applied to the assembled text, not cached |

- A unit's text depends on nothing before it other than through the gap and the file's skip
  context (**read**, the table above) — the worker starts from `last_pos = lo`, empty block
  indent, buffer `"\n"`, `line_number = 0`, cleared flags (`parallel.rs:256-266`).
- Units are top-level only: an inline `mod m { .. }`, an `impl` or a `const` array is one unit
  whatever its size (**read**, `parallel.rs:112-113`). An edit anywhere inside such an item misses
  the whole item.
- Macro definitions: a `macro_rules!` body is formatted as a nested snippet inside its item's unit
  (`formatting.rs:268-290`), so its output is a function of the unit text (**read**); X12b counted
  923 equal `MACRO_RULES` units on the registry.
- `#[rustfmt::skip]` items: a skipped `mod x;` unit writes nothing and its text is copied by the
  next gap (`parallel.rs:128-133`); the cached value is "empty, consumed nothing". Other skipped
  items write their source unchanged through `push_skipped_with_span` (`visitor.rs:673-697`) —
  a function of the text (**read**).

**Cached value.** E1's `UnitOutput` minus `broken`, with positions made relative: the `GapOp`
variant and `end − lo`; `last_pos − lo`; `text`; `macro_failure`, `lost_comment`,
`format_failure`; `skipped_range` with source lines relative to the unit's first line and the
`output_line_ranges` indices. Added: a digest of `[hi, last_pos)` when `last_pos > hi`. For
`--check` alone the text can be replaced by one bit "the text equals the source slice
`[lo, last_pos)`": the file is then unchanged exactly when every unit has the bit and the
re-run gaps reproduce the gap source (**derived**: the output is the concatenation of gaps and unit
texts, `parallel.rs:127-163`).

**Exactness argument.** For a hit, the stitched output equals the sequential output if (a) the unit
text is a function of the key (E1's equality on 1238 fixtures and `conform -i` 1220/1220 at M3
show worker output equals in-file output; X12b shows it equals standalone output), and (b) the
stored relative positions rebased by `lo` equal the ones a fresh worker would record (**derived**
from (a): a fresh worker's positions are `lo` plus offsets determined by the same text). Cases that
break it: a source read outside `[lo, max(hi, last_pos))` that changes text (the hypothesis row
above); a future change to the formatter that makes text depend on position (closed by the binary
digest in the key); a `Config` field not in `config_key_bytes` (closed at compile time,
`cli/cache.rs:87-121`).

### Cost model

`lints.rs` (`ide-db/src/generated/lints.rs`, 639,771 bytes, 11 top-level items, **measured** by
`dump_tree`): in-process format 28 ms best of 10 (`bench -n 10`, this window); parse floor
(`parse_floor flat`) 6.0 ms best of 20 → parse 21% (**derived**, 6.0 / 28). EXP-001 measured the
largest unit (a 287,600-byte `const` array) at 49% of unit work, the second array 223 KB.

| edit lands in | recomputed | bound, in-process | ratio to 28 ms |
|---|---|---|---|
| the 288 KB array | parse 6.0 + unit hashing 0.14 + gaps 0.3–0.7 + 0.49 × (28 − 6) = 10.8 | 17.2–17.6 ms | 0.62–0.63 |
| one of the 9 small items | parse 6.0 + 0.14 + 0.3–0.7 + under 0.5 | 6.9–7.3 ms | 0.25–0.26 |
| floor (all hits) | parse + hashing + gaps | 6.4–6.8 ms | 0.23–0.24 |

- Unit hashing: 640 KB at BLAKE3's 4,606 MB/s (EXP-006, one core) = 0.14 ms (**derived**). Gaps and
  stitching: 0.3–0.7 ms per large file (EXP-001, **measured** at an earlier head).
- `nodes.rs` (358,382 bytes, 1,959 items, largest unit 2% of unit work per EXP-001): parse floor
  13.8 ms of 37 ms in-process (both **measured**, this window) → an edit to one item costs at least
  13.8 + 0.08 + 0.3–0.7 + under 0.8 ≈ 15–15.4 ms, 0.41x (**derived**). The parse floor is the
  bound on any file with many small items: 0.21–0.37 of the file's time on the two files above,
  the 26–37% X12b gave.
- **Editor-save case** (one item changed, every other unit hits): file time × (0.23–0.63) on
  `lints.rs`, × 0.41 on `nodes.rs`, plus process costs (§2) that the cache does not touch.
- **CI cold case** (hit rate 0): no unit is skipped; the cache adds unit hashing (3.0 ms CPU per
  13.9 MB, 0.2% of 1,617 ms CPU at M3's no-cache run, **derived**) and writing the outputs (§ memory
  below). The CI case gains only when a cache from an earlier commit is restored and a file changed
  in a few items: today that file is a full miss.

### Memory and disk

- Units: 13,650 on the 1238 fixtures (X12b), 11.0 per file; top-level items 16,096 (**measured**,
  `dump_tree`; groups of `use`/`mod x;`/`extern crate` merge into fewer units). Mean 882 bytes per
  item; median file 2,955 bytes, 90th percentile 31,419 (**measured**).
- Entry without text (`--check` only): 32-byte key + 1-byte `GapOp` variant + 4 + 4 bytes of offsets
  + 1 byte of flags + 32-byte tail digest when present ≈ 42–74 bytes → 0.57–1.0 MB for the fixtures
  (**derived**: 13,650 × 42 and × 74).
- Entry with text (`--write`, print mode): + the formatted text, which on a formatted tree equals
  the input bytes: + 14.2 MB for the fixtures (**derived**). Writing 14.8 MB on a cold run and
  reading only the entries of the files that miss at file level needs an indexed layout (one file
  per content digest, or an indexed table), not today's read-everything file
  (`cli/cache.rs:312-327`), whose cap of 131,072 entries (`cli/cache.rs:71`) holds 11.9k files at
  11 units each (**derived**).

### Experiment and falsifier

An in-process harness (an example, no CLI change): for every fixture and every registry file, format
once while recording unit outputs; then apply a one-unit edit (the perturbation of X12b, or a renamed
identifier inside one unit) and assemble the output from cached units plus the fresh parse and
gaps; compare byte for byte with `format_source_with_config`, under all 31 `config_matrix` values.
A second check runs each unit with every byte outside `[lo, hi)` replaced by a space (newlines kept,
so positions and lines are unchanged) and compares the unit text. Falsified by any byte difference
in either check, or if the assembled path on files over 128 KiB is not below 0.5x of
`format_source_with_config` on 3 of 3 interleaved `bench`-style pairs (protocol §3).

## 2. A resident process

**What one CLI run pays before formatting** (**measured**, this window, 30–40 runs each, two
rounds, two cache directories):

| `chloro --check one.rs` | mean (min–max) wall ms | user / sys ms |
|---|---|---|
| `--no-cache` | 2.1–2.7 (1.9–5.0) | 1.4 / 0.9 |
| cache on, cache file with 1 entry | 5.0 (4.0–7.5) | — |
| cache on, cache file with 1227 entries | 5.4–5.9 (4.3–8.6) | 2.1 / 2.8 |

- The cache adds 2.3–3.3 ms per invocation over the 1.7–2.4 ms start-up M3 reports for
  `--no-cache` (M3 `tiny-one`: the harness passes `--no-cache`, so M3's start-up excludes it).
  `strace -tt` attributes it to `read` of `/proc/self/exe` (2,017,472 bytes, one syscall of 1.4 ms
  under strace), BLAKE3 of those bytes (about 0.7 ms between the read and the `munmap`; callgrind
  counts 4.95 M Ir in the hashing kernel of the 6.63 M Ir run, against 0.50 M Ir for the
  `--no-cache` run), and the atomic save (`create_new`, write, `rename`, directory scan,
  `cli/cache.rs:356-405, 479-504`) (**measured**). The executable is read on every run
  (`cli/cache.rs:204-229`).
- Of the 0.50 M Ir of a `--no-cache` one-file run, 0.26 M are in `ld-linux` (52%: tunables parsing
  0.10 M, symbol lookup 0.05 M) and 0.12 M in libc (**measured**); 0.50 M Ir is about 0.2 ms at the
  run's rate, so most of the 1.7 ms start-up is the kernel's exec, mapping, page faults and one
  worker thread (`orchestrate.rs:65` spawns one even for one file), not instructions (**derived**).

**What a resident process removes and keeps.**

- Removes per request: exec and dynamic linking, `available_parallelism` (cgroup reads seen in the
  trace), thread spawn, the executable digest, the cache-file read and atomic save: 4.0–5.7 ms
  (**derived**: 1.7–2.4 + 2.3–3.3).
- Keeps: the per-file content digest and per-unit entries in memory (≈ 50 bytes per unit,
  0.68 MB for the fixtures, **derived** from § 1), optionally the source and the tree.
- Tree memory: `Elem` is 16 bytes (`kind` u16, `is_node` bool, three u32, `cst/mod.rs:39-48`).
  Elements per source byte (**measured**, `dump_tree`): 0.354 over the 1238 fixtures (5,029,588
  elements, 14,190,145 bytes), 0.188 on `lints.rs` (120,397), 0.688 on `nodes.rs` (246,522). Trees
  for the whole fixture tree: 5.03 M × 16 = 80.5 MB plus 14.2 MB of source (**derived**); `lints.rs`
  1.93 MB, `nodes.rs` 3.94 MB.
- A kept tree only pays if an edit reparses part of it. The flat tree has no incremental reparse:
  `Tree::parse` parses a whole text (`cst/mod.rs:99-107`); `from_steps_at` builds a tree at an
  offset for fragments (`formatting.rs:50`). Reparsing one top-level item and splicing it means
  re-lexing from the edit until the token stream re-synchronises (a `/*` or `r#"` can swallow later
  items), then shifting `start` and `end` of every later element: 3.9 MB of `Elem` moved for
  `nodes.rs`, ≈ 0.4 ms at 10 GB/s (**derived**, hypothesis on the copy rate). rust-analyzer does
  the token-level and block-level form of this (`crates/syntax/src/parsing/reparsing.rs`,
  hypothesis: not read in this session).
- Invalidation: content digest per file (today's key), then per unit (§ 1); the editor sends the
  buffer text, so no file read is needed.

**Bound for the single-file save** (**derived**):

| file | today (CLI, cache on by default) | resident, no item cache | resident + item cache | resident + item cache + item reparse |
|---|---|---|---|---|
| median fixture, 2,955 B (0.31 ms format at 9.5 MB/s, M3 `bench`) | 1.7–2.4 + 2.3–3.3 + 0.31 = 4.3–6.0 ms | ≈ 0.31 ms | parse 0.07 + gaps + 1 unit ≈ 0.15 ms | ≈ 0.1 ms |
| `lints.rs` (M3 E: 27.6 ms mean with 4 threads, `--no-cache`) | 27.6 + 2.3–3.3 = 29.9–30.9 ms | ≈ 23–25 ms (M3 E 23.5 ms min, minus start-up) | 6.9–17.6 ms (§ 1) | 1–11 ms (the changed unit, 0.5–10.8 ms, plus gaps) |

- For files at or below the median, 93–95% of today's save latency is process and cache fixed cost
  ((4.3 − 0.31)/4.3 and (6.0 − 0.31)/6.0, **derived**).
- rustfmt keeps nothing resident: each format request runs a new `rustfmt` process; M3 `tiny-one`
  measured its start-up at 9.8–11.1 ms against chloro's 1.7–2.4 ms (**measured**, M3).
- Editor integration shapes, cost class only: (a) a process per request reading stdin and writing
  stdout, which is how rust-analyzer's `rustfmt.overrideCommand` calls a formatter (hypothesis,
  documentation); the `chloro` CLI has no stdin mode — sources are paths (`cli.rs:103-115`), the
  stdin path exists only as `examples/fmt_stdin.rs` — so (a) costs a CLI option and keeps the
  per-request start-up; (b) a socket or LSP `textDocument/formatting` server, which removes the
  start-up and adds a lifecycle (spawn, idle exit, version mismatch with the binary on disk, one
  server per `Config`): new code of the order of the CLI itself (650 lines today, `chloro/src/cli*`).

## 3. `--check` early exit and output-side costs

**Early exit.** `--check` compares the whole formatted string with the input after formatting
(`worker.rs:83-88`).

- On a tree that passes `--check`, no file differs: early exit saves 0 (**derived**). That is the
  CI case on formatted input and every M3 formatted row.
- On a failing tree, it saves at most the work after the first differing unit of each differing
  file, never the parse (the file-level gates in § 1 need it): ≤ 0.78–0.79 of a differing file's
  time (1 − parse share 0.21–0.22), about 0.39 if the first difference falls uniformly in the file
  (hypothesis). On raw fixtures 702 of 1238 files differ (EXP-006); M3's raw rows sit within 0.88–
  1.10 of the formatted rows, so the bound applies to roughly the raw run's time on those files.
- Exactness: the comparison must stop only at a byte that no later step changes. The visitor
  retracts one trailing newline after a removed import (`imports.rs:44-50`), the final fix-up
  truncates trailing newlines (`parallel.rs:171-172`), and a later unit's `format_failure` returns
  the whole file unchanged under `rustfmt_version = "1.8.0"` (`parallel.rs:168-170`) — an early
  "changed" there would be wrong. Exact under 1.9.0 when the compared prefix stops at the last
  stitched unit boundary minus one byte (**derived**).
- Stopping the whole run at the first differing file changes the reported count
  (`cli.rs:151-154`) and is a behaviour change, not a speed lever.

**Input/output costs at the warm run** (1238 fixtures, every file a hit; **measured**, this window):
wall 29.0–67.0 ms (mean 36.1–38.3) on 4 threads against M3's 28.9 ms mean; user 14.8 ms, sys
33.7 ms per run; 90.9 M Ir.

| component | evidence | bound on the warm run |
|---|---|---|
| `canonicalize` per discovered file (`orchestrate.rs:24`) | `strace -c`: 15,488 `readlink` calls (12.5 per file on this 12-component path); `realpath` of the same 1238 paths: 17.8 ms against 7.0 ms for `echo` (+10.8 mean, +10.6 min) | ≈ 10.7 ms, main thread, sequential |
| two sorts of `PathBuf` (`orchestrate.rs:12`, `cli.rs:119`) and `dedup` (`cli.rs:120`) | callgrind: `driftsort_main::<PathBuf>` 20.3 M Ir inclusive; the run's user time gives ≈ 6.1 G Ir/s | ≈ 3.3 ms (**derived**) |
| `fs::metadata` per file for the largest-first order (`orchestrate.rs:51`) | `stat` of 1238 paths: +3.2 ms over `echo` | ≈ 3.2 ms, main thread |
| `is_dir()` per entry (`orchestrate.rs:21`) | 3,855 `statx` in the run in total | within the 2.3 ms `find` floor |
| BLAKE3 of 14.2 MB on 4 threads (`worker.rs:56`) | 43 M Ir (48.3 M in `blake3::hash` minus 4.95 M for the executable); EXP-006: 3.02 ms on one core | ≈ 0.8 ms wall |
| `read_to_string` (`worker.rs:47`) | `xargs cat`: 27.4 ms on one process, 15.6 ms with 4 (shell start 2.3 ms included) | 13–25 ms; the floor |
| executable digest and cache save | § 2 | 2.3–3.3 ms |
| start-up | M3 `tiny-one` | 1.7–2.4 ms |

- Canonicalising the root once and each symlink entry (`DirEntry::file_type` comes from
  `getdents64`) keeps the dedup exact — a non-symlink child of a canonical directory is canonical
  (**derived**) — and removes ≈ 10.7 ms; one sort removes ≈ half of 3.3 ms; moving the size `stat`
  into the workers takes 3.2 ms off the main thread. Together ≈ 15–17 ms of a ≈ 37 ms warm run in
  this window: ≥ 0.45 of the warm run (**derived**, provisional window).
- On the cold run these are the same ≈ 15–17 ms against 434 ms (M3 no-cache): ≤ 4% (**derived**).
- **Cold minus no-cache at M3**: 446.3 − 434.3 = 12.0 ms in mean, ranges overlapping (σ 14.1 and
  16.7 ms). EXP-006 accounts for ≈ 2 ms of it: hashing 3.02 ms / 4 threads = 0.76 ms, executable
  digest 1.06 ms, save 0.21 ms, open 0.03 ms (**derived**); the remainder lies inside the spread.
- `--write` with the cache formats every rewritten file twice to record a verified fixed point
  (`worker.rs:108-109`): ≤ 0.5 of the formatting time of rewritten files (**derived**); dropping it
  trades the "every recorded digest was observed" property (`worker.rs:92-95` comment) for that time.
- Print mode writes each file with one `write_all` to stdout (`worker.rs:116`): no measured cost.

## 4. Allocator

**Share at the final head** (**measured**, `profile-final/callgrind.1.out`, 312,733,055 Ir, 49-file
sample, one thread): glibc `malloc.c` 41,255,799 Ir = 13.19% (`_int_free` 4.14%, `malloc` 2.99%,
`_int_malloc` 2.04%, `free` 1.89%, `realloc` 0.59%, `_int_realloc` 0.52%,
`_int_free_merge_chunk` 0.38%, `malloc_consolidate` 0.29%, `unlink_chunk` 0.22%,
`_int_free_maybe_consolidate` 0.14%). Outside the allocator proper: `RawVecInner::finish_grow`
1.07%, `memcpy` 2.95% (part of it from `realloc`). `alloc_count` on the same sample: 237,431
allocations, 120.9 MB requested (**measured**) → 174 Ir per allocation in glibc (**derived**).
The callgrind run had no `--cache-sim` (the profile header's `I1 cache:` and `D1 cache:` are
empty), so no cache-miss figure exists for any lever below.

| allocator | bound on Ir | basis | RSS risk | label |
|---|---|---|---|---|
| `mimalloc` (EXP-002 build) | ≤ 8.0%: 237,431 × 68 Ir = 16.1 M against 41.3 M | EXP-002: mimalloc's own cost ≈ 43 M Ir for 628,959 blocks (68 per block) against glibc 115.7 M (184 per block) | 4–10x at defaults, 1.9x with eager commit off (EXP-002, **measured**) | derived; per-block cost carried over is hypothesis |
| `mimalloc` + `MIMALLOC_ALLOW_THP=0` | as above | mimalloc's readme: `ALLOW_THP` on by default on Linux; eager arena commit "only increases the commit but not the actual" RSS; this host's THP mode is `madvise` (**measured**, `/sys/kernel/mm/transparent_hugepage/enabled`) | hypothesis: the RSS multiple is huge pages backing touched arena regions (2 MiB per first touch, per thread heap), which explains why switching eager commit off cut RSS in EXP-002 | hypothesis |
| `mimalloc` + `MIMALLOC_PURGE_DELAY=0` | EXP-002: +45% wall on `hir` | readme: `0` purges at once, lower memory, slower | 2.4x (56.5 MiB) | measured (EXP-002) |
| `jemalloc` (`tikv-jemallocator`) | ≤ 13.2% × (1 − jemalloc's per-op cost / glibc's); no figure | jemalloc `TUNING.md`: decay times trade CPU for memory, more arenas raise fragmentation; defaults `dirty_decay_ms` 10 s and 4 arenas per CPU (jemalloc(3), hypothesis) | a process that lives 10–450 ms never reaches a decay; peak RSS is what it touches plus per-arena chunks | hypothesis |
| `snmalloc`, `rpmalloc` | ≤ 13.2% × fraction; no figure | message-passing remote frees (snmalloc) and per-thread spans (rpmalloc) target cross-thread frees; E1's unit `String`s are allocated on workers and freed on the main thread (`parallel.rs:240-246`) | per-thread caches, unmeasured | hypothesis |
| bump arena per run (`bumpalo` 3.20.3, in the local registry) for intermediate strings | ≤ 8.4%: 63.4% of blocks are `String` creation and growth (`profile-flat.md` §3) × 13.19% | needs the sink/`String` change another record covers; `bumpalo::collections::String` works on stable | an arena holds every intermediate string until the file ends: up to the 186x-of-input requested bytes (`alloc_count`: 120.9 MB for 0.65 MB) if nothing is reset per item | derived |
| Rust `allocator_api` (per-collection allocators in std) | — | unstable, nightly only (`#![feature(allocator_api)]`, tracking issue rust-lang/rust#32838) | — | read (documentation) |
| glibc tunables | closed | EXP-009: no variant outside the spread | — | measured |

- Acceptance rule derived from protocol §1 and §3 and EXP-002's harness: `Ir` on the 49-file sample
  below head's 312.7 M; `bench -n 5` ranges non-overlapping on 3 of 3 interleaved pairs; process-
  level hyperfine on EXP-002's 12 workloads with wall ranges below head's on most workloads; and
  peak RSS within 1.2x of glibc's on every workload. The 1.2x is the largest RSS increase any kept
  change carries at M3 apart from the E1 split on `stdin-lints` (1.44x, a 15 MiB file); EXP-002's
  4–10x and 1.9x both fail it. Smallest experiment: EXP-002's `cli-mimalloc` build, run with
  `MIMALLOC_ALLOW_THP=0`, then additionally `MIMALLOC_ARENA_EAGER_COMMIT=0`; falsified if RSS stays
  above 1.2x or the wall ranges overlap head's on more than half the workloads.

## 5. Build and binary

**PGO in the release workflow.** M3 measured head-pgo against head: `Ir` 0.7825 (−21.7%),
`bench` 1.20–1.22x MB/s; wall, pgo/head in mean: 0.776–0.924 on one core and 0.799–0.977 on all
cores over the nine crates (**derived** from M3's crate tables, e.g. `fx-hir-ty` 294.2/343.0 and
83.6/95.3). `binaries.yml` builds through `taiki-e/upload-rust-binary-action`
(`.github/workflows/binaries.yml:79-88`) for five targets on three runner kinds (lines 37-52).
EXP-002 drafted the change: `llvm-tools-preview` on the toolchain step, a training step running
`benchmarks/pgo.sh` before the upload action, `RUSTFLAGS=-Cprofile-use=…`, limited to the three
x86-64 targets because the instrumented binary has to run on the runner; `aarch64-apple-darwin`
and `aarch64-pc-windows-msvc` stay unprofiled (cross-built on x86 runners), and Windows needs the
`cygpath` form of the profile path. Cost (**derived** from EXP-002): one instrumented build plus a
training run over the in-repo fixtures (13.9 MB, a few seconds) plus the final build: about 2x the
release build time per x86 target (fat LTO build 46.6 s on EXP-002's host). `cargo-pgo` wraps the
same steps; the script exists and runs (M3 harness notes).

**BOLT** (hypothesis throughout). Needs a relocatable link (`-Wl,--emit-relocs`), a profile from
`perf record` with LBR or from BOLT's instrumentation mode, and `llvm-bolt` with `merge-fdata`,
which `llvm-tools-preview` does not ship. GitHub-hosted runners are VMs; LBR access there is not
documented, so instrumentation is the available mode. BOLT rewrites ELF; macOS and Windows targets
are out. Literature: Panchenko et al., "BOLT: A Practical Binary Optimizer for Data Centers and
Beyond", CGO 2019, report up to 7.0% on top of PGO and LTO for data-center services and up to
20.4% for the GCC and Clang compilers — binaries with large, I-cache-bound text. chloro's `.text`
is 1,419,304 bytes (**measured**, `size -A`); with no I-cache data (no `--cache-sim`) the bound is
not derivable here: 0–5% on top of PGO, labelled hypothesis.

**Other flags.**
- `-C target-cpu=native`: inside the spread in EXP-002 and not shippable; `x86-64-v3` as an extra
  artifact has at most `native`'s headroom (hypothesis).
- The binary is linked by `LLD 22.1.6` (**measured**, `.comment`), so `-Wl,--icf=all` is available;
  its effect is on size, and a size change on a 1.42 MB `.text` has no measured time effect.
  rustc passes `--gc-sections` for executables by default (hypothesis, not checked on this link line).
- `opt-level = "s"` against 3: no data; the experiment is `callgrind --cache-sim=yes` plus
  `bench -n 5` pairs on both builds.
- `panic = "abort"`: rejected for the CLI's join-and-continue report (EXP-002, **read**
  `orchestrate.rs:79-83`).

**Static linking.** `ldd` lists `libgcc_s.so.1`, `libc.so.6` and the loader (**measured**). The
dynamic linker costs 0.26 M Ir of a 0.50 M Ir one-file run (**measured**), ≈ 0.1 ms; with the two
`openat`/`mmap` sequences seen in `strace`, static linking removes ≤ 0.1–0.3 ms of the 1.7 ms
start-up (≤ 6–18%, **derived**). Static musl replaces glibc's malloc with musl's, a change to the
13.19% allocator share in an unmeasured direction (hypothesis), so a musl build is an allocator
experiment as well. rustfmt's start-up includes loading `librustc_driver` (M3 harness notes).

**Huge pages.** Peak RSS 5–31 MiB on the crates (M3). System time is 2.3% of CPU on `hir-ty` on one
core (8.0 of 352.5 ms), 4.8% on 4 threads (17.8 of 370.5 ms), 5.7% on the 1238 fixtures (99.1 of
1,741.7 ms) (**measured**, this window, `--no-cache`); page faults are a part of it. THP for the heap
needs glibc's `glibc.malloc.hugetlb` tunable (glibc 2.39 here) on a `madvise` host: bound ≤ 2.3–5.7%
of CPU (**derived**), not tested by EXP-009's four variants.

## 6. I/O and scheduling

- `mmap` against `read_to_string`: the warm run reads 14.2 MB; a single `cat` process takes
  ≈ 25 ms and four take ≈ 13 ms after shell start (**measured**, this window). `mmap` removes the
  copy (14.2 MB at ≈ 10 GB/s ≈ 1.4 ms CPU, hypothesis on the rate) and adds a mapping, page-table
  faults and `munmap` per file (1238 files, two syscalls each); UTF-8 validation stays. Bound
  ≤ 1.4 ms CPU per 14 MB, ≈ 0.4 ms wall on 4 threads (**derived**); a file truncated during the run
  raises `SIGBUS` under `mmap` (hypothesis on risk class).
- **Queue efficiency at M3** (formatted input, `--check`; wall mean, CPU user+sys over all threads):

| crate | mc wall | mc CPU | CPU/4 | wall ÷ (CPU/4) | 1c wall | 1c ÷ mc |
|---|---|---|---|---|---|---|
| `fx-hir` | 35.3 | 121.0 | 30.3 | 1.17 | 113.8 | 3.22 |
| `fx-hir-ty` | 95.3 | 356.0 | 89.0 | 1.07 | 343.0 | 3.60 |
| `fx-ide-db` | 39.6 | 126.0 | 31.5 | 1.26 | 102.0 | 2.58 |
| `fx-syntax` | 20.3 | 68.8 | 17.2 | 1.18 | 67.8 | 3.34 |
| `fx-parser` | 14.9 | 45.0 | 11.3 | 1.32 | 37.2 | 2.50 |
| `reg-tokio` | 8.8 | 23.0 | 5.8 | 1.53 | 19.8 | 2.25 |
| `reg-regex-syntax` | 81.0 | 281.0 | 70.3 | 1.15 | 253.0 | 3.12 |
| `reg-clap-builder` | 31.4 | 105.0 | 26.3 | 1.20 | 93.3 | 2.97 |
| `reg-winnow` | 33.1 | 114.5 | 28.6 | 1.16 | 100.8 | 3.05 |

- `hir-ty`: 343.0 / 95.3 = 3.60x on 4 cores, efficiency 0.90 (**derived**). A work-conserving
  scheduler cannot go below CPU/4: the bound on any queue change is 1 − 1/(wall ÷ CPU/4) = 6.6%
  (`hir-ty`) to 34.6% (`reg-tokio`) (**derived**). Part of that gap is not queueing: start-up
  1.7–2.4 ms is 19–27% of `reg-tokio`'s 8.8 ms, and the main thread's discovery, sort and `stat`
  (§ 3) run before any worker. On `fx-ide-db` the gap is `lints.rs`'s critical path: 23.5–27.6 ms on
  4 threads (M3 E), of which parse and the 288 KB unit are sequential.
- **File-level and unit-level together.** Files ≥ 128 KiB (`worker.rs:34`) spawn `threads` more
  threads each (`orchestrate.rs:55-61`, `parallel.rs:201-237`) while the file workers keep running:
  up to 8 runnable threads on 4 cores, and the split costs 1.03x–1.19x CPU (M3 E). A single global
  queue of units across files would keep 4 threads and the same critical path (parse plus largest
  unit); its bound is the same CPU/4 line plus the 3–19% split CPU on split files (**derived**).
  Below-unit parallelism (elements of a `const` array) is the only lever on `lints.rs`'s critical
  path (EXP-001's open item).

## Ranked bounds, by scenario

Today's figures are M3 means unless marked. Bounds are upper bounds on the gain; products assume
the levers act on disjoint costs (hypothesis).

### Levers

| # | lever | addresses | bound | exactness / conformance risk | cost | smallest experiment and falsifier |
|---|---|---|---|---|---|---|
| L1 | item-level cache | latency of a re-format after an edit | file time → 0.23–0.63 (`lints.rs`), 0.41 (`nodes.rs`) in process; 0 on a cold run | medium: position rebasing, skip context, outside-span reads (§ 1) | chloro-core: a units API (~150 lines around `parallel.rs`); CLI: indexed store (~400 lines); no new dependency | § 1 harness; any byte difference, or < 2x on files > 128 KiB |
| L2 | resident process | per-request fixed cost | removes 4.0–5.7 ms per request (93–95% of a median-file save) | low for output; lifecycle and stale-binary risk | new mode or binary, protocol; order of the CLI's 650 lines | `fmt_stdin` loop in one process against the CLI per file on 100 saved files; falsified if per-request time is not below 1 ms for median files |
| L3 | tool key without reading the executable (path, size, mtime, inode stamp; re-hash on change) | per-run fixed cost | ≤ 2.3–3.3 ms per run (measured delta, includes the save) | low-medium: a binary replaced with the same stamp keeps old entries | ~30 lines in `cli/cache.rs` | hyperfine `one.rs` cache on/off; falsified if the delta stays above 1 ms |
| L4 | discovery: canonicalize root and symlinks only, one sort, sizes off the main thread | warm and small runs | ≈ 15–17 ms of a ≈ 37 ms warm run (≥ 0.45 of it); ≤ 4% cold | low: dedup exact for non-symlink children (derived); report order unchanged | ~40 lines in `orchestrate.rs`, `cli.rs` | `strace -c` (readlink count to ≈ 0) and hyperfine warm; falsified if warm mean drops < 5 ms |
| L5 | PGO in `binaries.yml` | CPU of every run | Ir −21.7%; wall 0.78–0.92 (1c), 0.80–0.98 (mc) | none on output (gates pass, M3) | CI: `llvm-tools-preview`, one training step, x86-64 only, ≈ 2x build time | already measured (M3); falsifier for the CI build: release artifact `Ir` not below head's |
| L6 | `mimalloc` with `ALLOW_THP=0` (± eager commit off) | allocator Ir | ≤ 8.0% Ir; wall unproven | none on output; RSS (4–10x at defaults) | 1 dependency behind `cli`, 3 lines | § 4 rule: RSS ≤ 1.2x glibc's and wall below on most workloads |
| L7 | `--check` early exit per file | failing CI runs | ≤ 0.78 of differing files' time; 0 on passing runs | medium: `format_failure` under 1.8.0, newline retraction | ~60 lines across worker and visitor | raw fixtures `--check` exit codes and counts identical; time on raw rows |
| L8 | global unit queue across files | multi-core tail | ≤ 6.6–34.6% of mc wall (CPU/4 line), CPU −3–19% on split files | low (same units, same stitching) | ~150 lines in `orchestrate.rs`/`parallel.rs` | M3 crate rows mc, 30 runs; falsified if no crate's range moves below head's |
| L9 | bump arena for intermediate strings | allocator Ir | ≤ 8.4% Ir | low on output; RSS up to the requested-bytes total without per-item reset | depends on the sink/`String` change | `Ir` and `alloc_count` after the sink change |
| L10 | THP via `glibc.malloc.hugetlb` | page faults | ≤ 2.3–5.7% CPU | none | environment or one `mallopt`-equivalent | EXP-009 harness with the tunable |
| L11 | static linking (glibc static or musl) | start-up | ≤ 0.1–0.3 ms of 1.7 ms | musl: allocator changes | CI target change | `tiny-one` 1c, 30 runs |
| L12 | BOLT after PGO | I-cache, branch layout | 0–5% (hypothesis) | none on output; Linux only | `llvm-bolt` install, `--emit-relocs`, instrumentation run | `callgrind --cache-sim` and `bench` pairs against head-pgo |
| L13 | `--write` without the verification pass | `--write` with cache | ≤ 0.5 of rewritten files' time | removes the observed-fixed-point guarantee | ~5 lines | n/a: a guarantee change, not a speed experiment |
| L14 | `mmap` input | read copy | ≤ 0.4 ms wall per 14 MB | `SIGBUS` on truncation | ~20 lines, `unsafe` | warm run hyperfine |

### Scenarios

| scenario | today (M3 unless marked) | levers that apply | compound bound | argument |
|---|---|---|---|---|
| CI cold run, whole tree (1238 fixtures, 4 threads) | 434.3 ms `--no-cache`, 446.3 ms cold cache | L5, L6, L8, L4, L12 | ≥ 0.70x: ≈ 300–310 ms | 434.3 × 0.877 (L5, `hir-ty` mc) × 0.92 (L6, if all of the 8.0% Ir shows in wall, hypothesis) × 0.93 (L8, CPU/4 line: 1,617/4 = 404 ms against 434) − 15 ms (L4) = 298 ms; L12 unbounded here |
| CI cold, one crate (`hir-ty`, 4 threads) | 95.3 ms | L5, L6, L8 | ≥ 0.75x: ≈ 72 ms | 95.3 × 0.877 × 0.92 × 0.934 = 71.8 ms |
| CI warm run (cache, 1238 hits) | 28.9 ms | L4, L3 | ≥ 0.45–0.55x of the warm run: ≈ 13–16 ms | warm-run components (§ 3) minus 15–17 ms (L4) and 1–3 ms (L3) scaled from this window's 37 ms to M3's 28.9 ms; floor = start-up + `find` + parallel read |
| CI warm, a PR that edits k files | 28.9 ms + full format of the k files | L1 (restored cache), L4, L3 | 28.9 → ≈ 14 ms plus 0.23–0.63 of each edited file | file-level misses become unit-level hits (§ 1) |
| editor save, median file (2,955 B) | 4.3–6.0 ms (start-up + cache + 0.31 ms) | L3, L2, L1 | 4.3–6.0 → 2.0–2.7 ms (L3) → ≈ 0.3 ms (L2) → ≈ 0.15 ms (L2 + L1) | § 2 table |
| editor save, `lints.rs` | 29.9–30.9 ms (27.6 + cache) | L1, L2, L3, item reparse | → 6.9–17.6 ms (L1 in process) → 1–11 ms (L2 + L1 + reparse) | § 1 and § 2 tables |
| one huge file, cold (`lints.rs`, 4 threads) | 27.6 ms (23.5 min) | L5, L6, below-unit split | ≥ 0.6x: parse 6.0 + largest unit 10.8 + gaps ≈ 17.5 ms before L5/L6; × 0.86 × 0.92 ≈ 14 ms | the critical path is parse + the 288 KB unit; L1 gives nothing cold; L8 does not shorten it |

## What this settles and what it leaves open

- The cache's executable digest makes every cache-enabled run 2.3–3.3 ms slower than `--no-cache`
  on a one-file run (**measured**, provisional window) — more than chloro's whole 1.7–2.4 ms
  start-up, and absent from M3's `tiny-one` rows, which run `--no-cache`.
- The warm run is dominated by system calls, not instructions: 33.7 ms sys against 14.8 ms user per
  run, 15,488 `readlink` calls from one `canonicalize` per file (**measured**); about 15–17 ms of a
  ≈ 37 ms warm run sits on the main thread before any worker starts (**derived**).
- An item cache's floor is the parse plus unit hashing and gaps: 0.23–0.24 of `lints.rs`'s format
  time and 0.41 of `nodes.rs`'s (**derived**), with `lints.rs` limited to 0.62–0.63 when the edit
  lands in its 288 KB array, which is one unit (**read**, units are top-level only).
- The dependency set of a unit's output is the unit text, `Config`, binary and the file's skip
  context; every absolute position in `UnitOutput` reaches only metadata (gap replay, `last_pos`,
  skipped ranges), not the text (**read**); that the text never reads source outside its span is
  evidence (X12b, 978,843 units), not proof — the blanking check in § 1 tests it.
- Open: an `Ir` and RSS figure for `mimalloc` with `MIMALLOC_ALLOW_THP=0` at this head; jemalloc,
  snmalloc and rpmalloc figures; any I-cache measurement (no `--cache-sim` run exists), hence BOLT
  and `opt-level` bounds; the hit rate of an item cache on real edit histories; incremental reparse
  of the flat tree, which no code supports today; and the wall times of this record's window, which
  ran beside another session's build and are to be re-measured on a quiet host before any
  comparison.
