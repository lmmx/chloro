# 2026-10-14: Profile baseline at 884f9c9

Follows [2026-10-11-phase-2-close.md](../2026-10-11-phase-2-close.md). This entry records
where instructions and allocations go on the current code, and a ranked list of
optimisation levers with upper bounds computed from these tables. No source code changed.
Labels: **measured** (a tool's output on the builds below), **read** (from source),
**hypothesis** (inferred, not measured).

## Setup

- **Commit:** `884f9c9`. `961c92b` and `884f9c9` change only versions, changelogs and
  README badges, so the formatter code is that of `f6c5be9` (G2) in the phase-2 entries.
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, valgrind 3.22.0.
- **Host:** 4 vCPU Xeon @ 2.80GHz, 15 GB. All CPU-heavy runs pinned with `taskset -c 0`.
  Other agents loaded cores 1–3 during this work (load average 7–8.7 for most of it).
- **Builds** (workspace has no `[profile.release]`, so release is opt-level 3, 16 codegen
  units, thin-local LTO, no debug info):

  | target dir | flags | callgrind Ir on the sample |
  |---|---|---|
  | `target/release` | default release | 587,211,777 |
  | `target/prof` | `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` | 587,212,855 |
  | `target/lto` | `CARGO_PROFILE_RELEASE_LTO=fat` `..._CODEGEN_UNITS=1` `..._PANIC=abort` | 539,699,788 |

  The line-table build differs from the default by 1,078 instructions (0.0002%), so the
  symbolised profiles below describe the default build.
- **Sample:** every 25th file of the C-locale-sorted list of `.rs` files under
  `chloro-core/tests/conformance/fixtures` (`find . -name '*.rs' | LC_ALL=C sort | awk
  'NR%25==0'`): 49 files, 650,213 bytes, copied as `NN_<basename>.rs`. 586,768,390 was
  recorded for `f6c5be9` in the phase-2 entry; 587,211,777 here (+0.08%; the cause of the difference is not examined).
- **Raw data and scripts:** `$S = /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/profile/`
  (`sample_files.txt`, `cg/`, `cachegrind/`, `dhat/`, `wall_*.txt`, `scripts/`).
  `$S/reproduce.sh REPO OUT` runs every command of this entry.
- **Commands** (`VG = setarch $(uname -m) -R taskset -c 0 valgrind`):
  - `$VG --tool=callgrind bench -n 1 --root SAMPLE` (one run per build: D1 made counts
    deterministic to 0.0006%);
  - `$VG --tool=callgrind --separate-callers14='*cursor::NodeData>::new'
    --separate-callers14='rowan::cursor::free' bench -n 1 --root SAMPLE` (item 2);
  - `$VG --tool=dhat --num-callers=40 bench -n 1 --root SAMPLE` (item 3);
  - `$VG --tool=cachegrind --cache-sim=yes` on `bench` and `parse_floor rowan` (item 4);
  - `parse_floor {lex,events,flat,rowan,rowan-shared} SAMPLE -n 1` under callgrind.
- **Aggregation:** `scripts/cgparse.py` reads the callgrind file directly (exclusive cost per
  function, call edges with call counts and inclusive cost); its exclusive sum equals the
  program total. Inclusive cost of a function is the sum of the edges into its outermost
  recursion level (names without `'N`). Components are matched by function name, and chloro
  modules by the function's source file (`scripts/components.py`).

## 1. Where the instructions go (measured)

Exclusive Ir by component, `target/prof`, total 587,212,855:

| component | Ir | % |
|---|---|---|
| chloro's own code (`chloro-core/src/formatter/*`) | 175,836,983 | 29.94 |
| allocator: glibc `malloc`/`free`/`realloc` family, `__rdl_*`, `RawVec`/`finish_grow` | 132,353,158 | 22.54 |
| parser: `ra_ap_parser`, `ra_ap_rustc_lexer` | 97,004,692 | 16.52 |
| `core`/`alloc`/`std`: `str` searchers, `memchr`, `fmt`, iterators | 54,378,012 | 9.26 |
| rowan green building: `NodeCache`, `GreenNodeBuilder`, `ThinArc`, the `parse` trivia callback | 49,316,331 | 8.40 |
| rowan red cursor: `NodeData::new`, `cursor::free`, `next_sibling`, `first_child`, … | 43,923,703 | 7.48 |
| libc `mem*`/`str*` (`memcpy` 10.78 M, `memcmp` 2.53 M) | 13,404,389 | 2.28 |
| `ra_ap_syntax` (accessors, `cast`, `kind_from_raw` 4.18 M) | 10,124,663 | 1.72 |
| rowan green reads (`child_at_range` 5.00 M) | 5,163,709 | 0.88 |
| rowan green free (`drop_slow` exclusive) | 3,310,416 | 0.56 |
| `hashbrown` (memo map 1.76 M, macro-args cache 0.16 M) | 1,910,951 | 0.33 |
| loader, bench harness, other | 485,848 | 0.08 |

Of the allocator's 132.35 M, glibc itself is 115.7 M (19.7%): `_int_free` 35.56 M, `malloc`
26.10 M, `_int_malloc` 20.00 M, `free` 18.01 M, `malloc_consolidate` 4.96 M, `realloc` +
`_int_realloc` 6.21 M, `unlink_chunk` 2.54 M, `_int_free_merge_chunk` 1.69 M.

chloro's own code by module (exclusive): `comment` 54.05 M (9.21%; `CharClasses::next`
28.32 M, `UngroupedCommentCodeSlices::next` 5.99 M, `find_uncommented` 5.91 M,
`combine_strs_with_missing_comments` 3.77 M), `nodes` 20.34 M (3.46%), `utils` 18.73 M
(3.19%; `unicode_str_width` 17.25 M), `lists` 13.13 M (2.24%), `overflow` 9.89 M,
`rustc_compat` 8.84 M, `expr` 8.76 M, `chains` 6.93 M, `visitor` 6.26 M, `span` 4.42 M
(`rustc_span` 4.37 M), `formatting` 3.85 M, `types` 3.15 M, `context` 3.10 M, every other
module below 3 M.

Inclusive Ir (outermost activation; nested entries overlap):

| function | calls | inclusive | % |
|---|---|---|---|
| `formatting::format_text` | 55 | 583,702,240 | 99.40 |
| `FmtVisitor::format_separate_mod` | 46 | 403,292,359 | 68.68 |
| file parse: `LexedStr::new` 29.94 + `to_input` 4.37 + `TopEntryPoint::parse` 42.13 + `intersperse_trivia` 79.80 + `TreeBuilder::finish` 0.58 | 49/47 | 156.82 M | 26.71 |
| — of which the trivia callback building the green tree (`parse::{closure#0}`) | 253,997 | 60,386,788 | 10.28 |
| `rustc_compat::subtree_rejected_by_rustc` (files and fragments) | 1,001 | 10,438,749 | 1.78 |
| green tree free at end of file (`cursor::free` from `format_text`) | 113 | 9,928,901 | 1.69 |
| `expr::format_expr` | 1,760 | 153,173,348 | 26.08 |
| `memoize::<format_expr>` | 10,232 | 285,298,480 | 48.59 |
| `expr::format_expr_uncached` | 21,483 | 284,580,977 | 48.46 |
| `overflow::rewrite_with_parens` (`Vec<OverflowableItem>`) | 7,755 | 191,907,556 | 32.68 |
| `expr::rewrite_call` | 2,117 | 112,836,350 | 19.22 |
| `matches::rewrite_match` | 250 | 94,652,878 | 16.12 |
| `closures::rewrite_closure` | 557 | 77,762,944 | 13.24 |
| `macros::rewrite_macro` | 816 | 67,280,486 | 11.46 |
| `macro_args::parse_macro_args` (lex, parse, build, splice of arguments) | 702 | 30,652,802 | 5.22 |
| `CharClasses::next` | 579,742 | 28,837,738 | 4.91 |
| `types::rewrite_path` | 12,010 | 22,167,473 | 3.78 |
| `utils::filtered_str_fits` | 5,563 | 19,119,640 | 3.26 |
| `comment::recover_comment_removed` | 24,240 | 17,798,059 | 3.03 |
| `utils::unicode_str_width` | 120,914 | 17,259,310 | 2.94 |
| `lists::write_list` | 12,934 | 12,911,220 | 2.20 |
| `nodes::Block::from_block_expr` | 2,571 | 11,630,307 | 1.98 |
| `comment::find_uncommented` | 41,675 | 10,593,519 | 1.80 |
| `comment::combine_strs_with_missing_comments` | 27,329 | 9,812,674 | 1.67 |
| `expr::span_ends_with_comma` | 640 | 8,939,994 | 1.52 |
| `comment::filter_normal_code` | 5,563 | 8,583,512 | 1.46 |
| `RunState::line_of` (line table built 29 times, from `walk_reorderable_items`) | 58 | 5,047,727 | 0.86 |
| memo overhead (see below) | — | about 9.7 M | 1.65 |

Memo overhead (measured, summed over all recursion levels): `memoize` exclusive 2.64 M,
`HashMap` insert/lookup 1.92 M, `String::clone` of the stored result 1.31 M, `MemoEntry`
drops (result string and red node) 3.63 M, `clear_memo` 0.16 M exclusive. On the sample,
`memoize::<format_expr>` runs 10,232 times and calls `format_expr_uncached` 9,841 times:
391 hits (3.8%).

Parse-only runs of the same sample (`parse_floor`, callgrind, measured): `lex` 29.6 M,
`events` 76.2 M, `flat` 107.4 M, `rowan` 168.1 M, `rowan-shared` 162.4 M. `rowan` mode drops
each tree through a `Parse`, so its trees are freed on the `ParseNodeDropper` thread, which
ran in this run (`drop_slow` 9.86 M); `rowan-shared` frees on the main thread (6.83 M).

## 2. Red-node creation (measured)

- `<rowan::cursor::NodeData>::new` (not inlined in rowan 0.15.15) runs 284,789 times on the
  sample — the count the phase-2 entry records for `f6c5be9`. Each call is one 64-byte
  `malloc` (DHAT: 284,789 blocks, 18,226,496 bytes, 45.3% of all blocks).
- Cost per red node: `NodeData::new` 31.49 M inclusive (20.10 M of it `__rust_alloc`) +
  `cursor::free` 43.97 M inclusive minus 11.05 M of green-tree drops at roots = 32.92 M +
  other red-cursor exclusive (navigation) 25.69 M = **90.1 M, 15.34% of the sample, 316 Ir per
  red node**. G2 measured 324 Ir saved per red node removed.
- 236,308 of the 260,454 `cursor::free` calls (90.7% of calls; 83.0% of nodes created) run in
  the same chloro function that created the node (first chloro frame of the
  `--separate-callers` contexts, `cg/red_new_all.txt`, `cg/red_free_all.txt`). Nodes freed
  elsewhere: `clear_memo` 4,814 (memo entries), `inner_attributes` closure 3,142,
  `rewrite_with_parens` 2,623, `Vec<Stmt>` drop 1,624.

Top 25 creation sites, by (first chloro frame, accessor below it). `nodes::child::<T>` and
`has_token` are attributed to their caller. "Calls" is the number of accessor calls from that
frame. Class (read from source): **D** one query on the node (presence, span, text, kind)
then dropped; **S** a `support::child` scan, where every node beyond one per call is a
rejected sibling (nodes − calls are discarded unread); **C** a `children_with_tokens` scan
whose trivia tokens and rejected nodes are discarded (the split is not measured); **K** kept
(stored in a struct or passed to a rewrite).

| # | chloro frame | accessor | nodes | calls | class |
|---|---|---|---|---|---|
| 1 | `Block::from_block_expr` (`stmts_of`, nodes.rs:486) | `children_with_tokens` | 15,835 | 20,977 | C |
| 2 | `format_expr_uncached` (expr.rs:221) | `PathExpr::path` | 9,056 | 9,056 | K |
| 3 | `outer_attributes` (nodes.rs:184) | `children_with_tokens` | 8,314 | 8,328 | C |
| 4 | `rewrite_chain_from` (`ArgList::args` collected) | `SyntaxNodeChildren::next` | 6,900 | 13,656 | K |
| 5 | `rewrite_chain_from` (receiver) | `child::<Expr>` | 6,852 | 6,852 | K |
| 6 | `Block::from_block_expr` (`tail_expr`) | `support::child::<Expr>` | 6,151 | 4,239 | S (1,912) |
| 7 | `rewrite_chain_from` (`field.expr()`, `a.expr()`) | `support::child::<Expr>` | 5,865 | 3,049 | S (2,816) |
| 8 | `FmtVisitor::visit_item` (`f.body().is_some()`, `item_attrs`) | `Fn::body` | 5,580 | 1,382 | D |
| 9 | `rewrite_path_segments` (`rewrite_segment`, types.rs:309) | `generic_arg_list` | 5,263 | 4,611 | D (≥ 4,611) |
| 10 | `rewrite_path_segments` (generic args, types.rs:328) | `SyntaxNodeChildren::new` | 5,232 | 5,232 | K |
| 11 | `format_expr_uncached` | `support::child::<Expr>` | 5,160 | 2,998 | S (2,162) |
| 12 | `matches::arm_span` (span only) | `MatchArm::expr` | 4,997 | 2,477 | D |
| 13 | `rewrite_chain_from` (method name) | `child::<NameRef>` | 4,834 | 4,834 | K |
| 14 | `types::path_segments` | `child::<PathSegment>` | 4,633 | 4,633 | K |
| 15 | `rewrite_path_segments` (`segment_ident` → `node_text`) | `child::<NameRef>` | 4,621 | 4,621 | D |
| 16 | `format_expr_uncached` (`CallExpr::arg_list`) | `support::child::<ArgList>` | 4,234 | 2,117 | S (2,117) |
| 17 | `FmtVisitor::visit_fn` (items.rs:1678) | `Fn::body` | 3,994 | 950 | S (3,044) |
| 18 | `FnSig::from_fn` | `generic_param_list` | 3,972 | 957 | S (3,015) |
| 19 | `pairs::rewrite_all_pairs` | `BinExpr::op_kind` (token scan) | 3,548 | 887 | D |
| 20 | `rewrite_chain_from` (args) | `SyntaxNodeChildren::new` | 3,474 | 3,474 | K |
| 21 | `rewrite_chain_from` | `child::<ArgList>` | 3,426 | 3,426 | K |
| 22 | `macros::rewrite_macro` | `children_with_tokens` | 3,374 | 5,006 | C |
| 23 | `nodes::inner_attributes` (34 containers with a `//!` or `#!`) | `children_with_tokens` | 3,250 | 34 | C |
| 24 | `items::param_ty_hi` (span only) | `child::<Pat>` | 3,026 | 3,028 | D |
| 25 | `Attribute::path_text` (`Meta::path` → text) | `SyntaxNodeChildren::new` | 2,994 | 2,994 | D |

Top 25: 134,585 nodes (47.3% of 284,789). By class: D 29,377 (row 9 counted as its 4,611
rejected first children), S-discarded 15,066, C 30,773, K 45,059, kept part of S rows 14,310.
By first chloro frame over all paths: `rewrite_chain_from` 33,383 (11.7%),
`format_expr_uncached` 31,090 (10.9%), `Block::from_block_expr` 29,702 (10.4%),
`rewrite_path_segments` 15,303 (5.4%), `outer_attributes` 10,654 (3.7%)
(`cg/red_new_paths_skiphelpers.txt`).

## 3. Allocations (DHAT, measured)

Total 628,959 blocks, 145,995,441 bytes (DHAT counts each `realloc` as a block;
`__rust_alloc` runs 581,028 times). Category by the frames between the allocator and the
first chloro frame (`scripts/dhatsum.py`):

| category | blocks | % blocks | bytes | % bytes |
|---|---|---|---|---|
| red nodes (`NodeData::new`) | 284,789 | 45.3 | 18,226,496 | 12.5 |
| `String` new (`with_capacity`, `clone`, `to_owned`, `String::from`, `format!`) | 149,203 | 23.7 | 84,196,518 | 57.7 |
| green tree (nodes, tokens, builder, `NodeCache`) | 68,153 | 10.8 | 6,887,884 | 4.7 |
| `String` growth (`finish_grow`, `reserve`) | 35,628 | 5.7 | 2,587,021 | 1.8 |
| `Vec` new (`collect`, `with_capacity`) | 31,615 | 5.0 | 5,579,489 | 3.8 |
| parser and lexer buffers (files and macro fragments) | 30,468 | 4.8 | 23,293,049 | 16.0 |
| `Rc`/`Box` | 14,445 | 2.3 | 678,480 | 0.5 |
| `Vec` growth | 14,397 | 2.3 | 2,936,704 | 2.0 |
| `HashMap`: memo | 190 | 0.0 | 1,520,856 | 1.0 |
| `HashMap`: macro-args cache | 64 | 0.0 | 86,872 | 0.1 |

- Of the `String` bytes, 68,756,124 (47% of all bytes) are 913 blocks from
  `FmtVisitor::new`'s `with_capacity(2 × file size)` (visitor.rs:91), which `from_context`
  replaces at once (visitor.rs:107). Tried before: no wall-time change. `format!`
  (`format_inner`) makes 5,436 blocks.
- Allocator Ir per block (measured, derived): red nodes 45.9 M for 284,789 blocks, 161 Ir
  each (alloc + free); every other block 86.4 M for 344,170 blocks, 251 Ir each.

Top 25 sites by block count, by innermost chloro source frame (inlined frames resolved through
line tables; `dhat/summary.md`, 400 rows in `dhat/summary_400.md`):

| # | chloro frame [line] | blocks | bytes | category |
|---|---|---|---|---|
| 1 | `TreeBuilder::finish_node` [formatting.rs:53] | 43,763 | 3,786,240 | green node |
| 2 | `combine_strs_with_missing_comments` [comment.rs:152] | 29,469 | 2,021,050 | String new |
| 3 | `stmts_of` [nodes.rs:486] | 18,406 | 1,177,984 | red node |
| 4 | `node_text` [nodes.rs:318] | 17,094 | 142,565 | String new |
| 5 | `write_list` [lists.rs:327] | 16,031 | 3,136,699 | String new |
| 6 | `rewrite_method_call` [chains.rs:317] | 10,826 | 62,949 | String new |
| 7 | `wrap_items` [overflow.rs:644] | 10,720 | 1,265,577 | String new |
| 8 | `outer_attributes` [nodes.rs:184] | 10,654 | 681,856 | red node |
| 9 | `ChainItemKind::from_ast` [chains.rs:187] | 10,326 | 660,864 | red node |
| 10 | `TreeBuilder::token` [formatting.rs:45] | 9,994 | 1,313,352 | green token |
| 11 | `child::<NameRef>` [nodes.rs:298] | 9,913 | 634,432 | red node |
| 12 | `memoize` `result.clone()` [context.rs:316] | 9,723 | 1,515,466 | String new |
| 13 | `single_ident_path` `push_str` [types.rs:71] | 9,527 | 93,937 | String growth |
| 14 | `format_expr_uncached` [expr.rs:221] | 9,056 | 579,584 | red node |
| 15 | `Context::rewrite_items` `collect` [overflow.rs:591] | 7,601 | 2,450,560 | Vec new |
| 16 | `join_rewrites` [chains.rs:699] | 6,904 | 335,964 | String new |
| 17 | `child::<Expr>` [nodes.rs:298] | 6,852 | 438,528 | red node |
| 18 | `FmtVisitor::visit_fn` [items.rs:1678] | 6,721 | 430,144 | red node |
| 19 | `make_subexpr_list` `vec![top]` [chains.rs:450] | 6,610 | 414,408 | Box/Vec |
| 20 | `RewriteContext::new` `Rc::new` (inlined) [context.rs:195] | 6,013 | 144,312 | Rc |
| 21 | `child::<PathSegment>` [nodes.rs:298] | 5,974 | 382,336 | red node |
| 22 | `MacroTokens::prefix_parse` [macro_args.rs:168] | 5,959 | 797,631 | parser buffers |
| 23 | `MacroTokens::build` [macro_args.rs:277] | 5,901 | 823,144 | parser buffers |
| 24 | `child::<Meta>` [nodes.rs:298] | 5,893 | 377,152 | red node |
| 25 | `rewrite_segment` [types.rs:309] | 5,263 | 336,832 | red node |

## 4. Caches (cachegrind, default model: I1 32 KB 8-way, D1 32 KB 8-way, LL 34 MB; measured)

| run | Ir | I1 misses | I1 miss rate | D refs | D1 misses | LL misses |
|---|---|---|---|---|---|---|
| `bench`, default | 588,694,071 | 7,324,842 | 1.24% | 262,220,316 | 2,217,341 | 74,697 |
| `bench`, LTO build | 540,630,938 | 6,261,085 | 1.16% | 230,974,652 | 2,098,655 | 73,373 |
| `parse_floor rowan` | 168,933,599 | 95,583 | 0.06% | 81,203,166 | 1,003,728 | 57,862 |

- `malloc.c` accounts for 474,173 I1 misses (6.5%) and 413,653 D1 read misses (27%); the
  remaining I1 misses spread over the formatter (`write_list` 140,619 the largest function).
- Wall clock on the sample, best of 20, three interleaved rounds (load 7, labelled noisy):
  `bench` 0.091–0.093 s, `parse_floor rowan` 0.023–0.024 s. Time ratio 3.9–4.0 against an Ir
  ratio of 3.49 (587.2 M / 168.1 M).
- **Hypothesis** (assumes 2.8 GHz with no frequency change): the sample takes about 255 M
  cycles; parsing about 64 M cycles for 168 M Ir (CPI 0.38); the remaining 419 M Ir take about
  191 M cycles (CPI 0.46). The CPI difference, 0.08 × 419 M ≈ 33 M cycles (13% of the
  sample), bounds what all micro-architectural effects that parsing does not share cost the
  formatter, I1 misses included. 7.2 M extra I1 misses account for it at about 4.6 cycles
  per miss. LL misses (75 K) and D1 misses (0.8% of data references) do not bound
  throughput.

## 5. Wall clock (`bench -n 5`, all 1238 fixtures, core 0)

| when | load average (1 min) | MB/s |
|---|---|---|
| before profiling | 0.05–0.47 | 4.49, 4.73 |
| after profiling | 7.0–8.3 (labelled noisy) | 3.59, 3.74 |

The drop follows the load from other agents; the binary did not change.

## 6. LTO, one codegen unit, `panic = "abort"` (measured)

| | default | LTO build | change |
|---|---|---|---|
| callgrind Ir, sample | 587,211,777 | 539,699,788 | −47.51 M (−8.09%) |
| I1 misses, sample | 7,324,842 | 6,261,085 | −14.5% |
| `.text` of `bench` | 1,841,953 B | 1,371,290 B | −25.5% |
| `bench -n 5`, five interleaved pairs (3 default-first, 2 LTO-first) | 3.91–4.78 MB/s | 4.67–5.19 MB/s | LTO ahead in 5 of 5 pairs, +4% to +23% |

- The wall-clock pairs ran at load 7.1–8.7 and are labelled noisy.
- Ir change by component (default → LTO): parser −15.7 M, allocator −10.6 M (Rust allocation
  wrappers inlined), green building −9.9 M, `core`/`std` −8.7 M, `ra_ap_syntax` −6.7 M,
  red cursor −4.7 M; chloro's own code +11.7 M (callees inlined into it).
- No `catch_unwind` in `chloro` or `chloro-core` (read), so `panic = "abort"` changes only the
  behaviour after a panic.
- In `[profile.release]` of the workspace, these settings would apply to the `chloro` binary
  and the examples, not to library users' builds (read: Cargo profiles are set by the root
  workspace being built).

## Levers

Ranked by upper bound. Bounds are on the 49-file sample as a share of 587.21 M Ir; 1% = 5.87 M. Lettered rows are parts of the row above them. Row 16 is bounded in cycles, not instructions. They overlap: the
red-node and `String` rows contain their allocator cost, so rows are not additive. The bound
is the cost the mechanism removes if it removes all of it.

| rank | lever (mechanism; rows it removes) | upper bound | arithmetic | scope | correctness risk | tried before |
|---|---|---|---|---|---|---|
| 1 | Global allocator in the CLI binary (e.g. mimalloc): cheaper malloc/free for all 628,959 blocks (§1 allocator, §3) | ≤ 19.7% | glibc functions 115.7 M / 587.2 M; the replacement's own cost is not measured | `chloro/src/main.rs` (`#[global_allocator]`), `chloro/Cargo.toml` | none for output; library users unaffected | mimalloc in `bench` (performance checkpoint): wall within noise, Ir not measured; reverted because a library cannot set the allocator |
| 2 | Repeat rewrites of expressions, patterns, types (decide-once or reuse across indentation) | 16.9% (earlier journal) | measured at `50d2ca8` on the same sample (formatter-work-measurements); not re-measured here | formatter-wide | high (layout equivalence) | X9/X9c: indentation reuse not exact |
| 3 | No red nodes at all (green or flat substrate for every access) | ≤ 15.3% | 90.1 M (§2) / 587.2 M | every module (several hundred typed accessors) | high | X3/X4 pass-level only |
| 4 | `String` per rewrite (`Option<String>` results concatenated): fewer intermediate strings, writes into a shared buffer | ≤ 8.8% | 184,831 String blocks × 251 = 46.4 M + `fmt` 5.4 M = 51.8 M | formatter-wide (`Rewrite` signature) | high for a full redesign | chain items without `format!` (`03e063f`) landed |
| 4a | `node_text` returning a source slice when the node lies in the file being formatted (fall back for macro fragments) | ≈ 1.0% | 17,094 × 251 = 4.3 M + `node_text`/`push` 1.8 M = 6.1 M | `nodes.rs:309-322` and callers | medium: fragment trees carry padding and spliced text | no |
| 4b | memo stores and returns a shared string (`Rc<str>`) instead of cloning on insert | ≤ 0.45% | 9,723 blocks; `String::clone` 1.31 M + frees ≈ 2.3–2.6 M | `context.rs:268-325`, `format_expr` callers | low | no |
| 4c | `single_ident_path` `String::new` + `push_str` growth; `rewrite_method_call` `String::from(".")` then growth | ≈ 0.85% | (9,527 + 10,826) blocks × 251 = 5.1 M | `types.rs:53-75`, `chains.rs:317` | low | no |
| 5 | LTO fat + 1 codegen unit (+ `panic = "abort"`) in `[profile.release]` | 8.09% (measured) | 587.21 M → 539.70 M | `Cargo.toml` | none for output | LTO + 1 CGU at `9202b42`: 2–8% wall, at the noise edge, not pursued |
| 6 | `CharClasses` scanning cost per character | ≤ 4.9% | `CharClasses::next` 28.84 M inclusive; callers `span_ends_with_comma` 6.30 M, `UngroupedCommentCodeSlices` 5.98 M, `LineClasses` 5.31 M, `CommentCodeSlices` 4.61 M, `find_uncommented` 4.68 M, `contains_comment` 1.14 M | `comment.rs:638-830` | low: same state machine, equivalence testable on both corpora | `skip_plain` landed (`a156c83`); X8 token index cost +3% (refuted) |
| 6a | `recover_comment_removed` slow path (`changed_comment_content` `eq_by`) | ≤ 2.5% | 14.52 M in 210 calls (69 K Ir each) | `comment.rs:1046-1105` | low | listed as untried (≈ 2.7%) in the performance checkpoint |
| 6b | `span_ends_with_comma` scans the whole span forward | ≤ 1.5% | 8.94 M in 640 calls (14 K Ir each); 433 from `format_expr_uncached` | `expr.rs:1433-1451` | medium (comments, raw strings) | a cache by span tried: no gain (different spans each call) |
| 7 | Green-tree reads on the top-25 creation paths: D rows, S rejected siblings, C trivia | 2.4–4.1% | (29,377 + 15,066) × 316 = 14.0 M (2.39%); with C rows (30,773 more) 23.8 M (4.05%) | `nodes.rs` (`stmts_of`, `outer_attributes`, `inner_attributes`, `child`), `visitor.rs:463,528,72`, `items.rs` (`param_ty_hi`, `FnSig::from_fn`), `matches.rs:41`, `types.rs:49,309`, `pairs.rs`, `chains.rs:172-205`, `expr.rs` | low per site; G1/G2 pattern with a regression test against the generated accessor | G1, G2 landed; `child::<N>` for the remaining `expr()`/`path()` accessors (83 sites, rows 6, 7, 11, 16 are of this kind) measured 607 → 605.5 M (−0.25%) pre-D1 and reverted as within noise — counts are now deterministic, so a 0.25% change is resolvable |
| 8 | Width of built strings: `unicode_str_width` ASCII path counts per byte | ≤ 2.9% | 17.26 M, 120,914 calls (143 Ir each) | `utils.rs:126-140` | low (ASCII path already special-cased; test exists) | one-pass ASCII width (`4edd403`) landed |
| 8a | `filter_normal_code` allocation + `LineClasses` for every snippet containing `/` | ≤ 1.5% | 8.58 M, 5,563 calls | `comment.rs:1062-1080`, `utils.rs:156` | low | no |
| 9 | Green tree freed per file (arena or no `Arc` tree) | ≤ 1.9% | `drop_slow` 11.16 M inclusive | rowan (representation change) | high | no |
| 10 | `rustc_compat` walk folded into tree building | ≤ 1.8% | 10.44 M | `rustc_compat.rs`, `formatting.rs:61-87` | medium | no |
| 11 | Memo overhead: fewer stored entries, no stored red node, no clone | ≤ 1.65% | 2.64 + 1.92 + 1.31 + 3.63 + 0.16 = 9.7 M; 391 hits of 10,232 | `context.rs:268-325` | medium (memo correctness argument) | memo on/off at `6b07131`: memo saves 19 M net; paths and literals excluded (`9202b42`) |
| 12 | One `NodeCache` shared across files and macro fragments | ≈ 1.0% | `parse_floor` `rowan` 168.07 M → `rowan-shared` 162.39 M = −5.68 M; 955 fragment builders each make a new cache | `formatting.rs:35-59`, `macro_args.rs:244,278` | low (interning gives equal trees) | measured in `parse_floor` (6% of parse on all fixtures), not in chloro |
| 13 | `Block::from_block_expr` cached per block | 0.8–1.0% (hypothesis) | 11.63 M × (1 − 1/1.73…1.94) = 4.9–5.6 M | `nodes.rs:375-389`, callers | low | deferred in phase-2 close |
| 14 | `to_input` copy of the lexed tokens | ≤ 0.93% | 4.37 M (files) + 1.07 M (fragments) | ra_ap_parser API | n/a without a parser change | no |
| 15 | `RunState::line_of` line-start table built with a `bytes().enumerate().filter()` chain (4.87 M exclusive in the iterator) | ≤ 0.86% | 5.05 M in 29 builds, all from `walk_reorderable_items` | `context.rs:126-138` | low | no |
| 16 | PGO / BOLT (code layout against the I1 misses) | ≤ 13% of cycles (hypothesis, §4) | CPI gap 0.08 × 419 M | build only | none for output | not measured |

Surprises in the data:
- LTO removes 8.1% of instructions, more than its earlier 2–8% wall-clock result suggested;
  most of the gain is in parser, allocator wrappers, green building and `ra_ap_syntax`
  accessors inlined across crates (`ra_ap_syntax` 10.12 M → 3.43 M).
- The memo hits 3.8% of its lookups on the sample, against 9.7 M of overhead.
- `recover_comment_removed`'s slow path and `span_ends_with_comma` cost 69 K and 14 K Ir per
  call: a few long scans, not many short ones.
- `walk_reorderable_items` builds a full line-start table (5.05 M) in 29 of 55 runs.
- `Block::from_block_expr` is the third-largest red-node creator (29,702) because
  `stmts_of` walks `children_with_tokens`, creating a red token for every whitespace and
  brace token of every block, on every one of the 1.73–1.94 calls per block.

## Not levers (measured, small)

| item | measurement |
|---|---|
| `Shape`/`Indent` arithmetic and strings | `shape` module 1.54 M exclusive (0.26%); `Indent::to_string_inner` 45,988 calls, 1.54 M |
| hashing (`FxHasher`, `hashbrown`) | 1.91 M (0.33%) for both maps; macro-args cache 64 blocks |
| `format!` | 5,436 blocks (0.9%); `format_inner` 1.80 M exclusive |
| the visitor buffer reservation (visitor.rs:91) | 47% of bytes in 913 blocks; tested before with no wall-time change |
| data cache | 75 K LL misses; D1 miss rate 0.8% |
| green-tree helpers (`has_token`, `child`, `span_without_attrs`, `node_text`) | rowan green reads 5.16 M (0.88%); `has_token` 2.59 M, `span_without_attrs` 2.84 M |
| `RewriteContext` `Rc<Cell<bool>>` per context | 6,013 blocks ≈ 1.5 M (0.26%) |
| `Vec` growth | 14,397 blocks (2.3%); `ChainItem` growth 6,563 of them |
| token index for comment queries (X8) | +3% Ir (refuted earlier) |
| `str` searchers over built strings (`lines`, `rfind('\n')`, `contains`, `trim`) | about 24 M in total (`CharSearcher` both directions 19.7 M, `trim_matches` 4.7 M), spread over more than 20 callers; the largest chloro caller, `filtered_str_fits`, 4.19 M |

## Current State

- The formatter executes 587.2 M instructions on the 49-file sample: 29.9% in chloro's own
  code, 22.5% in the allocator, 16.5% in the parser, 8.4% building and 0.6% freeing green
  trees, 7.5% in the red cursor ($S/cg/prof_components.txt)
- 284,789 red nodes are created on the sample at 316 instructions each, 15.3% of the total;
  83% are freed in the function that created them ($S/cg/red_top25.md)
- DHAT counts 628,959 blocks: 45.3% red nodes, 29.4% `String` creation and growth, 10.8%
  green tree ($S/dhat/summary.md)
- The formatter's simulated I1 miss rate is 1.24% against 0.06% for parsing alone
  ($S/cachegrind/)
- A fat-LTO, one-codegen-unit, `panic = "abort"` build executes 8.09% fewer instructions on
  the sample and was faster in 5 of 5 noisy wall-clock pairs ($S/wall_lto.txt)

## Missing

- No instruction count of the formatter with another global allocator (lever 1)
- No measurement of trivia versus kept elements in the `children_with_tokens` scans (class C)
- No PGO build measured
- No cycle-level counters on this host (`perf` is not installed); §4's CPI split rests on an
  assumed clock frequency
