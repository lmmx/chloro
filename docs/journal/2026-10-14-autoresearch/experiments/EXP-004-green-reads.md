# EXP-004: green-tree reads on the red-node creation paths

Ledger row E4 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the creation-path table this experiment works from is
section 2 of [../profile-baseline.md](../profile-baseline.md). Labels: **measured** (a
tool's output on the builds below), **read** (from source).

## Identity

| field | value |
|---|---|
| branch | `20261014-green-reads` |
| base | `4cd6336` (integration branch `claude/chloro-perf-research-swarm-nrscj8`; formatter code of `884f9c9`, the profile baseline) |
| commits | `4841d9b`, `5e7e36c`, `362e1eb`, `fb07072`, `62c4348`, `5c02ef2` (one per site group, in that order) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, valgrind 3.22.0 |
| host | 4 vCPU Intel Xeon @ 2.80GHz, 15 GB; CPU-heavy runs pinned with `taskset -c N`; gates of one commit ran on four cores at once (callgrind on core 0, hash checks on 1, `conform -i` on 2, clippy and tests on 3) |
| build | `cargo build --release --examples -p chloro-core` (default release profile, no `[profile.release]` in the workspace) |
| sample | every 25th file of the C-locale-sorted `.rs` list under `chloro-core/tests/conformance/fixtures`: 49 files, 650,213 bytes, copied as `NN_<basename>.rs` |
| `Ir` command | `setarch $(uname -m) -R taskset -c 0 valgrind --tool=callgrind bench -n 1 --root SAMPLE`; the `I refs` line |
| red-node count | sum of the `calls=` lines into `<rowan::cursor::NodeData>::new` in the callgrind file (`scratchpad/scripts/redcount.py`) |
| creation paths | the same run with `--separate-callers14='*cursor::NodeData>::new'`, grouped by the first `chloro_core::formatter` frame and the frame below it (`scratchpad/scripts/redpaths.py`) |

The unmodified build measures 587,589,233 Ir on this host against 587,211,777 in the
profile baseline (+0.064%); the red-node count, 284,789, is the baseline's. The cause of the
`Ir` difference is not examined; every comparison below is against the figure measured here
with the same binary in the same window.

## Hypothesis and falsification

Reading the D, S and C queries of the top-25 creation paths from the green tree, and
computing `Block::from_block_expr` once per block, removes 2.4–5% of instructions and
45k–75k red nodes on the sample with identical output. Falsified if any output changes, or if
the total gain is under 1%.

## What changed (read)

Every change keeps the function's signature and answers the same question from
`rowan::GreenNodeData` children; the mechanism is G1/G2's
([../../2026-10-10-green-attribute-queries.md](../../2026-10-10-green-attribute-queries.md),
[../../2026-10-11-parse-drop-thread.md](../../2026-10-11-parse-drop-thread.md)). Red nodes
are created only for elements the caller keeps, by `child_or_token_at_range` on the child's
range (a binary search on the green children) or, for an empty child, by position.

| commit | sites | helpers added (chloro-core/src/formatter/nodes.rs unless noted) |
|---|---|---|
| `4841d9b` statements | `stmts_of` classifies the statement list's children by green kind and offset (tail expression: the first expression child; `ExprStmt::expr` via `child`; `MacroCall::token_tree` and `TokenTree::l_curly_token` on the green token tree); `Block::from_block_expr` reads the `unsafe` token's start | `child_at`, `green_has_token`, `token_start`; `child` rewritten on `child_at` |
| `5e7e36c` attributes | `outer_attributes` and `inner_attributes` classify children on the green node and create a red element per attribute kept | `element_at`; `is_inner_doc_text` (span.rs) |
| `362e1eb` one-shot queries (class D) | `fn` body presence (`visit_item`, `visit_assoc_item`, `ExternItem`, `item_attrs`), `arm_span`, `param_ty_hi`, `mac_span`, generic-argument presence and the argument lists of `rewrite_segment`, `BinExpr::op_kind`/`op_token` (`rewrite_all_pairs`, `is_assignment`, `rewrite_assignment`, `rewrite_pair`), left-operand presence, `Attribute::path_text`/`is_skip`/`attr_meta`, `LitKind::of`, the closure `use` keyword, `has_c_variadic` | `has_child`, `child_range`, `green_child`, `attr_path_text`, `bin_op_kind`, `bin_op_text`, `push_non_trivia_text` |
| `fb07072` rejected siblings (class S) | `child::<N>` at `fn` bodies, names and generic parameter lists, `let` patterns/types/initializers/`else` blocks, parameter patterns and types, match arm bodies and guards, call argument lists, closure bodies and parameter lists, record literal field lists and bases, `use` tree paths, method-call generic arguments, tuple field names; `nth_child::<N>` at `BinExpr::rhs` and `IndexExpr::index`; `condition_expr`, `then_branch`, `else_branch`, `loop_body` at `to_control_flow` | `nth_child`, `condition_expr`, `then_branch`, `else_branch`, `loop_body` |
| `62c4348` macro scans (class C) | `rewrite_macro_name`, `tt_delimiter`, `tt_is_empty`, `MacroCall::token_tree` (`rewrite_macro_inner`, `convert_try_mac`, `macro_style`), `MacroBranch::new`'s delimiter kind, `MacroTokens::splice`'s child walk | `first_token_kind`, `green_children_with_ranges` |
| `5c02ef2` block cache | `RunState::block_of` keeps the `Block` of each block expression for the run, keyed by green address and offset like the memo, cleared by `clear_memo`; `to_control_flow`, `plain_block` and `as_block` take the run or the context (context.rs) | `RunState::blocks`, `RunState::block_of` |

`non_trivia_text` and `is_outer_doc_comment` are now used by tests only (`#[cfg(test)]`).
No `unsafe`, no new dependency, no change to which layout attempts are made: each site
computes the value the generated accessor computed.

## Regression tests (read)

Each green read has a test comparing it with the generated accessor or the cursor walk it
replaced, over sources containing every variant, with an assertion that each variant occurs.

| test | compared with | variants asserted |
|---|---|---|
| `stmts_of_matches_a_cursor_walk_on_every_statement_list` (nodes.rs) | the previous `children_with_tokens` walk, kept in the test module | `let`, item, expression, `;`-terminated, macro with `;`, macro with braces, tail macro, empty statement; empty block, block with tail, block without tail |
| `block_from_block_expr_matches_the_generated_accessors` | `BlockExpr::stmt_list`, `unsafe_token` | ≥ 20 blocks, ≥ 3 `unsafe` |
| `outer_attributes_match_a_cursor_walk_on_every_node`, `inner_attributes_match_a_cursor_walk_on_every_node` | the previous walks | normal and doc attributes; nodes whose leading attributes are all inner |
| `inner_doc_text_matches_the_comment_classification` | `is_inner_doc_comment`, `is_outer_doc_comment` on every comment token | `///`, `////`, `/** */`, `/***`, `/**/`, `//!`, `/*!`, plain |
| `has_child_and_child_range_match_the_generated_child_accessors` | `children().find_map(N::cast)` for `Expr`, `BlockExpr`, `Type`, `Pat`, `GenericArg`, `TokenTree`, `Path` on every node | each type found at least once |
| `bin_op_matches_the_generated_operator_accessors` | `BinExpr::op_kind`, `op_token` | all 29 operators, compound assignment included |
| `attr_path_text_matches_the_generated_meta_accessors` | `Attr::meta().path()` + `non_trivia_text` | attribute paths with comments, whitespace, raw identifiers, generic arguments |
| `lit_kind_matches_the_generated_literal_kind` (expr.rs) | `Literal::kind` | integer, float, string, raw string, byte string, C string, char, byte, bool, suffixes, attributed literal |
| `mac_span_matches_the_generated_accessors` (macros.rs) | `MacroCall::path`, `token_tree` | every delimiter; item, statement and expression position; attributes; trailing `;` |
| `has_c_variadic_matches_the_generated_param_accessor` (types.rs) | `params().last()` | `...` last, `...` only, no parameters, `self` parameter |
| `control_flow_accessors_match_the_generated_ones` (nodes.rs) | `IfExpr::condition`/`then_branch`/`else_branch`, `WhileExpr::condition`/`loop_body`, `ForExpr::iterable`/`loop_body`, `IndexExpr::base`/`index`, `BinExpr::lhs`/`rhs` | `else`, `else if`, `if let`, block conditions, missing bodies (`if {}`, `while {}`, `for x in {}`), labels, nested index |
| `macro_name_and_token_tree_queries_match_the_cursor_walks` (macros.rs), `green_token_walks_match_the_cursor_walks` (nodes.rs) | the previous `descendants_with_tokens`, `first_token` and `children_with_tokens` walks | every delimiter, empty and comment-only token trees, nested trees, `::` and raw-identifier paths |
| `block_of_matches_from_block_expr_and_is_cached` (context.rs) | `Block::from_block_expr` | nested, empty, `unsafe`, labeled blocks; a repeat call adds no entry; `clear_memo` empties the map |

The chloro-core library test count goes from 144 to 156.

## Per-commit results (measured)

`Ir` and red nodes on the sample, one callgrind run per build (counts are deterministic
since D1). Each row's gates ran on that commit's binary.

| commit | sites | red nodes | Ir | ΔIr | Ir per node removed | gates | decision |
|---|---|---|---|---|---|---|---|
| base `4cd6336` | — | 284,789 | 587,589,233 | — | — | — | — |
| `4841d9b` | `stmts_of`, `from_block_expr` | 262,032 (−22,757) | 583,244,951 | −4,344,282 (−0.74%) | 191 | pass | kept |
| `5e7e36c` | `outer_attributes`, `inner_attributes` | 251,215 (−10,817) | 580,903,783 | −2,341,168 (−0.40%) | 216 | pass | kept |
| `362e1eb` | D rows | 196,535 (−54,680) | 563,434,616 | −17,469,167 (−3.01%) | 319 | pass | kept |
| `fb07072` | S rows | 158,774 (−37,761) | 555,406,669 | −8,027,947 (−1.42%) | 213 | pass | kept |
| `62c4348` | macro scans | 145,144 (−13,630) | 551,103,784 | −4,302,885 (−0.77%) | 316 | pass | kept |
| `5c02ef2` | block cache | 141,544 (−3,600) | 549,937,449 | −1,166,335 (−0.21%) | 324 | pass | kept |
| **total** | | **−143,245 (−50.3%)** | | **−37,651,784 (−6.41%)** | 263 | | |

- No commit was rejected. The threshold for keeping a commit was `Ir` down by more than
  0.1% with identical output; the smallest kept change is the block cache at 0.21%.
- `362e1eb` was first measured at 563,459,429 with `Iterator::last` in `has_c_variadic`;
  clippy rejects `last` on a double-ended iterator, and the committed `next_back` form
  measures 563,434,616 (−24,813).
- `Block::from_block_expr` ran 2,571 times on the sample before the cache (callgrind call
  count at `fb07072`); the cache removes 3,600 red nodes and 1.17 M Ir, so a repeated call
  cost about 1,000 Ir after the first five commits had taken the statement walk off red
  nodes.
- Red-node creation after `5c02ef2`: 141,544 on the sample. Of these, 65,548 (46.3%) are
  created through `element_at`, the range search that creates a kept node.

## Gates (measured)

| gate | corpus | result, every commit |
|---|---|---|
| `cargo fmt --check` | | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean (after the `next_back` change in `362e1eb`) |
| `cargo test -p chloro-core` | 156 library tests at `5c02ef2`, 17 conformance batch tests, 4 + 10 integration tests | all pass |
| `bench --check` | 1238 fixture files, hashes recorded with the base binary | 0 changed |
| `bench --check` | cargo registry corpus under `~/.cargo/registry/src`: 19,463 files in 524 crates for `4841d9b` and `5e7e36c`; 19,978 files in 556 crates from `362e1eb` on (the clippy run fetched 32 more crates; the record was re-made with the base binary before `362e1eb` was checked) | 0 changed |
| `conform -i` | 1220 rust-analyzer fixtures | 1219/1220 identical, 0 not idempotent |

The registry corpus here was fetched from
[../corpus/registry-manifest.toml](../corpus/registry-manifest.toml) in this container;
crates.io served 524 crates for the manifest's lockfile-less `cargo fetch` against the 577
recorded in [../corpus/registry-crates.txt](../corpus/registry-crates.txt).

## Wall clock (measured)

`bench -n 5` on the 1238 fixtures (13.8 MB), base binary and `5c02ef2` binary interleaved,
three pairs, `taskset -c 0`, no other agent work on the host during the runs.

| pair | base, best of 5 | `5c02ef2`, best of 5 | difference |
|---|---|---|---|
| 1 | 3.000 s, 4.63 MB/s | 2.905 s, 4.78 MB/s | +3.2% |
| 2 | 2.946 s, 4.72 MB/s | 2.802 s, 4.96 MB/s | +5.1% |
| 3 | 2.980 s, 4.67 MB/s | 2.942 s, 4.72 MB/s | +1.1% |

Load average 0.59 before and 0.92 after the six runs. The base range is 4.63–4.72 MB/s and
the `5c02ef2` range 4.72–4.96 MB/s: the new binary is ahead in 3 of 3 pairs, and the
ranges meet at 4.72 MB/s, so the protocol's non-overlap condition for a wall-clock claim
is not met. The instruction count is the claim; the wall-clock direction agrees with it.

## Interpretation

- The hypothesis's range (2.4–5% of `Ir`, 45k–75k red nodes) is exceeded: 6.41% and
  143,245 red nodes. The baseline's bound counted the top-25 rows only and classified by
  the accessor named in the profile; the D and S commits also reached paths below the top
  25 (`LetStmt` and `Param` rewrites, `has_c_variadic` re-walking every parameter,
  closure keyword scans, `contains_skip`), and rows the baseline counted as kept (K)
  contained rejected siblings in inlined frames.
- 263 Ir per red node removed over the whole experiment, against 316 per node in the
  baseline's cost model and 324 in G2. The per-commit figure ranges from 191 (statements:
  the removed elements were mostly red tokens, which skip `cast`) to 324 (block cache).
- The class-D commit is the largest single step (3.01%): a query that creates a node and
  drops it pays allocation, cursor setup and free for one comparison.
- The memo did not prevent repeated `Block::from_block_expr` calls because the memo keys
  on the expression being rewritten and the block is rebuilt inside each attempt; the cache
  keys on the block itself.

## Decision

Kept: all six commits. Branch `20261014-green-reads` at `5c02ef2`, ready for the integrator's
gates 1–5 on the merged tree.

## Follow-up

Creation paths remaining at `5c02ef2` (red nodes on the sample, first chloro frame and
the frame below it; `scratchpad/cg/c6-paths.txt`):

| red nodes | first chloro frame | accessor below it |
|---|---|---|
| 65,548 | `nodes::element_at` | `child_or_token_at_range` (kept nodes located by `child`, `nth_child`, `child_at`, `element_at`) |
| 14,216 | `expr::format_expr_uncached` | `support::child` (`PathExpr::path` and the first-child `expr()` accessors) |
| 5,865 | `chains::rewrite_chain_from` | `support::child` (`FieldExpr::expr`, `AwaitExpr::expr`, `TryExpr::expr`, first children) |
| 2,921 | `patterns::rewrite_ident_pat` | `SyntaxNodeChildren::new` (`IdentPat::name`, first child) |
| 2,100 | `Param::rewrite` | `support::child` (the pattern and type kept for rewriting) |
| 2,092 | `expr::rewrite_struct_lit` | `SyntaxElementChildren` (a `children_with_tokens` walk inlined into it) |
| 2,059 + 677 + 677 | `MacroTokens::splice` | the spliced tree's `children_with_tokens` collected, `last_child` |
| 2,026 + 1,557 | `format_expr_uncached`, `rewrite_chain_from` | `ArgList::args` (every argument, kept) |
| 1,966 + 1,023 | `closures::rewrite_closure_fn_decl` | `ParamList::params` (every parameter, kept) |
| 1,945 | `Vec<OverflowableItem>` collection | `SyntaxNodeChildren` (tuple and array elements, kept) |
| 1,914 | `Type::rewrite` | `support::child` |
| 1,570 + 510 | `imports::UseTree::from_ast` | `SyntaxNodeChildren::new`, `support::child` |
| 1,356 + 1,035 + 957 | `Pat::rewrite` | `support::child`, `Vec<Pat>` collection, `SyntaxNodeChildren::new` |
| 1,120 | `Vec<NodeOrToken<SyntaxNode, SyntaxToken>>` collection | a collected `children_with_tokens` |

By first chloro frame: `element_at` 65,548 (46.3% of 141,544), `format_expr_uncached`
17,122, `rewrite_chain_from` 7,945, `rewrite_struct_lit` 3,430, `MacroTokens::splice`
3,413, `Pat::rewrite` 3,348, `rewrite_closure_fn_decl` 3,004, `rewrite_ident_pat` 2,922,
`UseTree::from_ast` 2,538, `Param::rewrite` 2,100.

- The `element_at` rows are kept nodes: the statements, attributes, children and operands
  the rewrite functions hold. Removing them needs the typed `ast::*` values passed between
  rewrite functions to carry something other than a red node (lever 3 of the profile
  baseline), not another green read.
- `rewrite_chain_from`'s nodes are the chain's receivers, method names and arguments,
  all kept; `make_subexpr_list` clones the chain's expressions into `SubExpr` values.
- `format_expr_uncached`'s `PathExpr::path` (9,056 nodes) is kept for `rewrite_path`.
- `rewrite_closure_fn_decl` and `FnSig::from_fn` collect parameters with `params()`
  (`SyntaxNodeChildren`), one kept node per parameter.
- `MacroTokens::splice` still collects the spliced tree's children as red elements
  (macro_args.rs, the `elements` vector after the splice) to return the nodes at the
  fragment positions.
