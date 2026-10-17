# EXP-008: `rustfmt_version`, reproducing rustfmt 1.8.0 next to 1.9.0

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md); implements the
design in [../rustfmt-versions.md](../rustfmt-versions.md) (the matrix), ledger row E8 in
[../experiments.md](../experiments.md). Labels as there: **measured** (run on this host),
**read** (from source), **hypothesis**.

## Identity

- Branch `20261014-rustfmt-version` from `3771338` (was `7861cd5`) (head of
  `claude/chloro-perf-research-swarm-nrscj8` at the time, the matrix commit); the
  integration branch moved during the work (E2's release profile, `codegen-units = 1`
  and `lto = "fat"`; E4's green-tree reads) and is merged in at `d1705c5` (was `99035f4`) (one import
  conflict in `expr.rs`, one call of `plain_block` that gained a `context` argument in
  `closures.rs`); commits listed under "Commits".
- Toolchains: `rustc 1.97.0 (2d8144b78 2026-07-07)` with `rustfmt 1.9.0-stable (2d8144b788
  2026-07-07)` on `PATH` (the release the snapshots come from); `rustup toolchain install
  1.88.0 --profile minimal --component rustfmt` gives `rustfmt 1.8.0-stable (6b00bc3880
  2025-06-23)`, run as `rustup run 1.88.0 rustfmt`. Every rustfmt invocation reads stdin
  with `--edition <e>` and no `rustfmt.toml`.
- Corpora: the 1220 rust-analyzer fixtures (`chloro-core/tests/conformance/fixtures/rust-analyzer`),
  the 1238-file fixture tree, and a registry fetched from
  [../corpus/registry-manifest.toml](../corpus/registry-manifest.toml) on this host
  (`cargo fetch`, 559 crates, 20,019 `.rs` files under
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`; the matrix's registry had 577
  crates and 20,593 files, so its counts and these are not the same corpus).
- Scratch: probes and outputs under the session scratchpad `probes/` (every reproducer
  below is tracked under `chloro-core/tests/conformance/rustfmt-versions/`).

## Scope decision (orchestrator)

- Supported: 1.9.0 (default, unchanged) and 1.8.0 as built in Rust 1.88.0. The Rust
  1.85.0 build of 1.8.0 formats every file both builds accept identically (matrix, Step 4)
  and differs only in parser acceptance (P3–P6); it is not a separate variant.
- Not supported: 1.7.0. It has no `style_edition` and no Style Guide 2024 rules
  (`--edition 2024` formats under Version One), its Version Two is refused on stable from
  `rustfmt.toml`, and on the fixtures it differs from 1.9.0 on 540 of the 981 files both
  accept (528 in `use` items alone); the matrix's own estimate for it is the style mapping
  plus F1–F6, F8–F11, F14 and P1–P8. `Config::set("rustfmt_version", "1.7.0")` returns an
  error naming this. The README's conformance section records the limitation.
- Exactness: every 1.8.0 class the matrix marks implementable is implemented; the three
  partial cases are measured again below and two of them turn out exact on this build.

## The knob

- `Config.rustfmt_version: RustfmtVersion` (`chloro-core/src/formatter/config.rs`), an enum
  `{ V1_8_0, V1_9_0 }` deriving `Copy, Ord, Hash`, default `V1_9_0`, with `as_str()`.
  `Config::set("rustfmt_version", "1.8.0" | "1.9.0")` sets it; `"1.7.0"` is rejected with
  the limitation in the message; any other value is `unsupported value`. The CLI takes it
  through the existing `--config rustfmt_version=1.8.0` (`chloro/src/cli/args.rs` calls
  `Config::set`). `Settings::rustfmt_version()` reads it next to `style_edition()`.
- Every branch is a comparison of that `Copy` enum at the call site, written as the
  `style_edition` branches are, with a comment naming the class (F-number or P-number of
  the matrix), the rustfmt source lines of both releases and the rustfmt commit, PR or
  CHANGELOG entry.
- The CLI cache key (`chloro/src/cli/cache.rs::config_key_bytes`) gains one trailing byte
  for the field, so a cache file written for one release is not read for the other; the
  pinned default key moves from 48 to 49 bytes.
- `rustc_compat::rejected_by_rustc` and `subtree_rejected_by_rustc` take the version
  (`formatting.rs::parse` and `macro_args.rs::MacroTokens` pass it), since three
  acceptance rules follow the toolchain.
- `RunState.format_failure` (`context.rs`) records a rustfmt panic reproduced on purpose
  (F6): `format_text` and `parallel::format_file` then return `None`, and
  `format_source_with_config` returns the input, as the snapshots expect for a file
  rustfmt writes nothing for. The parallel path or-s the flag of each unit into it.

## Compatibility matrix as implemented (measured)

chloro with `rustfmt_version=1.8.0` against `rustup run 1.88.0 rustfmt`, run live
(`examples/config_matrix --rustfmt`, which counts a file rustfmt rejects as "expected =
input"), and against the snapshots (`examples/conform -V 1.8.0`).

| corpus | edition | files | rustfmt 1.8.0 accepts | identical | differing | class of the difference |
|---|---|---|---|---|---|---|
| rust-analyzer fixtures, `conform -V 1.8.0` | 2024 | 1220 | 1001 | 1219 | 1 | `parser/test_data/parser/err/0024_many_type_parens.rs`, the parser error-recovery case that differs for 1.9.0 too |
| rust-analyzer fixtures, live | 2024 | 1220 | 1001 | 1219 | 1 | the same file |
| rust-analyzer fixtures, live | 2021 | 1220 | 999 | 1219 | 1 | the same file |
| fixture tree, `config_matrix rustfmt_version=1.8.0` | 2024 | 1238 | | 1237 | 1 | the same file |
| fixture tree, `config_matrix rustfmt_version=1.8.0,edition=2021` | 2021 | 1238 | | 1237 | 1 | the same file |
| registry, `config_matrix --root <registry> -n 30000 rustfmt_version=1.8.0` | 2024 | 20,019 | 19,711 | 20,007 | 12 | none version-related: 9 files on which both releases agree and chloro differs under both (`config_matrix default` against rustfmt 1.9.0 gives 19,701/19,711 with the same 9 plus `facet-core-0.30.0/src/macros.rs`, F13, identical under 1.8.0), and 3 files both releases reject (`ra_ap_parser` `0029_range_forms.rs`, `..z = 2`; `ra_ap_syntax` `0261_dangling_impl_undeclared_lifetime.rs`, `impl 'a + Sized`; `rustyline-14.0.0/src/tty/unix.rs`, rustfmt's `left behind trailing whitespace` error) that chloro formats; the `default` run skips rejected files, the versioned run counts them |
| registry subset (every third file), `config_matrix --root <registry> -n 5200 rustfmt_version=1.8.0,edition=2021` | 2021 | 6,673 | | 6,671 | 2 | `ethnum-1.5.3/src/serde.rs`, `image-0.25.10/src/color.rs`: both releases agree, chloro differs under both (the same files as at edition 2024); none of the matrix's F5 registry files (`event-listener`, `nix`, `winapi`, `winnow`) differ |

The fixture rows at edition 2021 include the 176 files rustfmt 1.9.0 rejects there (P1,
`let` chains) and rustfmt 1.8.0 formats: chloro formats them under the 2021 rules and the
output equals rustfmt 1.8.0's on every one. Rejected files (219 at 2024, 221 at 2021) are
returned unchanged, as rustfmt 1.8.0 writes nothing for them.

The 1.9.0 path (gate 2): `bench --check` 0 of 1238 outputs changed against the golden
hashes recorded from the base build at `3771338` (was `7861cd5`); `conform -i` 1219/1220 identical, 0 not
idempotent; the batch tests regenerate the 1220 `ra` snapshots byte for byte (no tracked
snapshot changes).

## Per-class implementation (read + measured)

Each row: the branch, the rustfmt change it reproduces, the test. `v1.8.0`/`v1.9.0` line
numbers are those of the rustfmt tags, as in the matrix.

| class | chloro branch | rustfmt | test |
|---|---|---|---|
| F1 raw labels | `expr.rs::jump_label_text`: under 1.8.0 a `'r#name` label of `break`/`continue` is written `'name` | `v1.8.0 src/expr.rs:201-210` formats `label.ident` (no `r#`); `v1.9.0 src/expr.rs:225-244` copies the snippet; commit `a43eef16`, PR #6425, CHANGELOG 1.9.0 #6411 | `rustfmt_version.rs::f1_raw_labels_in_break_and_continue`; fixtures `f1_*` |
| F2 labelled closure body | `closures.rs`: `get_inner_expr` passes `has_label` only under 1.9.0; `rewrite_last_closure` unwraps a labelled simple block under 1.8.0; `rewrite_closure_block` under 1.8.0 rewrites the inner `Block` with its span starting at the label, so the visitor starts one byte into the label and copies the rest of the label and the `{` as unformatted code | `v1.8.0 src/closures.rs:77` passes `block`, `:123` `needs_block` without label, `:381` unwrap without label check; rustc's `parse_block_common(lo, ..)` gives a labelled block the label's `lo`; `v1.9.0 src/closures.rs:75,136,385`; commit `5619b644`, PR #6468, CHANGELOG 1.9.0 #6465 | `f2_labelled_block_as_closure_body`; fixtures `f2_*` (14 cases) |
| F3 `use self::self` | `imports.rs::UseTree::normalize`: under 1.8.0 a trailing `self` is dropped after `self` as after any segment | `v1.8.0 src/imports.rs:572`; `v1.9.0 src/imports.rs:574-584`; commit `334670e0`, PR #6573, CHANGELOG 1.9.0 #6558 | `f3_use_self_self`; fixtures `f3_*` |
| F4 `super let` | `items.rs` `impl Rewrite for ast::LetStmt`: the early `None` for a `super` token applies under 1.9.0 only; 1.8.0 formats the statement as a `let` | `v1.9.0 src/items.rs:68-71` (rust-side `266da427`); `v1.8.0 src/items.rs:49` | `f4_super_let`; fixtures `f4_*`; snapshot `ra-1.8.0/parser/test_data_parser_inline_ok_let_stmt.rustfmt.rs` |
| F5 `_` import order | `imports.rs` `Ord for UseSegment`: under 1.8.0 and style edition ≤ 2021 the early-return comparator; `sort_use_trees` (used by `normalize` and `reorder.rs`) runs an insertion sort under 1.8.0, which is the standard library's `slice::sort` for at most 20 elements | `v1.8.0 src/imports.rs:945-958`; `v1.9.0 src/imports.rs:953-960`; commit `a57ce886`, CHANGELOG 1.9.0 #6668 | `f5_underscore_import_order_before_2024`; fixtures `f5_*`; partial case below |
| F6 Unicode gap panic | `missed_spans.rs::write_snippet`: under 1.8.0 a blank-line slice advances `line_start` by the newline count; when that is not a char boundary of the snippet, the next code slice or the end of the snippet sets `format_failure` (rustfmt's panic), and a comment slice clears it (rustfmt resets `line_start` there before slicing) | `v1.8.0 src/missed_spans.rs:209`, panic at `:216`; `v1.9.0 src/missed_spans.rs:210`; commit `74931504`, CHANGELOG 1.9.0 #5739 | `f6_unicode_whitespace_in_blank_lines`; fixtures `f6_*` (6 cases) |
| F13 `const { .. }` macro body | none: chloro prints the 1.8.0 output on both paths (const block items are not implemented, the open Missing item of the 2026-10-10 checkpoint) | rust-side `d7923b14` | `f13_macro_const_block` fixture; `facet-core-0.30.0/src/macros.rs` measured below |
| P1 `let` chains ≤ 2021 | `rustc_compat.rs::let_expr_allowed`: a chained `let` in an `if`/`while` condition is accepted before edition 2024 under 1.8.0 | rustc 1.88.0 parses the chain in every edition; rustc 1.97.0 rejects it before 2024 | `p1_let_chains_before_edition_2024`; `rustc_compat::tests::rustc_1_88_acceptance_differs_on_p1_p2_p8`; fixtures `p1_*` |
| P2 `[const]` bounds | `rustc_compat.rs::node_rejected`, `TYPE_BOUND` with `[` and `const` tokens rejected under 1.8.0 | rust-lang/rust#140838 (Rust 1.89.0) adds the syntax | the same tests; fixture `p2_const_bound` |
| P8 negative inherent impl | `rustc_compat.rs::node_rejected`, `IMPL` with `!` and no `for` rejected under 1.9.0 only; `items.rs::format_impl_ref_and_type` writes the polarity before the self type for an inherent impl (ported from `v1.8.0 src/items.rs:985-1046`, present in 1.9.0 too but unreachable there) | rustc 1.97.0: `inherent impls cannot be negative` | the same tests; fixture `p8_negative_impl` |

- F13 on `facet-core-0.30.0/src/macros.rs` (measured): chloro's output equals rustfmt
  1.8.0's byte for byte on both paths; rustfmt 1.9.0 formats the `const { .. }` bodies
  (5 hunks), so the file is a 1.9.0-path difference that predates this branch.
- The probe `macro_const_block.rs` of the matrix (a `macro_rules!` whose body is one
  `const { Some(1) }`) formats identically on 1.8.0 and 1.9.0 on this host: rustc 1.88.0
  reads that body as a statement list, so the matrix's description of the class applies
  to the registry file's shape only.

## Partial cases (measured)

The matrix names three. Each is measured here against `rustup run 1.88.0 rustfmt`.

1. **F2 inside macro calls: exact.** The matrix left the span arithmetic untraced
   (hypothesis). rustc's `parse_block_common(lo, ..)` builds the block of a labelled
   block expression with the label's `lo`; `v1.8.0 src/expr.rs:526` sets the visitor's
   `last_pos` to `block.span.lo()` and `visit_block` adds one byte for the `{`, so the
   missed-span code copies `a: {` (the label without its quote) as unformatted code with
   the block indentation and no newline, then the first statement. chloro reproduces it
   by giving the inner `Block` the label's start. The output is the same text inside and
   outside macro calls, and the matrix's "exact outside macros" was not right either: the
   same path is taken outside a macro whenever the body does not fit on one line without
   a block (`try_rewrite_without_block` fails). Reproducers (all identical to 1.8.0 at
   both editions):
   - `foo!(|| 'a: { break 'a 1; });` → `foo!(|| {        a: {        break 'a 1;` then
     `    });` (`f2_closure_label_macro`);
   - `bar(|| 'b: { some_function_with_a_long_name(argument_one, argument_two, argument_three, argument_four, argument_five) });`
     → `bar(|| {        b: {        some_function_with_a_long_name(` then one argument per
     line (`f2_closure_label_overflow_wide`), and the narrower variant where `bar(` itself
     breaks (`f2_closure_label_overflow`);
   - `foo!(|| 'a: { 1 });` → `foo!(|| { 1 });` (the three-line visitor result is rewritten
     as a single-line block, which drops the copied label).
2. **F5 non-transitive comparator: exact for lists of at most 20 names; partial above.**
   `slice::sort` in Rust 1.81 and later runs `insertion_sort_shift_left` on at most 20
   elements (`core::slice::sort::stable::sort`, `MAX_LEN_ALWAYS_INSERTION_SORT`) and
   driftsort above; rustfmt 1.8.0 (Rust 1.88.0 std) and chloro (Rust 1.97.0 std) share
   that threshold. `sort_use_trees` runs the insertion sort for every length under 1.8.0,
   so a list of at most 20 names, and a longer list without `_`-prefixed names (where the
   comparator is a total order and every stable sort agrees), match rustfmt 1.8.0; a
   longer list that mixes `_`-prefixed names with others may differ. Reproducer
   `rustfmt-versions/partial/f5_use_underscore_long_list.rs` (25 names, `--edition 2021`):
   rustfmt 1.8.0 prints `Ab, _x, ab, Cd, _y, cd, ef, gh, Gh, Ij, _k, _q, ij, mn, op, Mn,
   Op, _p, qr, Qr, _s, EF, KL, ST, _Z` (no panic on this input), chloro prints `_k, ab,
   ij, mn, op, Ab, Cd, Ij, Mn, Op, _p, _x, _y, cd, ef, gh, qr, Gh, Qr, _q, _s, EF, KL, ST,
   _Z`; both outputs are tracked as snapshots and the test checks chloro's. The
   `f5_use_long_list` case (27 names, none `_`-prefixed) is exact. CHANGELOG #6668's
   panic (driftsort's order check) is not modelled: chloro never panics.
3. **F6 panic: exact, including the offsets that land on a char boundary.** The matrix
   expected only the panic to be modelled. Since `write_snippet` is a line-by-line port,
   setting `line_start` to the 1.8.0 value reproduces whatever rustfmt 1.8.0 does with it,
   and the panic is reproduced by `format_failure` exactly where rustfmt slices with the
   misaligned offset. Reproducers (`f6_*`, identical to 1.8.0 at both editions):
   `// compile-flags: x\n\n\n\u{2029}\n` (rustfmt exit 101, chloro returns the input); a
   comment after the gap (`// a\n\n\n\u{2029}\n// b\nfn f() {}`: no panic on either, same
   output); code after the gap at file level and inside a block (exit 101 on 1.8.0);
   `\u{2028}` as the last byte of the gap (offset on a boundary, no panic, same output);
   the gap between two items without a comment (the whitespace-only path, no difference).

## Reference snapshots and regeneration

- `chloro-core/tests/conformance/snapshots/ra-1.8.0/<crate>/<case>.rustfmt.rs` holds the
  1.8.0 output only where it differs from `snapshots/ra/<crate>/<case>.rustfmt.rs`. On the
  current fixtures that is one file, `parser/test_data_parser_inline_ok_let_stmt.rustfmt.rs`
  (F4: `super let y;` → `let y;`). The three P2 fixtures rustfmt 1.8.0 rejects
  (`maybe_const_trait_bound.rs` and its neighbours in `parser/test_data/parser/inline/ok`)
  are already formatted, so their input equals the 1.9.0 snapshot and no 1.8.0 file is
  written; the matrix's count of four assumed otherwise.
- `cargo test -p chloro-core --test conformance batch` with
  `CHLORO_RUSTFMT_VERSIONS=1.8.0=1.88.0` runs `rustup run 1.88.0 rustfmt --edition 2024`
  on every fixture after the 1.9.0 run, writes or removes the `ra-1.8.0` file, and compares
  chloro with `rustfmt_version=1.8.0` against the resolved expectation
  (`helpers/rustfmt.rs::rustfmt_versions`, `batch.rs::update_version_snapshot`). Without
  the variable the batch tests behave as before.
- `chloro-core/tests/conformance/rustfmt-versions/<case>.rs` (32 reproducers) and
  `rustfmt-versions/partial/` (1): `tests/conformance/versions.rs` formats each with both
  releases at editions 2024 and 2021 and compares with
  `snapshots/rustfmt-versions/<case>.<version>.e<edition>.rustfmt.rs` (136 files, the input
  where that rustfmt rejects or panics); the same variable regenerates them (1.9.0 from the
  `rustfmt` on `PATH`). The partial case is checked against its tracked
  `<case>.1.8.0.e<edition>.chloro.rs`.
- `examples/conform -V 1.8.0` reports against "`ra-1.8.0` where present, else `ra`" with
  the formatter at `rustfmt_version=1.8.0`; `examples/config_matrix` takes comma-separated
  option lists, `--rustfmt <command>`, defaults the command to `rustup run 1.88.0 rustfmt`
  for `rustfmt_version=1.8.0`, and counts a rejected file against its input when
  `rustfmt_version` is set.
- `chloro-core/tests/rustfmt_version.rs`: one test per class with the matrix's minimal
  reproducer, asserting the 1.9.0 and the 1.8.0 output, and the `Config::set` test.

## Gates (measured)

1. `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
   `cargo test --workspace --all-features`: clean on the merged tree (225 tests pass, 0
   fail; the batch tests regenerate the 1.9.0 snapshots with no tracked change).
2. 1.9.0 unchanged: `bench --check golden.tsv` (golden recorded from the base build at
   `3771338` (was `7861cd5`); the orchestrator's note confirms no merged change alters an output): 0 of
   1238 changed, before and after the merge; `conform -i`: 1219/1220, 0 not idempotent.
3. 1.8.0: the matrix above; `conform -V 1.8.0`: 1219/1220, before and after the merge.
4. Instructions (`benchmarks/profile.sh`, callgrind `Ir` on the 49-file sample, `bench -n 1`,
   the same sample path for every binary, 2 runs each unless noted):

   | build | profile | Ir min | Ir max |
   |---|---|---|---|
   | best-known-good `966a7d5` (was `aa1e058`) (E4 merged) | release (`codegen-units = 1`, `lto = "fat"`) | 519,086,756 | 519,086,767 |
   | this branch merged (`d1705c5` (was `99035f4`)) | the same | 518,880,865 | 518,880,939 |
   | base `3771338` (was `7861cd5`) | release before E2 (16 codegen units, no LTO) | 588,231,029 | 588,232,849 |
   | this branch before the merge (`c490889` (was `0e712fc`)) | the same | 588,647,548 | (1 run) |
   | base `3771338` (was `7861cd5`) + the `Config` field alone | the same | 588,324,599 | 588,324,621 |
   | base `3771338` (was `7861cd5`) | `-C codegen-units=1` (1 run each) | 580,805,787 | |
   | this branch before the merge | `-C codegen-units=1` | 580,735,451 | |

   On the profile the integration branch now builds with, the merged tree runs 205,891
   fewer instructions than the best-known-good (−0.040%). On the earlier 16-unit profile
   the branch costs +416,519 (+0.071%), of which the `Config` field alone accounts for
   +93,570 (+0.016%; the struct grows from 168 to 176 bytes, `Settings` from 232 to 240);
   with one codegen unit the branch is 70,336 instructions below the base (−0.012%).
   Two earlier versions of the branch measured +0.225% and +0.188% on the 16-unit
   profile: +321,395 of that sat in `rustc_compat::walk` (the recursion gained an
   argument; it now passes one `&mut Walker`), the rest was a change in the inlining of
   `comment::changed_comment_content`'s character comparison across codegen units, which
   the one-unit measurements do not show.

## Surprises

- F2 is exact everywhere once the block's span starts at the label; the broken text is
  not macro-specific.
- F6 is exact in both directions; the "offsets that land on a char boundary" of the matrix
  need no separate model.
- The fixture count for `ra-1.8.0` is one, not four.
- `impl !Foo {}` needed the inherent-impl polarity branch of rustfmt's `format_impl`,
  which the port had left out because 1.9.0's parser never produces it.
- Rust 1.88.0's parser reads a `macro_rules!` body that is one `const { .. }` block as
  statements, so the matrix's F13 probe does not show the class; the registry file does.

## Follow-up

- F5 above 20 names with `_`-prefixed idents: porting driftsort's run detection and merge
  order would make it exact; the order check panic would then also be reproducible.
- `RustfmtVersion` for the Rust 1.85.0 build of 1.8.0 (P3–P6 acceptance) is one more
  variant in `rustc_compat.rs` if wanted; not started.
- Const block items (F13 on the 1.9.0 path) remain the open Missing item.

## Commits

| commit | content |
|---|---|
| `0eac98f` (was `875425c`) | `feat(config)`: `RustfmtVersion`, `Config.rustfmt_version`, `Config::set`, `Settings::rustfmt_version()`, the CLI cache key |
| `1e891ed` (was `3259c1d`) | `feat(rustc_compat)`: P1, P2, P8; `Walker`; inherent-impl polarity in `format_impl_ref_and_type` |
| `9215f61` (was `1f23fd5`) | `feat(expr)`: F1 |
| `cd95765` (was `98f0559`) | `feat(closures)`: F2 |
| `64d5fa8` (was `9298a6a`) | `feat(imports)`: F3, F5, `sort_use_trees` |
| `e59ad4c` (was `9dfef1a`) | `feat(items)`: F4 |
| `28aa21e` (was `2279dad`) | `feat(missed_spans)`: F6, `RunState.format_failure` |
| `c490889` (was `0e712fc`) | `test(conformance)`: snapshots, reproducer suite, unit tests, `conform -V`, `config_matrix --rustfmt` |
| `d1705c5` (was `99035f4`) | merge of `origin/claude/chloro-perf-research-swarm-nrscj8` (`966a7d5` (was `aa1e058`)) |
| the branch head | `docs`: this record, the README conformance section, ledger row E8 |
