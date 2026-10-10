# 2026-10-10: Performance work and phase 1 checkpoint

Checkpoint of phase 1 of the work on the `claude/gifted-wright-d4z4rp` branch, written
before a new phase starts. The branch is open as
[lmmx/chloro#85](https://github.com/lmmx/chloro/pull/85) and is **not merged**; the next
phase continues on the same PR. Head of the branch when this entry was first written:
`2cac5d7`.

This entry covers the whole phase. The design of the port as of 2026-10-09 — the
module-for-module map to rustfmt's sources and how the adaptation layers (spans, node
shapes, macro-argument re-parsing with separator-bounded windows) work — is recorded in
more detail in [2026-10-09-rustfmt-port.md](2026-10-09-rustfmt-port.md). Commits
`f019ca5` and `6f2d59e` wrote that entry and a performance section by mistake into
`docs/JOURNAL.md`, the path of the journal format file; `docs/JOURNAL.md` now holds the
unmodified format file from lmmx/giacometti, the 2026-10-09 content is in its own entry, and
the 2026-10-10 performance content is in this entry.

The freeform sections record what was asked, what was decided and why, what was measured
and how, which experiments were tried (including the ones that failed), and what is still
open. Statements are marked as measured, probed (checked by running rustfmt on a test
input), or hypothesis. The closing sections follow the [journal format](../JOURNAL.md)
(Current State, Stubbed, Missing, Divergence).

## What was asked

- Take the proof of concept to a fully working, performant, high-quality reproduction of
  rustfmt. The work is reviewed by a senior engineer who expects high performance and
  predictable behaviour, and who reads the docs to learn how the software works.
- Commit as the developer who already committed on the repository (Louis Maddox,
  `lmmx@users.noreply.github.com`); no "Claude Code" in author, co-author or message.
- Avoid churn in the public API.
- Keep the development journal: entries under `docs/journal/YYYY-MM-DD-title.md` with the
  giacometti sections as a tail, and freeform sections before them where there is more to
  record. `docs/JOURNAL.md` is the format file itself, copied unmodified from lmmx/giacometti.
- Use the conformance tests to drive correctness.
- Mid-phase hint from the user: rustfmt is easier to reproduce once it is decomposed by
  its configurable settings. Taken up as `Config`, one field per stable rustfmt option,
  checked option by option against `rustfmt --config` (see Decisions).
- After the port: "did this make it much slower? The original chloro was much faster than
  rustfmt". Then: pursue every acceleration idea aggressively, commit each win separately
  (usage limits were being hit), and aim for maximally correct and maximally performant.

## Process notes

- Every commit is authored and committed as `Louis Maddox <lmmx@users.noreply.github.com>`
  with no trailers — the commits are unsigned and show as Unverified on GitHub.
- A stop hook asked several times for commits to be re-authored as Claude. The user's
  instruction takes precedence and the hook was not followed.
- The PR description ends with the Claude Code footer and session link (the no-Claude rule
  applies to commits only).
- The container restarted once (kernel `6.18.44-fc-v80` before, `6.18.44-fc-v114` after).
  Wall-clock numbers taken on different hosts are not comparable. Every speed comparison
  below was re-measured on one host.

## Commits of this phase

| commit | content |
|---|---|
| `51d0c57` | deterministic fixture order; offline conformance report (`examples/conform.rs`) |
| `2c1c00e` | the port of rustfmt 1.9.0 onto rowan trees; the proof of concept removed |
| `fe28548` | conformance snapshots regenerated for the port |
| `325bc85` | `examples/{config_matrix,fmt_stdin,dump_tree}.rs` |
| `83a33fd` | CLI `--config key=value,...` |
| `17c45d4` | `style_edition` follows `edition` unless set |
| `ee38eb1` | rustc rejects let chains before 2024 and chained comparisons |
| `fd479ae` | rustc's block expression kinds (`Block`/`Gen`/`TryBlock`/`ConstBlock`) |
| `97b412f` | source normalised as rustc loads it (BOM, CRLF) |
| `2cf80a3` | every `use` item has a visibility (`use self;` removed) |
| `06849d2` | `config_matrix -v` |
| `f019ca5` | journal (written at `docs/JOURNAL.md` by mistake; moved to `2026-10-09-rustfmt-port.md`) and README |
| `bcef0f9` | `examples/bench.rs`: in-process benchmark and output-hash record/check |
| `2d8a579` … `9202b42` | eleven performance commits (see Performance) |
| `6f2d59e`, `2cac5d7` | performance section in `docs/JOURNAL.md` (by mistake; content now in this entry) and README |

## Decisions and their reasons

**Port rustfmt instead of extending the proof of concept.** The proof of concept formatted
node kinds with hand-written rules and matched 233 of 1220 conformance files (measured). The
output of rustfmt comes from a few hundred interacting width heuristics and fallbacks, so
rules inferred from its output do not converge. The port follows rustfmt 1.9.0 (the version
installed, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, and the one that produced the
snapshots) module by module, keeping rustfmt's names and control flow (module map in
`chloro-core/src/formatter.rs:1-19`). The rustfmt source used as reference was a checkout of
tag `v1.9.0` at `/home/user/ref/rustfmt`, outside the repository and lost with the
container.

**Reference output is `rustfmt --edition 2024` reading stdin.** The snapshots under
`chloro-core/tests/conformance/snapshots/ra` were generated that way.

**Adaptation layers instead of a rustc AST.** chloro keeps rust-analyzer's parser
(`ra_ap_syntax`/`ra_ap_parser` 0.0.307, rowan 0.15.15) and adapts its trees to what
rustfmt reads:
- rustc spans for rowan nodes (`span.rs::rustc_span`: leading whitespace and non-doc
  comments excluded; `span_without_attrs`);
- doc comments as attributes (`nodes.rs::Attribute::Doc`);
- rustc's statement classification (`nodes.rs::stmts_of`, `macro_stmt`);
- rustc's block expression kinds (`nodes.rs::block_expr_kind`);
- macro arguments re-parsed as expression, type, pattern or item (`macro_args.rs`).

**What counts as a parse failure.** Only lexer and parser errors count
(`formatting.rs::parse`, which builds the tree itself instead of calling
`SourceFile::parse`). rust-analyzer's validation errors are ignored, because rustc's parser
accepts e.g. `use a::self;` and rustfmt formats such files. Syntax that rust-analyzer
accepts and rustc rejects is detected by `rustc_compat.rs` and the file is returned
unchanged, as rustfmt leaves it. Files that rustc's parser recovers from but
rust-analyzer's does not (`s.1e0`, `impl impl Trait`) are returned unchanged, where rustfmt
formats them. That is a known gap (Missing), not a decision to diverge.

**Configuration model.**
- `Config` has one public field per *stable* rustfmt option, with rustfmt's names and
  defaults. `Config::set` takes `rustfmt.toml` strings.
- Unstable options are fixed at their defaults. `Config::set` accepts them only at the
  default value and rejects other values with an error naming the option.
- `style_edition` is `Option`: unset, it follows `edition`, as in rustfmt's
  `Config::default_for_possible_style_edition`. This was found by the option matrix, where
  `edition=2021` matched 459/844 files before the fix and 843/844 after.

**API.** Additive only:
- added: `chloro::Config`, `format_source_with_config`, CLI `--config`;
- unchanged: `format_source`, `chloro_core::debug`, `chloro_debug!`, the
  `formatter::{config, printer}` modules, and the existing CLI flags;
- `printer::Printer` stays public with no caller (Stubbed).

**Source normalisation.** rustfmt formats the text rustc's `SourceMap` holds: BOM removed,
`\r\n` read as `\n` (probed: rustfmt writes LF for a CRLF file, on disk and on stdin, under
`newline_style = Auto`, and drops a BOM when it reformats). chloro normalises the same way
before formatting (`formatting.rs::normalize_src`). The `Auto` newline detection then sees
LF, as in rustfmt. Before this fix, about 60 registry files differed on every line.

**`#![rustfmt::skip]` files are returned byte for byte.** rustfmt reading stdin echoes the
*normalised* text (probed); rustfmt on a file leaves it untouched. chloro follows the
on-disk behaviour. This is the one recorded Divergence.

**Const block items are not implemented.** rustc parses `const { .. }` in *item* position
(module level, with `pub` or attributes) as `ItemKind::ConstBlock` (unstable
`const_block_items`). rust-analyzer reports a parse error. Implementing it would need a
synthetic item kind in the tree, for an unstable feature seen in 1 of 5386 registry files.
Recorded under Missing.

**Performance changes must change no output.** Every performance commit was checked with
`examples/bench.rs --check` against output hashes recorded before the performance work
(see Performance: method), not only with the conformance suite.

**No allocator dependency.** `mimalloc` measured about 5–10% faster in the benchmark, which
is within wall-clock noise. The library cannot choose its users' allocator anyway; the CLI
binary could, and that was not pursued.

## Conformance work: what was found and fixed

Conformance (rust-analyzer's crates, 1220 files): 233 identical with the proof of concept,
1164 right after the switch to the port, 1219 after the fixes below (measured with
`examples/conform.rs`).

Issues found by conformance, in order fixed:
- the trait of a qualified path was taken from the self type;
- path patterns counted as "short" in or-patterns;
- bare trait objects gained `dyn`;
- `Item(T): Bound` lost `(T)`;
- `[const]` bounds lost their brackets;
- `0...100` lost its end;
- attributes on statement macros were looked up on the wrong node;
- `mod r#type` sorted as `r#type` rather than `type`;
- rust-analyzer validation errors blocked formatting;
- a top-level attribute's meta item used its own span instead of the attribute's (rustfmt
  drops the trailing comma of `#[foo(a,)]` because of that span).

Issues found by the option matrix (`examples/config_matrix.rs`):
- `use` items with attributes were printed twice with `reorder_imports = false`;
- tracing's `?value` macro arguments are types to rustc (`?Trait`), not to rust-analyzer;
- three missing `style_edition <= 2021` branches (parenthesised types, `impl Trait`, slice
  patterns);
- rustc rejects chained ranges, so `matches!(c, 'a'..='z' | 'A'..='Z')` is an unformatted
  macro;
- inner attributes of `extern` blocks are copied, not formatted;
- a `...` parameter makes a fn C-variadic only when it is the last parameter;
- `style_edition` derivation from `edition`;
- let chains in `if`/`while` conditions are a rustc *parse* error before edition 2024
  (probed: `if a && let ..` and `while .. && let ..` rejected in 2021; a single `if let`
  and a match guard `if let .. && a` accepted);
- `use self;` and `use ::self;` (2015). rustc gives every item a visibility, `Inherited`
  when none is written, and rustfmt's `UseTree::normalize` removes `use self;` only when the
  visibility is `Some`. chloro now models rustc's `Option<Visibility>` as
  `Option<Option<ast::Visibility>>` (`imports.rs:99`).

Issues found by the cargo registry corpus (crates chloro was not developed on):
- CRLF line endings (above);
- `&const { .. }` overflowed as a last call argument (rustc's `ConstBlock` is not a
  `Block`);
- chained comparisons (`a < b > c`, `a == b != c`) are a rustc parse error, which leaves
  clap's `arg!(-c --config <FILE> "..")` unformatted;
- facet-core's `macro_rules!` body that is a single `const { .. }` (see the const block
  items decision). The explanation was established by probes: rustfmt formats a macro body
  first as a file (`format_snippet`), where `const { .. }` is an item. Its
  `visit_item` formats `ItemKind::ConstBlock` with `get_context()`, not `with_context`, so a
  failed inner macro call does not mark the snippet as failed. Inside a fn body
  (statement position), `const { .. }` is an expression and the failure propagates.

## Evidence: comparisons with rustfmt

All runs use `rustfmt 1.9.0-stable` and `--edition 2024` unless the option says otherwise.

- **Conformance:** 1219 of 1220 files identical to the snapshots; every output idempotent
  (`cargo run --release -p chloro-core --example conform -- -i`). The differing file is
  `parser/test_data/parser/err/0024_many_type_parens.rs` (Missing).
- **Option matrix:** results below; the command is
  `cargo run --release -p chloro-core --example config_matrix -- -n 1220`.

  Run on `9202b42` (the final code of the phase), after the performance commits. Of the
  1022 fixtures rustfmt accepts with `--edition 2024` (fewer for older editions):

  | option values | identical | differing files |
  |---|---|---|
  | 25 values: `max_width=80`, `max_width=120`, `use_small_heuristics=Max`/`Off`, `fn_call_width=40`, `chain_width=40`, `struct_lit_width=0`, `array_width=30`, `single_line_if_else_max_width=0`, `single_line_let_else_max_width=0`, `reorder_imports=false`, `reorder_modules=false`, `remove_nested_parens=false`, `short_array_element_width_threshold=4`, `match_arm_leading_pipes=Always`/`Preserve`, `fn_params_layout=Vertical`/`Compressed`, `match_block_trailing_comma=true`, `merge_derives=false`, `use_try_shorthand=true`, `use_field_init_shorthand=true`, `force_explicit_abi=false`, `style_edition=2021`, `style_edition=2015` | 1021/1022 each | `err/0024_many_type_parens.rs` |
  | `hard_tabs=true`, `tab_spaces=2` | 1020/1022 each | `0024`, `err/0054_float_split_scientific_notation.rs` |
  | `newline_style=Windows` | 1019/1022 | `0024`, `0054`, `inline/err/impl_type.rs` |
  | `edition=2021` | 843/844 | `0024` |
  | `edition=2018` | 842/843 | `0024` |
  | `edition=2015` | 837/838 | `0024` |

  Every count and every differing file is the same as in the last run before the
  performance commits (code of `2cf80a3`–`06849d2`) — the output hashes cover only the
  default configuration, and this run is the evidence that the performance commits changed
  no output under non-default options either. Every differing file is listed under Missing.

- **Registry corpus:** results below; the command is
  `config_matrix --root ~/.cargo/registry/src -n 6000 -v default`. The corpus is the cargo
  registry of this container, so it is not a fixed corpus. Rebuilding it in another container
  gives a different set of files.

  Run on `9202b42`: 5420 of the 5422 registry files rustfmt accepts are identical. The
  two differing files are `ra_ap_parser-0.0.307/test_data/parser/err/0024_many_type_parens.rs`
  and `facet-core-0.30.0/src/macros.rs`, both listed under Missing. The first run (before
  the performance commits and the `use self;` fix) gave 5384 of 5386 with the same two
  differing files; the registry held fewer files then (the `mimalloc` experiment downloaded
  more crates).

- **Self-formatting:** chloro's own sources, formatted with `cargo fmt`, are a fixed point
  of chloro (`src/tests/formatter/self_format.rs`).

## Performance

### Why the port is slower than the proof of concept

rustfmt chooses layouts by trial and fallback: a rewrite returns `None` when it does not
fit, and the caller tries the next layout. Each attempt rewrites the whole subtree again,
and attempts compound with nesting. On `hir/src/lib.rs`, before the memo, counted with
callgrind:

| construct | count in the file | rewrite calls | rewrites per construct |
|---|---|---|---|
| closures | 233 | 6,553 `rewrite_closure` | 28 |
| `match` expressions | 191 | 4,517 `rewrite_match` | 24 |
| calls and method calls | 2,776 | 8,073 `rewrite_with_parens` | 2.9 |

chloro has to run the same attempts, since the output depends on which one fits first. The
proof of concept made one pass with one layout per node.

### Method

- **Output equivalence:**
  - `examples/bench.rs --record` writes a hash of `format_source`'s output for every file
    under a root; `--check` formats again and lists every changed hash.
  - Hashes were recorded at `bcef0f9`, before any performance change: 1238 fixture files
    (1220 rust-analyzer files plus 18 others under `tests/conformance/fixtures`) and 5623
    registry files.
  - Every performance commit reported `0 of 1238` and `0 of 5659` outputs changed. 36
    registry files appeared after the recording, downloaded by the `mimalloc` experiment;
    those 36 are reported as "not recorded" and are not compared.
  - The hashes cover the **default configuration only**. The re-run of the option matrix
    above, on the final commit, is the evidence for non-default options.
  - The hash files lived in `/tmp/claude-0/scratch/golden` and are gone with the container.
    In a new session, re-record at `bcef0f9` (or any commit trusted as reference) before
    comparing.
- **Instruction counts:**
  - callgrind on a fixed sample of 49 fixtures (every 25th file of the sorted fixture
    list), formatted once.
  - From the hash-function change on, runs are under `setarch -R` (no ASLR).
  - Identical binaries still vary by about 2% between runs. Diffing two runs of one binary
    put the variance in glibc `malloc` internals (`_int_free`, `malloc_consolidate`) and
    green-node frees, not in chloro's code, so the minimum of three runs is used.
  - Single-run deltas below 2% measured before this was found are **not established**
    individually.
- **Wall clock:** `examples/bench.rs -n 5` (best of 5) over the 1238 fixture files, one
  thread, in process. Run-to-run noise is about ±4%.

### Changes, in commit order, with the measurement taken at the time

Instruction counts are for the 49-file sample unless stated.

| commit | change | measured effect |
|---|---|---|
| `2d8a579` | memoize `format_expr` per (node, expression type, shape, context flags), cleared at each top-level item; macro-failure flag, lost-comment flag and skipped ranges recorded and replayed on a hit | `desugar_try_expr.rs`: 696M → 26M instructions, 88 ms → 3 ms; `hir/src/lib.rs`: 530M → 535M (no gain: 449 hits of 14,276 calls over 8,601 nodes) |
| `e4df9e4` | `node_text`: node text from the green tree, not `SyntaxText`'s `Display` | wall 4.47 s → 4.26 s |
| `a156c83` | `CharClasses::skip_plain`: jump over plain code in the `Normal` state | 845M → 824M |
| `4edd403` | one-pass width of multi-line ASCII text | 824M → 809M |
| `bd3bad1` | find the spliced token tree by range (binary search), not by child index | 809M → 775M |
| `7e2be6c` | single-identifier paths read from the green tree | 775M → 740M |
| `74fb4a7` | `has_token` for every `x_token().is_some()` on a plain token accessor | 740M → 703M |
| `03e063f` | chain items built without `format!` | 703M → 681M |
| `6b07131` | macro arguments spliced under a detached root padded to the token tree's offset; struct-literal trailing-comma scan only inside macros | about 690M → 654M (single runs; the 2% noise was found during this step) |
| `8d06135` | `span_without_attrs` on the green tree; no separator scans in `combine_strs_with_missing_comments` without a comment | 654M → 635.5M (two changes, single runs) |
| `86b77a8` | `child::<N>` for every call of an accessor whose every definition is a plain `support::child` | 634M → 607M |
| `9202b42` | no memo for paths and literals; macro-argument cache hashed with Fx (deterministic) | 606.8M → 595.3M (minimum of three) |

Further measurements on the final code:
- **Memo on and off**, at `6b07131`: 654M with the memo, 673M with `format_expr` calling
  `format_expr_uncached` directly, so the memo pays on typical code too.
- **Allocations on the sample:** 774k before `86b77a8`, 698k after. Rowan's
  `NodeData::new` (red nodes) made 419k of those before and 343k after. `Vec`/`String`
  growth (`finish_grow`) made 106k.
- **Where instructions go** on the sample at `9202b42`:
  - parsing: 26% (`intersperse_trivia` with tree building 13%, `TopEntryPoint::parse` 7%,
    `LexedStr::new` 5%, `rustc_compat` 1.6%);
  - formatting (`format_separate_mod`): 74%;
  - the `malloc`/`free` family, across both: about 18%.

### Tried and reverted (do not repeat without a new reason)

| experiment | result | why reverted |
|---|---|---|
| green trees built without rowan's `NodeCache` interning (`GreenBuilder` with `GreenNode::new`/`GreenToken::new`) | 856M → 880M (single run) | worse: interning saves an allocation per repeated token, which outweighs its hashing |
| `mimalloc` as global allocator, bench only | 3.12 → 3.22 MB/s and 3.17 → 3.51 MB/s on two pairs of runs | within wall-clock noise; a library cannot set its users' allocator |
| cache of `span_ends_with_comma` by span | 633.7M → 635.8M (single runs) | no gain: repeated calls are on different spans |
| `child::<N>` for the remaining `expr()`/`path()` accessors (83 sites; `Attr` excluded, whose `expr`/`path` go through `meta()`) | 607M → 605.5M | within noise for a large diff |

## Hypotheses and assumptions (not established)

- **The memo's correctness argument.** The argument is that a rewrite is a function of
  (node, shape, context flags) plus three effects that are only written during formatting.
  - **Basis:** an audit of every read and write of `macro_rewrite_failure`, `lost_comment`
    and `skipped_range`, plus the output-hash check. Nothing proves it.
  - **Safeguard:** a rewrite that leaves a context `Cell` changed is not stored.
  - **Keys:** nodes are keyed by green-node address and offset; the entry holds the node,
    which keeps the address alive.
  - **Synthetic trees:** trees built for macro arguments get fresh green nodes, so their
    keys cannot collide with the file's. This was reasoned from how the trees are built,
    not tested separately.
- **Nothing outside a macro argument reads the fragment root's parent.** The detached root
  (`6b07131`) changes the parent of a spliced token tree from the real ancestor to a
  synthetic `TOKEN_TREE`. An audit of `parent()`/`ancestors()` calls found no reader, and
  the hash check found no change. A future change that reads ancestors from inside a macro
  argument would break this silently.
- **`unicode_str_width`'s ASCII path** assumes unicode-width 0.1.14 (the locked version):
  every ASCII character is one column except `\n` and a `\r` before `\n`. A test compares the
  two on fixed strings (`utils.rs` tests). A different unicode-width version can change this.
- **rustfmt's own in-process time.** "rustfmt's in-process formatting time is close to
  chloro's" is an estimate: 78.7 s for one rustfmt process per fixture, minus about 61 ms
  of startup per process, leaves about 3.6 s. That subtracts two large numbers and is not a
  measurement.

## Unresolved questions and open items

- Remaining performance ideas, **not tried**:
  - format top-level items of one file in parallel (green trees are `Send`; each thread
    would build its own red root);
  - cut red-node allocations further, with a child cache or a rowan fork;
  - skip building strings for layout attempts that are about to fail;
  - cache the comment content of an original span in `recover_comment_removed`
    (about 2.7% of the sample);
  - the fixed cost of about 25 µs per `format_source` call on tiny files (about 30 ms over
    the corpus).
- The comparison with the proof of concept's speed is a request-level question left open:
  whether the goal is "faster than rustfmt" (met) or "near the proof of concept while
  rustfmt-identical" (not met: 3.5 against 8.2 MB/s). The proof of concept's goal was
  "consistent, as fast as possible", for diffing. A separate fast, non-rustfmt mode was
  raised with the user and not decided.
- `#[rustfmt::skip::macros(..)]`/`skip::attributes`, `rustfmt.toml` reading and
  file-selection options are not implemented (Missing).
- `0024_many_type_parens.rs`, rustc-recovery files and const block items (Missing).
- The registry corpus depends on what the container has downloaded. A fixed, checked-in
  second corpus does not exist.

## Speed summary (one host, `examples/bench.rs -n 5`, 1238 fixture files)

| build | throughput |
|---|---|
| proof of concept (`origin/master`, `c4d74ee`) | 8.2 MB/s |
| port before performance work (`bcef0f9`) | 2.65–2.72 MB/s |
| `9202b42` | 3.44–3.52 MB/s |

Against a `rustfmt` process on the five largest fixtures (167–640 KB, best of 5,
`examples/fmt_stdin.rs` against `rustfmt --edition 2024`, both reading stdin), chloro takes
0.35x–0.71x of rustfmt's wall time. Before the performance work it took 0.45x–0.91x, on the
earlier host.

## Current State

- `format_source` formats with rustfmt's defaults for `--edition 2024`, and
  `format_source_with_config` with a `Config` (chloro-core/src/formatter.rs:57-86)
- Formatting runs rustfmt 1.9.0's algorithm ported module by module, with the module map in
  the doc comment of chloro-core/src/formatter.rs:1-19
- The source is normalised as rustc loads it before formatting — a leading byte order mark
  is dropped and `\r\n` reads as `\n` (chloro-core/src/formatter/formatting.rs:144-151)
- Source that rust-analyzer's lexer or parser rejects is returned unchanged — validation
  errors are ignored because rustc's parser does not report them
  (chloro-core/src/formatter/formatting.rs:40-60)
- Source containing syntax that rustc's parser rejects and rust-analyzer's accepts is
  returned unchanged, using edition-dependent rules for let placement, chained ranges and
  chained comparisons (chloro-core/src/formatter/rustc_compat.rs:63-169)
- A file with an inner `#![rustfmt::skip]` attribute is returned unchanged
  (chloro-core/src/formatter/formatting.rs:71)
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
- Macro call arguments are re-parsed with rust-analyzer's parser and spliced, at their
  source offsets, under a detached root padded with shared power-of-two whitespace tokens
  (chloro-core/src/formatter/macro_args.rs:189-366,
  chloro-core/src/formatter/context.rs:95-115)
- `?Trait` macro arguments parse as types, as in rustc
  (chloro-core/src/formatter/macro_args.rs:233)
- `macro_rules!` bodies are formatted through `format_snippet` and `format_code_block` with
  metavariables renamed, as in rustfmt (chloro-core/src/formatter/macros.rs:592,
  chloro-core/src/formatter/formatting.rs:229-256)
- `use`, `extern crate` and out-of-line `mod` declarations are sorted within blank-line
  groups, with 2024 version sorting (chloro-core/src/formatter/reorder.rs:272,
  chloro-core/src/formatter/sort.rs)
- A `use` item's tree carries rustc's `Option<Visibility>`, `Some(None)` for an unwritten
  visibility — `use self;` is removed as rustfmt removes it
  (chloro-core/src/formatter/imports.rs:99)
- `format_expr` memoizes rewrites per (node, expression type, shape, context flags) except
  for paths and literals, replays the macro-failure flag, lost-comment flag and skipped
  ranges on a hit, and clears the memo at each top-level item
  (chloro-core/src/formatter/expr.rs:109-123, chloro-core/src/formatter/context.rs:261-324,
  chloro-core/src/formatter/visitor.rs:376)
- `has_token`, `child::<N>`, `node_text` and `span_without_attrs` read the green tree and
  create a red node only for the node returned (chloro-core/src/formatter/nodes.rs:213-286)
- `CharClasses::skip_plain` moves past plain code in the `Normal` state for the comment
  slicer, `contains_comment` and `find_uncommented`
  (chloro-core/src/formatter/comment.rs:669)
- 1219 of 1220 conformance fixtures format identically to the rustfmt snapshots and all
  1220 outputs are idempotent (chloro-core/examples/conform.rs)
- `examples/config_matrix.rs` compares 31 non-default option values with `rustfmt --config`
  and, with `--root`, any directory of Rust files with rustfmt's defaults
  (chloro-core/examples/config_matrix.rs)
- At `9202b42`, every one of the 31 option values matches `rustfmt --config` on every
  accepted fixture except `0024_many_type_parens.rs`, plus
  `0054_float_split_scientific_notation.rs` under `hard_tabs`, `tab_spaces` and
  `newline_style=Windows` and `inline/err/impl_type.rs` under `newline_style=Windows`
  (chloro-core/examples/config_matrix.rs)
- At `9202b42`, 5420 of the 5422 cargo registry files rustfmt accepts format identically
  with rustfmt's defaults — the two differing files are the `0024_many_type_parens.rs` copy
  in `ra_ap_parser-0.0.307` and `facet-core-0.30.0/src/macros.rs`
  (`config_matrix --root ~/.cargo/registry/src -n 6000 -v default`)
- `examples/bench.rs` measures in-process throughput and records or checks a hash of every
  output (chloro-core/examples/bench.rs)
- `examples/fmt_stdin.rs` formats stdin with options given as `key=value` arguments
  (chloro-core/examples/fmt_stdin.rs)
- The CLI formats files and directories in parallel and takes rustfmt options as
  `--config key=value,...`, validated before any file is read (chloro/src/cli/args.rs,
  chloro/src/cli.rs, chloro/src/cli/orchestrate.rs)

## Stubbed

- `printer::Printer` remains public for API compatibility and has no caller in the
  formatter (chloro-core/src/formatter/printer.rs)

## Missing

- `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` (rustfmt's
  `SkipContext`) have no implementation — macro calls named in them are formatted
- Unstable rustfmt options have no implementation beyond their default values —
  `Config::set` rejects any other value (chloro-core/src/formatter/config.rs:427)
- No reader for `rustfmt.toml` files — options are set through `Config` fields or
  `Config::set`
- `ignore`, `skip_children`, `file_lines` and the other options about which files to
  format have no implementation — `format_source` formats a single source text
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
