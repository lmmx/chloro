# EXP-025: a scratch arena for the formatter's intermediates

Ledger row E25 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md) (with E27's amendments: both samples, the per-file figure).
The brief is trick T5 of [../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md)
§ 5 and that record's note that the arena must not be measured together with the sink;
the allocator's time share is [EXP-014](EXP-014-allocators.md)'s (glibc 13–16% of one thread's
time; replacing the global allocator failed the RSS rule and is closed); the site table, the
realloc copies and the heap-layout term are [EXP-020](EXP-020-sink.md)'s; the costs of bump
allocation (retained capacity, peak RSS, lifetimes) are section D of
[../../2026-10-15-external-research-brief/03-research-report.md](../../2026-10-15-external-research-brief/03-research-report.md);
the per-file figure and the registry sample are [EXP-027](EXP-027-harness.md)'s. Labels:
**measured** (a tool's output on the builds below), **read** (from source), **derived**
(arithmetic on measured figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-arena` from `4f1b9d9` (integration branch; its code is the BKG `05ab7b4`, the commits above it are documentation) with `origin/20261015-harness` merged (`21042dc`: E27's `profile.sh --per-file`, `--sample-list`, `bench --phase parse`), worktree `/home/user/wt/arena` |
| base | `21042dc`: the BKG's formatter with E27's `bench`; `bench-base` (sha256 `0be68d2cb0d7a9f2`) and `chloro-base` (`4ddf33f7a0b1aa33`) copied before the first change; the head's are `c15b5ab3259cf7cf` and `8123fab448c1448f` |
| commits | `368add0` (the arena, `formatter/scratch.rs`, and chains' lists and strings in it), `a61d975` (overflow's list items), `6a3df77` (types), `eaab530` (this file and the ledger row); the follow-up: `b2dd419` (the arena keeps pointers; a formatting test sized for Miri), `224870a` (the arena on `bumpalo`), then the section *Soundness and the safe variant* |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, hyperfine 2.0.0, glibc 2.39; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz; other agents built and ran callgrind throughout (load average 6.1–24.5 during this experiment). `Ir` is unaffected; wall-clock figures are **measured, provisional** (protocol § 6) |
| `Ir` commands | `benchmarks/profile.sh --core 3 --bench <bin> --runs 3 --per-file` (fixture sample) and the same with `--sample-list benchmarks/samples/registry49.txt`; `--sim` for the co-metric (on core 2, alongside; the counts do not depend on the core) |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures, recorded under `/home/user/chloro/chloro-core/tests/conformance/fixtures`, so `--root` must name that path) and `registry-e8d87c2.tsv` (25,024 files present, 20,850 recorded) |
| attribution build | the base with a temporary patch (not committed: `scratchpad/e25/trace-hooks.patch`, `alloc_trace.rs`): an `alloc-trace` feature whose hooks mark the phase (parse, formatting, inside a top-level unit) and count top-level units in `visit_items_with_reordering` at depth 0 (the units of `parallel.rs`), and an example `alloc_trace` whose `#[global_allocator]` logs every `alloc`/`dealloc`/`realloc` with its block identity, phase, unit, size, whether glibc moved it, and the frame-pointer chain (24 frames: return and frame addresses); built with `RUSTFLAGS="-C force-frame-pointers=yes" CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` into `target-trace`, symbolised with `addr2line -i` (`scratchpad/e25/scripts/sym.py`), analysed by `an.py`, `an2.py`, `bound.py`, `bound2.py`, `lines.py` |

## Hypothesis and falsification

The formatter's remaining allocator cost (about 12% of instructions in allocator arcs plus
`realloc` copies; 13–16% of one thread's time) is mostly malloc/free pairs of short-lived
intermediate strings and `realloc` growth of the newest one, and a per-item arena (bump
allocation, the newest allocation grown in place, reset at top-level-item boundaries) or a
string pool removes most of it exactly.

Falsified if: instructions are not down by at least 3% on the per-file figure of the fixture
sample; the registry sample regresses; peak RSS exceeds 1.2x the base on `stdin-lints` or
`fx-hir-ty`; any output changes. Per step: reverted if the per-file figure does not fall by at
least half of the step's measured site cost, or the RSS rule fails. Stop rule: if neither
mechanism can reach 3% by the step-1 measurement, nothing is built.

## Commands

```text
S=scratchpad/e25
# step 1: attribution
RUSTFLAGS="-C force-frame-pointers=yes" CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build --release -p chloro-core --example alloc_trace --features alloc-trace --target-dir target-trace
alloc_trace $S/sample-fx $S/trace/fx; alloc_trace $S/sample-reg $S/trace/reg      # the two samples, copied
python3 -I $S/scripts/sym.py $S/trace/fx $S/bin/alloc_trace-base                    # inline chains per return address
python3 -I $S/scripts/an.py $S/trace/fx                                             # lifetimes, kinds, sizes, reallocs, tcache, sites
python3 -I $S/scripts/an2.py $S/trace/fx; python3 -I $S/scripts/lines.py $S/trace/fx '^chains'   # by module, site pair, line
python3 -I $S/scripts/bound2.py $S/trace/fx 230136129                               # bounds of (a) and (b)
python3 -I $S/scripts/cg.py <callgrind.out> <pkl>; python3 -I $S/scripts/allocsplit.py <pkl>   # E20's scripts
rustc -C opt-level=3 $S/micro/m.rs; valgrind --tool=callgrind ./m <mode> 10000 / 110000          # per-operation costs
# measurement, per step
benchmarks/profile.sh --core 3 --bench <bin> --runs 3 --per-file [--sample-list benchmarks/samples/registry49.txt]
python3 -I $S/scripts/allocdiff.py <a.pkl> <b.pkl>     # allocator family, memcpy inside _int_realloc, scratch:: functions
benchmarks/profile.sh --core 2 --bench <bin> --sim
$S/rss.sh c3 chloro-base chloro-c3                   # hyperfine memory_peak_resident, pinned, interleaved
$S/pairs.sh bench-base bench-<step> 3                # taskset -c 3 bench -n 5, order alternating
bench -n 1 [--threads 4] --check fixtures-e8d87c2.tsv --root /home/user/chloro/chloro-core/tests/conformance/fixtures
bench -n 1 --check registry-e8d87c2.tsv --root /root/.cargo/registry/src
target/release/examples/conform -i; target/release/examples/conform -i -V 1.8.0
```

## Step 1: attribution (measured)

### Where the allocations are

One process per sample, every allocation event logged. "Formatter" means the innermost
`chloro-core` frame of the allocating stack is in `src/formatter/` and the event happens after
the parse; "parse" is the parse phase; "parser" is rust-analyzer's parser during formatting
(macro arguments re-parsed).

| | fixture sample | registry sample |
|---|---|---|
| events (alloc, free, realloc) | 177,319, 177,319, 15,541 | 196,261, 196,261, 38,505 |
| formatter blocks (bytes at allocation) | 163,509 (86.96 MB) | 167,914 (137.35 MB) |
| parse / parser-in-formatting / tree blocks | 2,863 / 9,005 / 1,942 | 7,260 / 18,637 / 2,450 |
| formatter `realloc`s | 10,638 | 27,134 |
| formatter allocator arcs (callgrind, inclusive, base) | 27,688,249 (11.93% of 232.05 M), 313,380 calls | 32,918,990 (10.76% of 305.87 M), 324,175 calls |

### Lifetimes

"Unit-local": allocated and freed inside the same top-level unit. "Frame-local": freed while
the physical frame of the innermost formatter function that allocated it is still on the stack
(the frame address and its return address are found in the free's chain); otherwise the block
was returned upward. Inlining makes a physical frame cover several source functions, so
frame-local is a little wider than "freed in the function that made it".

| formatter blocks | fixture | registry |
|---|---|---|
| unit-local | 151,449 (92.6%) | 159,654 (95.1%) |
| outlive their unit (the memo's clones until `clear_memo`; `nodes::to_cached`) | 11,675 (7.1%) | 7,509 (4.5%) |
| file-level (outside every unit) | 385 | 751 |
| unit-local and frame-local | 68,962 (45.5% of unit-local) | 62,004 (38.8%) |
| unit-local, returned upward | 82,487 | 97,650 |
| unit-local and freed in the module that allocated them | 77,275 | 81,328 |
| lifetime in allocation events, median / p90 / p99 | 9 / 201 / 7,448 | 8 / 102 / 4,357 |

### Kinds and sizes

| | fixture | registry |
|---|---|---|
| `String` | 97,049 (59.4%) | 108,295 (64.5%) |
| `Vec` | 53,920 (33.0%) | 43,594 (26.0%) |
| `Rc` (`RewriteContext::new`, via `get_context`) | 12,119 (7.4%) | 14,711 (8.8%) |
| `HashMap`, `Box`, other | 421 | 1,314 |
| request ≤ 16 / ≤ 32 / ≤ 64 / ≤ 128 / ≤ 256 / ≤ 1032 / larger bytes | 50,338 / 27,133 / 20,629 / 26,972 / 21,574 / 15,259 / 1,604 | 65,772 / 30,073 / 19,430 / 28,545 / 13,421 / 8,908 / 1,765 |

### `realloc`

"Newest": no other allocation (of any block, or, second figure, of any unit-local formatter
block, which is what an arena holding those blocks would see) happened since this block was
allocated or last grown, so a bump arena could grow it in place.

| formatter `realloc`s | fixture | registry |
|---|---|---|
| all | 10,638 | 27,134 |
| of the newest allocation (whole stream) | 5,689 | 19,298 |
| of unit-local blocks: newest in the unit-local stream / older | 5,711 / 4,713 | 19,222 / 7,583 |
| moved by glibc (bytes copied) | 5,088 (939,315 B) | 17,350 (1,596,739 B) |
| older and moved (an arena could not avoid these copies either) | 2,654 (497,693 B) | 5,625 (538,157 B) |
| for comparison: parse-phase `realloc`s moved (bytes copied) | 1,226 (5,470,848 B) | 1,450 (6,908,720 B) |

The `memcpy` inside `_int_realloc` (5.18 M on the fixture sample) is mostly the parse's vector
growth (5.47 MB copied against the formatter's 0.94 MB): EXP-020's heap-layout term, not the
formatter's.

### glibc's fast path

| | fixture | registry |
|---|---|---|
| `malloc` calls; reaching `_int_malloc` (callgrind arcs) | 177,764; 11,203 (6.3%), so 93.7% served by the tcache | 197,112; 11,280 (5.7%), 94.3% |
| `malloc` `Ir` per call: tcache path / `_int_malloc` path / mean | 44.4 / 335 / 62.8 | 44.2 / 415 / 65.4 |
| `free` `Ir` per call (mean); reaching `_int_free_merge_chunk` | 92.5; 10,120 (5.7%) | 95.4; 15,837 (8.0%) |
| `realloc` `Ir` per call (mean, with its copies) | 651 | 428 |
| tcache simulation of the formatter's blocks (7 per bin, ≤ 1032 B): `malloc` hits; `free`s kept | 91.0%; 90.4% | 87.0%; 86.1% |

A malloc/free pair on the tcache path costs about 125–137 `Ir`; nine in ten formatter pairs take
it. glibc's tcache is already a per-thread free list, so a pool competes with it, an arena does
not.

### Per-operation costs of the two mechanisms (micro-benchmark)

`scratchpad/e25/micro/m.rs`, `rustc -C opt-level=3`, callgrind, `Ir` per iteration from the
difference of 10,000 and 110,000 iterations; each iteration makes a 16–48-byte string, pushes a
word and drops it ("two" makes a second, larger string that copies the first, as a caller does
with a returned string).

| mode | one string | two strings |
|---|---|---|
| no allocation (stack buffer) | 84 | 135 |
| `String` (glibc, tcache path every time) | 252 (pair ≈ 168) | 478 (≈ 172 per string) |
| bump arena (thread-local, reset every 64) | 92 (≈ 8) | 161 (≈ 13 per string) |
| pool of `String`s (thread-local `RefCell<Vec<String>>`, take/give) | 166 (≈ 82) | 311 (≈ 88 per string) |

### Sites (fixture sample; base)

Innermost formatter function; blocks, share unit-local, share frame-local, `realloc`s (newest in
the unit-local stream / moved / bytes copied).

| blocks | unit-local | frame-local | reallocs | site | kind |
|---|---|---|---|---|---|
| 12,074 | 99.7% | 0% | 0 | `context::new` (`get_context`'s `Rc`s) | Rc |
| 9,857 | 4.7% | 0.1% | 9 | `context::memoize` (the memo's clones) | String, Vec |
| 9,518 | 99.8% | 25.9% | 134 / 76 / 7,378 | `utils::concat` | String |
| 8,862 | 100% | 0.3% | 672 / 111 / 3,222 | `types::single_ident_path` | String |
| 8,842 | 100% | 0% | 4 | `overflow::Context::rewrite` (the call's string) | String |
| 7,597 | 100% | 100% | 13 | `chains::from_ast` (entry lists, call arguments) | Vec |
| 6,859 | 100% | 100% | 53 / 51 / 16,960 | `overflow::rewrite_items` (the list items) | Vec |
| 5,803 | 100% | 0% | 1,441 / 372 / 15,048 | `comment::combine_strs_with_missing_comments` | String |
| 5,498 | 100% | 100% | 89 / 29 / 597 | `chains::rewrite_method_call` | String, Vec |
| 4,628 | 100% | 100% | 0 | `types::{closure#0}` (4,525 of them `segment_ident`'s copy of the name, types.rs:52) | String, Vec |
| 4,525 | 100% | 100% | 1 | `types::rewrite_segment` | String |
| 3,754 | 99.9% | 0% | 92 / 67 / 1,289 | `nodes::non_trivia_text` | String |
| 3,047 | 100% | 100% | 2 | `chains::make_subexpr_list` | Vec |
| 3,010 | 100% | 100% | 0 | `chains::ChainFormatterShared::new` (`rewrites`) | Vec |
| 2,969 | 100% | 100% | 0 | `patterns::rewrite_ident_pat` | String |
| 2,438 | 100% | 0% | 0 | `types::path_segments` | Vec |
| 1,476 | 100% | 57.4% | 2,235 / 1,232 / 103,047 | `items::rewrite` | String |

### Bounds (derived)

Saving per block: an arena removes the pair (155.3 `Ir`, the profile's means) and costs about
what the Rust-side allocation code it replaces costs (the micro-benchmark: 8–13 against ≈ 170 for
the whole `String` pair); a pool saves ≈ 85. A `realloc` costs 320 plus 0.81 `Ir` per byte when
glibc moves it; an arena grows the newest in place (≈ 10) and copies the others. Reach sets:
"hot paths" are the brief's (lists, chains, overflow, types, `push_rewrite_inner`) and
EXP-020's follow-up sites (`rewrite_macro`, `rewrite_attrs`, `rewrite_closure_fn_decl`,
`from_context`, `get_context`'s `Rc`s). Bounds as a share of the per-file net figure.

| mechanism and reach | fixture blocks | fixture bound | registry blocks | registry bound |
|---|---|---|---|---|
| (a) arena, every unit-local formatter block (needs the protocol changed: an arena type returned between modules) | 151,449 | 26.93 M, 11.70% | 159,654 | 33.71 M, 11.07% |
| (a) arena, unit-local and freed in the allocating module (module-local types, protocol kept) | 77,275 | 13.19 M, 5.73% | 81,328 | 17.16 M, 5.64% |
| (a) arena, unit-local and frame-local | 68,962 | 11.33 M, 4.92% | 62,004 | 13.12 M, 4.31% |
| **(a) arena, hot paths, module-local** | 47,160 | **7.95 M, 3.45%** | 35,053 | **6.45 M, 2.12%** |
| (a) arena, hot paths, frame-local | 41,566 | 6.58 M, 2.86% | 28,538 | 4.79 M, 1.57% |
| (b) pool, every formatter `String` (take and give at every site) | 97,049 | 11.83 M, 5.14% | 108,295 | 17.81 M, 5.85% |
| (b) pool, `String`s taken and given back on the hot paths | 32,269 | 3.53 M, 1.53% | 24,770 | 3.08 M, 1.01% |
| (b) pool, taken on the hot paths, given back wherever dropped | 47,192 | 5.40 M, 2.35% | 38,412 | 4.68 M, 1.54% |

The pool's bounds assume every `realloc` of a pooled string avoided (an upper bound). With an
arena cost of 15 instead of the measured 8–13 the hot-path module-local bound is 7.24 M (3.15%);
the cost model over-states the formatter's arcs by about 6% (29.56 M modelled against 27.69 M
measured), which puts it near 3.2%.

### Reading and the stop rule

- The hypothesis's premise holds: the formatter's allocator cost is malloc/free pairs of
  short-lived blocks (92.6% and 95.1% die inside their unit; median lifetime 8–9 allocation
  events), mostly small (77% of requests ≤ 128 bytes on the fixture sample, 86% on the registry sample),
  mostly strings (59–65%). `realloc` is a
  minority (10,638 against 163,509 blocks on the fixture sample, about 4.2 M of the 27.7 M), and
  only 55% (fixture) to 72% (registry) of the unit-local `realloc`s grow the newest allocation.
- What an arena can reach without changing the `Option<String>` protocol is half of that:
  54% (fixture) and 61% (registry) of the unit-local blocks are returned upward from the frame
  that made them, and 49% (both samples) leave their module, as `String`s. The ceiling of a
  protocol-changing arena (11–12%) is the T5 rewrite of every signature, not a prototype.
- The data favour the arena over the pool: the pool's own operations cost about half of glibc's
  tcache path (82–88 against ≈ 170 per string), so it saves half what the arena saves per block
  and reaches fewer blocks (strings only; both ends of each string's life converted).
- Stop rule: the arena on the hot paths with module-local types bounds at 3.45% of the
  per-file figure on the fixture sample (2.12% on the registry sample); the pool at 1.53–2.35%.
  The arena can reach 3% only with almost all of that set realised, so the rule does not stop the
  experiment; the pool is not built.

## Step 2: what was built (read)

### The arena (`368add0`, `formatter/scratch.rs`)

- A bump arena per thread (`thread_local!`, `const` initialised). An allocation bumps a
  pointer in the current chunk (16 KiB first, doubling to 1 MiB, larger when one request needs
  it); a new chunk comes from the global allocator.
- The newest allocation grows in place; a dropped value's storage is given back when it is the
  newest (its end is the bump pointer).
- `live` counts the values that hold storage. When it reaches zero the arena resets: the bump
  pointer returns to the start of the newest chunk, the other chunks are freed, and the newest
  too when above 1 MiB. Every scratch value lives inside one rewrite of one top-level item, so
  the arena is empty at every unit boundary (`parallel.rs`'s units) at the latest; in practice
  it resets far more often (4,786 times on the fixture sample, below). No hook in the item loop
  is needed and the output buffer's lifetime is untouched.
- `ScratchVec<T>` (push, pop, extend, collect, deref to a slice, `Debug`) and `ScratchString`
  (push, `push_str`, `fmt::Write`, deref to `str`). Neither is `Send` nor `Sync`. Values that leave
  a module stay `String`s, so the `Option<String>` protocol between modules is unchanged.
- Safety argument (module documentation): a chunk byte is handed out at most once between
  resets, and a reset or a chunk's release happens only when no value holds storage; a value
  forgotten with `mem::forget` stops resets (the arena only grows) but never frees storage in
  use; values cannot reach another thread's arena. Six unit tests compare the types with `Vec`
  and `String` over interleaved growth, check in-place growth and the newest's give-back, a
  reset when the last value is dropped, new chunks for large values and the release of a chunk
  above 1 MiB, that elements are dropped once, and that each thread has its own arena.
- No new dependency (`bumpalo` 3.20.3 is in the registry; its `Bump` has no `live` count and
  its collections carry a lifetime the call sites would have to thread).

### The three paths

| commit | path | malloc/free pairs removed (fixture sample) |
|---|---|---|
| `368add0` | chains: the sub-expression list, `rev_children` (consumed with `pop`/`last`) and the chain's `children`, a method call's generic and value arguments, the shared formatter's `rewrites`, a method call's callee string, `trim_tries`'s result and line buffer | 18,607 |
| `a61d975` | overflow: `Context::rewrite_items`'s list items (`write_list_into` and `write_list_capacity` take the slice, the `&[ListItem]` instantiation `reorder.rs` already uses) | 6,858 |
| `6a3df77` | types: `path_segments` (room for four) and `rewrite_segment`'s text | 6,963 |

Not converted, with the reason: `push_rewrite_inner` makes no allocation of its own (its cost is
the output buffer's growth and the frees of other modules' strings); `types::single_ident_path`,
`overflow::Context::rewrite`'s result, `ChainItem::rewrite`'s string and the
`rewrite_last_item_with_overflow` placeholder leave their module (the `Rewrite` trait, `ListItem`'s
fields); `segment_ident`'s copy (4,525 blocks) is a pure copy, better removed than moved into
an arena; `Context::new`'s item vector is a reused allocation when its callers pass a `Vec`;
`memoize` is E28's; `get_context`'s `Rc`s are not strings and live in `context.rs`.

## Results (measured)

### Instructions, per step

Per-file figures are the sum of each file's minimum over three runs, net of the empty sample;
"whole" is the minimum of three runs of the whole sample. "Site cost" is the fall of the
allocator family's exclusive `Ir` (`malloc`, `free`, `realloc`, `_int_*`, `unlink_chunk`,
`malloc_consolidate`, the `RawVec` growth helpers) between the whole-sample profiles of the
step and its parent, without the `memcpy` inside `_int_realloc` (the heap-layout term).

| step | fixture whole | fixture per-file net | change, per-file | site cost (pairs removed) | per-file / site | registry whole | registry per-file net | change, per-file | registry site cost | per-file / site | decision |
|---|---|---|---|---|---|---|---|---|---|---|---|
| base `21042dc` | 232,051,557 | 230,136,129 | | | | 305,873,581 | 304,433,417 | | | | |
| `368add0` arena, chains | 229,262,911 | 228,311,993 | −1,824,136 (−0.79%) | 2,655,224 (18,607) | 69% | 304,728,718 | 303,558,119 | −875,298 (−0.29%) | 1,290,185 (9,489) | 68% | kept |
| `a61d975` overflow | 227,938,553 | 227,394,246 | −917,747 (−0.40%) | 895,199 (6,858) | 103% | 304,519,318 | 303,087,234 | −470,885 (−0.16%) | 546,260 (4,197) | 86% | kept |
| `6a3df77` types | 228,741,632 | 226,446,273 | −947,973 (−0.42%) | 1,094,985 (6,963) | 87% | 302,919,449 | 301,988,902 | −1,098,332 (−0.36%) | 1,242,963 (7,707) | 88% | kept |
| **head against base** | **−3,309,925 (−1.43%)** | **−3,689,856 (−1.60%)** | | 4,645,408 (32,428) | 79% | **−2,954,132 (−0.97%)** | **−2,444,515 (−0.80%)** | | 3,079,408 (21,393) | 79% | |

`a61d975`'s tree is the one measured as "overflow and chains" before the commits were reordered
(identical binary, sha256 `38d742f933cda627`), and `6a3df77`'s the one measured as the head
(`c15b5ab3259cf7cf`); see *Rejected attempts* for the reorder.

Full rows (per-file sums, empty sample, cross-file term):

| build | sample | per-file sum | empty sample | whole net | cross-file term |
|---|---|---|---|---|---|
| base | fixtures | 245,724,303 | 318,126 | 231,733,431 | +1,597,302 |
| base | registry | 320,021,591 | 318,126 | 305,555,455 | +1,122,038 |
| `368add0` | fixtures | 243,907,223 | 318,270 | 228,944,641 | +632,648 |
| `368add0` | registry | 319,153,349 | 318,270 | 304,410,448 | +852,329 |
| `a61d975` | fixtures | 242,989,476 | 318,270 | 227,620,283 | +226,037 |
| `a61d975` | registry | 318,682,464 | 318,270 | 304,201,048 | +1,113,814 |
| `6a3df77` | fixtures | 242,040,915 | 318,258 | 228,423,374 | +1,977,101 |
| `6a3df77` | registry | 317,583,544 | 318,258 | 302,601,191 | +612,289 |

The whole-sample figure of the types step rose by 0.80 M while its per-file figure fell by 0.95 M:
the `memcpy` inside `_int_realloc` rose by 1.76 M (EXP-020's heap-layout term: the parse's vectors
of the next file meet another heap). On the registry sample the same step fell on both.

### Where it went (fixture sample, base against head, whole-sample profiles)

| quantity | base | head |
|---|---|---|
| `malloc` / `free` calls | 177,764 / 177,764 | 145,336 / 145,337 |
| allocator family, exclusive | 35,240,073 | 30,594,665 (−4,645,408) |
| formatter allocator arcs (`allocsplit.py`) | 27,688,249 (313,380 calls) | 23,079,448 (248,455 calls) |
| registry: formatter allocator arcs | 32,918,990 (324,175 calls) | 29,771,099 (281,248 calls) |
| `scratch::` functions and the scratch types' drop glue, exclusive | 0 | 908,600: out-of-line drops of `ScratchVec<ListItem>` 430,673, `ScratchVec<ChainItem>` 267,892 and `ScratchVec<String>` 167,267 (they drop the elements, which `Vec`'s drop did inline before), `ScratchVec<String>::push` 38,456, the rest 2,312; besides, `drop_glue::<ChainItemKind>` is a function of its own now (184,223) |
| `memcpy` inside `_int_realloc` | 5,179,920 | 5,671,188 |
| formatter blocks (trace at the head) | 163,509 | 131,081 (−32,428) |
| registry: formatter blocks | 167,914 | 146,521 (−21,393) |

### The arena at work (measured; the head with counters, temporary, one process per sample)

| | fixture | registry |
|---|---|---|
| first allocations (bytes) | 32,433 (5,196,753) | 21,397 (3,337,023) |
| grown in place / grown by a move (bytes copied) | 48 / 21 (6,720) | 119 / 22 (37,120) |
| storage given back as the newest | 19,391 (60%) | 15,195 (71%) |
| resets (`live` reached zero) | 4,786 | 4,632 |
| chunks taken / chunk releases | 4 / 3 | 3 / 1 |
| most bytes of chunks held at once | 196,608 | 114,688 |
| most bytes bumped in one chunk | 65,241 | 41,152 |

The arena never held more than 192 KiB, and after the run it holds one chunk (at most 1 MiB by
construction) per thread that formatted.

### Co-metric (`--sim`, fixture sample, one run each)

| | base | head | change |
|---|---|---|---|
| `Ir` | 232,051,721 | 228,744,976 | −1.43% |
| `Bcm` | 2,938,083 | 2,944,771 | +0.2% |
| `Bim` | 708,116 | 672,061 | −5.1% |
| `Bcm+Bim` | 3,646,199 | 3,616,832 | −0.8% |
| `I1mr` | 4,694,667 | 4,855,563 | +3.4% |
| `D1mr` / `D1mw` | 689,162 / 533,800 | 699,206 / 554,631 | +1.5% / +3.9% |

Fewer indirect mispredicts (glibc's calls gone), more instruction-cache misses (new
monomorphisations: the scratch types' code and drop glue for each element type) and slightly more
data misses.

### Peak RSS (measured)

`rss.sh`: hyperfine `-N -i -w 1 -r 10 --metrics time_wall_clock,memory_peak_resident`, base and
the commit in one invocation, `chloro --check --no-cache <path>` pinned with `taskset -c 3` (one
worker); `lints-t4` unpinned with `--threads 4` (item-level threads, an arena per worker). Maximum
over the ten runs, MiB; the base column is the head's invocation (the base read within ±0.6% in
the other two, `lints-t4` within ±4%). 14:14 UTC (head) and 14:43 UTC, load 11–15.

| workload | base | `368add0` | `a61d975` | `6a3df77` (head) | head / base |
|---|---|---|---|---|---|
| `stdin-lints` (`ide-db/src/generated/lints.rs`, 639,689 B) | 9.95 | 9.95 | 9.95 | 9.89 | 0.995 |
| `lints`, `--threads 4` | 14.62 | 14.33 | 14.75 | 14.48 | 0.990 |
| `syntax/src/ast/generated/nodes.rs` | 14.00 | 13.95 | 14.01 | 13.95 | 0.997 |
| `hir/src/lib.rs` | 9.00 | 8.95 | 8.95 | 8.95 | 0.995 |
| `ide/src/hover/tests.rs` | 4.75 | 4.78 | 4.78 | 4.70 | 0.988 |
| `rust-analyzer/src/config.rs` | 5.38 | 5.42 | 5.40 | 5.35 | 0.995 |
| `fx-hir-ty` (`/tmp/chloro-bench/formatted/fx-hir-ty/tree`, 122 files) | 8.08 | 8.06 | 7.84 | 7.82 | 0.967 |

Every commit is within 0.967–1.043x of the base in its own invocation (the largest, `lints-t4` at
`a61d975`, 14.75 against 14.14 MiB, is the run-to-run spread of the four-thread run: the base read
14.14–14.72 across the three invocations); far inside the 1.2x rule. The arena holds at most
192 KiB (above), which is what the table shows: nothing.

### Wall clock (measured, provisional)

`pairs.sh`: `taskset -c 3 bench -n 5` on the 1238 fixtures (13.9 MB) under
`/home/user/chloro/chloro-core/tests/conformance/fixtures`, the base binary and a commit's,
order alternating, three pairs (six in the last row). Other agents' builds and callgrind runs
shared the host and core 3 throughout; none of these is a speedup claim.

| binary (per-file `Ir` against the base) | window UTC, load (1, 5, 15 min) at start | base MB/s per pair | commit MB/s per pair | commit / base per pair | ranges |
|---|---|---|---|---|---|
| overflow alone (the pre-reorder `f358c6f`, −0.41%) | 14:31–14:33, 13.84 11.67 12.78 | 5.14, 6.65, 6.16 | 6.17, 6.69, 6.41 | 1.200, 1.006, 1.041 | overlap |
| `368add0` (−0.79%) | 14:57–14:58, 12.17 14.32 14.27 | 8.08, 7.10, 6.58 | 8.68, 8.59, 6.62 | 1.074, 1.210, 1.006 | overlap |
| `a61d975` (−1.19%) | 14:33–14:34, 13.00 11.92 12.78 | 6.25, 6.61, 8.00 | 7.48, 7.32, 7.08 | 1.197, 1.107, 0.885 | overlap |
| `a61d975` again | 14:59–15:00, 13.88 14.06 14.16 | 7.86, 7.69, 7.83 | 7.85, 6.72, 6.78 | 0.999, 0.874, 0.866 | overlap |
| `6a3df77`, head (−1.60%) | 14:30–14:31, 13.08 10.82 12.63 | 6.59, 6.55, 5.90 | 5.56, 5.89, 6.53 | 0.844, 0.899, 1.107 | overlap |
| head again | 14:34–14:35, 11.63 11.63 12.61 | 4.56, 5.99, 6.56 | 8.60, 6.04, 7.23 | 1.886, 1.008, 1.102 | overlap |
| head again | 14:58–14:59, 12.68 14.02 14.16 | 8.10, 8.23, 7.15 | 7.92, 7.56, 6.57 | 0.978, 0.919, 0.919 | overlap |
| head, six pairs | 15:01–15:03, 14.16 14.08 14.16 | 7.65, 7.40, 7.21, 7.76, 7.72, 7.05 | 7.59, 7.50, 7.75, 7.66, 7.57, 8.08 | 0.992, 1.014, 1.075, 0.987, 0.981, 1.146 | overlap |

The same base binary read 4.56–8.23 MB/s within 35 minutes. Over the fifteen head pairs the
ratio's median is 0.992 (0.844–1.886); a −1.6% instruction change, with `I1mr` +3.4% against it,
is not resolved by wall clock on this host. A quiet-host run is owed (binaries:
`scratchpad/e25/bin/bench-{base,k1,k2,c3}`).

## Gates (measured)

| gate | corpus | result |
|---|---|---|
| `cargo fmt --all --check` | | clean at `368add0`, `a61d975` and `6a3df77` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean at `368add0`, `a61d975` and `6a3df77` |
| `cargo test --workspace --all-features` | | 303 passed, 0 failed, 1 ignored at `368add0` and at `6a3df77`'s tree; and at `a61d975` (six tests added in `scratch.rs`) |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures against `fixtures-e8d87c2.tsv` | 0 changed, 0 differ between 4 threads and `format_source`, at each commit |
| `bench --check` | registry against `registry-e8d87c2.tsv` (20,850 recorded of 25,024) | 0 changed, at each commit |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220, 0 not idempotent, at both versions, at `6a3df77`'s tree |
| peak RSS | the table above | 0.967–1.043x of the base at every commit |

`config_matrix` (gates 6 and 7) was not run: no output changed on either hash corpus.

## Rejected attempts (measured)

| attempt | measurement | why | decision |
|---|---|---|---|
| the pool of `String` buffers | not built | step 1: per-string cost 82–88 `Ir` against the arena's 8–13 and glibc's ≈ 170; bound 1.53–2.35% (fixture) on the hot paths, strings only, both ends of each life converted | not built |
| the arena's first form: `with_capacity` and the first growth out of line (`set_capacity`), `collect` through `new` + `extend` with the first slot taken by the cold `grow_for` | overflow alone: per-file −1,281,327 (fixture), −615,743 (registry); whole −1,997,995 | `set_capacity` ≈ 29 `Ir` and `grow_for` ≈ 45 per first allocation (9,435 and 6,912 calls over the three paths); the list's collect loop was inlined into `Context::rewrite` by accident of size | replaced before the commits |
| `collect` as `Vec`'s (first element, then room for `max(4, lower + 1)`) | whole +202,098 against the first form with all three paths | the collect became an out-of-line function (1.72 M, as `Vec`'s `from_iter` is in the base) | replaced by an `extend` that takes room for four on the first element, and `#[inline(always)]` on the bump, the first allocation and the release |
| the commits in the order overflow, chains, types (`f358c6f`, `e53685f`, `9699ba1`, not pushed) | overflow first: per-file −951,027 (fixture), −401,863 (registry) against the base; the later two trees are `a61d975`'s and `6a3df77`'s | the first commit failed clippy `-D warnings`: with overflow as the only user, `ScratchString` and three `ScratchVec` methods were dead code | reordered so that the arena arrives with chains, which uses all of it; only `368add0` was measured anew |

Overflow measured first (−951,027) and second (−917,747) agree within 4%; the first form's
−1,281,327 for the same step is the collect loop's inlining, not the arena.

## Soundness and the safe variant (follow-up, measured)

The integrator held the merge: `scratch.rs` was a hand-written arena-backed `Vec` and `String`
with thirteen `unsafe` blocks (596 lines with its tests), which meets protocol § 1.4 in letter
(a safety argument, six tests) but is a large surface for −1.60% / −0.80%. Two questions:
does Miri find anything, and does a safe arena (`bumpalo`, whose `'bump` lifetime proves at
compile time that no scratch value outlives its scope) keep the gain.

### Miri

- Installed with `rustup component add miri rust-src --toolchain nightly-2026-05-10` (through
  the proxy, under two minutes) and `cargo +nightly-2026-05-10 miri setup` (sysroot built in
  1 min 31 s).
- **Run 1**, `6a3df77`'s `scratch.rs`, Miri's defaults (Stacked Borrows, permissive
  provenance), `cargo +nightly-2026-05-10 miri test -p chloro-core --lib scratch`:
  `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 203 filtered out; finished in
  173.32s`. No undefined behaviour reported. Two warnings, verbatim heads:
  `warning: integer-to-pointer cast --> chloro-core/src/formatter/scratch.rs:157:21` (in
  `Arena::free_chunks`, `base as *mut u8`, reached from `Arena::release`) and the same at
  `scratch.rs:182:25` (`Arena::drop`). The arena kept its pointers as `usize` addresses and
  cast them back; under permissive provenance Miri gives such pointers wildcard provenance and
  cannot check their accesses precisely, so run 1 was weak evidence.
- **Fix, `b2dd419`:** the arena's current pointer, chunk start and end, and its chunk list are
  `*mut u8` derived from each chunk's allocation (`with_addr` for the aligned bump, `add`
  elsewhere); no integer-to-pointer cast is left. The bump arithmetic is the same: per-file
  `Ir` +26,688 (+0.012%) on the fixture sample and +20,157 (+0.007%) on the registry sample
  against `6a3df77` (226,472,961 and 302,009,059).
- The snapshot test chosen first (`tests::formatter::method_chains::struct_literal_receiver_in_call_arg`,
  a chain, an overflowed call and a two-segment path) cannot run under Miri: `error:
  unsupported operation: 'open' not available when isolation is enabled`, from `insta`'s
  `env.rs`, which spawns `cargo metadata` before comparing. `b2dd419` adds
  `scratch::tests::formatting_through_the_arena_users`: one input through all three users
  (a broken chain with generic method calls, an overflowed argument list, multi-segment paths
  with generic arguments), compared with rustfmt's output by `assert_eq!`, and checks that the
  arena was used and is empty afterwards.
- **Final runs on `b2dd419`**, the seven `scratch` tests:
  - `MIRIFLAGS=-Zmiri-strict-provenance` (Stacked Borrows): `test result: ok. 7 passed; 0
    failed; 0 ignored; 0 measured; 203 filtered out; finished in 178.23s`;
  - `MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-tree-borrows"`: `test result: ok. 7 passed; 0
    failed; 0 ignored; 0 measured; 203 filtered out; finished in 178.23s`.
  No undefined behaviour, no warning, no leak report in either.
- What this shows: the executions the tests make (interleaved growth in place and by move,
  give-back of the newest, resets, chunk release above 1 MiB, element drops, two threads, and
  one real formatting input through chains, overflow and types) are free of undefined
  behaviour under both aliasing models with strict provenance. What it does not show: that the
  dynamic invariant (no reset while a value holds storage) holds for every input; that rests on
  the `live` count and on the review argument in the module documentation.

### The safe variant: `bumpalo`

`224870a` replaces the hand-written arena by `bumpalo` 3.20.3 (feature `collections`, no
default features, no transitive dependency; it was already in the registry):

- `scratch.rs` (74 lines, no `unsafe`): a `RefCell<Bump>` per thread; `with_scratch(|bump| ..)`
  lends `&Bump` to a closure whose result cannot borrow it, so every scratch value is proved at
  compile time not to outlive the closure; `reset_scratch()` takes the bump mutably through
  `try_borrow_mut` and so can only reset when no closure holds it.
- The sites use `bumpalo::collections::{Vec, String}` (`with_capacity_in`, `from_iter_in`,
  `new_in`, `.bump()`), the same allocations as `6a3df77`. Entry points: `rewrite_chain_from`,
  `overflow::Context::rewrite`, `types::rewrite_path` (and the two `path_segments` readers in
  `types.rs` and `patterns.rs`). The `'bump` lifetime is the existing `'a` of `ChainItemKind`,
  `Chain` and `ChainFormatterShared` (the syntax handles are covariant, so they shorten to the
  bump's borrow); `rewrite_path_segments` gains a parameter (`#[allow(clippy::too_many_arguments)]`).
- **Variant A, as specified:** reset at every unit boundary of `visit_items_with_reordering`
  only. **Variant B, committed:** A plus a reset when the outermost `with_scratch` returns (the
  point at which the hand-written arena's `live` count reached zero).

`Ir` (per-file net; whole in brackets), three runs each, `--core 3`, 15:50–16:02 UTC at load 1–4:

| build | fixture sample | against base | against `6a3df77` | registry sample | against base | against `6a3df77` |
|---|---|---|---|---|---|---|
| base `21042dc` | 230,136,129 (232,051,557) | | | 304,433,417 (305,873,581) | | |
| hand-written, `6a3df77` | 226,446,273 (228,741,632) | −1.603% | | 301,988,902 (302,919,449) | −0.803% | |
| hand-written, pointers, `b2dd419` | 226,472,961 (228,767,736) | −1.592% | +0.012% | 302,009,059 (302,943,310) | −0.796% | +0.007% |
| `bumpalo` A (reset at unit boundaries) | 226,509,274 (227,605,075) | −1.576% | +0.028% | 301,985,671 (305,880,171) | −0.804% | −0.001% |
| **`bumpalo` B, `224870a`** | **226,560,329** (227,577,469) | **−1.554%** | +0.050% | **302,021,415** (304,929,947) | **−0.792%** | +0.011% |

B keeps 97% of the hand-written arena's per-file gain on the fixture sample (3.58 M of 3.69 M)
and 99% on the registry sample (2.41 M of 2.44 M). The whole-sample figures move with the
heap-layout term in both directions (cross-file terms of 0.70 M and 2.59 M for B against
1.98 M and 0.61 M for `6a3df77`): `bumpalo`'s retained chunk is another size than the
hand-written arena's, and the parser's vectors of the next file meet another heap. On the
registry sample A's whole figure reads +6,590 over the base while its per-file figure reads
−0.80%; by E27's rule the per-file figure is the lever's own effect.

Peak RSS (`rss.sh`, the same harness as above, maximum over ten runs, MiB; 15:50 and 15:56 UTC,
load 1.0–3.8). The base column is A's invocation; B's ratios are against the base of its own
invocation, which read within 1.6% of these):

| workload | base | A | A / base | B | B / base |
|---|---|---|---|---|---|
| `stdin-lints` | 9.95 | 10.09 | 1.014 | 9.95 | 1.001 |
| `lints`, `--threads 4` | 14.52 | 14.80 | 1.019 | 14.49 | 0.989 |
| `nodes.rs` | 14.00 | 14.01 | 1.001 | 13.95 | 1.001 |
| `hir/src/lib.rs` | 8.94 | 9.02 | 1.009 | 8.95 | 0.995 |
| `hover/tests.rs` | 4.70 | 4.68 | 0.997 | 4.77 | 1.019 |
| `config.rs` | 5.38 | 5.69 | 1.059 | 5.41 | 1.003 |
| **`fx-hir-ty`** | 8.08 | **9.89** | **1.224** | 8.16 | 1.010 |

A fails the 1.2x rule on `fx-hir-ty`; a repeat gave the same (median 9.78 against 8.03 MiB,
maximum 9.84 against 8.08; the hand-written arena in the same invocation 7.78 and 7.83). A top-level
unit can hold thousands of rewrites (an `impl` block is one unit), and `bumpalo` reclaims only
the very last allocation on drop, so with resets at unit boundaries alone the scratch of a whole
`impl` stays until its end. B resets as often as the hand-written arena did and stays within
0.989–1.019x.

Gates on `224870a`: `cargo fmt --check`; clippy `-D warnings`; `cargo test --workspace
--all-features` 298 passed, 0 failed, 1 ignored (the six tests of the hand-written types go with
them, the formatting test stays); 0 of 1238 fixtures changed, sequential and `--threads 4`, 0
differ between the two; 0 of the registry hashes changed (25,077 files present); `conform -i`
1220/1220 at 1.9.0 and 1.8.0, 0 not idempotent. Miri, `-Zmiri-strict-provenance`, on B's
formatting test: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 203 filtered out;
finished in 6.37s`, no warning. The CLI's text section: base 1,886,122 bytes, hand-written
1,898,362, B 1,898,546.

Wall clock (`pairs.sh`, measured, provisional; load 0.9–1.4, the quietest window of the day):

| pair | base or `b2dd419` MB/s | other MB/s | ratio per pair | ranges |
|---|---|---|---|---|
| base, `b2dd419` (three, then six pairs) | 7.79, 9.51, 8.01; 8.57, 9.39, 8.89, 10.16, 9.37, 9.84 | 9.22, 9.25, 9.30; 9.11, 9.50, 9.07, 8.96, 10.56, 9.79 | 1.184, 0.973, 1.161; 1.063, 1.012, 1.020, 0.882, 1.127, 0.995 | overlap |
| base, B (three, then six pairs) | 9.86, 9.57, 9.04; 8.28, 8.10, 8.64, 9.92, 8.42, 9.99 | 9.23, 10.01, 8.77; 9.57, 9.39, 9.32, 10.44, 8.95, 9.44 | 0.936, 1.046, 0.970; 1.156, 1.159, 1.079, 1.052, 1.063, 0.945 | overlap |
| `b2dd419`, B | 8.98, 9.01, 8.96 | 9.01, 8.92, 9.38 | 1.003, 0.990, 1.047 | overlap |

Neither arena's wall-clock effect is resolved on this host; the two are indistinguishable from
each other.

### Recommendation

**`bumpalo`, variant B (`224870a`, the branch head).** It keeps 97–99% of the gain (per-file
−1.55% and −0.79% against −1.60% and −0.80%), has no `unsafe` of chloro's own, turns the
hand-written arena's dynamic `live`-count invariant into a compile-time one (no scratch value
outlives `with_scratch`'s closure) plus a `RefCell` check on reset that cannot fire wrongly,
and passes the RSS rule. Its costs: a dependency (`bumpalo`, no transitive dependency; a
measured reason in the table above), `with_scratch` closures at three entry points, a
`&Bump` parameter through the chain and path helpers, and resets at two places (a unit boundary
and the outermost return) that must stay together: A shows that the unit boundary alone is not
enough. The hand-written arena (`b2dd419`) is the fallback if the dependency is refused: it is
Miri-clean under strict provenance with both aliasing models on its tests, 0.05% faster on the
fixture sample, and keeps its sixteen `unsafe` blocks. Neither is the answer if a 1.6% / 0.8%
instruction lever with no resolved wall-clock gain does not justify touching `chains.rs` while
E26 works there; the measurement does not decide that.

## Interpretation

- The arena does what it was built for: on the paths converted, 79% of the measured allocator
  cost of the moved allocations is gone from the per-file figure (69%, 103%, 87% per step on the
  fixture sample; 68%, 86%, 88% on the registry sample); the rest is the scratch types' own code
  (0.91 M exclusive, most of it element drops that used to be inlined). Growth in place and give-back work (60–71% of drops give their storage back,
  4,600–4,800 resets per sample, at most 192 KiB held), and peak RSS does not move.
- The hypothesis is refuted at its 3% threshold: −1.60% on the fixture sample's per-file figure
  (−0.80% on the registry sample). The bound said why before anything was built: under the
  `Option<String>` protocol only allocations that stay in their module are reachable (77,275 of
  163,509 formatter blocks), the hot paths hold 47,160 of them (3.45% at full realisation), and of
  those 32,428 could be moved without touching a trait, `ListItem` or another agent's module. The
  other 14,732: strings bound by the protocol inside the module (the `Rewrite` trait, `ListItem`'s
  fields; 4,347), the pure copy (4,525), `Context::new`'s vectors (2,231, reused allocations when
  the caller passes a `Vec`), and small sites of closures, macros and attributes (3,629) left for
  the time box.
- What the measurement says about T5's premise: the remaining allocator cost is not `realloc`
  growth (4.2 M of 27.7 M on the fixture sample; only 55–72% of it on the newest allocation) but
  malloc/free pairs, 91% of them already on glibc's tcache path at ≈ 130 `Ir` a pair. An arena
  removes a pair for ≈ 10 `Ir`, so it pays wherever it reaches; the limit is reach, which the
  protocol sets, not the allocator.
- The registry sample gains half as much as the fixture sample in relative terms: its formatter
  is a smaller share (the parser 43% against 36%, EXP-027) and its allocations are less in chains
  and calls (generated bindings: attributes, statics, struct fields).

## Decision

The hypothesis is refuted as stated (per-file −1.60% < 3% on the fixture sample). The three
steps each pass their keep rule (69%, 103%, 87% of their measured site cost on the per-file figure;
the registry sample falls at every step; peak RSS 0.967–1.043x; no output change) and stay on
`20261015-arena` as a lever of −3,689,856 per-file `Ir` (−1.60%) on the fixture sample and
−2,444,515 (−0.80%) on the registry sample, with one new module of `unsafe` code (its argument and
tests in `scratch.rs`). Whether that is worth the module is the integrator's call; the wall-clock
pairs above do not settle it on this host.

Follow-up (*Soundness and the safe variant*): the branch head is now `224870a`, the same three
paths on `bumpalo` with a reset when the outermost borrower returns: per-file −1.55% and
−0.79%, peak RSS within 1.02x, no `unsafe` of chloro's own; recommended over the hand-written
arena, which stays reachable at `b2dd419` (Miri-clean under strict provenance with both aliasing
models on its tests).

Merge notes: E26 edits `chains.rs`, `overflow.rs` and `expr.rs` (width pre-checks): the arena
touches `chains.rs` (types of `ChainItemKind::MethodCall`, `Chain::children`,
`ChainFormatterShared::rewrites`, `rev_children`'s loop, `callee_str`, `trim_tries`) and one
function of `overflow.rs` (`rewrite_items` and its two callers' arguments); E28 changes
`context.rs`, which this branch does not touch. Not to be measured together with the sink
(E20's remaining sites are the same allocations).

## Follow-up

- The arena's ceiling is a protocol question: 11.7% (fixture) and 11.1% (registry) if
  unit-local intermediates could be returned between modules in arena storage. That is T5's
  "every use of `String`" rewrite; a narrower form is a scratch return type for the module-private
  helpers that are today `Option<String>` only by habit (`rewrite_segment` was one).
- Remove rather than move: `segment_ident`'s copy pushed straight into the segment (4,525 blocks,
  ≈ 0.7 M), `rewrite_method_call`'s argument vector built only to be passed on (1,747), and
  `Context::new`'s collect when the caller already has a `Vec`. Each is an exact allocation
  removal, not an arena use.
- The largest unconverted sites by count: `get_context`'s `Rc` pair (12,074 blocks; `context.rs`,
  after E28), `utils::concat` (9,518, a quarter frame-local), `single_ident_path` (8,862, returned
  to five modules), `overflow::Context::rewrite`'s result (8,842), `combine_strs_with_missing_comments`
  (5,803, with 1,441 reallocs).
- The scratch types' drop glue is out of line for `ListItem`, `ChainItem` and `String` elements
  (0.87 M, and `ChainItemKind`'s 0.18 M); an explicit `#[inline]` drop, or a drop that skips the
  element loop for element types without drop glue, may recover part of it.
