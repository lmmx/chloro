# EXP-007: a flat syntax array under the formatter (feasibility)

Ledger row E7 in [../experiments.md](../experiments.md). Labels as in
[../profile-baseline.md](../profile-baseline.md): **measured** (a tool's output on the
builds below), **read** (from source), **derived** (arithmetic on measured numbers),
**hypothesis** (inferred, not measured).

## Identity

| field | value |
|---|---|
| branch | `20261014-flat-cst`, from `4cd6336` (head of `claude/chloro-perf-research-swarm-nrscj8`; formatter code of BKG `884f9c9`) |
| formatter code changed | none; `format_source` does not call anything added here |
| additions | `chloro-core/examples/flat_cst/` (`cst.rs` substrate, `ast.rs` generated typed layer, `gen_ast.py` generator, `ext.rs` hand-written accessors, `workload.rs`, `main.rs` harness); this file |
| toolchain | `rustc 1.97.0`, `cargo 1.97.0`, valgrind 3.22.0, Python 3 |
| host | 4 vCPU Xeon @ 2.80GHz, the host of the baseline; sibling experiments ran on other cores during the wall-clock runs (labelled) |
| build | `target/release` default profile (opt-level 3, 16 codegen units, no LTO) |
| sample | the baseline's 49 files (`find … \| LC_ALL=C sort \| awk 'NR%25==0'`, 650,213 bytes); the harness keeps the 42 that lex and parse without error (649,841 bytes; the 7 dropped files are error fixtures of 5–239 bytes) |
| callgrind command | `setarch $(uname -m) -R taskset -c 1 valgrind --tool=callgrind target/release/examples/flat_cst MODE SAMPLE` |
| wall-clock command | `taskset -c 1 target/release/examples/flat_cst MODE -n 5` (all fixtures) |

## Question and falsification

**Question.** Can a flat preorder array with a generated typed layer that keeps
`ra_ap_syntax`'s `ast` names replace rowan under the formatter, and what is the gain bounded
to? Nothing in the formatter is ported.

**Refuted if** any of: the typed layer cannot be generated mechanically from `rust.ungram`
with the same names; the flat tree built from the same parser steps differs from the rowan
tree in kinds, ranges or token texts on any fixture; the query workload over the flat typed
layer costs as much as over red nodes; or the audit finds a rowan dependency with no flat
equivalent.

## 1. API surface audit (measured: type-resolved)

Method: copies of `rowan 0.15.15` and `ra_ap_syntax 0.0.307` with `#[deprecated]` on every
public method, trait method, struct and enum (1,044 attributes), patched into a `cargo check
-p chloro-core --lib` through `--config patch.crates-io.*`; rustc's deprecation warnings give
one record per use site with the resolved path. 2,140 sites in `chloro-core/src/`, deduplicated
on (file, line, column, path). Raw tables, per file and per item, are in the scratch audit
(`sites.tsv`, `audit.md`); the totals follow. Test modules (`#[cfg(test)]`) are not counted.

### 1a. rowan methods (table A): 38 distinct, 247 sites

| rowan item | sites | files (sites) |
|---|---|---|
| `SyntaxToken::kind` | 32 | attr 11, macros 8, nodes 6, macro_args 2, span 2, closures/expr/types 1 |
| `GreenNodeData::children` | 21 | rustc_compat 10, nodes 8, types 2, span 1 |
| `SyntaxNode::children_with_tokens` | 20 | nodes 5, items 3, types 3, macro_args 2, macros 2, attr/closures/expr/patterns/visitor 1 |
| `GreenNodeData::kind` | 17 | rustc_compat 10, nodes 4, macro_args 2, types 1 |
| `SyntaxNode::green` | 16 | nodes 7, macro_args 5, context 1, rustc_compat 1, span 1, types 1 |
| `Language::kind_from_raw` | 12 | nodes 9, rustc_compat/span/types 1 |
| `SyntaxNode::text_range` | 12 | macro_args 5, items 2, nodes 2, context/patterns/span 1 |
| `GreenTokenData::kind` | 11 | nodes 6, rustc_compat 4, span 1 |
| `SyntaxToken::text` | 11 | expr 2, items 2, patterns 2, attr/macros/nodes/pairs/types 1 |
| `NodeOrToken::into_token` | 10 | attr 3, nodes 2, closures/expr/macros/patterns/types 1 |
| `GreenTokenData::text` | 9 | nodes 5, rustc_compat 2, span 1, types 1 |
| `SyntaxToken::text_range` | 8 | macro_args 4, macros 2, patterns 1, span 1 |
| `SyntaxElement::kind` | 7 | items 2, types 2, attr/macros/visitor 1 |
| `SyntaxNode::text` | 7 | items 3, printer 3, matches 1 |
| `NodeOrToken::as_token` | 6 | attr 2, macros 2, items 1, types 1 |
| `SyntaxNode::children` | 6 | types 2, items/macro_args/nodes/patterns 1 |
| `SyntaxNode::kind` | 5 | macro_args 2, attr/nodes/visitor 1 |
| `GreenTokenData::text_len` | 3 | nodes 2, span 1 |
| `Language::kind_to_raw` | 3 | formatting 2, context 1 |
| `NodeOrToken::as_node` | 3 | macros 2, rustc_compat 1 |
| `SyntaxNode::first_token` | 3 | macros 2, macro_args 1 |
| `SyntaxNode::new_root` | 3 | macro_args 2, formatting 1 |
| `SyntaxNode::parent` | 3 | visitor 2, nodes 1 |
| `GreenNode::new` | 2 | macro_args |
| `GreenNodeData::text_len` | 2 | nodes |
| `NodeOrToken::into_node` | 2 | macro_args, nodes |
| `SyntaxNode::descendants_with_tokens` | 2 | macros, nodes |
| `GreenNodeBuilder::{start_node, token, finish_node, finish}` | 4 | formatting |
| `GreenToken::new` | 1 | context |
| `SyntaxElement::text_range` | 1 | macro_args |
| `SyntaxNode::child_or_token_at_range` | 1 | nodes |
| `SyntaxNode::last_child` | 1 | macro_args |
| `SyntaxNode::last_token` | 1 | macro_args |
| `SyntaxText::len` | 1 | matches |
| `SyntaxToken::green` | 1 | macro_args |

Per file: nodes 65, macro_args 33, rustc_compat 29, macros 22, attr 20, types 17, items 14,
span 10, formatting 7, patterns 7, context 4, expr 5, visitor 5, closures 3, printer 3,
matches 2, pairs 1. Not used anywhere: `next_sibling`, `prev_sibling`, `siblings*`,
`ancestors`, `preorder*`, `token_at_offset`, `covering_element`, `clone_for_update`,
`detach`, `splice_children`, `descendants` (nodes only).

### 1b. `ast` accessors (tables B and C)

| class | distinct | sites | notes |
|---|---|---|---|
| generated accessors (`ast/generated/nodes.rs`) | 126 on 94 types | 234 | top: `PrefixExpr::expr` 11, `TryExpr::expr` 11, `CastExpr::expr` 9, `RefExpr::expr` 9, `TypeBoundList::bounds` 8, `MacroCall::token_tree` 6; per file: types 48, expr 45, items 41, patterns 23, closures 11, matches 10 |
| `AstNode`/`AstToken` trait methods | 6 | 404 | `AstNode::syntax` 378 (items 133, types 56, expr 33, nodes 26, patterns 21, visitor 21), `AstNode::cast` 19, `AstToken::cast` 3, `AstToken::text` 2, `AstNode::can_cast` 1, `AstToken::syntax` 1 |
| `Has*`, `RangeItem`, `IsString` trait methods | 16 | 67 | `HasName::name` 27, `HasGenericParams::generic_param_list` 15, `HasGenericArgs::generic_arg_list` 4, `HasLoopBody::loop_body` 3, `HasTypeBounds::type_bound_list` 3, `RangeItem::start` 3, … |
| hand-written inherent accessors (`expr_ext.rs`, `node_ext.rs`, `token_ext.rs`) | 29 | 61 | `Fn::body` 6, `BinExpr::lhs` 5, `BinExpr::rhs` 5, `ClosureExpr::body` 4, `IndexExpr::base` 4, `BinExpr::op_kind` 3, `Comment::kind` 3, `IndexExpr::index` 3, `Literal::kind` 3, `ArgList::args` 2, `IfExpr::{condition, then_branch, else_branch}` 2 each, 17 items with 1 site (`Abi::abi_string`, `BinExpr::op_token`, `Comment::is_inner`, `Comment::is_outer`, `Const::body`, `ForExpr::iterable`, `Impl::self_ty`, `Impl::trait_`, `IsString::is_raw`, `Literal::token`, `MacroDef::args`, `MacroDef::body`, `MatchGuard::condition`, `NameRef::text`, `Path::segments`, `PrefixExpr::op_token`, `Static::body`, `WhileExpr::condition`, `HasDocComments::doc_comments`) |
| `ast::` type and variant mentions (table D) | 84 types | 1,127 (871 lines) | `Expr` 90 direct + 359 variant mentions; `Type` 40 + 43; `Visibility` 32; `Pat` 31 + 73; `Item` 26 + 75. Non-generated types among the 84: `Comment` (token), `CommentKind`, `ElseBranch`, `LiteralKind` |

Per file, all classes: items 371 (3,627 lines), expr 367, types 285, patterns 151, nodes 140,
visitor 112, matches 80, macros 73, chains 71, utils 69, macro_args 59, pairs 46, reorder 44,
attr 36, imports 34, rustc_compat 31, span 21, overflow 18, formatting 12, printer 11, stmt
10, context 4; zero sites: comment, config, lists, missed_spans, shape, sort, formatter.rs.

30 struct and enum definitions hold ast or rowan values in 63 fields or variants (table F2;
e.g. `nodes::Block`, `nodes::StmtKind`, `macros::MacroArg`, `overflow::OverflowableItem`,
`chains::ChainItemKind`, `items::FnSig`, `context::MemoEntry`, `attr::MetaParser`,
`macro_args::MacroTokens`); 3 already borrow (`&'a [ast::TypeBound]`).

### 1c. Classification (read)

| class | sites | what it is on the flat substrate |
|---|---|---|
| **mechanical rename** | 1,962 | generated accessors (234: all 126 names exist in `ast.rs`, §2); `AstNode`/`Has*`/`RangeItem` trait methods (471: same names; `syntax()` returns a `Copy` handle instead of `&SyntaxNode`, so `.clone()` and `&` adjust); `SyntaxNode`/`SyntaxToken`/`NodeOrToken`/`SyntaxElement` methods that `cst.rs` provides under the same names (130: `kind`, `text_range`, `children`, `children_with_tokens`, `first_token`, `last_token`, `last_child`, `descendants_with_tokens`, `text` on tokens, `into_*`/`as_*`); type mentions (1,127: `ast::X` becomes `ast::X<'t>` where a lifetime is required, elided elsewhere) |
| **equivalent operation on the flat array** | 164 | the green fast paths (92: `green()` 16, `GreenNodeData::{children, kind, text_len}` 40, `GreenTokenData::{kind, text, text_len}` 23, `kind_from_raw` 12, `child_or_token_at_range` 1) exist only to avoid red nodes and become plain `children_with_tokens()` loops (`nodes.rs` `has_token`, `child`, `node_text`, `non_trivia_text`, `has_leading_attrs`, `may_have_inner_attrs`, `span_without_attrs`; `span.rs` `rustc_span`; `rustc_compat.rs`'s whole green walk; `types.rs` 4; `macro_args.rs` 7); `SyntaxNode::text` 7 and `SyntaxText::len` 1 become source slices; `parent()` 3 (§3d); the 29 hand-written accessors, 61 sites (`ext.rs` ports 25 of them; the 4 token-level ones, `Comment::{kind, is_inner, is_outer}`, `Literal::kind`, `IsString::is_raw`, `doc_comments`, read token text) |
| **design problem** | 14 sites in 3 files, plus lifetimes | `macro_args.rs`: `splice` (copies the token tree into a new root padded with whitespace tokens, `GreenNode::new` 2, `new_root`, `last_child`, `SyntaxToken::green`), `build`/`parse_maybe_bound` (`TreeBuilder`, synthetic `DYN_TRAIT_TYPE` node); `context.rs`: the memo key from the green pointer (`green()`, `text_range`), `MemoEntry::_node`, `RunState::padding` (`GreenToken::new`, `kind_to_raw`); `formatting.rs`: `TreeBuilder` over `GreenNodeBuilder` (7). Across files: handles that borrow the tree give 30 types a lifetime parameter and need fragment trees to outlive the `MacroArgsCache` and memo entries that hold their handles (§4) |

## 2. Prototype

### 2a. Substrate (`cst.rs`, 672 lines, no `unsafe`)

- `Elem` (16 bytes): `kind: SyntaxKind` (2), `is_node: bool`, `end: u32` (index after the
  subtree; `index + 1` for a token), `start: u32`, `len: u32`. `Tree<'s>` owns
  `Vec<Elem>`, an optional parent column `Vec<u32>`, and borrows the source `&'s str`.
- Built by `Tree::from_steps(src, &lexed, &output, with_parents)` from the same
  `LexedStr` + `TopEntryPoint::SourceFile.parse` + `intersperse_trivia` steps that
  `formatting::parse` feeds to `GreenNodeBuilder`; token offsets are the running total of
  token lengths (every token's text is checked against the source slice at that offset in
  `check` mode, split floats included). Lexer errors and `StrStep::Error` return the same
  "no tree" answer as `formatting::parse`.
- `Node<'t>` and `Token<'t>` are `Copy` (`&'t Tree`, `u32`); identity is `(tree address,
  index)`. Methods: `kind`, `text_range`, `text` (a source slice, no copy), `parent`
  (column or backward search), `ancestors`, `children`, `children_with_tokens`,
  `first_child`, `last_child`, `first_child_or_token`, `last_child_or_token`, `first_token`
  (first token-element of the subtree in index order), `last_token`, `descendants`,
  `descendants_with_tokens`, `index`; `Token`: `kind`, `text_range`, `text`, `parent`,
  `next_token`, `prev_token`. `NodeOrToken<N, T>` with `into_node`/`into_token`/`as_node`/
  `as_token`, and `kind`/`text_range` on elements.
- `support::{child, children, token}`, `AstNode<'t>` (`can_cast`, `cast`, `syntax`),
  `AstToken<'t>`, `AstChildren<'t, N>`: the shape of `ra_ap_syntax::ast`'s own `support`
  module and traits, with a lifetime.
- Fragments: `Tree::from_steps_at(src, &lexed, &output, base, with_parents)` lays out a
  re-parsed slice of the file at its file offset (§4).

### 2b. Generated typed layer (`gen_ast.py` 567 lines → `ast.rs` 7,859 lines)

`gen_ast.py RUST_UNGRAM ORACLE_NODES_RS OUT` parses `rust.ungram` (169 rules) and lowers it
with the rules of rust-analyzer's `xtask codegen grammar` (`lower_rule`,
`lower_separated_list`, the manual-label list, `extract_enums`, `extract_struct_traits`,
`extract_enum_traits`), then emits the same names over `Node<'t>`:

| generated | count |
|---|---|
| node structs (`Copy`, `Debug`, `Eq`, `Hash`) | 154 (+9 `AnyHas*`) |
| enums with `From` impls and `AstNode` (nested enums delegate: `Stmt::Item`) | 15 |
| accessors: `child` → `Option<X<'t>>`, `children` → `AstChildren<'t, X<'t>>`, `token` → `Option<Token<'t>>` | 501 (161, 29, 311) |
| `Has*` trait impls on nodes / enums | 153 / 15 |
| traits (`HasName`, `HasVisibility`, `HasLoopBody`, `HasArgList`, `HasModuleItem`, `HasGenericParams`, `HasGenericArgs`, `HasTypeBounds`, `HasAttrs`, `HasDocComments`) | 10 |

The generator reads `ra_ap_syntax`'s own `ast/generated/nodes.rs` for the token-name table
(`T!['(']` → `l_paren_token`) and compares its result with it: the set of (type, accessor,
kind, target), the enum variant lists, the trait impls and the `AnyHas*` kind lists **equal
the oracle's** (measured: 0 differences). Two of ra_ap_syntax's hand-written exceptions are
modelled: `Stmt`'s `';'` alternative is dropped, and `ForExpr`/`WhileExpr` implement
`HasLoopBody` by hand (`ext.rs`). `ext.rs` (357 lines) ports 25 of the 29 hand-written
accessors the formatter uses (§1b), with ra_ap_syntax's child-selection rules
(`IfExpr::condition` skips a leading block when the then-branch is missing, `RangeItem`
splits at the `..` token, `Impl::{self_ty, trait_}` by position, …).

### 2c. Compile time and code size (measured)

| item | value |
|---|---|
| `ast.rs` | 7,859 lines after rustfmt (5,032 as emitted); 164 structs, 15 enums, 10 traits, 511 `pub fn`, 633 impls |
| `cargo build --release --example flat_cst`, touched `main.rs`, best of 3 wall | 2.891 s wall, 4.79 s user (runs: 5.94, 3.26, 2.89 s wall) |
| `cargo build --release --example parse_floor` (a 200-line example, same dependencies), best of 3 | 1.010 s wall, 1.92 s user (runs: 1.05, 1.09, 1.01 s); the typed layer, `ext.rs`, the two workload instantiations and the harness add about 1.9 s wall and 2.9 s user per release build of the example |
| `cargo check --example flat_cst`, best of 2 | 0.896 s wall, against 0.164 s for `parse_floor` |
| `.text` of `flat_cst` / `parse_floor` | 939,481 B / 863,279 B (+76 KB for the typed layer, `ext.rs`, the two workload instantiations and the harness; accessors are generic over nothing, so only the ones called are compiled to code) |

### 2d. Equivalence (`check` mode, measured)

On all 1,042 fixture files that parse (13.88 MB): for every element in preorder, the flat
tree and the rowan tree agree on kind, `TextRange` and node/token class; every token's
`text()` equals the source slice at its range; the parent column equals rowan's `parent()`
and the backward search equals the column; the two workload instantiations (§3b) produce the
same checksum over 3,675,456 accessor results and 274,205 statement-list elements. Zero
differences.

## 3. Measurements

### 3a. Build cost (callgrind, sample, 42 files)

The harness parses every file once while loading (to select the files that parse), so mode
totals share that cost; the figures below are differences between modes.

| quantity | Ir | derived from |
|---|---|---|
| `events` mode total (lex + parse, no tree, plus loading) | 173,711,250 | measured |
| rowan tree: build, `new_root`, drop (as `formatting::parse` + end of file) | 90,083,389 | `build-rowan` 263,794,639 − `events` |
| flat tree: build and drop | 27,823,725 | `build-flat` 201,534,975 − `events` |
| flat tree with parent column | 30,249,685 | `build-flat -p` 203,960,935 − `events` |
| parent column alone | 2,425,960 | difference of the two |
| ratio rowan / flat | 3.24× (2.98× with the column) | |

Cross-check with the formatter's own profile (baseline §1): file parse inclusive 156.82 M
minus lex, `to_input` and parse (76.44 M) is 80.38 M for `intersperse_trivia` plus the tree,
plus 9.93 M freeing the tree at the end of the file: 90.3 M, the figure measured here. The
earlier `parse_floor` (two arrays, parent and sibling links, no `end` column) costs 31.1 M
over events on the same sample (measured in this session: 107,552,373 − 76,409,710).

Memory allocation calls (`malloc` from the callgrind call graph): `build-rowan` 53,716 for 42
files; `build-flat` 3 per file (two `Vec`s and the open-node stack).

### 3b. Query workload (callgrind, sample)

`workload.rs` is one source file included in two modules; it compiles against
`ra_ap_syntax::ast` over `SyntaxNode` and against `ast.rs` over `Node<'t>` without a change.
Per file it walks every node in preorder and, for every expression, reads `kind`,
`text_range` and the typed children `format_expr_uncached` reads for that kind (`lhs`/`rhs`/
`op_token`, `expr`/`arg_list`/`args`, `receiver`/`name_ref`/`generic_arg_list`, `condition`/
`then_branch`/`else_branch`, match arms with `pat`/`guard`/`expr`, closure params and body,
paths, literals' tokens, macro call token-tree delimiters, …); for every block it runs
`nodes::stmts_of`'s `children_with_tokens` loop with `rustc_span` and `has_token` per
statement; for every path it walks `qualifier`/`segment`, `name_ref` text,
`generic_arg_list` and its args as `types::rewrite_path_segments` does. Each answer is
folded into a checksum. The red instantiation reads `rustc_span` and `has_token` from the
green tree, as `span.rs` and `nodes.rs` do today.

| quantity | red (rowan + `ra_ap_syntax::ast`) | flat (`cst.rs` + `ast.rs`) | ratio |
|---|---|---|---|
| workload Ir beyond the build (`query-*` − `build-*`) | 119,050,879 | 15,534,014 | 7.66× |
| `run` inclusive Ir (callgrind) | 118,933,744 | 15,532,663 | 7.66× |
| with the parent column | — | 15,534,136 | |
| accessor results (same for both) | 117,496 | 117,496 | |
| Ir per accessor result (derived) | 1,012 | 132 | |
| red `NodeData::new` calls (measured) | 278,850 | 0 | |
| Ir per red node created / per red node avoided (derived) | 426.5 | 55.7 | |
| `malloc` calls beyond the build | 290,374 | about 16,800 (the `Vec` of segments per path) | |

The workload creates 278,850 red nodes on the sample; the formatter creates 284,789 on the
same files (baseline §2). The red-side cost per red node, 426 Ir, is above the formatter's
316 because the workload also hashes every answer and reads `text_range` for it.

### 3c. Memory (counting allocator, `lints.rs`, 639,689 bytes of source, measured)

| tree | elements | retained bytes | per element | per source byte | peak during build |
|---|---|---|---|---|---|
| rowan green tree (interned by `NodeCache`) | 120,391 | 2,049,936 | 17.0 | 3.20 | 2,222,192 |
| flat | 120,391 | 2,192,800 | 18.2 | 3.43 | 2,193,056 |
| flat with parent column | 120,391 | 2,741,000 | 22.8 | 4.28 | 2,741,256 |
| parser state while building (`LexedStr`, `Input`, `Output`), both | | 1,835,008 live, 8,469,076 peak | | | |
| rowan red handles, all 53,506 nodes held at once | 53,506 | 3,948,608 | 73.8 | | |

The flat array is 16 bytes per element plus the 14% spare capacity of its `with_capacity(2 ×
tokens)` reservation (120,391 × 16 = 1,926,256 bytes; `shrink_to_fit` or an exact count
removes the rest). rowan's green tree is smaller on this file because `NodeCache` interns
equal tokens and subtrees, and `lints.rs` is a table of near-identical entries. Both keep the
source text (639,689 bytes) alive; the formatter holds it anyway.

### 3d. `parent()` (callgrind, sample)

| quantity | Ir | derived from |
|---|---|---|
| parent column at build | 2,425,960 | §3a |
| `parent()` of every node from the column, plus the node iteration | 5,235,732 | `parent-column` 209,196,667 − `build-flat -p` |
| `parent()` of every node by backward search | 56,035,609 | `parent-search` 257,570,584 − `build-flat` |

The search costs the size of the preceding siblings' subtrees (the last item of a file scans
the whole file). The formatter calls `parent()` at 3 sites (read): `nodes.rs:445`
(`MacroCall` → `MACRO_EXPR`, which starts one element before the call: a search of one
step) and `visitor.rs:501-502` (`AssocItem` → `ASSOC_ITEM_LIST` → `TRAIT`/`IMPL`: a search
over the preceding items of the impl, or the kind passed down by the caller, which knows
whether it is visiting a trait). The column is not required by these sites; it costs 2.4 M
Ir and 4 bytes per element when present.

### 3e. Wall clock (all 1,042 parsing fixtures, 13.88 MB, best of 5, core 1; load from sibling experiments 2–4, labelled noisy)

| mode | s | beyond `events` (0.314 s) |
|---|---|---|
| `build-rowan` | 0.751 | 0.437 |
| `build-flat` | 0.342 | 0.028 |
| `build-flat -p` | 0.346 | 0.032 |
| `query-red` | 1.243 | 0.492 beyond `build-rowan` |
| `query-flat` | 0.486 | 0.144 beyond `build-flat` |

Build: 15.6× less time beyond the parser floor; workload: 3.4× (against 7.66× in Ir: the flat
workload's instructions are cheaper to count than to run, hypothesis: the red workload's time
is dominated by allocator and pointer-chasing work with a lower IPC, the flat one by indexed
loads with a higher IPC). The formatter takes 3.04 s on these fixtures (BKG `bench -n 5`).

### 3f. The memo key (read)

`RewriteContext::memoize` keys an entry by `(green node address, offset, tag, shape, flags)`
and stores the `SyntaxNode` in the entry so the green node stays allocated and its address
is not reused. On the flat array a node's identity is its index: the key becomes `(tree id,
index, tag, shape, flags)` (two `u32`s instead of `usize + u32`), the entry needs no node,
and the per-entry red-node drop in `clear_memo` (3.63 M on the sample, baseline) disappears.
A tree id is needed because fragment trees (§4) are separate trees with their own index
space: a per-run counter in `RunState`, assigned when a tree is built. rowan's interning does
not change identity (equal subtrees at different offsets have different keys today and
different indices tomorrow).

## 4. Macro fragments

### 4a. What the formatter relies on today (read: `macro_args.rs`)

1. `MacroTokens::build` parses the argument text (a slice of the file) with
   `TopEntryPoint::{Expr, Type, Pattern, MacroItems}` into a detached green node; for items
   it unwraps `MACRO_ITEMS`; it runs `subtree_rejected_by_rustc` on the detached root.
2. `splice` copies the token tree's children into a new `GreenNode`, replacing each argument's
   token run by its fragment node (a tree element straddling a fragment boundary aborts),
   then builds a root `[padding whitespace tokens…, copied token tree]` where the padding is
   `RunState::padding(offset)`: whitespace tokens of 2^k spaces summing to the token tree's
   start offset, shared across calls. `SyntaxNode::new_root(root).last_child()` is the
   token-tree copy, whose `text_range()` equals the original's (`debug_assert`), so every
   fragment node and its descendants have their file offsets, which spans, `snippet()` and
   comment recovery read.
3. `parse_maybe_bound` builds a synthetic `DYN_TRAIT_TYPE > TYPE_BOUND_LIST > TYPE_BOUND`
   around `?` + whitespace gap + the parsed path type, with `TreeBuilder`.
4. The fragment nodes' `parent()` is the token-tree copy, never the file; `Stmt::attrs` on a
   `MacroCall` statement inside a fragment takes the `outer_attributes(m.syntax())` branch
   when the parent is not `MACRO_EXPR` (the same branch the flat design takes, see below).
5. `ParsedMacroArgs` (holding `MacroArg::Expr(ast::Expr)` …) lives in `RunState::macro_args`
   for the run, keyed by token-tree range; memo entries may also hold fragment nodes. The
   fragment roots stay allocated through these handles.

### 4b. Flat design (prototyped: `Tree::from_steps_at`, `fragments` mode)

- A fragment is a `Tree` of its own, built from the fragment's own `LexedStr` and `Output`
  with `base` = the argument's file offset; every element's `start` is a file offset and
  `src` is the whole file, so `text_range()`, `text()`, `rustc_span` and `snippet()` answer
  exactly as for the file's tree. No padding tokens, no copy of the token tree, no
  `GreenNode::new`: the splice step reduces to parsing each argument at its offset.
  Measured on all fixtures: 10,142 whole-interior expression fragments (186,532 elements)
  built at their offsets; every element lies inside its token tree and every token's text is
  the file's text at its range.
- `parent()` of a fragment root is `None` (rowan: the token-tree copy). Of the 3 `parent()`
  sites, `nodes.rs:445` already handles a non-`MACRO_EXPR` parent, and `visitor.rs:501` is
  reached through items whose ancestors lie inside the fragment tree.
- `parse_maybe_bound`: the synthetic node is three `Enter`s, the `?` token (one byte at
  `toks[start].lo`), the gap as a whitespace token, then the parsed type's steps at their
  offset, then three `Exit`s: a 20-line `FragmentBuilder` over `Vec<Elem>`, every text a
  source slice. `MacroItems` unwrapping is `root.children().next()` with the same
  single-child check. `subtree_rejected_by_rustc` runs on the fragment tree's root.
- Ownership (the real design problem): handles borrow the tree (`Node<'t>`), and fragment
  trees are created during formatting while `RewriteContext<'a>` is borrowed, then held by
  `MacroArgsCache` entries and memo entries. Options, each with its cost: (i) an append-only
  arena with stable addresses owned by `format_text`'s frame and passed as `&'a Arena` into
  the context, giving every tree the run's lifetime `'a` (the lifetime `RewriteContext<'a>`
  already carries for the source text) — no per-handle cost, one small crate (`typed-arena`)
  or 30 lines with `UnsafeCell`; (ii) `Rc<Tree>` in the handle — no lifetime parameter, 16
  bytes per handle, a reference-count increment per handle copy (handles stop being `Copy`);
  (iii) a `u32` tree id with a thread-local registry — `Copy`, no lifetime, one indexed
  lookup per access. The prototype uses the borrowed form for the file tree; the measured
  numbers (§3) are for that form.
- Fragments not covered by the prototype's check: comma-split arguments, type/pattern/item
  fragments, `parse_maybe_bound`, `lazy_static!`; their offsets follow the same rule.

## 5. Formatter-level estimate (derived)

Share of the sample's 587,211,777 Ir (baseline). The flat cost per red node avoided is taken
from the workload (§3b: 15,532,663 / 278,850 = 55.7 Ir), which includes the generated layer's
own code; the assumption is that the formatter's typed queries cost the same on the flat
layer as the workload's, which imitates them.

| term | arithmetic | Ir | % |
|---|---|---|---|
| (a) red nodes avoided, net of flat access cost | 284,789 × (316 − 55.7) | 74.1 M | 12.6 |
| (b) tree build and free | 90,083,389 − 27,823,725 (− 30,249,685 with the parent column) | 62.3 M (59.8 M) | 10.6 (10.2) |
| (c) `ra_ap_syntax` accessor and `kind_from_raw` code replaced (its replacement is inside (a)'s 55.7) | baseline §1 row | 10.1 M | 1.7 |
| **(a)+(b)+(c)** | | **146.5 M (144.1 M)** | **24.9 (24.5)** |
| (d) `node_text` copies become slices (lever 4a, separate) | baseline | 6.1 M | 1.0 |

Not in the estimate: fragment trees (today `parse_macro_args` is 30.65 M inclusive, lexing
and parsing included; the build-and-splice share would shrink by the §3a ratio, not
measured); the ownership scheme's cost (zero for the arena, a reference-count increment per
stored handle for `Rc`); the green-read helpers (5.16 M, assumed cost-neutral as
`children_with_tokens` loops); allocator savings beyond the red nodes and the green tree
(53,716 `malloc` calls per 42 files in the build become 126). The 8.09% LTO gain (baseline
§6) overlaps with (c): 6.7 M of it is `ra_ap_syntax` accessors inlined.

## 6. Port effort (read, from the audit)

| item | count |
|---|---|
| formatter modules touched | 23 of 29 (plus `formatting.rs`); untouched: comment, config, lists, missed_spans, shape, sort |
| call sites | 2,140: mechanical 1,962 (§1c), equivalent operation 164, design 14 |
| lines with `ast::` mentions to annotate or elide a lifetime | 871 |
| types gaining a lifetime parameter | 30 (63 fields/variants), plus `RewriteContext`, `FmtVisitor`, `RunState` (memo, macro-args cache) |
| hand-written accessors to port | 29 (25 done in `ext.rs`, 4 token-level) |
| files with a design problem | 3: `macro_args.rs` (726 lines; `splice`, `build`, `parse_maybe_bound` ≈ 150 lines), `context.rs` (memo key, entry, padding ≈ 40 lines), `formatting.rs` (`parse`, `TreeBuilder` ≈ 40 lines) |
| test modules | not audited (`--lib`); they build trees with `SourceFile::parse(..).tree()` and need a flat equivalent |
| generated code to maintain | `gen_ast.py` regenerates `ast.rs` from `rust.ungram` on a `ra_ap_syntax` upgrade; the oracle comparison reports any lowering drift |
| dependency change | `rowan` leaves `chloro-core`; `ra_ap_syntax` stays for `SyntaxKind`/`T!` (both are `ra_ap_parser`'s) or is replaced by `ra_ap_parser` + `text-size` directly |

## 7. Risks to conformance and how the gates catch them

| risk | mechanism | gate that catches it |
|---|---|---|
| node or token ranges differ from rowan's (offsets counted from token lengths) | `rustc_span`, `snippet`, comment recovery read wrong text | `check` mode: 0 differences on 1,042 files; then `bench --check` golden (1,238 hashes) and `conform -i` (1,220) |
| split float tokens (`x.0.1`): the `.` token is emitted as a static `"."`, not a source pointer | a token's `text()` must still be the source slice | `check` asserts `text() == src[range]` for every token: holds (the `.` is in the source at that offset) |
| trivia attachment (which comments belong to which node) | same `intersperse_trivia` steps feed both builders; the flat tree cannot differ | `check` preorder equality |
| fragment offsets (padding replaced by `base`) | spans inside macro arguments | `fragments` mode (10,142 fragments) for whole interiors; comma-split and typed fragments untested until ported; `conform -i` covers 816 macro calls on the sample alone |
| `parent()` of a fragment root is `None` instead of the token-tree copy | `Stmt::attrs` branch | read: same branch taken; `conform` |
| memo identity across trees | a fragment node and a file node with the same index must not collide | tree id in the key; `conform -i` idempotence and golden hashes |
| `rustc_compat` rules over `children_with_tokens` instead of green children | a rejected file formatted or vice versa | its unit tests; `config_matrix` and the registry corpus (12 rejected files there) |
| output changes from any of the above | | `bench --check` 0 of 1,238 changed; `conform -i` 1,219/1,220, 0 not idempotent; `config_matrix -n 1220`; registry corpus |

## 8. Gates on this branch (measured, formatter unchanged)

| gate | result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 (the example is one of the targets) |
| `cargo test --workspace --all-features` | exit 0; every suite passes (146 tests, 0 failed) |
| `bench --check golden/fixtures-4cd6336.tsv` (recorded from the unmodified build at `4cd6336`) | 0 of 1,238 outputs changed |
| `conform -i` | 1,219/1,220 identical, 0 not idempotent (BKG: 1,219/1,220, 0) |

## 9. Decision (facts)

- A typed layer with `ra_ap_syntax`'s names is generated mechanically: 501 accessors, 154
  structs, 15 enums, 168 trait impls, equal to ra_ap_syntax's own generated set; the
  formatter uses 126 of the accessors and 29 hand-written ones (25 ported here).
- The flat tree equals the rowan tree on every fixture, and the same query source runs on
  both; the flat side costs 7.66× fewer instructions on the formatter-shaped workload and
  3.24× fewer to build and free.
- The port is bounded to gain about 146 M of 587 M instructions (24.9%; 24.5% with a parent
  column; 26% with `node_text` as a slice) on the sample, under the stated assumption, and
  0.44 s of 3.04 s of wall time from the build alone on the fixtures (the workload's time
  ratio, 3.4×, is below its instruction ratio).
- The port touches 2,140 sites in 23 modules, 1,962 of them renames or lifetime
  annotations, 164 rewrites of green fast paths into plain child loops, and 14 sites in 3
  files with design content: macro fragments (parse at the file offset, no splice; 150
  lines), the memo key (tree id + index) and the tree builder. The open design decision is
  tree ownership for fragment trees: an arena with the run's lifetime (no per-handle cost),
  `Rc` handles (not `Copy`), or a tree-id registry.
- The flat tree is not smaller than rowan's interned green tree on `lints.rs` (18.2 against
  17.0 bytes per element; 22.8 with the parent column); it removes the 64-byte red node per
  access and all but 3 allocations per file from the build.

## Current State

- `format_source` and every formatter module are unchanged; the prototype is an example
  (`chloro-core/examples/flat_cst/`), not compiled into the library
- `gen_ast.py` reproduces ra_ap_syntax's generated `ast` set over the flat tree with 0
  differences (chloro-core/examples/flat_cst/gen_ast.py)
- `flat_cst check` reports 0 differences between the two trees and the two workloads on
  1,042 fixtures; `flat_cst fragments` verifies 10,142 fragment trees
- Measured on the sample: build 90.1 M → 27.8 M Ir; workload 118.9 M → 15.5 M Ir; 278,850 red
  nodes → 0

## Missing

- No formatter path runs on the flat substrate: the 24.9% is an estimate from a workload,
  not a measurement of the formatter
- Comma-split, type, pattern and item fragments and `parse_maybe_bound` are designed, not
  prototyped
- The ownership scheme for fragment trees is not chosen or measured
- Token-level hand-written helpers (`Comment::kind`, `Literal::kind`, `IsString::is_raw`,
  `doc_comments`) are not ported in `ext.rs`
- Instruction-per-cycle on this host is not measured (`perf` is absent), so the wall-clock
  against instruction gap in §3e is a hypothesis
