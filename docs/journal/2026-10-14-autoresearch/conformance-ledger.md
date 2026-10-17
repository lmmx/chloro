# 2026-10-14: Conformance ledger (rustfmt 1.9.0)

Every mismatch between chloro and `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)` known or
found on 2026-10-14, with a reproducer, the root cause, a classification and its state.
The fixes are on the branch `20261014-conformance-fixes`
(worktree `/home/user/wt/conformance`), which starts at `884f9c9` and has the integration
branch `claude/chloro-perf-research-swarm-nrscj8` merged in three times (at `73d75ce` (was `80e4740`); at
`fe34bd2` (was `8d418ee`), the flat syntax array of E7b; at `ca64d10` (was `b4917ad`), the hot paths of E3). After the
third merge, four commits bring the branch's instruction count on the profile sample below
the base's (section Cost on the profile sample). The final gates ran on `8a8f09d` (was `b0aa3d2`); the
commit that adds this text changes only this file. File and line references to chloro are
to `8a8f09d` (was `b0aa3d2`) unless a commit is named; references to rustfmt are to the `v1.9.0` checkout
at `/home/user/ref/rustfmt-v1.9.0/src`.

## Method

- Reference: `rustfmt` 1.9.0 run as
  `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt --edition 2024`
  on stdin. The rustup proxy `rustfmt` was not used for any count: during the session the
  default toolchain of the container changed for a time to `nightly-2026-02-19` (another
  session's work), and the proxy then failed with "rustfmt is not installed".
- rustfmt 1.8.0: `/root/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin/rustfmt`
  (`1.8.0-stable (6b00bc3880 2025-06-23)`), for the `rustfmt_version = 1.8.0` checks.
- Corpora: the 1238 `.rs` files under `chloro-core/tests/conformance/fixtures` (1220
  rust-analyzer files with snapshots plus 18 `asterism` files), and the cargo registry
  under `~/.cargo/registry/src`, which held 20,847 `.rs` files when the baseline was taken
  and 20,850 at the end (other sessions add crates).
- Gates per commit: `cargo fmt --check`; `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`; `cargo test -p chloro-core`; `conform -i`;
  `bench --check` against `scratchpad/golden/fixtures-884f9c9.tsv` (paths rewritten to
  the worktree); the option matrix and the registry through the comparison tool below.
- Comparison tool (scratch, not in the repository): `scratchpad/conformance/tool`, a binary
  that runs rustfmt once per file and option, caches its output and exit status by content
  hash, and for every file records whether chloro's output equals rustfmt's, whether
  chloro's output is a fixed point, whether chloro panics, and whether chloro parsed the
  file (a `newline_style = Windows` run emits `\r\n` only for a file it formats). A file
  rustfmt rejects counts as unchanged input. Its counts for files rustfmt accepts are the
  counts `examples/config_matrix.rs` reports; the parse flags are what config_matrix does
  not report:
  - "rustfmt rejects, chloro parses": a file chloro would format under some option although
    rustfmt leaves it as it is (invisible under the default options when the file is
    already formatted);
  - "rustfmt accepts, chloro rejects": a file chloro returns unchanged although rustfmt
    formats it (invisible when the file is already formatted).
- The gates ran on each fix commit's own tree before the next commit, except for `00aac21` (was `7978e1c`)
  and `b278aba` (was `f43d45f`), which were committed in a loop whose output was lost to an interruption;
  their gates were run afterwards on copies made with `git archive` (each with its own
  target directory and fresh file times): fmt, clippy and all tests pass, `conform -i`
  1219/1220 with 0 not idempotent, `bench --check` 0 of 1238 changed
  (`scratchpad/conformance/gate-ab.log`, `gate-b.log`).
- Final runs on `716c609` (was `b464243`) with the repository's own examples:
  `config_matrix --rustfmt <stable rustfmt> -n 1220 -v` and
  `config_matrix --rustfmt <stable rustfmt> --root ~/.cargo/registry/src -n 30000 -v default`
  (outputs in `scratchpad/conformance/final/`), pinned with `taskset -c 2`.
- Final gates on `8a8f09d` (was `b0aa3d2`) (`scratchpad/perf/gates.sh`, outputs in `scratchpad/perf/gates/`,
  pinned with `taskset -c 2`): `cargo fmt --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test -p chloro-core`, `conform -i` and
  `conform -i -V 1.8.0`, `bench --check` against both `884f9c9` goldens, the two
  `config_matrix` runs above, and `benchmarks/profile.sh`. The script ran clippy on
  `9468265`, which clippy rejected for one `needless_borrow` in `rustc_syntax.rs`; the
  amended commit `8a8f09d` (was `b0aa3d2`) changes that borrow and passes clippy, and the other gates ran on
  its build.
- Differential check of the cost commits (`scratchpad/perf/zz_edition_record.rs`, an
  example kept out of the repository): built at `8488b22` (was `11e5945`) (before them) and at `9468265`,
  it formats the 1238 fixtures and the 20,850 registry files under `edition` 2015, 2018,
  2021 and 2024 with `rustfmt_version` 1.9.0 and 1.8.0 and records a hash of each output:
  176,704 outputs, identical in the two records. The probe lists of cases 4–12 give the
  same rustc agreement as before (149 of 150 in edition 2015, case 20; 150 of 150 in the
  other editions).

## Counts before and after

Fixtures, default options, `examples/conform.rs -i`:

| | identical | not idempotent |
|---|---|---|
| `884f9c9` | 1219/1220 | 0 |
| `716c609` (was `b464243`), `rustfmt_version = 1.9.0` | 1220/1220 | 0 |
| `716c609` (was `b464243`), `rustfmt_version = 1.8.0` (`conform -i -V 1.8.0`) | 1220/1220 | 0 |
| `8a8f09d` (was `b0aa3d2`), `rustfmt_version = 1.9.0` | 1220/1220 | 0 |
| `8a8f09d` (was `b0aa3d2`), `rustfmt_version = 1.8.0` | 1220/1220 | 0 |

Option matrix over the 1238 fixture files, from the comparison tool (identical / files
rustfmt accepts; "parses" = files rustfmt rejects and chloro parses; "rejects" = files
rustfmt accepts and chloro returns unchanged). `examples/config_matrix.rs --rustfmt <stable
rustfmt> -n 1220 -v` on `78fcb58` (was `729fd36`) gives the same counts: 1022/1022 for each of the 28
values with `--edition 2024`, 844/844, 843/843 and 838/838 for `edition=2021`, `2018` and
`2015`, no differing file (20 min 54 s; `scratchpad/conformance/final/config_matrix-fixtures.txt`).
At `884f9c9` config_matrix reported the "identical" column of the first group (the
2026-10-10 journal records the same counts at `9202b42`).

| option | `884f9c9` identical | parses | rejects | head identical | parses | rejects | not a fixed point (head, all rustfmt's) |
|---|---|---|---|---|---|---|---|
| `array_width=30` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `chain_width=40` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `default` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `edition=2015` | 837/838 | 14 | 3 | 838/838 | 0 | 0 | 0 |
| `edition=2018` | 842/843 | 13 | 3 | 843/843 | 0 | 0 | 0 |
| `edition=2021` | 843/844 | 12 | 3 | 844/844 | 0 | 0 | 0 |
| `fn_call_width=40` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `fn_params_layout=Compressed` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `fn_params_layout=Vertical` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `force_explicit_abi=false` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `hard_tabs=true` | 1020/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `match_arm_leading_pipes=Always` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `match_arm_leading_pipes=Preserve` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `match_block_trailing_comma=true` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `max_width=120` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `max_width=80` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 2 |
| `merge_derives=false` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `newline_style=Windows` | 1019/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `remove_nested_parens=false` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `reorder_imports=false` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `reorder_modules=false` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `short_array_element_width_threshold=4` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `single_line_if_else_max_width=0` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `single_line_let_else_max_width=0` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `struct_lit_width=0` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `style_edition=2015` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `style_edition=2021` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `tab_spaces=2` | 1020/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `use_field_init_shorthand=true` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `use_small_heuristics=Max` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `use_small_heuristics=Off` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |
| `use_try_shorthand=true` | 1021/1022 | 12 | 3 | 1022/1022 | 0 | 0 | 0 |

Registry, default options (`--edition 2024`), the 20,847 files of the baseline list:

| | `884f9c9` | `716c609` (was `b464243`) |
|---|---|---|
| rustfmt accepts, output identical | 20,526 of 20,536 | 20,536 of 20,536 |
| rustfmt rejects (exit 1, no output), chloro returns the input | 307 | 309 |
| rustfmt rejects, chloro changes the file | 2 | 0 |
| rustfmt rejects, chloro parses the file | 13 (12 + rustyline, below) | 1 (rustyline, case 21) |
| rustfmt accepts, chloro returns the file unchanged | 4 (3 + diesel `static_cond.rs`, below) | 1 (diesel `static_cond.rs`, case 24) |
| rustfmt exits 1 with output (internal error), chloro's output equal | 1 | 1 |
| rustfmt does not finish in 120 s (identical when waited for, case 22) | 1 | 1 |
| chloro output not a fixed point | 5, all as rustfmt (below) | 6, all as rustfmt (case 23) |
| chloro panics | 0 | 0 |

`examples/config_matrix.rs --rustfmt <stable rustfmt> --root ~/.cargo/registry/src -n 30000
-v default` on `78fcb58` (was `729fd36`), over the registry as it was at the end (20,850 files): 20,540 of
the 20,540 files rustfmt accepts are identical, no differing file (15 min 51 s;
`scratchpad/conformance/final/config_matrix-registry.txt`). The count includes pest_meta
2.9.2 `src/grammar.rs`, on which rustfmt took 9 min 36 s (case 22).

## Commits on `20261014-conformance-fixes`

| commit | change |
|---|---|
| `0b82b81` (was `72af2ac`) | nested blocks of a macro definition get trailing semicolons (case 1) |
| `34d66b7` (was `e0ff879`) | a repeat array `[x; n]` is not block-like (case 2) |
| `2c43d18` (was `56a373b`) | doc comments in macro arguments are tokens (case 3) |
| `00aac21` (was `7978e1c`) | `rustc_compat` rejects `<-`, `..a = b` and `a..=` (cases 4–6) |
| `b278aba` (was `f43d45f`) | `rustc_compat` rejects ambiguous `+` in types and unbraced const parameter defaults (cases 7, 8) |
| `bbc2e88` (was `5f043e8`) | `rustc_compat` rejects `builtin #` constructs (case 9) |
| `c1ef5d5` (was `ccf6f21`) | `rustc_compat` rejects edition 2015 trait method parameters that are patterns (case 10) |
| `8e054fa` (was `f519a29`) | `rustc_compat` rejects misplaced doc comments and pre-2021 C strings (cases 11, 12) |
| `1da9b5b` (was `83e9fde`) | `rustc_syntax`: parenthesized first bound of a bare trait object (case 13, `0024_many_type_parens.rs`) |
| `169ed0c` (was `0479fd3`) | `rustc_syntax`: float literal field names (case 14, `0054_float_split_scientific_notation.rs`) |
| `c6bdaf1` (was `1be8a3f`) | `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` (case 15) |
| `1073a87` (was `0856215`) | const block items (case 16, facet-core `macros.rs`) |
| `652bae9` (was `7c778e0`) | `impl impl Trait {}` (case 17, `inline/err/impl_type.rs`) |
| `0219b8c` (was `8e7ec67`) | merge of the integration branch at `73d75ce` (was `80e4740`) (E1, E2, E4, E6, E8) |
| `b64675d` (was `b6e858e`) | an unknown literal prefix is no lexer error before edition 2021 (case 18) |
| `78fcb58` (was `729fd36`) | merge of the integration branch at `fe34bd2` (was `8d418ee`) (E7b flat syntax array); the fixes re-expressed on `cst::Tree` |
| `716c609` (was `b464243`) | module map entry of `rustc_syntax` (a doc comment in `formatter.rs`) |
| `0486e62` (was `2ad6240`) | merge of the integration branch at `ca64d10` (was `b4917ad`) (E3 hot paths) |
| `3307555` (was `91e8e02`) | cost: the skip context is an `Option<Rc<SkipContext>>`, and item attributes are read for it only when a path starts with `rustfmt` |
| `8488b22` (was `11e5945`) | cost: const block items take their block from `RunState::block_of` |
| `46f65c9` (was `6b7120f`) | cost: `rustc_compat` in one pass over the flat tree's elements, ancestors found on demand |
| `8a8f09d` (was `b0aa3d2`) | cost: `reparse` reuses the first lex and parse and skips the edits that cannot apply |

Every fix commit carries a regression test: `chloro-core/src/tests/formatter/rustfmt_parity.rs`
(cases 1–3, 13–18, each comparing chloro's output with rustfmt 1.9.0's for the same input)
or the `rustc_compat` tests (cases 4–12, `chloro-core/src/formatter/rustc_compat.rs`,
`accepted_by_rustc`, `rejected_by_rustc_parser`, `edition_dependent_lexing_and_params`).
No existing test or snapshot was changed (`git diff 8d418ee b464243 -- chloro-core/tests
chloro-core/src/tests` adds `rustfmt_parity.rs` and its module line only). At the second
merge the 2015 example of this branch's own `edition_dependent_lexing_and_params` test was
corrected: it put `&self` after other parameters, which rustc rejects, and the flat
`Tree::parse` in the test reports the parse error that `SourceFile::parse` had tolerated.

`bench --check` against `fixtures-884f9c9.tsv`: from `1da9b5b` (was `83e9fde`) on, one output changes,
`parser/test_data/parser/err/0024_many_type_parens.rs` (the file case 13 targets); the
commits before change none (their targets differ from rustfmt only under other options or
in the registry). `bench --check scratchpad/golden/registry-884f9c9.tsv --root
~/.cargo/registry/src` on `78fcb58` (was `729fd36`): 12 of 20,850 outputs change, the targeted files —
axum `method_routing.rs`, combine `stream/mod.rs`, ethnum `serde.rs` (case 1), image
`color.rs` (case 2), diesel `frame_clause.rs`, icu_locale_core `calendar.rs` and
`dictionary_break.rs` (case 3), rayon `iter/extend.rs` (case 4), ra_ap_parser
`ok/0029_range_forms.rs` (case 5), ra_ap_syntax `validation/0261_dangling_impl_undeclared_lifetime.rs`
(case 7), ra_ap_parser `err/0024_many_type_parens.rs` (case 13), facet-core `macros.rs`
(case 16).

## Cost on the profile sample

`benchmarks/profile.sh --bench <examples/bench> --core 2` counts the instructions (callgrind
`Ir`) of `examples/bench` formatting the 49-file sample once. This branch's runs used
`--sample /tmp/chloro-bench/sampleCF`, a path of the length of the default one; the count
moves with the sample path by a few hundred thousand instructions, not with the code.

| build | Ir, `sampleCF` | against the base |
|---|---|---|
| `ca64d10` (was `b4917ad`) (integration branch, the base) | 316,299,962 | |
| `0486e62` (was `2ad6240`) (merge) | 326,538,280 | +3.24 % |
| `3307555` (was `91e8e02`) | 323,097,065 | +2.15 % |
| `8488b22` (was `11e5945`) | 323,014,855 | +2.12 % |
| `46f65c9` (was `6b7120f`) | 313,471,141 | −0.89 % |
| `8a8f09d` (was `b0aa3d2`) | 312,736,806 | −1.13 % |

With the default sample path (`/tmp/chloro-bench/sample49`): `ca64d10` (was `b4917ad`) 316,299,929,
`8a8f09d` (was `b0aa3d2`) 312,736,691 (−1.13 %). The coordinator's count for `ca64d10` (was `b4917ad`) on that path,
316,076,716, comes from a bench binary at another path; the limit set for this branch is
317.7 M (+0.5 % of that count).

Where the merge's 10,238,318 instructions went (callgrind, `ca64d10` (was `b4917ad`) against `0486e62` (was `2ad6240`)):

- `rustc_compat`: 15.23 M inclusive against 9.56 M. The rules added on this branch read
  every token's text, kept doc comment and token tree state per token, and scanned the
  children of every binary expression for `<-`, `..a = b` and comparisons; the recursive
  walk kept the path from the root. At `8a8f09d` (was `b0aa3d2`) the rules cost 5.95 M inclusive: one loop
  over the elements, a token's text read only for a comment, the macro input of a token
  tree scanned for the `:::` rule only, one look at a binary expression's operator, and
  ancestors searched for only after a rule's other conditions hold (`rustc_compat.rs:43`).
- The skip context: 3.44 M (`3307555` (was `91e8e02`)). `RewriteContext::new` allocated an
  `Rc<SkipContext>` with two `HashSet`s, dropped when `get_context` replaced it;
  `visit_item` cloned the context and ran two `format!` and a `path_text` per attribute of
  every item; `Attribute::rewrite` built every attribute's path text.
- `reparse` on the sample's files with syntax errors: a second lex and parse, a deep copy
  and three copies of the owned tree; 0.26 M inclusive at `8a8f09d` (was `b0aa3d2`).
- `Block::from_block_expr`: a second caller kept it out of line in `RunState::block_of`
  (0.08 M, `8488b22` (was `11e5945`)).

## Cases fixed

### 1. Trailing semicolons in nested blocks of a macro definition

- Reproducer: `macro_rules! m { () => { if a { return 1 } }; }`
- rustfmt 1.9.0: `return 1;` inside the `if` block; a trailing `return` directly in the
  body keeps no semicolon.
- chloro at `884f9c9`: `return 1` (no semicolon) in every block of the body; the extra
  column moved line breaks (axum 0.7.9 `src/routing/method_routing.rs`, combine 4.6.8
  `src/stream/mod.rs`, ethnum 1.5.3 `src/serde.rs`).
- Root cause: rustfmt's `FmtVisitor::from_context` builds the nested visitor with
  `from_psess`, which sets `is_macro_def: false` (rustfmt `visitor.rs:803-812`,
  `visitor.rs:842`); only the snippet's top-level visitor omits the semicolon
  (`visitor.rs:1067-1075`, `utils.rs:289-303`). chloro copied `is_macro_def` into nested
  visitors (`visitor.rs:108` at `884f9c9`).
- Classification: rustfmt behaviour not ported.
- Fix: `0b82b81` (was `72af2ac`) (`visitor.rs:137`, `from_context`).

### 2. A repeat array is not block-like

- Reproducer: `let a = [aaaa..; bbbb..].into();` longer than a line.
- rustfmt 1.9.0: `.into()` one block indent deeper than the `let`.
- chloro at `884f9c9`: `.into()` at the `let`'s indent (image 0.25.10 `src/color.rs`); a
  match arm body or last argument that is a repeat array laid out as a block.
- Root cause: rust-analyzer has one `ARRAY_EXPR` for `[a, b]` and `[x; n]`; rustc has
  `ExprKind::Array` and `ExprKind::Repeat`, and rustfmt lists only `Array` in
  `is_block_expr` (`utils.rs:480`, `Repeat` at `utils.rs:527`), `can_flatten_block_around_this`
  (`matches.rs:627`) and `can_be_overflowed_expr` (`expr.rs:1536`). chloro matched every
  `ArrayExpr` (`utils.rs:232`, `matches.rs:630`, `expr.rs:1375` at `884f9c9`).
- Classification: adaptation-layer bug (node kind coarser than rustc's).
- Fix: `34d66b7` (was `e0ff879`) (`nodes.rs:295` `is_repeat_array`; `utils.rs:354`, `matches.rs:640`,
  `expr.rs:1403`).

### 3. Doc comments in macro arguments

- Reproducer: `m!(\n    /// doc\n    A, B);`, `m!(a, /// doc\n);`, `m!(a /// doc\n, b);`
- rustfmt 1.9.0: each left as written; `m!(struct A; /// doc\n struct B;)` gets the doc
  comment on its own line as the attribute of `struct B`.
- chloro at `884f9c9`: formats the calls (diesel 2.3.14
  `src/expression/functions/aggregate_expressions/frame_clause.rs`, icu_locale_core 2.3.0
  `src/preferences/extensions/unicode/keywords/calendar.rs` and `dictionary_break.rs`).
- Root cause: rustc's lexer makes a doc comment a `DocComment` token, rust-analyzer's
  makes it trivia. rustfmt's `parse_macro_arg` tries a fragment kind only when
  `Parser::nonterminal_may_begin_with` allows it, which is false for a doc comment except
  for items (rustfmt `parse/macros/mod.rs:24-72`), and `parse_macro_args` returns `None`
  for a token after an argument that is not `,`, `;` or the end
  (`parse/macros/mod.rs:98-150`). chloro skipped doc comments with all trivia
  (`macro_args.rs:119` at `884f9c9`).
- Classification: adaptation-layer bug (lexer difference).
- Fix: `2c43d18` (was `56a373b`) (`macro_args.rs:176` `doc_at`, `macro_args.rs:415` `parse_macro_arg`,
  `macro_args.rs:431` `parse_macro_args`).

### 4–12. Syntax rust-analyzer's parser accepts and rustc's rejects

Each file below is a parse error to rustc, so rustfmt leaves it as it is; chloro formatted
it (visible under `hard_tabs`, `tab_spaces = 2`, `newline_style = Windows`, or under the
default options where the file is not already formatted). Classification of all nine:
rustc parse error not modelled in `rustc_compat` (rust-analyzer accepts the syntax or
reports it only in its validation pass, which chloro does not run). Rules in
`chloro-core/src/formatter/rustc_compat.rs`; the probe lists are
`scratchpad/conformance/cases_compat.txt` and `cases2.txt` (150 inputs; on `716c609` (was `b464243`)
rustc's acceptance and chloro's agree on all of them in editions 2018, 2021 and 2024, and
on all but `trait T { fn f(usize); }` in 2015, case 20).

| case | reproducer | rustc's error | rule | commit | evidence |
|---|---|---|---|---|---|
| 4 | `a <- b` | `<` and an adjacent `-` lex as `<-`, "unexpected token: `<-`" | `bin_expr_rejected`, `rustc_compat.rs:178` | `00aac21` (was `7978e1c`) | rayon 1.12.0 `src/iter/extend.rs` (`extend!($self <- vecs)` in a `macro_rules!` body, formatted by chloro as `$self < -vecs`) |
| 5 | `..z = 2;` | a range that starts an expression ends it: "expected one of ..., found `=`" (accepted where an outer operator loop continues: `x = ..z = 2`, `|x| ..z = 2`, `return ..z = 2`) | `prefix_range_assigned`, `rustc_compat.rs:233` | `00aac21` (was `7978e1c`) | ra_ap_parser 0.0.307 `test_data/parser/ok/0029_range_forms.rs` |
| 6 | `0..=;`, `match x { 0..= => {} }` | E0586 inclusive range with no end | `inclusive_without_end`, `rustc_compat.rs:254` | `00aac21` (was `7978e1c`) | ra_ap_syntax 0.0.307 `validation/0038_endless_inclusive_range.rs` |
| 7 | `&dyn A + B`, `*const impl A + B`, `x as dyn A + B`, `fn() -> dyn A + B`, `Fn() -> impl A + B` | "ambiguous `+` in a type" (`AllowPlus::No`) | `ambiguous_plus`, `rustc_compat.rs:268` | `b278aba` (was `f43d45f`) | `validation/0045_ambiguous_trait_object.rs`, `validation/0261_dangling_impl_undeclared_lifetime.rs` (`&impl 'a + Sized`) |
| 8 | `struct A<const N: i32 = i32::MAX>;` | "expressions must be enclosed in braces to be used as const generic arguments" | `const_param_default_rejected`, `rustc_compat.rs:309` | `b278aba` (was `f43d45f`) | `inline/ok/const_param_default_path.rs` |
| 9 | `builtin # asm("")`, `builtin # format_args(..)`, `builtin#offset_of(..)` (2021+), `builtin # offset_of(Foo, (bar))` | unknown `builtin #` construct; reserved prefix; "offset_of expects dot-separated field and variant names" | `offset_of_rejected`, `rustc_compat.rs:290`, and `ASM_EXPR`/`FORMAT_ARGS_EXPR` | `bbc2e88` (was `5f043e8`) | `inline/ok/asm_expr.rs`, `asm_kinds.rs`, `asm_label.rs`, `builtin_expr.rs`, `global_asm.rs`, `offset_of_parens.rs` |
| 10 | edition 2015: `trait T { fn f((a, b): (u8, u8)) {} }` | E0642 (anonymous parameters: a pattern must start like `name:`) | `trait_param_rejected`, `rustc_compat.rs:375` | `c1ef5d5` (was `ccf6f21`) | `test_data/parser/ok/0063_trait_fn_patterns.rs` under `edition = 2015` |
| 11 | `struct S { a: u8, /// doc\n }`, `fn f() {} //! doc`, `#[a]\n//! doc\nfn f() {}` | "found a documentation comment that doesn't document anything", "expected item after doc comment", "expected outer doc comment" | `Walker::doc_token`, `rustc_compat.rs:511`, and `walk`, `rustc_compat.rs:565` (an outer doc comment followed by `}` `)` `]` `,` `;` `=>` `=` `>` or the end of the file; an inner doc comment outside the leading position of a file or module, block, `impl`/`trait`, `extern` or `match` body) | `8e054fa` (was `f519a29`) | `test_data/lexer/ok/single_line_comments.rs` |
| 12 | edition 2015/2018: `let _ = c"g";` | `c` is an identifier, the string a separate token | `Walker::doc_token`, `rustc_compat.rs:511` | `8e054fa` (was `f519a29`) | `inline/ok/expr_literals.rs` under `edition = 2015` and `2018` |

rustfmt 1.8.0 (rustc 1.88.0) and rustfmt 1.9.0 give the same output and exit status on
every input of the probe lists for cases 4–12 with `--edition 2024`; the inputs where they
differ are the const block items of case 16.

### 13. Parenthesized first bound of a bare trait object (`0024_many_type_parens.rs`)

- Reproducer: `let _: Box<(?Sized) + (for<'a> Trait<'a>) + (Copy)>;`
- rustfmt 1.9.0: `Box<?Sized + (for<'a> Trait<'a>) + (Copy)>`; `Box<(Copy) + Send>` stays;
  `Box<(?Sized)>` stays.
- chloro at `884f9c9`: the file unchanged — rust-analyzer's parser rejects all three forms.
- Root cause: rustc's `parse_ty_tuple_or_parens` reads `(Path) + ..` as a bare trait object
  whose first bound is the path, with the parentheses in the bound's span
  (`parse_remaining_bounds_path(.., parse_plus = true)`), and `(TraitObject) + ..` with a
  single bound and no trailing `+` as a bare trait object whose first bound is that bound,
  without them (`parse_remaining_bounds`); `(?Sized)` alone is a parenthesized trait
  object. rustfmt prints a bound's parentheses when the text of its span has them
  (rustfmt `types.rs:642-645`). rust-analyzer starts a bare trait object only at a path and
  has no type starting with `?`. The lead given for this case (a type-rewriting rule alone)
  does not reach the file, which never parsed.
- Classification: rustc grammar rust-analyzer's parser lacks; representable in
  rust-analyzer's tree (`DYN_TRAIT_TYPE` without `dyn`, `TYPE_BOUND` with parentheses).
- Fix: `1da9b5b` (was `83e9fde`). `rustc_syntax::with_inserted_dyn` (`rustc_syntax.rs:263`) parses the
  file again with `dyn ` inserted before the `(` (or before the `?` when no `+` follows),
  one insertion per round at the first error (`insertions`, `rustc_syntax.rs:437`; the
  context must allow `+` in rustc and the parenthesized type must be a path without a
  qualified self or `for<..>` and a path), and leaves the inserted tokens out of the tree;
  `parens_left_by_rustc` (`types.rs:660`) drops the parentheses of the first bound when
  they hold a `?` bound or a `for<..>` binder. 29 probes give rustfmt's output, nine of
  them forms rustc rejects and chloro leaves as written (`&(Copy) + Send`,
  `((Copy)) + Send`, `(Copy + Sync) + Send`, `(<T as X>::Y) + Send`, `('a) + Send`,
  `(for<'a> fn(&'a u8)) + Send`, `(dyn Copy) + Send`, `Box<(?Sized + ) + Copy>`, and a
  file with a missing `;`).

### 14. A float literal without `.` as a field name (`0054_float_split_scientific_notation.rs`)

- Reproducer: `let a = s.1e0;` (differs under `hard_tabs`, `tab_spaces = 2`,
  `newline_style = Windows`).
- rustfmt 1.9.0: formats the file, `s.1e0` as written.
- chloro at `884f9c9`: the file unchanged.
- Root cause: rustc's `break_up_float` reads `1e0` as one component
  (`DestructuredFloat::Single`), a field name; the error (no such field) comes from type
  checking. rust-analyzer's tree builder reports "illegal float literal" (its error
  position is a token index, not a text offset) and wraps the literal in `ERROR`.
- Classification: rustc grammar rust-analyzer's tree builder lacks; representable
  (`NAME_REF` holding the literal).
- Fix: `169ed0c` (was `0479fd3`), `float_fields_as_names` (`rustc_syntax.rs:498`). A literal with a sign or
  a suffix (`1e+2`, `1e0f32`) stays an error, as in rustc.

### 15. `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]`

- Reproducer: `#[rustfmt::skip::macros(foo)]\nfn f() {\n    foo!(  a,b  );\n}`
- rustfmt 1.9.0: `foo!(  a,b  );` as written; `bar!` and `a::foo!` formatted;
  `skip::attributes(derive)` keeps derives unmerged.
- chloro at `884f9c9`: every call formatted (listed as Missing in the 2026-10-10 journal).
- Root cause: rustfmt's `SkipContext` (`skip.rs`), updated from item attributes in
  `visit_item` (`visitor.rs:443-444`, restored at `visitor.rs:628`) and from the crate's
  attributes in `format_file` (`formatting.rs:218`), read in `rewrite_macro`
  (`macros.rs:161-166`), `Attribute::rewrite` (`attr.rs:335-341`) and `rewrite_attrs`
  (`attr.rs:403`).
- Classification: rustfmt feature not ported. No registry file uses it (0 matches for
  `rustfmt::skip::` in the registry).
- Fix: `c6bdaf1` (was `1be8a3f`) (`skip.rs`, `visitor.rs:405`, `visitor.rs:418`, `macros.rs:184`,
  `attr.rs:505`, `attr.rs:548`; the context holds it as an `Option<Rc<SkipContext>>` since
  `3307555` (was `91e8e02`)); the parallel path (`parallel.rs`) reads the file's inner
  attributes into each worker's context.

### 16. Const block items (facet-core 0.30.0 `src/macros.rs`)

- Reproducer: `pub const {   a( ) ; }` in a file; a `macro_rules!` body that is a single
  `const { .. }` holding a macro call rustfmt cannot format.
- rustfmt 1.9.0: `const {\n    a();\n}` (the visibility dropped); in the macro body, the
  block formatted with the failing call left as written, which leaves an `if` line at the
  wrong indent (rustfmt's output, not idempotent; chloro's second pass equals rustfmt's).
- chloro at `884f9c9`: the file unchanged; the macro body formatted through
  `format_code_block` as a statement, where the inner failure leaves the body unchanged.
- Root cause: rustc parses `const { .. }` in item position as `ItemKind::ConstBlock`
  (unstable `const_block_items`); rustfmt formats it with `get_context` (rustfmt
  `visitor.rs:539-558`), so a macro failure inside does not reach the visitor and
  `format_snippet` keeps the body. rust-analyzer reports "expected an item".
- Classification: unstable-feature syntax rust-analyzer's parser lacks; representable as a
  `CONST` item without a name whose body is a `BLOCK_EXPR`.
- Fix: `1073a87` (was `0856215`) (`rebuild_const_blocks`, `rustc_syntax.rs:567`; `const_block_item`,
  `nodes.rs:302`; `visitor.rs:509`). Under `rustfmt_version = 1.8.0` the edit is not made:
  rustc 1.88.0's parser rejects const block items (probed with rustfmt 1.8.0).

### 17. `impl impl Trait {}` (`inline/err/impl_type.rs`)

- Reproducer: `impl impl NotType {}`, `impl Trait2 for impl NotType {}` (differs under
  `newline_style = Windows`).
- rustfmt 1.9.0: formats the file.
- chloro at `884f9c9`: the file unchanged.
- Root cause: rustc's parser accepts `TyKind::ImplTrait` as the self type (AST validation
  reports E0562 later); rust-analyzer's `impl_type` reports "expected trait or type",
  ends the impl and parses `impl NotType {}` as a second impl.
- Classification: rustc grammar rust-analyzer's parser lacks; representable
  (`IMPL_TRAIT_TYPE` as the impl's type).
- Fix: `652bae9` (was `7c778e0`) (`rebuild_impl_self_types`, `rustc_syntax.rs:693`); `impl impl A for B {}`
  stays an error, as in rustc.

### 18. Unknown literal prefixes before edition 2021

- Reproducer: `m!(foo#bar,   x);\nfn  f() {}` with `edition = 2018`.
- rustfmt 1.9.0 `--edition 2018`: `fn f() {}` formatted, the call as written (its
  arguments do not parse); `--edition 2021`: "prefix `foo` is unknown", file rejected.
- chloro at `884f9c9`: the file unchanged in every edition.
- Root cause: rust-analyzer's lexer reports "unknown literal prefix" for an identifier
  followed by `#`, a string or a character in every edition (`ra_ap_parser-0.0.307
  src/lexed_str.rs`, `UnknownPrefix`); rustc reserves the prefixes from edition 2021.
- Classification: lexer edition difference.
- Fix: `b64675d` (was `b6e858e`) (`lexer_rejects`, `rustc_syntax.rs:60`, for the file and for macro
  arguments).

## Cases open

### 19. Doc comments as attributes of expressions

- Reproducers and rustfmt 1.9.0 output against chloro's:
  - `foo(/// d\n a,   b);` — rustfmt `foo(\n    /// d\n    a,\n    b,\n);`, chloro
    `foo(\n    /// d\n    a, b,\n);`
  - `[/// d\n 1, 2];` — the same pattern with an array.
  - `let x = /// d\n 1;` — rustfmt indents the `1` under the `let`, chloro leaves it at
    column 0 after the comment.
  - `a; /// d\n b` (the doc comment after `a;` on the same line) — rustfmt moves it to
    its own line before `b`, chloro keeps it after `a;`.
- Root cause: rustc parses a doc comment before an expression as an outer attribute of the
  expression; rustfmt's expression span includes its attributes (rustfmt `spanned.rs:38-56`)
  and `format_expr` rewrites them with the expression (rustfmt `expr.rs:490-498`), so a list
  item with one is multi-line. chloro's `outer_attributes` reads attributes among a node's
  children (`nodes.rs:169`); rust-analyzer leaves a doc comment before an expression outside
  the expression's node.
- Classification: adaptation-layer bug (attribute attachment), open.
- Plan: treat the doc comments that precede an expression in an argument list, an array, a
  `let` initializer or a block's tail as `Attribute::Doc` of the expression, and widen the
  expression's span to them as rustfmt's `span_with_attrs` does. No registry file of the
  corpus differs for this reason (doc comments on expressions trigger rustc's
  `unused_doc_comments` warning).

### 20. Edition 2015 anonymous trait method parameters

- Reproducer: `trait T { fn f(usize); }` with `edition = 2015`.
- rustfmt 1.9.0: formats it; chloro: the file unchanged (rust-analyzer's parser rejects a
  parameter without a pattern in a trait method).
- Classification: rustc grammar (edition 2015) rust-analyzer's parser lacks; representable
  (`PARAM` with only a type, as in `fn(u8)` pointer types). Open; a tree edit in
  `rustc_syntax` in the style of cases 16 and 17 is the plan.

### 21. rustfmt's internal error on rustyline 14.0.0 `src/tty/unix.rs`

- rustfmt 1.9.0 exits 1 with "error[internal]: left behind trailing whitespace" (a comment
  line of its own output, line 491) and still writes the formatted file to stdout
  (rustfmt `formatting.rs:543-553` records `ErrorKind::TrailingWhitespace`, `lib.rs:230-233`
  counts it as an operational error, `bin/main.rs:323` turns that into exit code 1).
- chloro's output is byte-for-byte rustfmt's stdout; chloro reports no error.
- Classification: rustfmt reports an error and formats; chloro formats.
- Decision: chloro does not mirror the failure. `format_source` has no error channel, and
  returning the input unchanged would differ from the text rustfmt writes. The 2026-10-14
  E8 registry report lists this file among those "both rustfmt releases reject"; by exit
  status it is, by output it is not.

### 22. rustfmt's run time on pest_meta 2.9.2 `src/grammar.rs`

- The file is 47 KB of generated code on 2 lines. rustfmt 1.9.0 takes 9 min 36 s on an
  idle core (`taskset -c 2`), more than the comparison tool's 120 s limit, so the tool
  lists it as "does not finish"; chloro formats it in 0.2 s, and the two outputs are
  identical (`config_matrix` waits without a limit and counts it as identical).
- Not a mismatch; recorded because the comparison tool's counts leave the file out.

### 23. Non-idempotent outputs that are rustfmt's

- image 0.25.10 `src/codecs/pnm/decoder.rs`, naga 22.1.0 `src/back/msl/writer.rs`,
  `src/back/spv/writer.rs`, `src/valid/expression.rs`, rustyline 14.0.0 `src/tty/unix.rs`
  (default options), facet-core 0.30.0 `src/macros.rs` (after case 16), and
  `hir-def/src/nameres/path_resolution.rs`, `ide-db/src/rename.rs` (fixtures,
  `max_width = 80`): chloro's output is not a fixed point, and rustfmt's own output is not
  either; chloro applied to rustfmt's first output equals rustfmt's second output in every
  one. Typical form: a match arm `=> return Err(..),` becomes a block with `return ..;`
  whose semicolon makes the line overflow, which the next pass breaks.

### 24. Minor forms recorded, not fixed

- `builtin # offset_of(Foo, bar,)` (trailing comma): rustc accepts it, rust-analyzer
  rejects it; rustfmt copies `offset_of` as written. Only the surrounding text could
  differ.
- diesel 2.3.14 `src/macros/static_cond.rs` (`#![cfg_attr(rustfmt, rustfmt_skip)]`): both
  return the file as written; the comparison tool lists it under "chloro returns the file
  unchanged" because its parse check cannot tell an echoed file from a rejected one.
- `default const impl Trait for T {}` and `default const unsafe impl ..` (specialisation of
  a const trait impl, both unstable; found while adding rustfmt 1.10.0, 2026-10-17):
  rust-analyzer reads `default const` as the start of a `const` item and reports "expected a
  name", so chloro leaves the file unchanged; rustfmt 1.9.0 writes `default impl<T> const
  Trait`, rustfmt 1.10.0 `default const impl<T> Trait`.

## Policy

- A file with `#![rustfmt::skip]` is returned byte for byte, where `rustfmt` reading stdin
  echoes the normalised text (no byte order mark, `\n` line endings). chloro follows
  `rustfmt` run on a file, which leaves a skipped file untouched; this is the one recorded
  Divergence of `docs/journal/2026-10-10-performance-checkpoint.md` and is unchanged.

## Current State

- chloro formats all 1220 conformance fixtures as rustfmt 1.9.0 does, and as rustfmt 1.8.0
  does under `rustfmt_version = 1.8.0`, with every output a fixed point
  (`examples/conform.rs -i`, `-V 1.8.0`)
- Files that rust-analyzer's lexer or parser rejects are parsed again by
  `chloro-core/src/formatter/rustc_syntax.rs` for the five constructs rustc accepts (cases
  13, 14, 16, 17, 18), from an owned tree of the parser's steps laid out with
  `cst::TreeBuilder` over the source (`rustc_syntax.rs:213`), from the lexer's and the
  parser's output of the first parse (`formatting.rs:45`)
- `rustc_compat` rejects the syntax of cases 4–12 for files and macro arguments
  (`node_rejected`, `rustc_compat.rs:397`; `Walker`, `rustc_compat.rs:474`), in one pass
  over the flat tree's elements (`walk`, `rustc_compat.rs:565`)
- Macro arguments treat doc comments as rustc's lexer does (`macro_args.rs:176`)
- `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` apply inside the
  item that carries them (`skip.rs`, `visitor.rs:405`)
- Const block items format as rustfmt's `ItemKind::ConstBlock` (`visitor.rs:509`)
- `config_matrix -n 1220` reports every fixture identical for all 31 option values (9 min
  56 s), and `config_matrix --root ~/.cargo/registry/src -n 30000 default` 20,540 of 20,540
  registry files identical (14 min 25 s), on `8a8f09d` (was `b0aa3d2`) as on `78fcb58` (was `729fd36`)
- `bench --check` against the `884f9c9` hashes changes 1 of 1238 fixture outputs and 12 of
  20,850 registry outputs, the files the fixes target, on `8a8f09d` (was `b0aa3d2`)
- `benchmarks/profile.sh` counts 312,736,691 instructions on the 49-file sample at
  `8a8f09d` (was `b0aa3d2`) against 316,299,929 at the base `ca64d10` (was `b4917ad`) (default sample path); `rustc_compat`
  costs 5.95 M of them

## Missing

- Doc comments before expressions are not attributes of the expressions (case 19)
- Edition 2015 anonymous trait method parameters do not parse (case 20)

## Divergence

- A file with `#![rustfmt::skip]` is returned byte for byte where `rustfmt` reading stdin
  echoes the normalised text (Policy above)
- rustfmt's internal error on rustyline 14.0.0 `src/tty/unix.rs` has no counterpart in
  chloro, which returns the same text without an error (case 21)

## Reproduce

```sh
cd /home/user/wt/conformance
cargo build --release -p chloro-core --examples
cd chloro-core
../target/release/examples/conform -i
../target/release/examples/conform -i -V 1.8.0
RUSTFMT=/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt
taskset -c 2 ../target/release/examples/config_matrix --rustfmt $RUSTFMT -n 1220 -v
taskset -c 2 ../target/release/examples/config_matrix --rustfmt $RUSTFMT \
    --root ~/.cargo/registry/src -n 30000 -v default
# fixture hashes recorded at 884f9c9, paths rewritten to this tree
sed 's#/home/user/chloro/#/home/user/wt/conformance/#' \
    /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/golden/fixtures-884f9c9.tsv > /tmp/golden.tsv
../target/release/examples/bench --check /tmp/golden.tsv
../target/release/examples/bench --check /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/golden/registry-884f9c9.tsv \
    --root ~/.cargo/registry/src
# instruction count on the 49-file sample
cd ..
benchmarks/profile.sh --bench target/release/examples/bench --core 2
benchmarks/profile.sh --bench target/release/examples/bench --core 2 \
    --sample /tmp/chloro-bench/sampleCF
```
