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
**Summary:** +100,008 / -13,293

| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |
|---|---|
| `- )` × 219<br>`- r#"` × 140<br>`- },` × 108<br>`- check(` × 90<br>`- );` × 87<br> | `+ },` × 137<br>`+"#)` × 104<br>`+ check(r#"` × 79<br>`+}` × 75<br>`+ }` × 69<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact |   +   |    -    | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 2 | 8 | 4.8% | 310 | 6,483 | [`hir/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_lib.diff) |
| 2 | 7 | 3 | 9.8% | 377 | 3,858 | [`ide/src_goto_definition`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_goto_definition.diff) |
| 3 | 3 | 10 | 4.3% | 260 | 6,090 | [`ide_assists/src_handlers_extract_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_function.diff) |
| 4 | 1 | 37 | 1.4% | 161 | 11,159 | [`ide/src_hover_tests`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_hover_tests.diff) |
| 5 | 55 | 1 | 46.9% | 728 | 1,551 | [`hir_expand/src_builtin_derive_macro`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_expand/src_builtin_derive_macro.diff) |
| 6 | 15 | 4 | 12.1% | 365 | 3,020 | [`rust_analyzer/src_lsp_to_proto`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_lsp_to_proto.diff) |
| 7 | 28 | 6 | 14.8% | 347 | 2,352 | [`hir_ty/src_next_solver_interner`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_interner.diff) |
| 8 | 5 | 36 | 4.0% | 165 | 4,158 | [`rust_analyzer/src_config`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_config.diff) |
| 9 | 10 | 19 | 5.9% | 207 | 3,519 | [`hir_def/src_expr_store_lower`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store_lower.diff) |
| 10 | 95 | 2 | 39.6% | 446 | 1,125 | [`ide/src_navigation_target`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_navigation_target.diff) |
| 11 | 8 | 30 | 4.7% | 176 | 3,774 | [`ide/src_rename`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_rename.diff) |
| 12 | 41 | 7 | 18.4% | 346 | 1,881 | [`ide_assists/src_handlers_auto_import`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_auto_import.diff) |
| 13 | 59 | 5 | 23.7% | 358 | 1,513 | [`hir_def/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_lib.diff) |
| 14 | 13 | 31 | 5.5% | 171 | 3,088 | [`ide_assists/src_handlers_generate_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_function.diff) |
| 15 | 14 | 29 | 5.8% | 177 | 3,066 | [`ide/src_references`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_references.diff) |
| 16 | 26 | 16 | 8.8% | 215 | 2,446 | [`ide/src_highlight_related`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_highlight_related.diff) |
| 17 | 44 | 12 | 14.1% | 257 | 1,818 | [`ide_assists/src_handlers_generate_delegate_trait`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_delegate_trait.diff) |
| 18 | 11 | 60 | 3.7% | 121 | 3,271 | [`ide_completion/src_render`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_completion/src_render.diff) |
| 19 | 33 | 21 | 9.0% | 198 | 2,197 | [`ide_assists/src_handlers_add_missing_match_arms`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_add_missing_match_arms.diff) |
| 20 | 38 | 20 | 10.1% | 203 | 2,015 | [`ide_assists/src_handlers_destructure_tuple_binding`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_destructure_tuple_binding.diff) |
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
