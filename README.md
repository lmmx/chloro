# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 32](https://img.shields.io/badge/cargo%20tree-32-blue)](https://crates.io/crates/chloro)
[![Binary Size: 1.7M](https://img.shields.io/badge/build%20size-1.7M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

chloro is a minimal Rust code formatter.

## Motivation

For when you want to format two source files in a consistent way, as fast as possible.

## How it works

Using [rowan][rowan] from the rust-analyzer project, which can give both green and red trees.
The latter are notoriously expensive, but a formatter should only need the former.

Proof of concept library/CLI to explore a fast and low memory code formatter [WIP],
with use cases of code diffing in mind.

[rowan]: https://github.com/rust-analyzer/rowan

## Rustfmt Conformance

Diff 'leaderboard' for how well formatting with chloro matches rustfmt,
as tested on rust-analyzer's [crates][ra-crates]:

[ra-crates]: https://github.com/rust-lang/rust-analyzer/blob/master/crates/syntax/src/ast/generated.rs

<!-- just: conf-md -->
**Summary:** +93,922 / -12,996

| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |
|---|---|
| `- )` × 214<br>`- r#"` × 140<br>`- },` × 104<br>`- }` × 99<br>`- )` × 93<br> | `+"#)` × 104<br>`+ check(r#"` × 79<br>`+ },` × 79<br>`+}` × 75<br>`+ }` × 65<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact |   +   |    -    | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 2 | 9 | 3.9% | 251 | 6,474 | [`hir/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_lib.diff) |
| 2 | 7 | 3 | 10.0% | 384 | 3,855 | [`ide/src_goto_definition`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_goto_definition.diff) |
| 3 | 1 | 34 | 1.4% | 161 | 11,159 | [`ide/src_hover_tests`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_hover_tests.diff) |
| 4 | 3 | 14 | 3.7% | 224 | 6,080 | [`ide_assists/src_handlers_extract_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_function.diff) |
| 5 | 15 | 4 | 12.2% | 369 | 3,020 | [`rust_analyzer/src_lsp_to_proto`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_lsp_to_proto.diff) |
| 6 | 5 | 12 | 5.4% | 228 | 4,187 | [`rust_analyzer/src_config`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_config.diff) |
| 7 | 61 | 1 | 51.8% | 782 | 1,509 | [`hir_expand/src_builtin_derive_macro`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_expand/src_builtin_derive_macro.diff) |
| 8 | 95 | 2 | 41.3% | 463 | 1,122 | [`ide/src_navigation_target`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_navigation_target.diff) |
| 9 | 28 | 7 | 13.7% | 322 | 2,355 | [`hir_ty/src_next_solver_interner`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_interner.diff) |
| 10 | 41 | 6 | 17.5% | 329 | 1,882 | [`ide_assists/src_handlers_auto_import`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_auto_import.diff) |
| 11 | 8 | 36 | 4.1% | 156 | 3,788 | [`ide/src_rename`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_rename.diff) |
| 12 | 58 | 5 | 23.3% | 353 | 1,512 | [`hir_def/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_lib.diff) |
| 13 | 14 | 29 | 5.7% | 175 | 3,066 | [`ide/src_references`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_references.diff) |
| 14 | 26 | 20 | 8.1% | 197 | 2,440 | [`ide/src_highlight_related`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_highlight_related.diff) |
| 15 | 33 | 19 | 9.0% | 198 | 2,197 | [`ide_assists/src_handlers_add_missing_match_arms`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_add_missing_match_arms.diff) |
| 16 | 38 | 17 | 10.0% | 202 | 2,016 | [`ide_assists/src_handlers_destructure_tuple_binding`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_destructure_tuple_binding.diff) |
| 17 | 13 | 51 | 4.3% | 133 | 3,094 | [`ide_assists/src_handlers_generate_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_function.diff) |
| 18 | 11 | 69 | 3.4% | 112 | 3,270 | [`ide_completion/src_render`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_completion/src_render.diff) |
| 19 | 10 | 76 | 3.1% | 107 | 3,499 | [`hir_def/src_expr_store_lower`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store_lower.diff) |
| 20 | 96 | 8 | 24.5% | 274 | 1,120 | [`rust_analyzer/tests_slow-tests_ratoml`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/tests_slow-tests_ratoml.diff) |
<!-- /just: conf-md -->

## Installation

Add chloro to your `Cargo.toml`:
```toml
[dependencies]
chloro = "0.5"
```

#### CLI Installation

- pre-built binary: `cargo binstall chloro` (requires [cargo-binstall][cargo-binstall]),
- build from source: `cargo install chloro --features cli`

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall

## License

This project is licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
