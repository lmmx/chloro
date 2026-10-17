# 2026-10-14: Profile of the flat substrate at fe34bd2 (was 8d418ee)

Re-profile of the integration branch after E7b ([EXP-007b](experiments/EXP-007b-flat-port.md):
rowan and `ra_ap_syntax` replaced under the formatter by `chloro_core::cst`), with E2's
release profile (fat LTO, one codegen unit) and E4's block cache in place. Method, sample,
tools and table formats are those of [profile-baseline.md](profile-baseline.md), which
profiles `884f9c9`; this entry ends with a re-ranked lever table. No source code changed.
Labels: **measured** (a tool's output on the builds below), **read** (from source),
**hypothesis** (inferred, not measured).

## Setup

- **Commit:** `fe34bd2` (was `8d418ee`) (merge of `20261014-flat-port`; best-known-good of
  [experiments.md](experiments.md)).
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, valgrind 3.22.0.
- **Host:** 4 vCPU Xeon @ 2.80GHz, 15 GB (the host of profile-baseline.md). Valgrind runs
  pinned with `taskset -c 0`. During §5 another agent's `cargo build` was pinned to core 0
  and kept it at 100% (`/proc/stat`, 3 s samples); §5 records both cores.
- **Builds** (workspace `[profile.release]`: `codegen-units = 1`, `lto = "fat"`, unwind):

  | target dir | flags | callgrind Ir on the sample |
  |---|---|---|
  | `target/release` | workspace release profile | 390,496,192 |
  | `target/prof2` | same + `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` | 390,496,251 |

  The line-table build differs by 59 instructions, so the symbolised profiles describe the
  release build. Under `benchmarks/profile.sh` the release `bench` measures 390,689,376
  (ledger: 390,690,264). The same binary copied to a scratch path measures 392,515,236
  under `profile.sh` (+1,825,860, +0.47%): the count depends on the binary's path string,
  so `profile.sh` figures compare only for binaries at the same path (measured).
- **Sample:** the 49 files of profile-baseline.md (`find . -name '*.rs' | LC_ALL=C sort |
  awk 'NR%25==0'` under the fixtures, 650,213 bytes, copied flat as `NN_<basename>.rs`;
  byte-identical to the first profile's copies).
- **Then:** two columns. "doc" is profile-baseline.md (`884f9c9`, default 16-unit build,
  587,211,777 Ir). "LTO" is the first profile's fat-LTO, one-unit, `panic = "abort"` build
  of `884f9c9` (`$S1/cg/lto.out`, 539,699,788 Ir), re-aggregated here with the same scripts;
  it shares this build's inlining and isolates E4 + E7b + E8 from E2.
- **Raw data and scripts:** `$S = /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/profile2/`
  (`cg/`, `dhat/`, `cachegrind/`, `wall/`, `scripts/`, `reproduce.sh`); the first
  profile's data is `$S1 = .../scratchpad/profile/`.
- **Commands** (`VG = setarch $(uname -m) -R taskset -c 0 valgrind`):
  - `$VG --tool=callgrind bench -n 1 --root SAMPLE` (release and line-table builds);
  - `$VG --tool=dhat --num-callers=40 bench -n 1 --root SAMPLE`;
  - `$VG --tool=cachegrind --cache-sim=yes bench -n 1 --root SAMPLE`;
  - `benchmarks/profile.sh --core 0`;
  - `taskset -c {0,1} target/release/examples/bench -n 5`, twice each.
- **Aggregation:** `scripts/cgparse.py` (the first profile's, plus exclusive cost per
  `(function, source file of the cost line)` from callgrind's `fi=`/`fe=` lines);
  `scripts/components.py` with rowan groups replaced by `cst building`
  (`TreeBuilder`, `Tree::from_steps_at`), `cst reads` (`cst/mod.rs`), `cst typed layer`
  (`cst/generated.rs`, `cst/ast.rs`), `typed_arena`, `cst free`. It prints two splits:
  **by function** (the first profile's rule: an instruction belongs to the function it runs
  in) and **by source** (for chloro-hosted functions only, an instruction belongs to the
  file its line comes from, so `cst` accessors and `core` code inlined into chloro count as
  themselves). `scripts/bysrc.py` splits one function by inlined source.
  `scripts/dhatsum.py` with the rowan categories replaced by `cst tree` (frames through
  `TreeBuilder`/`Tree::`) and `typed_arena`.

## 1. Where the instructions go (measured)

Exclusive Ir by component, `target/prof2`, total 390,496,251:

| component | by function | % | by source | % |
|---|---|---|---|---|
| chloro's own code (`formatter/*`) | 172,353,581 | 44.14 | 79,766,786 | 20.43 |
| parser: `ra_ap_parser`, `ra_ap_rustc_lexer` | 87,238,243 | 22.34 | 87,721,375 | 22.46 |
| allocator: glibc family, `__rust_*`, `RawVec`/`finish_grow` | 57,536,439 | 14.73 | 60,909,805 | 15.60 |
| `core`/`alloc`/`std`: `str` searchers, `memchr`, `fmt`, iterators | 44,597,607 | 11.42 | 121,677,613 | 31.16 |
| libc `mem*` (`memcpy` 8.39 M, `memcmp` 2.44 M) | 10,853,454 | 2.78 | 10,853,454 | 2.78 |
| `cst` building (`TreeBuilder::steps` closure 10.69 M) | 10,794,157 | 2.76 | 10,794,157 | 2.76 |
| `cst` reads (`cst/mod.rs`: children, kind, range, parent) | 3,720 | 0.00 | 10,648,677 | 2.73 |
| `cst` typed layer (`generated.rs`, `ast.rs`: `cast`, accessors) | 5,130,788 | 1.31 | 5,395,966 | 1.38 |
| drop glue (chloro types 0.87 M, other 0.08 M) | 952,567 | 0.24 | 952,567 | 0.24 |
| `hashbrown` (memo, block cache, macro-args cache) | 541,956 | 0.14 | 1,276,773 | 0.33 |
| loader, bench harness, libc other | 483,613 | 0.12 | 483,613 | 0.12 |
| `typed_arena` (fragment trees) | 10,126 | 0.00 | 15,465 | 0.00 |

Of the allocator's 57.54 M, glibc is 49,852,702 (12.77%): `_int_free` 14.77 M, `malloc`
10.32 M, `_int_malloc` 7.81 M, `free` 7.22 M, `realloc` + `_int_realloc` 5.88 M,
`_int_free_merge_chunk` 1.50 M, `malloc_consolidate` 1.02 M, `unlink_chunk` 0.80 M; Rust's
`finish_grow` 4.31 M, `do_reserve_and_handle` 1.86 M.

chloro by module, by function: `comment` 55.52 M (14.22%; `CharClasses::next` 27.75 M,
`UngroupedCommentCodeSlices::next` 5.97 M, `find_uncommented` 5.91 M,
`combine_strs_with_missing_comments` 2.95 M), `utils` 23.40 M (`unicode_str_width` 20.69 M),
`lists` 13.74 M, `overflow` 10.10 M, `expr` 9.49 M, `rustc_compat` 9.38 M, `visitor`
9.01 M, `chains` 8.29 M, `nodes` 4.99 M, `items` 4.56 M, `context` 4.31 M, `types` 4.16 M,
`span` 3.83 M, every other module below 2 M. By source, `comment.rs` lines are 38.04 M and
the rest of each module's cost is inlined `core`, `cst` and `RawVec` code.

Inclusive Ir (outermost activation; nested entries overlap). Several entry points of the
first profile are inlined in this build; the row names the symbol measured instead.

| function | calls | inclusive | % |
|---|---|---|---|
| `formatting::format_text` | 55 | 388,130,558 | 99.39 |
| `FmtVisitor::visit_items_with_reordering` (`format_separate_mod` is inlined) | 61 | 283,400,244 | 72.57 |
| file parse, `formatting::parse` | 55 | 103,678,885 | 26.55 |
| — `LexedStr::new` 25.29 + `to_input` 4.28 + `TopEntryPoint::parse` 34.55 + `Tree::from_steps_at` 30.35 + `rustc_compat::walk` 9.14 | 55/53/53/53/46 | 103.61 M | 26.53 |
| — of which the tree-building callback (`TreeBuilder::steps::{closure#0}`, all trees) | 263,020 | 10,689,037 | 2.74 |
| `LexedStr::new` / `to_input` / `TopEntryPoint::parse` / `intersperse_trivia`, files and fragments | 2,717/2,013/1,018/1,018 | 30.31 / 5.30 / 38.62 / 31.34 M | 7.76 / 1.36 / 9.89 / 8.03 |
| — `event::process` (inside `parse`) / `intersperse_trivia` exclusive | 2,013/1,018 | 15.17 / 13.74 M exclusive | 3.88 / 3.52 |
| `rustc_compat::walk` (files and fragments) | 83,854 | 9,562,977 | 2.45 |
| `memoize::<format_expr>` | 10,232 | 208,251,185 | 53.33 |
| `expr::format_expr_uncached` | 21,483 | 207,658,306 | 53.18 |
| `overflow::Context::rewrite` (`rewrite_with_parens` is inlined into its callers) | 10,711 | 152,648,646 | 39.09 |
| `expr::rewrite_call` (absorbs inlined callers; call count not comparable) | 7,492 | 143,277,387 | 36.69 |
| `chains::rewrite_chain_from` | 3,239 | 96,700,611 | 24.76 |
| `matches::rewrite_match` | 250 | 64,712,827 | 16.57 |
| `closures::rewrite_closure` | 557 | 52,812,629 | 13.52 |
| `macros::rewrite_macro` | 664 | 39,404,195 | 10.09 |
| `macro_args::parse_macro_arg` (`parse_macro_args` is inlined) | 962 | 15,735,998 | 4.03 |
| `CharClasses::next` | 579,742 | 28,260,998 | 7.24 |
| `types::rewrite_path` | 12,010 | 11,736,116 | 3.01 |
| `utils::filtered_str_fits` (`filter_normal_code` inlined: `LineClasses` 6.99 M) | 5,563 | 19,735,932 | 5.05 |
| `comment::recover_comment_removed` | 24,240 | 17,640,687 | 4.52 |
| `utils::unicode_str_width` | 120,914 | 20,689,332 | 5.30 |
| `lists::write_list` | 12,934 | 14,460,724 | 3.70 |
| `comment::find_uncommented` | 41,675 | 10,500,420 | 2.69 |
| `comment::combine_strs_with_missing_comments` | 27,329 | 10,644,235 | 2.73 |
| `expr::span_ends_with_comma` | 640 | 8,783,409 | 2.25 |
| line table (`OnceCell::try_init` in `line_of`, from `reorderable_group_len` → `line_range`) | 29 | 4,935,658 | 1.26 |
| block cache, `RunState::block_of` (`Block::from_block_expr` inside) | 2,571 | 1,765,103 | 0.45 |
| `FmtVisitor::visit_assoc_item` (of which `parent()` searches 1.89 M, §2) | 292 | 91,525,765 | 23.44 |
| memo overhead (below) | — | about 5.6 M | 1.44 |

Memo (measured unless marked): `memoize` exclusive 3.20 M (hash, lookup and insert inlined),
`reserve_rehash` 0.60 M, the stored result's clone `malloc` 0.56 M + `memcpy` 0.30 M
(10,136 calls), and the frees of up to 9,841 stored results at `clear_memo` (inlined;
called at visitor.rs:397; about 0.97 M at the 95 Ir per `free` of this run, hypothesis). The entry no
longer holds a node, so the first profile's 3.63 M of red-node drops are gone. `memoize`
runs 10,232 times and calls `format_expr_uncached` 9,841 times: 391 hits (3.8%), unchanged.

## 2. The `cst` layer (measured)

- **Building.** `TreeBuilder::steps::{closure#0}` runs 263,020 times (one per parser step,
  files and fragments) for 10,689,037 Ir, 40.6 Ir per step; by source, `cst/mod.rs` 6.68 M,
  `Vec::push` 1.90 M, `ptr` 1.06 M, `RawVec` capacity checks 0.43 M. The rowan callback it
  replaces ran 263,020 times for 60,272,680 Ir in the LTO build (229 Ir per step; folded
  under the name `parse_maybe_bound::{closure#0}`). Each tree is one `Vec<Elem>`
  (`TreeBuilder::new`, 1,019 blocks, 3,251,648 bytes) and one builder stack (1,018 blocks);
  no growth reallocations.
- **Reads.** All of `cst/mod.rs` is inlined: 10.65 M by source, 3,720 by function. The typed
  layer is 5.13 M by function (`Expr::cast` 0.83 M, the `Rewrite` impls on `Pat`, `Param`,
  `Type` generated types 1.53 M, `CastExpr::expr` 0.30 M, `PathExpr::path` 0.25 M). Hosts of
  inlined `cst/mod.rs` lines: `rustc_compat::walk` 1.90 M, `visit_assoc_item` 1.89 M (the
  backward `parent()` search, 292 calls, 6.5 K Ir each), `span::rustc_span` 1.16 M,
  `FnSig::from_fn` 0.70 M, `outer_attributes` 0.64 M, `rewrite_chain_from` 0.66 M (both
  levels), `single_ident_path::only_node` 0.31 M; 210 host functions in all.
- **Bounds checks.** `slice/index.rs` lines inlined into chloro functions are 7.70 M;
  `rustc_compat::walk` 1.38 M, `rustc_span` 0.52 M, `outer_attributes` 0.44 M,
  `FnSig::from_fn` 0.37 M are `elems[i]` reads (read); the rest index strings and vectors.
- **Ownership.** `typed_arena`: 18 chunk allocations (52,992 bytes) and 15 K Ir for 965
  fragment trees; `RunState::keep_tree` 62 K Ir inclusive; the arena drop 0.20 M inclusive
  for 46 runs.
- **Against what it replaced** (LTO columns): red cursor 39.20 M + `ra_ap_syntax` 3.43 M +
  green reads 3.17 M = 45.80 M for reads; the `cst` read side is 16.04 M (reads + typed
  layer, by source), 4.11% of the run. Building: rowan 39.38 M (green building group) +
  2.43 M (green free); `cst` 10.79 M.

## 3. Allocations (DHAT, measured)

Total 278,743 blocks, 124,868,544 bytes (628,959 and 145,995,441 at `884f9c9`; −55.7%
blocks, −14.5% bytes). Allocator Ir per block: 57.54 M / 278,743 = 206 (glibc alone 179).

| category | blocks | % blocks | bytes | % bytes | at `884f9c9` (blocks) |
|---|---|---|---|---|---|
| `String` new (`with_capacity`, `clone`, `to_owned`, `String::from`, `format!`) | 141,158 | 50.6 | 83,545,103 | 66.9 | 149,203 |
| `String` growth | 35,628 | 12.8 | 2,595,853 | 2.1 | 35,628 |
| `Vec` new | 32,638 | 11.7 | 5,893,731 | 4.7 | 31,615 |
| parser and lexer buffers (files and fragments) | 30,468 | 10.9 | 23,293,049 | 18.7 | 30,468 |
| `Rc`/`Box` | 18,790 | 6.7 | 854,096 | 0.7 | 14,445 |
| `Vec` growth | 17,644 | 6.3 | 3,529,952 | 2.8 | 14,397 |
| `cst` tree (`Vec<Elem>`, builder stack) | 2,037 | 0.7 | 3,512,256 | 2.8 | green tree 68,153 |
| `HashMap`: memo / macro-args cache / other (block cache) | 190 / 64 / 101 | 0.1 | 1,589,440 | 1.3 | 190 / 64 / — |
| `typed_arena` chunks | 18 | 0.0 | 52,992 | 0.0 | — |
| red nodes | 0 | 0 | 0 | 0 | 284,789 |

- `FmtVisitor::new`'s `with_capacity(2 × file size)` (visitor.rs:96) is still 913 blocks
  and 68,756,124 bytes (55% of all bytes).
- `RewriteContext::new` allocates two `Rc<Cell<bool>>` (context.rs:227 `inside_macro`,
  context.rs:231 `macro_rewrite_failure`, the second added by E7b): 12,086 blocks; their
  `drop_slow` is 1.17 M inclusive.
- `String` growth is the same 35,628 blocks as at `884f9c9`; `String` new fell by 8,045
  (`node_text` returns a slice; 17,094 blocks at `884f9c9`).

Top 25 sites by block count, by innermost chloro source frame (`dhat/summary.md`; 400 rows
in `dhat/summary_400.md`):

| # | chloro frame [line] | blocks | bytes | category |
|---|---|---|---|---|
| 1 | `combine_strs_with_missing_comments` [comment.rs:152] | 29,469 | 2,021,050 | String new |
| 2 | `write_list` [lists.rs:327] | 16,031 | 3,136,699 | String new |
| 3 | `rewrite_method_call` [chains.rs:321] | 10,826 | 62,949 | String new |
| 4 | `wrap_items` [overflow.rs:644] | 10,720 | 1,265,577 | String new |
| 5 | `memoize` `result.clone()` [context.rs:345] | 9,723 | 1,515,466 | String new |
| 6 | `single_ident_path` `push_str` [types.rs:69] | 9,527 | 93,937 | String growth |
| 7 | `Context::rewrite_items` `collect` [overflow.rs:591] | 7,601 | 2,450,560 | Vec new |
| 8 | `join_rewrites` [chains.rs:709] | 6,904 | 335,964 | String new |
| 9 | `make_subexpr_list` [chains.rs:457] | 6,610 | 552,544 | Box/Vec |
| 10 | `RewriteContext::new` `inside_macro` [context.rs:227] | 6,043 | 145,032 | Rc |
| 11 | `RewriteContext::new` `macro_rewrite_failure` [context.rs:231] | 6,043 | 145,032 | Rc |
| 12 | `MacroTokens::prefix_parse` [macro_args.rs:178] | 5,959 | 797,631 | parser buffers |
| 13 | `MacroTokens::build` [macro_args.rs:287] | 5,901 | 823,144 | parser buffers |
| 14 | `rewrite_segment` `String::with_capacity(128)` [types.rs:301] | 4,622 | 591,744 | String new |
| 15 | `segment_ident` `to_owned` of the name slice [types.rs:52] | 4,621 | 38,146 | String new |
| 16 | `formatting::parse` (lexer, parser) [formatting.rs:42] | 4,240 | 21,415,214 | parser buffers |
| 17 | `non_trivia_text` [nodes.rs:146] | 3,846 | 33,314 | String growth |
| 18 | `ChainItem::rewrite` [chains.rs:256] | 3,696 | 19,009 | String new |
| 19 | `ChainItem::from_ast` [chains.rs:346] | 3,331 | 1,095,360 | Vec growth |
| 20 | `ChainItem::from_ast` [chains.rs:430] | 3,232 | 1,043,840 | Vec growth |
| 21 | `Chain` construction [chains.rs:553] | 3,202 | 192,912 | String new |
| 22 | `MacroTokens::prefix_parse` [macro_args.rs:177] | 2,985 | 42,228 | parser buffers |
| 23 | `MacroTokens::prefix_parse` [macro_args.rs:176] | 2,948 | 54,640 | parser buffers |
| 24 | `rewrite_ident_pat` [patterns.rs:288] | 2,920 | 16,731 | String new |
| 25 | `MacroTokens::build` [macro_args.rs:286] | 2,865 | 40,236 | parser buffers |

Top 25: 173,865 blocks (62.4%). Of the first profile's top 25, the twelve red-node and
green-tree rows and the `node_text` row are gone; every other row has the same block count
except `RewriteContext::new`'s `Rc` (6,013 at `884f9c9`, now 6,043 for each of two).

## 4. Caches (cachegrind, default model: I1 32 KB 8-way, D1 32 KB 8-way, LL 34 MB; measured)

| run | Ir | I1 misses | I1 miss rate | D refs | D1 misses | LL misses |
|---|---|---|---|---|---|---|
| `884f9c9`, default (doc) | 588,694,071 | 7,324,842 | 1.24% | 262,220,316 | 2,217,341 | 74,697 |
| `884f9c9`, LTO build (doc) | 540,630,938 | 6,261,085 | 1.16% | 230,974,652 | 2,098,655 | 73,373 |
| `fe34bd2` (was `8d418ee`), `target/prof2` | 391,413,147 | 5,345,731 | 1.37% | 169,004,048 | 1,320,960 | 61,431 |

- I1 misses fall 14.6% against the LTO build while Ir falls 27.6%, so the miss rate rises
  from 1.16% to 1.37%. The largest I1 contributors: `malloc.c` 364,913 (6.8%), `lists.rs`
  324,843 (`write_list` 103,524), inlined `alloc` 273,392, `expr.rs` 233,134
  (`cachegrind/bench_annot.txt`).
- `malloc.c` has 161,807 D1 read misses (20.8%; 413,653 and 27% at `884f9c9`). D1 misses
  fall 37% against the LTO build; the miss rate stays at 0.8%. LL misses (61 K) do not
  bound throughput.
- The cycle split of profile-baseline.md §4 needs a parse-only run of the same build;
  `parse_floor` has no `cst` mode, so it is not repeated (Missing).

## 5. Wall clock (`bench -n 5`, all 1238 fixtures, release `bench`, sha256 `cbcc51c4…`)

| core | condition | load average (1 min) | MB/s (best of 5) |
|---|---|---|---|
| 0 | another agent's `cargo build` pinned to core 0, core 0 at 100% | 2.41–2.65 | 3.67, 3.39 |
| 1 | core 1 at 2–4% busy | 1.76–2.20 | 7.79, 7.82 (1.785 s, 1.778 s) |

The core-0 figures measure a shared core and are labelled contended. The core-1 figures
compare with EXP-007b's 7.01–7.54 MB/s (core 1, load 0) and not with profile-baseline.md's
4.49–4.73 MB/s (core 0, other window); no speedup claim rests on this section
(`wall/bench_n5.txt`).

## Then and now (measured)

Exclusive Ir by component (by function), in M:

| component | doc `884f9c9` | LTO `884f9c9` | `fe34bd2` (was `8d418ee`) | change vs LTO |
|---|---|---|---|---|
| total | 587.21 | 539.70 | 390.50 | −149.20 (−27.6%) |
| chloro's own code | 175.84 | 187.59 | 172.35 | −15.24 |
| allocator (glibc part) | 132.35 (115.7) | 121.72 | 57.54 (49.85) | −64.18 |
| parser | 97.00 | 81.27 | 87.24 | +5.97 |
| `core`/`alloc`/`std` | 54.38 | 45.73 | 44.60 | −1.13 |
| rowan green building → `cst` building | 49.32 | 39.38 | 10.79 | −28.59 |
| rowan red cursor | 43.92 | 39.20 | 0 | −39.20 |
| libc `mem*` | 13.40 | 14.65 | 10.85 | −3.80 |
| `ra_ap_syntax` → `cst` typed layer | 10.12 | 3.43 | 5.13 | +1.70 |
| rowan green reads → `cst` reads (inlined; 10.65 by source) | 5.16 | 3.17 | 0.00 | −3.17 |
| rowan green free → `cst` free + `typed_arena` | 3.31 | 2.43 | 0.01 | −2.42 |
| `hashbrown` | 1.91 | 0.68 | 0.54 | −0.14 |
| drop glue, loader, harness, other | 0.49 | 0.45 | 1.44 | +0.99 |

chloro modules, LTO → now: `nodes` 15.81 → 4.99, `macro_args` 11.19 → 0.95, `context`
7.48 → 4.31, `span` 4.71 → 3.83, `comment` 55.74 → 55.52, `utils` 23.39 → 23.40, `lists`
13.94 → 13.74, `visitor` 9.27 → 9.01, `rustc_compat` 8.53 → 9.38, `chains` 6.05 → 8.29,
`overflow` 8.58 → 10.10, `expr` 8.65 → 9.49 (`chains`, `overflow`, `expr` absorb inlined
`cst` reads).

Inclusive, LTO → now: `format_text` 535.35 → 388.13 M; `memoize::<format_expr>` 267.32 →
208.25; `Context::rewrite` 190.95 → 152.65; `rewrite_chain_from` 129.04 → 96.70;
`visit_assoc_item` 121.42 → 91.53; `rewrite_match` 87.87 → 64.71; `rewrite_closure` 71.79 →
52.81; `rewrite_macro` 49.30 → 39.40; `parse_macro_arg` 18.75 → 15.74; `rewrite_path`
19.93 → 11.74; `intersperse_trivia` 80.92 → 31.34; tree-building callback 60.27 → 10.69;
`Block::from_block_expr` 9.94 (2,571 calls) → `block_of` 1.77 (2,571 calls);
`rustc_compat::walk` 8.67 → 9.56; `rustc_span` 4.14 → 3.80; memo overhead about 9.7 (doc)
→ 5.6.

- **Gone:** red nodes (284,789 blocks, 90.1 M in the doc's cost model), the green tree
  (68,153 blocks), rowan building and free, `node_text` copies, the memo's node drops.
- **Unchanged to the instruction** (same code, same inputs): `CharClasses::next` 28,260,998,
  `unicode_str_width` 20,689,332, `find_uncommented` 10,500,420, `span_ends_with_comma`
  8,783,409, `filtered_str_fits` 19.75 → 19.74 M, `recover_comment_removed` 17.84 → 17.64 M,
  `write_list` 14.47 → 14.46 M, the line table 4.93 → 4.94 M, memo hit rate 3.8%, `String`
  growth blocks, parser-buffer blocks.
- **Grew:** parser +5.97 M against the LTO build (`event::process` +2.53 M,
  `Converter::push` +0.79 M, `Marker::complete` +0.60 M; inlining differs, and the LTO build
  had `panic = "abort"`; cause not examined); `rustc_compat::walk` +0.89 M (EXP-007b
  records +1.4 M); the typed layer +1.70 M by function against `ra_ap_syntax`'s 3.43 M,
  while the whole read side falls from 45.80 M to 16.04 M; `Rc`/`Box` blocks +4,345 (the second
  `Rc<Cell<bool>>` per context adds 6,043).
- **Shares:** the allocator falls from 22.5% (doc) to 14.7%; glibc from 19.7% to 12.8%.
  chloro's own code rises from 29.9% to 44.1% of a smaller total; the parser from 16.5% to
  22.3%; `comment` + `utils` alone are 20.2%.

## Levers

Ranked by upper bound. Bounds are on the 49-file sample as a share of 390.50 M Ir; 1% =
3.905 M. Lettered rows are parts of the row above them. Rows overlap: the `String` rows
contain their allocator cost, `find_uncommented` and `span_ends_with_comma` contain
`CharClasses`. The bound is the cost the mechanism removes if it removes all of it.
Allocator cost per block: 206 Ir (§3).

| rank | lever (mechanism; rows it removes) | upper bound | arithmetic | scope | correctness risk | tried before |
|---|---|---|---|---|---|---|
| 1 | Global allocator in the CLI binary: cheaper malloc/free for 278,743 blocks | ≤ 12.8% | glibc 49.85 M / 390.50 M; the replacement's own cost not measured | `chloro/src/main.rs`, `chloro/Cargo.toml` | none for output; library users unaffected | E2: mimalloc −12.4% `Ir` at `884f9c9`, wall inside the spread, peak RSS 2–10x; rejected |
| 2 | `String` per rewrite: fewer intermediate strings, writes into a shared buffer | ≤ 10.7% | (141,158 + 35,628) blocks × 206 = 36.49 M + `fmt` 5.38 M (`format_inner` 1.76, `fmt::write` 1.49, `write_str` 1.30, `pad` 0.83) = 41.87 M | formatter-wide (`Rewrite` signature) | high for a full redesign | chain items without `format!` (`03e063f`) landed |
| 2a | `combine_strs_with_missing_comments` result string | ≤ 1.6% | 29,469 × 206 = 6.08 M | `comment.rs:152` and callers | low–medium | no |
| 2b | `write_list` + `wrap_items` strings | ≤ 1.4% | (16,031 + 10,720) × 206 = 5.52 M | `lists.rs:327`, `overflow.rs:644` | medium | no |
| 2c | `single_ident_path` and `segment_ident` returning the source slice; `rewrite_method_call`'s `String::from(".")` growth | ≤ 1.3% | (9,527 + 4,621 + 10,826) × 206 = 5.15 M | `types.rs:50-72`, `chains.rs:321` | low (`node_text` is a slice since E7b) | no (E3 lists it, running) |
| 2d | memo stores a shared string (`Rc<str>`) instead of cloning | ≤ 0.47% | `malloc` 0.56 + `memcpy` 0.30 + frees ≈ 0.97 (hypothesis) = 1.83 M | `context.rs:303-350` | low | no (E3 lists it) |
| 3 | Parser output encoding: steps decoded twice (`event::process` builds `Output`, `intersperse_trivia` re-reads it) | ≤ 7.4% | 15.17 + 13.74 M exclusive = 28.91 M; a direct event-to-tree path still does part of this work | `ra_ap_parser` (fork or upstream API) | medium (trivia attachment rules) | no |
| 3a | `to_input` copy of the lexed tokens | ≤ 1.4% | 5.30 M (files 4.28, fragments 1.02) | `ra_ap_parser` API | n/a without a parser change | no |
| 3b | Macro fragments lexed and parsed twice (`prefix_parse` window, then `build` on the consumed slice) | ≤ 1.55% | `build`'s `LexedStr::new` 1.57 + `to_input` 0.45 + `parse` 4.03 = 6.05 M; lexing only 4.24 M (1.09%) | `macro_args.rs:176-300` | medium (prefix and top entry points differ) | no |
| 4 | `CharClasses` scanning cost per character | ≤ 7.2% | `CharClasses::next` 28.26 M inclusive; callers `span_ends_with_comma` 6.18, `UngroupedCommentCodeSlices` 5.87, `LineClasses` 5.21, `find_uncommented` 4.59, `CommentCodeSlices` 4.51, `contains_comment` 1.12 M | `comment.rs` | low (equivalence testable on both corpora) | `skip_plain` landed; X8 token index +3% (refuted); E3 running |
| 4a | `recover_comment_removed` slow path (`UngroupedCommentCodeSlices` 8.06 M + `changed_comment_content` flatten 6.26 M) | ≤ 3.7% | 14.32 M of 17.64 M | `comment.rs` | low | E3 running |
| 4b | `span_ends_with_comma` scans the whole span | ≤ 2.25% | 8.78 M in 640 calls (13.7 K Ir each); 433 from `format_expr_uncached` | `expr.rs:1455-1473` | medium (comments, raw strings) | span cache: no gain; E3 running |
| 4c | `filter_normal_code` (`LineClasses` for every snippet with `/`) | ≤ 1.8% | 6.99 M inside `filtered_str_fits` | `utils.rs`, `comment.rs` | low | E3 running |
| 5 | Width of built strings: `unicode_str_width`'s ASCII path loops per byte | ≤ 5.3% | 20.69 M, 120,914 calls (171 Ir each); by source `non_null` 8.73, `utils.rs` 6.55, slice iteration 3.22 M; callers `filtered_str_fits` 5.19, `definitive_tactic` 4.86, `write_list` 2.43 M | `utils.rs:126-140` | low (test exists) | one-pass ASCII width (`4edd403`) landed |
| 6 | `cst` read layer: cheaper child iteration, no backward `parent()` search, fewer bounds checks | ≤ 4.1% (whole layer) | reads 10.65 + typed layer 5.40 = 16.04 M by source; bounds checks on `elems[i]` partly inside the 7.70 M of §2 | `cst/mod.rs`, `generated.rs`, callers | low per site (equality test against rowan exists) | E7b |
| 6a | `visit_assoc_item` takes the trait/impl kind from its callers instead of two `parent()` searches | ≤ 0.48% | 1.89 M in 292 calls | `visitor.rs`, its two callers | low | EXP-007b follow-up |
| 7 | Tree build: exact element count, push without capacity checks, or building inside the parser (row 3) | ≤ 2.7% | callback 10.69 M; of it `Vec::push`/`RawVec`/`ptr` 3.39 M (0.87%) | `cst/mod.rs:216-305` | low | EXP-007b follow-up (capacity) |
| 8 | `rustc_compat` walk folded into tree building or walked as a slice | ≤ 2.45% | 9.56 M, 83,854 calls; of it `cst/mod.rs` 1.93 M, bounds checks 1.40 M, `Vec` 1.17 M (the `path`) | `rustc_compat.rs`, `formatting.rs:40-43` | medium | no; EXP-007b follow-up (slice walk) |
| 9 | Memo overhead: fewer stored entries, no clone | ≤ 1.44% | 3.20 + 0.60 + 0.56 + 0.30 + 0.97 (hypothesis) = 5.64 M; 391 hits of 10,232 | `context.rs:303-350` | medium (memo correctness argument) | memo on/off at `6b07131`: memo saves 19 M net |
| 10 | Line-start table built with a `bytes().enumerate().filter()` chain | ≤ 1.26% | 4.94 M in 29 builds (`reorderable_group_len` → `line_range`) | `context.rs:156-169` | low | no (E3 lists it) |
| 11 | One `Rc<Cell<bool>>` per `RewriteContext` instead of two (or a plain `Cell` in the run) | ≤ 0.64% | 12,086 blocks × 206 = 2.49 M; the one E7b added ≈ 1.25 M (0.32%) | `context.rs:222-233`, `visitor.rs` | low | no |
| 12 | PGO / BOLT (code layout against the I1 misses: 1.37% miss rate) | not bounded here (hypothesis) | E2: PGO alone −16.2% `Ir` at `884f9c9`; not measured on this head | build only (`benchmarks/pgo.sh`) | none for output | E2: kept as a script |

Observations in the data:
- The flat layer costs 16.04 M for reads where rowan's cursor, typed layer and green reads
  cost 45.80 M in the LTO build; the typed layer alone (5.13 M) costs more than
  `ra_ap_syntax`'s 3.43 M because the reads it calls are inlined into it.
- `cst` building is 40.6 Ir per parser step against 229 for rowan; the remaining file-parse
  cost (94.5 M before `rustc_compat`) is lexer 25.3 M, grammar and `event::process`
  34.6 M, step decoding and the callback 30.4 M, `to_input` 4.3 M.
- The comment scanners, width measurement and the line table execute the same instruction
  counts as at `884f9c9`; together with `filtered_str_fits` and `recover_comment_removed`
  they are the largest chloro-side items now.
- `profile.sh` counts move by 0.47% with the path of the `bench` binary.

## Not levers (measured, small)

| item | measurement |
|---|---|
| `typed_arena` | 15 K Ir by source, 18 blocks, 965 fragment trees |
| `cst` tree memory | 1,019 element vectors, 3.25 MB, no growth; spare capacity costs bytes, not `Ir` |
| `cst` tree free | inside the 0.20 M arena drop and the file tree's single `free` |
| hashing (`hashbrown`, three maps) | 0.54 M by function, 1.28 M by source (0.33%) |
| `format!` | `format_inner` 1.76 M exclusive |
| the visitor buffer reservation (visitor.rs:96) | 913 blocks, 55% of bytes; tested before with no wall-time change |
| data cache | 61 K LL misses; D1 miss rate 0.8% |
| block cache | `block_of` 1.77 M inclusive for 2,571 calls (1,460 `to_cached` vectors) |
| `Shape`/`Indent` | `shape` module 1.54 M exclusive |
| `str` searchers over built strings | `CharSearcher` both directions 14.92 M, `memrchr` 6.07 M, `trim_matches` 4.71 M, `is_contained_in` 2.33 M, `StrSearcher::new` 0.94 M: 28.97 M (7.4%) spread over more than 30 callers; the largest, `filtered_str_fits` 4.26 M (1.1%) |
| `Vec` growth | 17,644 blocks; `ChainItem::from_ast` 6,563 of them |

## Current State

- The formatter executes 390.5 M instructions on the 49-file sample: 44.1% in chloro's own
  code (by function), 22.3% in the parser, 14.7% in the allocator, 2.8% building `cst`
  trees, 4.1% in `cst` reads and the typed layer by source ($S/cg/prof_components.txt)
- Against the LTO build of `884f9c9`: −149.2 M (−27.6%); rowan and `ra_ap_syntax` cost
  87.6 M by function there and the `cst` layer 15.9 M here (−71.7 M); the allocator −64.2 M
- DHAT counts 278,743 blocks: 63.4% `String` creation and growth, 10.9% parser buffers,
  0.7% `cst` trees, 0 red nodes ($S/dhat/summary.md)
- I1 miss rate 1.37%, D1 0.8%, 61 K LL misses ($S/cachegrind/)
- The largest remaining levers by upper bound: the CLI allocator (12.8%), `String` per
  rewrite (10.7%), parser output decoding (7.4%), `CharClasses` (7.2%), width
  measurement (5.3%)

## Missing

- `bench -n 5` on a quiet core 0 (core 0 was held by another agent's build)
- A parse-only floor on the `cst` tree (`parse_floor` has no `cst` mode), and with it the
  cycle split of profile-baseline.md §4
- PGO on this head; an instruction count with another global allocator on this head
- The cause of the parser's +5.97 M against the LTO build of `884f9c9`
- The slow-path call count of `recover_comment_removed` (210 at `884f9c9`; same code)
