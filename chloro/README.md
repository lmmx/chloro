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
**Summary:** +91,221 / -12,961

| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |
|---|---|
| `- )` × 219<br>`- r#"` × 140<br>`- },` × 108<br>`- check(` × 90<br>`- );` × 87<br> | `+ },` × 131<br>`+"#)` × 104<br>`+ check(r#"` × 79<br>`+}` × 75<br>`+ }` × 69<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact |   +   |    -    | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 2 | 10 | 3.4% | 221 | 6,504 | [`hir/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_lib.diff) |
| 2 | 1 | 28 | 1.4% | 161 | 11,163 | [`ide/src_hover_tests`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_hover_tests.diff) |
| 3 | 3 | 11 | 3.6% | 219 | 6,119 | [`ide_assists/src_handlers_extract_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_function.diff) |
| 4 | 7 | 5 | 8.7% | 336 | 3,871 | [`ide/src_goto_definition`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_goto_definition.diff) |
| 5 | 15 | 3 | 12.6% | 380 | 3,021 | [`rust_analyzer/src_lsp_to_proto`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_lsp_to_proto.diff) |
| 6 | 55 | 1 | 46.5% | 722 | 1,553 | [`hir_expand/src_builtin_derive_macro`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_expand/src_builtin_derive_macro.diff) |
| 7 | 29 | 4 | 14.9% | 351 | 2,356 | [`hir_ty/src_next_solver_interner`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_interner.diff) |
| 8 | 5 | 38 | 3.5% | 145 | 4,156 | [`rust_analyzer/src_config`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_config.diff) |
| 9 | 96 | 2 | 40.9% | 460 | 1,125 | [`ide/src_navigation_target`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_navigation_target.diff) |
| 10 | 10 | 21 | 5.1% | 179 | 3,527 | [`hir_def/src_expr_store_lower`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store_lower.diff) |
| 11 | 8 | 36 | 3.9% | 149 | 3,785 | [`ide/src_rename`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_rename.diff) |
| 12 | 26 | 15 | 7.7% | 189 | 2,460 | [`ide/src_highlight_related`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_highlight_related.diff) |
| 13 | 13 | 41 | 4.4% | 137 | 3,114 | [`ide_assists/src_handlers_generate_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_function.diff) |
| 14 | 32 | 20 | 8.1% | 181 | 2,226 | [`ide_assists/src_handlers_add_missing_match_arms`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_add_missing_match_arms.diff) |
| 15 | 115 | 6 | 25.6% | 259 | 1,010 | [`hir_def/src_expr_store`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store.diff) |
| 16 | 11 | 63 | 3.1% | 102 | 3,272 | [`ide_completion/src_render`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_completion/src_render.diff) |
| 17 | 41 | 19 | 9.6% | 182 | 1,887 | [`ide_assists/src_handlers_generate_delegate_trait`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_delegate_trait.diff) |
| 18 | 61 | 13 | 14.6% | 217 | 1,486 | [`hir_def/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_lib.diff) |
| 19 | 14 | 60 | 3.4% | 103 | 3,054 | [`ide/src_references`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_references.diff) |
| 20 | 4 | 218 | 0.8% | 42 | 5,039 | [`hir_ty/src_tests_traits`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_tests_traits.diff) |
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
