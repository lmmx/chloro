# EXP-007b: the formatter on the flat syntax array

Ledger row E7b in [../experiments.md](../experiments.md); the feasibility study it builds
on is [EXP-007-flat-cst.md](EXP-007-flat-cst.md); procedure and gates in
[../protocol.md](../protocol.md). Labels as there: **measured** (a tool's output on the
builds below), **read** (from source), **derived** (arithmetic on measured numbers).

## Identity

| field | value |
|---|---|
| branch | `20261014-flat-port` |
| base | `dc20cd7` (head of `claude/chloro-perf-research-swarm-nrscj8` when the branch opened: E6, E1, E2 merged); the integration branch moved to `aa1e058` (E4 merged) during the port and is merged into the branch (`e259666`) |
| commits | `b327b02` substrate; `9031948` the sweep; `ac5c15a` tests, examples, lints; `0fc5056` dependencies; `e259666` the E4 merge; `353a18e` a clippy nit; then the records |
| compared builds | **base**: `aa1e058` built in a worktree (`/home/user/wt-base`); **port**: the branch head. Both `cargo build --release -p chloro-core --examples` and `cargo build --release -p chloro --features cli` under the workspace release profile (`codegen-units = 1`, `lto = "fat"`, E2) |
| toolchain | `rustc 1.97.0`, `cargo 1.97.0`, valgrind 3.22.0, Python 3 (the generator and its test) |
| host | 4 vCPU Intel Xeon @ 2.80GHz, 15 GB, the host of the E4 and E7 records; callgrind pinned to core 3, `bench -n 5` pairs to core 1 |
| sample | `benchmarks/profile.sh`'s 49 files (every 25th of the C-sorted fixture list, 650,213 bytes) |
| golden hashes | recorded from the unmodified `dc20cd7` build: 1,238 fixture outputs and 19,463 registry outputs (`bench --record`); the orchestrator's note that E4 changed no output makes them valid for `aa1e058` too |

Lineage: the design is the change-of-direction entry's option 2
([../../2026-10-10-change-of-direction.md](../../2026-10-10-change-of-direction.md)), measured
as a bare substrate in [../../2026-10-10-representations.md](../../2026-10-10-representations.md)
and deferred in [../../2026-10-11-phase-2-close.md](../../2026-10-11-phase-2-close.md); the
autoresearch entry's "Lineage of the flat syntax array" traces what each added.

## Question and falsification

**Question.** Does replacing rowan under the formatter with E7's flat array and generated
typed layer, with macro-argument fragments as trees in a run-lifetime arena, remove the
instructions E7 estimated (24.9% of the then baseline) with byte-identical output?

**Refuted if** any output hash of the two corpora changes, `conform -i` drops below
1,219/1,220 or reports a non-idempotent output, any `config_matrix` option value loses a
file, or `Ir` on the sample falls by less than 20%.

## The design as built (read)

The orchestrator's decisions (EXP-007 §4b, ownership option i) as they stand in the code:

- **Substrate.** `chloro_core::cst` (`src/cst/mod.rs`, 930 lines): `Tree<'s>` is one
  `Vec<Elem>` in preorder over a borrowed source (`Elem`: kind, node flag, end index, start
  offset, length; 16 bytes) and an id; `Node<'t>` and `Token<'t>` are `(&'t Tree, u32)`,
  `Copy`, with `kind`, `text_range` (a `cst::TextRange` of two `u32`), `text` (a source
  slice), `children`, `children_with_tokens`, `first_token`/`last_token` (rowan's chain
  of first/last children, `None` at a childless node), `descendants*`, `parent`,
  `index`, `tree_id`. `TreeBuilder` lays elements out from parser steps
  (`intersperse_trivia`) or one at a time, from any base offset; `Tree::parse` builds a
  file's tree (`None` on a lexer or parser error, the conditions of the old
  `formatting::parse`) and `Tree::parse_with_errors` the tree tests look at. No `unsafe`.
- **Typed layer.** `src/cst/generated.rs` (7,865 lines) is `gen_ast.py`'s output over the
  checked-in `rust.ungram` of `ra_ap_syntax 0.0.307`: 154 node structs, 15 enums, 10
  `Has*` traits, 501 accessors, equal to `ra_ap_syntax`'s generated set (the generator
  still compares with the oracle when given its path; the test
  `generated_layer_is_up_to_date` regenerates and compares the file). `src/cst/ast.rs`
  re-exports it and holds what `ra_ap_syntax` writes by hand and the formatter uses: the
  `expr_ext`/`node_ext` accessors with their child-selection rules, `RangeItem`,
  `ElseBranch`, `LiteralKind`, the token types with `Comment`'s kinds and `IsString`,
  `HasDocComments::doc_comments`.
- **Ownership (decision 1).** `typed-arena 2.0.2` is the one new dependency of
  `chloro-core`. `RunState<'a>` holds `&'a TreeArena<'a>` and a tree counter: the file's
  tree is 0 and is owned by `format_text`'s frame; every macro-argument fragment is built
  by `Tree::from_steps_at` at its file offset over the whole file's text, numbered
  1, 2, ..., and appended to the arena (`RunState::keep_tree`). Handles borrow `'a`, the
  lifetime `RewriteContext<'a>` and `FmtVisitor<'a>` already carry for the source.
  `MacroArgsCache<'a>` and the memo hold handles and indices, never a tree.
- **Memo key (decision 2).** `(tree id, node index, tag, shape, flags)`; the entry holds the
  result and the three replayed effects (`macro_failure`, `lost_comment`,
  `skipped_range`) and no node. `clear_memo` is unchanged in what it drops.
- **Fragments (decision 1, EXP-007 §4b).** `macro_args.rs` parses each argument's token
  window with the same prefix-parse and separator-window logic, then `build` lays the
  consumed text out at `base + lo` with the matching `TopEntryPoint`; `MacroItems`
  unwrapping and `subtree_rejected_by_rustc` run on the fragment's root; `splice` keeps
  only its straddle check (a token-tree element crossing a fragment boundary still rejects
  the call) and returns the fragment nodes; `parse_maybe_bound` builds the synthetic
  `DYN_TRAIT_TYPE > TYPE_BOUND_LIST > TYPE_BOUND` with `TreeBuilder` (`?`, the gap, the
  parsed type's steps). No padding tokens, no copy of the token tree, no `GreenNode::new`.
- **Spans and text (decision 3).** `rustc_span`, `span_without_attrs`, `token_span`,
  `node_range_span` read `text_range()` and the leading children; `node_text` returns the
  source slice; split float tokens are laid out by the builder from the parser's token
  lengths, which the debug build checks against the source at every token.
- **`parent()` (decision 4).** No parent column. `Node::parent` is a backward search
  (the nearest earlier node whose subtree extends past the index). The two formatter sites
  keep calling it: `Stmt::attrs` on a `MacroCall` statement (the search stops at the
  `MacroExpr` after the attribute elements) and `visit_assoc_item` (a search over the
  preceding items of the impl or trait, twice). The second is a candidate for passing the
  kind down from the caller (§follow-up).
- **`rustc_compat.rs` (decision 5).** The same rules over `Node` children, with the path
  of `(node, child index)` pairs; child indices count tokens as before, since
  `condition_index` and `walk` enumerate `children_with_tokens`.
- **Dependencies (decision 6).** `rowan` and `ra_ap_syntax` are dev-dependencies (the
  equality test of `cst` and `examples/parse_floor.rs`); `ra_ap_parser` stays. The
  no-dev graph of `chloro-core` is 16 crates (37 before); `cargo tree -e no-dev -i syn`
  is empty.
- **Traceability (decision 7).** The formatter modules keep their functions and control
  flow; the diff is the type and import changes below. Handles being `Copy`, `.clone()`
  on nodes became copies and `&SyntaxNode` parameters became `Node<'_>` by value.
- **Lifetimes.** `RewriteContext<'a>` is invariant in `'a` (the run state holds arena
  handles behind `RefCell`), so `Rewrite::rewrite(&self, &RewriteContext<'_>, Shape)` keeps
  its signature and a node's lifetime stays independent of the context's: a function that
  mixes file nodes and fragment nodes names the relation (`fn convert_try<'c: 'a>` in
  `chains.rs`), and the three fragment-producing functions return arena-lifetime nodes.
  Thirty-one structs and enums gained a lifetime parameter (`Block<'a>`, `Stmt<'a>`,
  `Attribute<'a>`, `MacroArg<'a>`, `OverflowableItem<'a>`, `UseTree<'a>`, `FnSig<'a>`,
  `StructParts<'a>`, `StaticParts<'a>`, `ChainItem<'a>`, ...). `macro_rewrite_failure` is
  an `Rc<Cell<bool>>` so a nested `FmtVisitor::from_context` shares it without borrowing
  the context for `'a`.
- **E4's block cache.** `RunState::block_of` builds each block expression's `Block` once
  per run, keyed by (tree id, index); the cache stores a `CachedBlock` (statement kinds,
  node indices, spans; no handles) and rebuilds the `Block` over the caller's tree, so the
  cache carries no lifetime. E4's other changes (green reads) are superseded: the
  generated accessors and the child loops are what those reads avoided red nodes for.

## What changed per module (read: `git diff --numstat dc20cd7 HEAD`)

| module | lines +/− | what |
|---|---|---|
| `nodes.rs` | 230/196 | the green helpers as child loops; `Attribute`, `Block`, `Stmt`, `StmtKind` with `'a`; `CachedBlock`; `node_text` as a slice |
| `macro_args.rs` | 106/131 | fragments as trees at file offsets; `splice` reduced; `parse_maybe_bound` with `TreeBuilder` |
| `context.rs` | 132/45 | `RunState<'a>` with the arena, tree ids and `block_of`; the memo key |
| `rustc_compat.rs` | 51/70 | the walk over `Node` children |
| `formatting.rs` | 16/84 | `parse` is `Tree::parse` + `rejected_by_rustc`; `TreeBuilder` and its test moved to `cst` |
| `span.rs` | 24/26 | by-value tokens and nodes; `rustc_span` over `children_with_tokens` |
| `parallel.rs` | 21/20 | workers share the file's tree and own an arena each |
| `items.rs` | 90/88 | lifetimes on `Generics`, `WhereClause`, `FnSig`, `FnParam`, `StructParts`, `VariantData`, `FieldDef`, `StaticParts`, `TyAliasRewriteInfo`; `node_text` owners; `block_of` |
| `chains.rs` | 51/41 | lifetimes on `SubExpr`, `ChainItem`, `Chain`; `'c: 'a` at `convert_try` |
| `attr.rs` | 46/41 | `MetaParser<'a>` over `Element<'a>`; by-value tokens |
| `imports.rs` | 43/43 | lifetimes on `UseTree`, `UseSegment`, `UseSegmentKind` |
| `types.rs` | 43/45 | `single_ident_path` over children; `SegmentParam<'a>`, `Bounds<'a>` |
| `expr.rs` | 48/42 | `ControlFlow<'a>`, `StructLitField<'a>`; `to_control_flow` takes the run; `block_of` |
| `macros.rs` | 40/38 | `MacroArg<'a>`, `MacroDef<'a>`; by-value elements in `parse_macro_def` |
| `overflow.rs` | 27/27 | `OverflowableItem<'a>`; `Context<'a, 'b, 'c>` |
| `matches.rs` | 27/23 | `ArmWrapper<'a>`; `as_block` takes the context |
| `closures.rs` | 18/11 | `plain_block` takes the context |
| `visitor.rs` | 19/19 | `RunState<'a>`; the parent failure flag as `Rc` |
| `stmt.rs`, `patterns.rs`, `printer.rs`, `pairs.rs`, `reorder.rs`, `config.rs`, `utils.rs` | 13/13, 13/13, 13/13, 12/8, 7/5, 5/5, 4/4 | lifetimes, imports, `node_text` owners, `is_assign_op` |
| total, formatter | 1,099/1,051 | 25 of 29 modules (`comment`, `lists`, `missed_spans`, `shape`, `sort` untouched); against the merged base (`aa1e058`, with E4's green code) the diff is 1,219/2,107 |

Examples: `dump_tree` prints the flat tree, `spacing` parses with `Tree::parse`; `bench`,
`conform`, `config_matrix`, `alloc_count`, `fmt_dir`, `fmt_stdin` are unchanged.

## Commands

```sh
# goldens from the unmodified base build (dc20cd7), then on every port build:
target/release/examples/bench --record $S/golden/fixtures.tsv
target/release/examples/bench --record $S/golden/registry.tsv --root ~/.cargo/registry/src
target/release/examples/bench --check  $S/golden/fixtures.tsv
target/release/examples/bench --check  $S/golden/registry.tsv --root ~/.cargo/registry/src
target/release/examples/conform -i
target/release/examples/config_matrix -n 1220          # both builds, per-option counts compared
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
# measurements, base = /home/user/wt-base/target/release, port = target/release:
taskset -c 3 benchmarks/profile.sh --bench $BIN/examples/bench --annotate 40
taskset -c 1 $BIN/examples/bench -n 5                   # 3 interleaved pairs
$BIN/examples/alloc_count chloro-core/tests/conformance/fixtures
python3 -c 'resource.getrusage(RUSAGE_CHILDREN).ru_maxrss' around $BIN/examples/fmt_stdin < lints.rs
stat -c %s $BIN/chloro; size $BIN/chloro
cargo tree -p chloro-core -e no-dev --prefix none | sort -u
```

The registry corpus was fetched by `corpus/registry-manifest.toml` into a fresh
`~/.cargo/registry/src` (19,463 `.rs` files at recording time; later builds added
crates, and `--check` reports the added files as "not recorded").

## Gates (measured, port build at `353a18e`)

| gate | result | base (`aa1e058`) |
|---|---|---|
| `bench --check` fixtures | 0 of 1,238 outputs changed | — |
| `bench --check` registry | 0 of 20,022 outputs changed (19,463 recorded, the rest not recorded) | — |
| `conform -i` | 1,219/1,220 identical, 0 not idempotent | 1,219/1,220, 0 |
| `config_matrix -n 1220` | every one of the 31 option values has the base's count (1021/1022 for 25 values, 1020/1022 for `hard_tabs=true` and `tab_spaces=2`, 1019/1022 for `newline_style=Windows`, 843/844, 842/843 and 837/838 for the three editions) and the same 35 listed files | identical |
| `cargo test --workspace --all-features` | all pass (156 lib, 17 conformance batch, 10 + 4 integration, 7 `chloro`, 1 doc) | |
| `cargo fmt --all -- --check` | exit 0 | |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 | |
| `cargo tree -e no-dev -i syn` | empty | empty |
| `cst` equality test | every third fixture (≥ 300 files): kind, range, node/token class, token text, `parent`, `first_token`, `last_token`, `text` equal to rowan's | |

### After the E8 merge (`bc6c76e`, measured)

The integration branch moved again (E8: `rustfmt_version`, rustfmt 1.8.0 support, head
`bf3e8fd`) and is merged into the branch at `bc6c76e`; the conflicts are resolved for the
flat substrate with E8's behaviour kept (its `Toolchain`/`Walker` in `rustc_compat.rs`
re-expressed over `Node`, the `format_failure` flag in `RunState<'a>`, `format_file`
returning `Option`, `sort_use_trees` over `UseTree<'_>`). On the merged tree:

| gate | result |
|---|---|
| `cargo fmt --all -- --check`, clippy `-D warnings` | exit 0 |
| `cargo test --workspace --all-features` | all pass (158 lib, 19 conformance batch, 10 + 10 + 4 integration incl. `rustfmt_version`, 7 `chloro`) |
| `bench --check` fixtures / registry | 0 of 1,238 / 0 of 20,022 changed |
| `conform -i` / `conform -V 1.8.0` | 1,219/1,220, 0 not idempotent, both |
| `config_matrix -n 1220` | per-option counts and listed files identical to the base's |
| `Ir`, sample | 390,689,701 (against 391,198,779 before the merge; the base moved too: E8's BKG) |

## Measurements

| metric | base `aa1e058` | port | change |
|---|---|---|---|
| `Ir`, sample, `benchmarks/profile.sh` (measured) | 518,953,599 | 391,198,779 | −127,754,820 (−24.62%) |
| `bench -n 5`, 1,238 fixtures (13.9 MB), best of 5, core 1, 3 interleaved pairs, load 0 (measured) | 2.607, 2.505, 2.593 s (5.33, 5.55, 5.36 MB/s) | 1.982, 1.977, 1.843 s (7.01, 7.03, 7.54 MB/s) | the ranges 2.505–2.607 s and 1.843–1.982 s do not overlap; −20.9% to −28.9% per pair |
| heap allocations, `alloc_count` over the 1,238 fixtures (measured) | 14,612,436 (1.051 per input byte), 5,906,451,686 bytes requested | 8,309,517 (0.598 per byte), 5,563,674,858 bytes | −6,302,919 (−43.1%) allocations, −5.8% bytes |
| red nodes created on the sample | 141,544 (E4's figure) | 0 by construction | |
| peak RSS, `fmt_stdin` on `lints.rs` (639,689 bytes), `ru_maxrss`, 3 runs (measured) | 10,028–10,144 KB | 9,604–9,680 KB | about −440 KB (−4.4%) |
| `chloro` binary, `--features cli`, release profile (measured) | 2,059,824 bytes, `.text` 1,737,746 | 1,923,184 bytes, `.text` 1,636,190 | −136,640 bytes (−6.6%); `.text` −5.8% |
| `bench` example binary (measured) | 1,855,080 bytes, `.text` 1,575,201 | 1,717,672 bytes, `.text` 1,472,861 | −7.4% |
| release build of `chloro-core` (lib only, fat LTO) after touching `src/lib.rs`, 2 runs (measured) | 14.37, 14.64 s | 14.83, 14.93 s | +0.3 to +0.5 s (the 7,865-line generated layer) |
| dependencies, `chloro-core` no-dev (measured) | 37 crates | 16 crates | −21 (`ra_ap_syntax`, `rowan` and their graph: `ra_ap_stdx`, `countme`, `hashbrown`, `memoffset`, `text-size`, `triomphe`, `smol_str`, `itertools`, `either`, `crossbeam-*`, `jod-thread`, `libc`, `rustc-hash`, ...), +1 (`typed-arena`) |
| dependencies, `chloro --features cli` no-dev | 69 | 48 | |

The `Ir` figure of the base matches the orchestrator's best-known-good (518,951,890 at
`6a12d86`) to 0.0003%. E7 estimated 146.5 M of the then 587 M (24.9%) with the red-node
term computed before E4 halved the red-node count; the port removes 127.8 M of the 519 M
after E4 (24.6%).

### Where the instructions went (callgrind, inclusive of libc; measured)

| function group | base | port |
|---|---|---|
| `malloc`, `_int_malloc`, `free`, `_int_free`, `malloc_consolidate`, `realloc`, `_int_realloc` | 87.9 M | 45.1 M |
| tree build: `TreeBuilder::finish_node` 11.95 M, `NodeCache::token` 11.75 M, `GreenNode::new` 6.55 M, `node_hash` 3.64 M (base); `cst::TreeBuilder::steps` closure 10.69 M (port) | 33.9 M | 10.7 M |
| red nodes: `NodeData::new` 5.24 M, `cursor::free` 3.07 M, `SyntaxNodeChildren::next` 3.89 M, `nodes::element_at` 9.29 M | 21.5 M | 0 |
| `rustc_compat::walk` | 8.34 M | 9.78 M |
| `span::rustc_span` | 3.87 M | 3.79 M |
| `comment::CharClasses::next`, `utils::unicode_str_width`, `ra_ap_parser::event::process`, `intersperse_trivia`, lexer | unchanged (27.7 M, 20.7 M, 15.2 M, 13.7 M, ...) | unchanged |

The remaining profile is the formatting logic and the parser: the four largest functions
of the port are `CharClasses::next` (7.1%), `unicode_str_width` (5.3%), `event::process`
(3.9%) and `_int_free` (3.8%).

## What surprised

- **Two hand-written accessors differed from `ra_ap_syntax`'s.** E7's `ext.rs` returned
  the first and second token tree for `MacroDef::args`/`body`; `ra_ap_syntax` returns
  `args` only when there are two and `body` as the last one, so `macro m { rules }` was
  left unformatted until `bench --check` reported 1 of 1,238 outputs changed
  (`parser/inline/ok/macro_def_curly.rs`). E7's workload never called these two. The
  substrate's `first_token`/`last_token` scanned the subtree in preorder; rowan descends
  the chain of first (last) children and answers `None` at a childless node. The fixture
  equality test found the difference on the first run; the formatter calls them only on
  token trees, whose first and last children are tokens, so the output was unaffected.
- **The invariance of `RewriteContext<'a>`.** Holding arena handles behind `RefCell` makes
  the context invariant in `'a`; the first attempt to tie node and context lifetimes in
  helper signatures (`rewrite_with_parens<'a>(context: &RewriteContext<'a>, items:
  OverflowableItem<'a>)`) did not compile from `Rewrite::rewrite`, whose node and context
  lifetimes are unrelated. Keeping them unrelated everywhere, and naming `'c: 'a` at the
  one place a fragment node joins a file node's structure, needed no change to the trait.
- **Test sources with syntax errors.** Three `nodes.rs` tests parse sources such as
  `super let d = 1;` that `SourceFile::parse(..).tree()` returned a tree for; `Tree::parse`
  returns `None` on them, hence `Tree::parse_with_errors`.
- **`rustc_compat::walk` costs 1.4 M more** (9.78 M against 8.34 M): it now visits
  every element through the 16-byte array instead of the green children slice, and its
  `path` holds 16-byte handles. Not examined further.
- **The compiler drove the sweep in four rounds:** 463 errors after the import renames,
  148 after the struct lifetimes, 90, 27, then 4; 2,120 changed lines in the formatter.

## Follow-up

- `visit_assoc_item` can take the trait/impl kind from its two callers instead of two
  backward `parent()` searches per associated item.
- `rustc_compat::walk` reads `Elem` by value per child; a `&[Elem]` slice walk may win
  back the 1.4 M.
- The `Tree` reserves `2 × tokens + 8` elements; an exact count from the parser output
  would remove the 14% spare capacity EXP-007 §3c measured.
- `benchmarks/compare_rustfmt.sh` (process-level, against rustfmt) is not run here; the
  README's process-level figures are from before this change and are left as they are.

## Current State

- The formatter reads `crate::cst` only; `rowan` and `ra_ap_syntax` are dev-dependencies
- All gates pass at `353a18e`: both goldens, conform, config_matrix, tests, fmt, clippy
- `Ir` on the sample: 518,953,599 → 391,198,779 (−24.62%); `bench -n 5` ahead in 3 of 3
  pairs with non-overlapping ranges; allocations on the fixtures −43%

## Missing

- `benchmarks/compare_rustfmt.sh` process-level numbers against rustfmt on this build
- A measurement of the block cache on the flat tree alone (E4 measured −0.21% on rowan;
  here it is kept for its semantics, not re-measured in isolation)
- A parent column measured against the two `parent()` sites (decision 4 left it out)
