# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 32](https://img.shields.io/badge/cargo%20tree-32-blue)](https://crates.io/crates/chloro)
[![Binary Size: 1.6M](https://img.shields.io/badge/build%20size-1.6M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

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
**Summary:** +97,036 / -13,578

| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |
|---|---|
| `- }` × 112<br>`- },` × 93<br>`- );` × 87<br>`- ///` × 72<br>`- },` × 64<br> | `+}` × 75<br>`+///` × 75<br>`+ },` × 72<br>`+ }` × 65<br>`+ }` × 53<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact |   +   |    -    | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 7 | 1 | 41.0% | 1,560 | 3,801 | [`rust_analyzer/src_config`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_config.diff) |
| 2 | 2 | 10 | 3.9% | 248 | 6,433 | [`hir/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_lib.diff) |
| 3 | 1 | 21 | 1.8% | 202 | 11,160 | [`ide/src_hover_tests`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_hover_tests.diff) |
| 4 | 3 | 16 | 3.7% | 227 | 6,083 | [`ide_assists/src_handlers_extract_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_function.diff) |
| 5 | 15 | 4 | 12.5% | 378 | 3,019 | [`rust_analyzer/src_lsp_to_proto`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_lsp_to_proto.diff) |
| 6 | 60 | 2 | 53.4% | 793 | 1,486 | [`hir_expand/src_builtin_derive_macro`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_expand/src_builtin_derive_macro.diff) |
| 7 | 6 | 20 | 5.2% | 202 | 3,915 | [`ide/src_goto_definition`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_goto_definition.diff) |
| 8 | 29 | 5 | 16.5% | 377 | 2,290 | [`hir_ty/src_next_solver_interner`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_interner.diff) |
| 9 | 88 | 3 | 38.0% | 436 | 1,147 | [`ide/src_navigation_target`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_navigation_target.diff) |
| 10 | 8 | 38 | 4.0% | 152 | 3,788 | [`ide/src_rename`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_rename.diff) |
| 11 | 73 | 6 | 26.8% | 344 | 1,283 | [`hir_def/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_lib.diff) |
| 12 | 46 | 12 | 13.5% | 239 | 1,768 | [`ide_assists/src_handlers_auto_import`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_auto_import.diff) |
| 13 | 33 | 18 | 9.4% | 207 | 2,194 | [`ide_assists/src_handlers_add_missing_match_arms`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_add_missing_match_arms.diff) |
| 14 | 14 | 43 | 4.6% | 140 | 3,029 | [`ide/src_references`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_references.diff) |
| 15 | 13 | 50 | 4.2% | 131 | 3,088 | [`ide_assists/src_handlers_generate_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_function.diff) |
| 16 | 139 | 7 | 35.2% | 305 | 867 | [`rust_analyzer/tests_slow-tests_ratoml`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/tests_slow-tests_ratoml.diff) |
| 17 | 124 | 8 | 29.1% | 268 | 921 | [`hir_def/src_signatures`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_signatures.diff) |
| 18 | 21 | 48 | 5.3% | 136 | 2,571 | [`ide_assists/src_handlers_extract_variable`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_variable.diff) |
| 19 | 11 | 93 | 2.8% | 93 | 3,267 | [`ide_completion/src_render`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_completion/src_render.diff) |
| 20 | 43 | 25 | 10.4% | 191 | 1,832 | [`ide_assists/src_handlers_generate_delegate_trait`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_delegate_trait.diff) |
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
