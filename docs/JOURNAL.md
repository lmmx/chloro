# chloro development journal

This journal records how chloro reproduces rustfmt, the plan that was followed, and the
measured state of the work. The freeform sections come first; the closing sections use the
[giacometti journal format](https://github.com/lmmx/giacometti/blob/master/docs/JOURNAL.md)
(Current State, Stubbed, Missing, Divergence).

## Goal

chloro formats Rust source the way `rustfmt --edition 2024` does, without depending on
`rustc`'s parser. "The way rustfmt does" is meant literally: for any input, the output
should be byte-identical to rustfmt's, including rustfmt's quirks, and inputs that rustfmt
refuses (syntax errors) are returned unchanged.

## Approach: port rustfmt, adapt the tree

The proof of concept that preceded this work formatted node kinds with hand-written rules
(`formatter/node/*.rs`, removed). That design cannot converge on rustfmt's output: rustfmt's
behaviour is the product of a few hundred interacting width heuristics and fallbacks, and
reproducing it rule by rule from observed output never ends. The proof of concept matched
233 of the 1220 conformance files.

The replacement is a module-for-module port of rustfmt 1.9.0 (the version that produced the
reference snapshots), keeping rustfmt's control flow, names and comments so that every
formatting decision can be checked against the code it reproduces:

| chloro module | rustfmt source |
|---|---|
| `formatting.rs` | `formatting.rs`, `lib.rs` (`format_snippet`, `format_code_block`) |
| `visitor.rs`, `missed_spans.rs` | `visitor.rs`, `missed_spans.rs` |
| `items.rs`, `imports.rs`, `reorder.rs` | `items.rs`, `imports.rs`, `reorder.rs`, `vertical.rs` |
| `expr.rs`, `stmt.rs`, `chains.rs`, `closures.rs`, `matches.rs`, `pairs.rs` | same names |
| `patterns.rs`, `types.rs`, `attr.rs`, `macros.rs` | same names |
| `macro_args.rs` | `parse/macros/mod.rs`, `parse/macros/lazy_static.rs` |
| `overflow.rs`, `lists.rs`, `comment.rs`, `shape.rs`, `utils.rs`, `sort.rs` | same names |
| `context.rs` | `rewrite.rs` |
| `config.rs` | `config/mod.rs`, `config/options.rs` (stable options only) |

rustfmt works on rustc's AST; chloro has rust-analyzer's lossless rowan tree. Three
adaptation layers bridge the two:

- **Spans** (`span.rs`). rustfmt locates comments by slicing the source between AST spans.
  `rustc_span` computes rustc's span for a rowan node: rowan attaches leading comments and
  whitespace to the following node, rustc does not. rustc's empty spans for absent
  generics, where clauses and return types are rebuilt where rustfmt reads them
  (`items.rs`: `Generics`, `WhereClause`, `FnSig`).
- **Shapes of nodes** (`nodes.rs`). Doc comments are attributes in rustc and trivia in
  rust-analyzer (`Attribute::Doc`). Block statements are reclassified as rustc does
  (`stmts_of`, `macro_stmt`): a macro call followed by `;` or using braces is a statement
  macro, anything else is an expression.
- **Macro arguments** (`macro_args.rs`). rustfmt formats `foo!(a, b)` by running rustc's
  parser over the token tree, preferring expression, type, pattern, item. rust-analyzer
  keeps token trees opaque. chloro lexes the token tree interior, parses each argument
  with rust-analyzer's `PrefixEntryPoint` over a window that ends at the next top-level
  `,`/`;` (widened when the separator belongs to the argument, as in `|a, b| a + b`), and
  splices the resulting green nodes into a copy of the tree. The spliced nodes keep their
  original byte offsets, so comment recovery inside macro arguments works unchanged.
  Parsed arguments are cached per token tree, because rustfmt rewrites a call once per
  candidate shape.

Where rust-analyzer's parser accepts syntax that rustc's parser rejects, rustfmt leaves the
file unformatted. `rustc_compat.rs` lists those constructs (each checked against rustfmt
1.9.0); a file containing one is returned unchanged. The same check rejects macro
arguments that rustc would not parse. Conversely, rust-analyzer's *validation* pass flags
semantic errors that rustc's parser accepts (`use a::self;`), so chloro only counts lexer
and parser errors (`formatting.rs::parse`).

rustfmt formats the text rustc's `SourceMap` holds, not the bytes of the file: rustc removes
a leading byte order mark and replaces `\r\n` with `\n` when it loads a file.
`format_source_with_config` formats the same normalised text (`formatting.rs::normalize_src`),
which is why a CRLF file comes back with LF line endings under the default
`newline_style = Auto`: rustfmt's auto-detection inspects the normalised text.

rust-analyzer's node kinds are sometimes coarser than rustc's. One `BlockExpr` covers rustc's
`Block`, `Gen` (`async`/`gen`), `TryBlock` and `ConstBlock`, which rustfmt treats
differently in overflow, closures and match arms; `nodes.rs::block_expr_kind` recovers the
rustc kind and every such decision goes through it.

## Configuration

`Config` (`config.rs`) has one public field per *stable* rustfmt option, with rustfmt's
names and defaults, and `Config::set(key, value)` accepts the strings a `rustfmt.toml`
would contain. `style_edition` is an `Option`: unset, it follows `edition`, as rustfmt
derives it in `Config::default_for_possible_style_edition`. Unstable rustfmt options are not configurable; `Config::set` accepts them
only at their default value. The formatting code reads options through `Settings`
accessors named after the rustfmt options, so a fixed unstable option such as
`indent_style` or `brace_style` appears at its use site (as an accessor or a comment) rather
than disappearing. The rustfmt branches for non-default values of unstable options
(visual indent, `brace_style = AlwaysNextLine`, imports granularity, comment wrapping) are
not ported.

## Plan and progress

1. Deterministic, offline conformance harness: `examples/conform.rs` compares
   `format_source` with the checked-in rustfmt snapshots of 1220 rust-analyzer source files
   in about 5 seconds, reports idempotence (`-i`) and prints per-file diffs (`-d`). Done.
2. Port the infrastructure (shape, spans, comments, lists, config). Done.
3. Port expressions, patterns, types, chains, closures, matches, macros. Done.
4. Port items, imports, reordering, the visitor and missed spans; switch `format_source`
   to the port and delete the proof of concept. Done.
5. Drive the conformance diff to zero. 233 → 1164 identical files after the switch, then
   1219 after fixing the issues listed below.
6. Check every stable option against rustfmt (`examples/config_matrix.rs`). Done; see the
   results below.
7. Check a corpus chloro was not developed on: every `.rs` file in the local cargo registry
   (`config_matrix --root ~/.cargo/registry/src default`). Done; see the results below.

Issues found by conformance and fixed after the switch, in order: the trait of a
qualified path was taken from the self type; path patterns counted as "short" in
or-patterns; bare trait objects gained `dyn`; `Item(T): Bound` lost `(T)`; `[const]`
bounds lost their brackets; `0...100` lost its end; attributes on statement macros were
looked up on the wrong node; `mod r#type` sorted as `r#type` instead of `type`;
rust-analyzer validation errors blocked formatting; a top-level attribute's meta item
used its own span instead of the attribute's (rustfmt drops the trailing comma of
`#[foo(a,)]` because of that span).

The option matrix found: `use` items with attributes were printed twice with
`reorder_imports = false`; tracing's `?value` macro arguments are types to rustc
(`?Trait`) but not to rust-analyzer; three `style_edition <= 2021` branches were missing
(parenthesised types, `impl Trait`, slice patterns); rustc rejects chained ranges, which
makes `matches!(c, 'a'..='z' | 'A'..='Z')` an unformatted macro; inner attributes of
`extern` blocks are copied, not formatted; a `...` parameter only makes a fn C-variadic
when it is the last parameter; `edition = 2021` did not change the Style Guide edition
(rustfmt derives `style_edition` from `edition`); rustc rejects let chains before edition
2024; `use self;` is removed because every rustc item has a visibility, including the
implicit one, which also removes `use ::self;` in edition 2015.

The registry corpus found: CRLF files kept their line endings (rustfmt formats the
normalised source); `&const { .. }` overflowed as a last argument (rustc's `ConstBlock` is
not a `Block`); rustc rejects chained comparisons, which leaves clap's
`arg!(-c --config <FILE> "..")` unformatted.

## Performance

rustfmt's output is defined by trial and fallback: a rewrite returns `None` when it does not
fit, and the caller tries the next layout (one line, overflowing the last argument,
vertical, a chain on one line, then broken). Each attempt rewrites the whole subtree again.
chloro has to run the same attempts, since the output depends on which one fits first, so
the port started at a third of the speed of the proof of concept (2.7 MB/s against 8.2 MB/s
on the same machine). The proof of concept made one pass with one layout per node and
matched rustfmt on 233 of 1220 files.

Every performance change is checked to be behaviour-preserving, not just conformant:
`examples/bench.rs --record` hashes the output for every fixture and every file of a cargo
registry (6800 files), and `--check` after the change must report no difference.
Instruction counts come from callgrind on a fixed 49-file sample (minimum of three runs; the
allocator's internals vary by about 2% between runs).

Changes, in the order they were made:

- Rewrites of expressions are memoized per top-level item, keyed by node, shape and context
  flags (`RewriteContext::memoize`). Three effects of a rewrite on the run (the macro failure
  flag, the lost comment flag and the skipped line ranges) are only written during
  formatting and read afterwards; they are recorded with the result and replayed on a hit.
  Retries compound with nesting: on `ide-assists/src/handlers/desugar_try_expr.rs`
  (nested calls in closures in chains) the instruction count fell from 696M to 26M, and
  the wall time from 88 ms to 3 ms. On typical code the hit rate is low and the gain small.
- Red nodes: rowan allocates a red node for every child an accessor looks at, including on a
  miss. Presence checks of modifier tokens (`has_token`), typed child lookups (`child`),
  node text (`node_text`), single-identifier paths and the first significant child of a node
  read the green tree instead, creating a red node only for the node returned.
- Macro arguments are spliced under a detached root padded to the token tree's offset,
  instead of into a copy of the whole tree, which copied every ancestor's child list per
  macro call.
- The comment scanner jumps over runs of plain code in its `Normal` state, where only `r`,
  `"`, `'` and `/` can change its state.
- Smaller items: the width of multi-line ASCII text in one pass, chain items without
  `format!`, no separator scans when no comment lies between two strings, and no
  trailing-comma scan of struct literals outside macros.

Tried and reverted, because they did not pay: building green trees without rowan's
`NodeCache` (the interning saves more allocations than its hashing costs), `mimalloc` (about
10%, not worth a dependency of the library), caching the trailing-comma scan by span, and
replacing the remaining `expr()`/`path()` accessors (within noise).

What is left is spread out: parsing by rust-analyzer's parser is a quarter of the
instructions, allocation (mostly red nodes and growing strings) about a fifth, and the rest
is rustfmt's algorithm.

## Measurements

- Conformance: 1219 of 1220 files identical to rustfmt; every output is idempotent
  (`cargo run --release -p chloro-core --example conform -- -i`).
- Options: `examples/config_matrix.rs` compares 31 non-default option values with
  `rustfmt --config` over the 1022 fixtures rustfmt accepts (fewer for older editions, where
  rustfmt rejects more files). Every value matches on every file except
  `0024_many_type_parens.rs`, plus two rustc-recovery files under `hard_tabs`, `tab_spaces`
  and `newline_style` (see Missing).
- Unseen corpus: 5384 of the 5386 files in the local cargo registry that rustfmt accepts
  (193 crate versions of 156 crates, among them clap, serde, syn, regex and facet) format
  identically; the two differing files are listed under Missing.
- Self-formatting: chloro's own sources, formatted by `cargo fmt`, are a fixed point of
  chloro.
- Speed (`cargo run --release -p chloro-core --example bench`, one thread, in process):
  3.5 MB/s over the conformance corpus, against 2.7 MB/s for the port before the
  performance work and 8.2 MB/s for the proof of concept, on the same machine. On the five
  largest fixtures (167–640 KB) a chloro process takes 0.35x–0.71x the wall time of a
  `rustfmt` process on the same file.

## Current State

- `format_source` formats with rustfmt's defaults for `--edition 2024`, and
  `format_source_with_config` with a `Config` (chloro-core/src/formatter.rs:57-86)
- Formatting runs rustfmt 1.9.0's algorithm ported module by module, with the module map in
  the doc comment of chloro-core/src/formatter.rs:1-19
- The source is normalised as rustc loads it before formatting — a leading byte order mark
  is dropped and `\r\n` reads as `\n` (chloro-core/src/formatter/formatting.rs:141-151)
- Source that rust-analyzer's lexer or parser rejects is returned unchanged — validation
  errors are ignored because rustc's parser does not report them
  (chloro-core/src/formatter/formatting.rs:34-60)
- Source containing syntax that rustc's parser rejects and rust-analyzer's accepts is
  returned unchanged, using the edition-dependent rules in
  chloro-core/src/formatter/rustc_compat.rs:128-170
- A file with an inner `#![rustfmt::skip]` attribute is returned unchanged
  (chloro-core/src/formatter/formatting.rs:70-80)
- Trailing newlines are truncated to one and the newline style is applied after formatting,
  as in rustfmt's `format_lines` and `apply_newline_style`
  (chloro-core/src/formatter/formatting.rs:111-180)
- `Config::set` accepts every stable rustfmt option by its rustfmt name and accepts
  unstable options only at their default values (chloro-core/src/formatter/config.rs:327,
  chloro-core/src/formatter/config.rs:427)
- `style_edition` follows `edition` unless set explicitly
  (chloro-core/src/formatter/config.rs:572)
- Width heuristics follow rustfmt's `use_small_heuristics` resolution, scaling above
  `max_width = 100` only (chloro-core/src/formatter/config.rs:295)
- rustc's block expression kinds (`Block`, `Gen`, `TryBlock`, `ConstBlock`) are recovered
  from rust-analyzer's single `BlockExpr` (chloro-core/src/formatter/nodes.rs:288-320)
- Macro call arguments are re-parsed with rust-analyzer's parser and spliced into the tree
  at their original offsets, under a detached root padded to the token tree's offset
  (chloro-core/src/formatter/macro_args.rs:189-366)
- `?Trait` macro arguments parse as types, as in rustc
  (chloro-core/src/formatter/macro_args.rs:233)
- `macro_rules!` bodies are formatted through `format_snippet` and `format_code_block` with
  metavariables renamed, as in rustfmt (chloro-core/src/formatter/macros.rs:592,
  chloro-core/src/formatter/formatting.rs:229-256)
- `use`, `extern crate` and out-of-line `mod` declarations are sorted within blank-line
  groups, with 2024 version sorting (chloro-core/src/formatter/reorder.rs:272,
  chloro-core/src/formatter/sort.rs)
- 1219 of 1220 conformance fixtures format identically to the rustfmt snapshots and all
  1220 outputs are idempotent (chloro-core/examples/conform.rs)
- 31 non-default option values match `rustfmt --config` on the conformance fixtures, apart
  from the files under Missing (chloro-core/examples/config_matrix.rs)
- 5384 of 5386 cargo registry files format identically to rustfmt
  (`config_matrix --root ~/.cargo/registry/src default`)
- Expression rewrites are memoized per top-level item, with their effects on the run
  replayed on a hit (chloro-core/src/formatter/context.rs:261-324,
  chloro-core/src/formatter/expr.rs:109)
- Hot syntax queries read the green tree and create a red node only for the node returned
  (chloro-core/src/formatter/nodes.rs:236-286)
- chloro-core/examples/bench.rs measures in-process throughput and records or checks a hash
  of every output, so that a performance change can be shown to change no output
- chloro-core/examples/fmt_stdin.rs formats stdin with options given as `key=value`
  arguments, for side-by-side checks against rustfmt
- The CLI formats files and directories and takes rustfmt options as
  `--config key=value,...`, validated before any file is read (chloro/src/cli/args.rs,
  chloro/src/cli.rs)

## Stubbed

- `printer::Printer` remains public for API compatibility and has no caller in the
  formatter (chloro-core/src/formatter/printer.rs)

## Missing

- `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` (rustfmt's
  `SkipContext`) have no implementation — macro calls named in them are formatted
- Unstable rustfmt options have no implementation beyond their default values: `Config::set`
  rejects any other value (chloro-core/src/formatter/config.rs:427)
- No reader for `rustfmt.toml` files: options are set through `Config` fields or
  `Config::set`
- `ignore`, `skip_children`, `file_lines` and other options about which files to format
  have no implementation — `format_source` formats a single source text
- rustc's recovery for a parenthesised first bound of a bare trait object type
  (`Box<(?Sized) + Copy>` loses the first parentheses in rustc's AST) has no equivalent —
  chloro-core/tests/conformance/fixtures/rust-analyzer/parser/test_data/parser/err/0024_many_type_parens.rs
  keeps the parentheses
- rustc's parser recovers from some errors that rust-analyzer's parser reports, such as a
  float literal in a field access (`s.1e0`) or `impl impl Trait {}` — chloro returns such
  files unchanged where rustfmt formats them
  (chloro-core/tests/conformance/fixtures/rust-analyzer/parser/test_data/parser/err/0054_float_split_scientific_notation.rs,
  chloro-core/tests/conformance/fixtures/rust-analyzer/parser/test_data/parser/inline/err/impl_type.rs)
- Const block items (`const { .. }` in item position, the unstable `const_block_items`) are
  a parse error to rust-analyzer — a file containing one is returned unchanged, and a
  `macro_rules!` body that is a single `const { .. }` is formatted as a statement, where a
  failed inner macro call leaves the whole body unformatted while rustfmt's item path
  ignores the failure (facet-core 0.30.0, src/macros.rs)

## Divergence

- A file with `#![rustfmt::skip]` is returned byte for byte, where `rustfmt` reading stdin
  echoes the normalised text (no byte order mark, `\n` line endings) — chloro matches
  `rustfmt` run on a file, which leaves a skipped file untouched
  (chloro-core/src/formatter.rs:76)
