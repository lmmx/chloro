---
description: "What chloro's output has been compared against, rustfmt 1.9.0 and 1.8.0, the options covered, and the known differences."
---

# rustfmt compatibility

<p class="lede">chloro aims to produce exactly the bytes rustfmt produces. This page lists
what its output has been compared against, how the comparisons were made, and what they do
not cover.</p>

Each result below compares chloro's output with the output of a named rustfmt binary on the
same source with the same options. The comparisons cover the corpora and options listed
here; they do not show that chloro and rustfmt agree on every possible input.

## Reference versions

| | |
|---|---|
| rustfmt release reproduced by default | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, shipped with Rust 1.97.0 |
| second release, with `rustfmt_version = "1.8.0"` | `rustfmt 1.8.0-stable (6b00bc3880 2025-06-23)`, shipped with Rust 1.88.0 |
| how rustfmt is run | `rustfmt --edition 2024` (or the edition under test) on standard input, with no `rustfmt.toml` |
| what counts as identical | the same bytes; where rustfmt rejects a file, the expected output is the input, unchanged |

## Results

| what varies | values | corpus | identical |
|---|---|---|---|
| rustfmt release | 1.9.0, the default | 1220 rust-analyzer source files, default options | 1220 of 1220, none non-idempotent |
| rustfmt release | 1.8.0 | the same | 1220 of 1220, none non-idempotent |
| stable options | 31 non-default values of 24 options (below) | the same, every file rustfmt accepts under the option | every file (1022 of 1022 for each value at edition 2024) |
| edition | 2015, 2018, 2021, 2024 | the same | 838/838, 843/843, 844/844, 1022/1022 |
| style edition | 2015, 2021, 2024 | the same | 1022/1022 each |
| code chloro was not developed on | default options, rustfmt 1.9.0 | 20,850 files of 577 crates from crates.io | 20,540 of the 20,540 files rustfmt formats |
| the same at rustfmt 1.8.0 | default options | 20,019 files of 559 crates, measured before the last conformance fixes | 20,007 of 20,019: 9 files the later fixes target, 3 that both releases reject |

Notes on the table:

- **The rust-analyzer corpus.** 1238 `.rs` files from rust-analyzer are checked in under
  `chloro-core/tests/conformance/fixtures/`, with rustfmt's output for each stored as a
  snapshot. 1220 of them are compared; 216 of those are files rustfmt rejects, where the expected
  output is the input.
- **Non-idempotent** counts files whose chloro output, formatted again, changes. None does.
- **The option rows** count only the files rustfmt accepts under the option, so an edition
  that rejects more files has a smaller denominator.
- **The registry corpus** is 577 crates downloaded from crates.io by a committed manifest,
  compared live against rustfmt. Every change merged during the performance work was also
  checked to leave the output of every file in both corpora unchanged, except for the files
  a conformance fix was meant to change.

### Options

The option values compared with rustfmt 1.9.0:

`max_width` 80 and 120, `hard_tabs`, `tab_spaces` 2, `use_small_heuristics` Max and Off,
`fn_call_width` 40, `chain_width` 40, `struct_lit_width` 0, `array_width` 30,
`single_line_if_else_max_width` 0, `single_line_let_else_max_width` 0, `reorder_imports`
false, `reorder_modules` false, `remove_nested_parens` false,
`short_array_element_width_threshold` 4, `match_arm_leading_pipes` Always and Preserve,
`fn_params_layout` Vertical and Compressed, `match_block_trailing_comma`, `merge_derives`
false, `use_try_shorthand`, `use_field_init_shorthand`, `force_explicit_abi` false,
`newline_style` Windows, `style_edition` 2021 and 2015, `edition` 2021, 2018 and 2015.

`attr_fn_like_width`, `struct_variant_width` and `disable_all_formatting` are accepted but
not part of the comparison.

## rustfmt releases

rustfmt 1.8.0 and 1.9.0 differ in two ways, and `rustfmt_version` reproduces both:

- **Formatting.** A handful of constructs come out differently, for example raw labels in
  `break 'r#a` (1.8.0 drops the `r#`), a labelled block as a closure body, `use self::self;`,
  `super let`, the import order of `_`-prefixed names at editions before 2024, and a
  `macro_rules!` body that is a single `const { .. }` block.
- **Acceptance.** Each release's rustfmt formats only what its own toolchain's parser
  accepts. For example, rustfmt 1.8.0 formats `if a && let …` at edition 2021 and
  `impl !Trait {}`, which 1.9.0 rejects and leaves alone, and rejects `[const]` bounds,
  which 1.9.0 formats.

On the rust-analyzer corpus at edition 2024, the two releases give identical output on all
but one of the files both accept. A reproducer for each difference, at both releases and
editions 2021 and 2024, is in `chloro-core/tests/conformance/rustfmt-versions/`; the
[rustfmt versions record][versions] names each class and the rustfmt change behind it.

rustfmt 1.7.0 and earlier are refused: 1.7.0 has no 2024 Style Guide rules, and 540 of the
981 rust-analyzer files both releases accept come out differently. Nightly rustfmt is not
compared.

## What is not covered

- **`rustfmt.toml`.** chloro reads no configuration file. Options come from `--config` or
  `Config`.
- **Unstable options.** chloro always uses rustfmt's defaults for them, and setting one to
  another value is an error ([Configuration](guide/configuration.md#unstable-rustfmt-options)).
- **rustfmt's command line.** There are no `--emit`, `--files-with-diff`, `--edition` or
  standard input modes; `--check` reports a count, not a diff. Editions are set with
  `--config edition=…`.
- **Out-of-line modules.** rustfmt formats the files a crate's `mod` declarations lead to;
  chloro formats the files and directories you name, whatever modules they declare.
- **Syntax rust-analyzer's parser cannot represent.** chloro parses with rust-analyzer's
  parser and rebuilds the tree for the cases where rustc's parser accepts something
  rust-analyzer rejects. Two such constructs remain (below).
- **Error reporting.** Both tools leave a file that does not parse unchanged, print its
  error and exit with status 1. chloro's message is rust-analyzer's parser's, worded like
  rustc's where its tokens allow, at the position rustc reports for most errors; it prints
  the first error of a file only, without rustc's notes and suggestions
  ([command line](guide/cli.md#exit-status)). Where rustfmt fails with an internal error but
  still writes output, chloro writes the same output and reports no error.

## Known differences

Six cases are recorded and open in the [conformance ledger][ledger]:

| | what | effect |
|---|---|---|
| 19 | A doc comment before an expression in an argument list, array, `let` or statement position: rustc treats it as an attribute of the expression and rustfmt formats it with the expression; chloro leaves it where it is. | Layout differs around the comment. No registry file is affected; rustc warns about such comments (`unused_doc_comments`). |
| 20 | Edition-2015 trait methods with anonymous parameters, `fn f(usize);`, which rust-analyzer's parser rejects. | chloro reports a parse error and leaves the file unchanged; rustfmt formats it. |
| 21 | rustfmt's internal error on one file of rustyline 14.0.0. | rustfmt exits with an error and writes the output chloro writes. |
| 22 | rustfmt's run time on a 47 KB generated file of pest_meta 2.9.2. | rustfmt takes over nine minutes and chloro 0.2 s, for identical output. |
| 23 | Files where rustfmt's own output is not a fixed point (eight known). | chloro's output matches rustfmt's at every pass. |
| 24 | Minor forms: a trailing comma in `builtin # offset_of(…)`, which rust-analyzer rejects. | chloro reports a parse error and leaves the file unchanged. |

A difference not listed here is a bug: please [open an issue](https://github.com/lmmx/chloro/issues)
with the input, the options, and the rustfmt version.

## Reproducing the comparison

The `rustfmt` on `PATH` is the 1.9.0 reference; `rustup run 1.88.0 rustfmt` is the 1.8.0
one.

```sh
cargo run --release -p chloro-core --example conform -- -i              # 1220 files against the 1.9.0 snapshots
cargo run --release -p chloro-core --example conform -- -i -V 1.8.0     # the same at rustfmt_version = 1.8.0
cargo run --release -p chloro-core --example config_matrix -- -n 1220   # 31 option values against live rustfmt
cargo run --release -p chloro-core --example config_matrix -- --root ~/.cargo/registry/src -n 30000 default
cargo test -p chloro-core --test conformance batch                      # regenerate the snapshots
```

[Development](maintainers/development.md#conformance) explains the snapshots and how they are
regenerated.

[ledger]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/conformance-ledger.md
[versions]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/rustfmt-versions.md
