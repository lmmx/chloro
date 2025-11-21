# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)

<!-- [![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn) -->
<!-- ra_ap_syntax has tracing in -->

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
**Summary:** +151,092 / -16,339

| **Top 5 <del>Removed</del> Lines** | **Top 5 <ins>Added</ins> Lines** |
|---|---|
| `- }` × 154<br>`- {` × 135<br>`- },` × 93<br>`- );` × 87<br>`- ///` × 72<br> | `+ &self,` × 1,157<br>`+ &mut self,` × 652<br>`+ self,` × 518<br>`+ ) {` × 463<br>`+ db: &dyn HirDatabase,` × 351<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact |   +   |    -    | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 3 | 1 | 27.8% | 2,081 | 7,496 | [`hir/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_lib.diff) |
| 2 | 7 | 2 | 47.0% | 1,820 | 3,869 | [`rust_analyzer/src_config`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_config.diff) |
| 3 | 4 | 10 | 7.5% | 462 | 6,130 | [`ide_assists/src_handlers_extract_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_extract_function.diff) |
| 4 | 2 | 28 | 2.5% | 278 | 11,104 | [`ide/src_hover_tests`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_hover_tests.diff) |
| 5 | 18 | 5 | 25.0% | 682 | 2,728 | [`hir/src_semantics`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir/src_semantics.diff) |
| 6 | 27 | 4 | 31.2% | 771 | 2,472 | [`hir_ty/src_next_solver_interner`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_interner.diff) |
| 7 | 16 | 9 | 16.3% | 484 | 2,961 | [`rust_analyzer/src_lsp_to_proto`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/rust_analyzer/src_lsp_to_proto.diff) |
| 8 | 64 | 3 | 56.4% | 838 | 1,487 | [`hir_expand/src_builtin_derive_macro`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_expand/src_builtin_derive_macro.diff) |
| 9 | 11 | 20 | 9.2% | 329 | 3,577 | [`hir_def/src_expr_store_lower`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store_lower.diff) |
| 10 | 8 | 35 | 6.6% | 255 | 3,866 | [`ide/src_goto_definition`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_goto_definition.diff) |
| 11 | 65 | 6 | 45.1% | 652 | 1,447 | [`hir_def/src_lib`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_lib.diff) |
| 12 | 13 | 34 | 7.9% | 258 | 3,267 | [`hir_ty/src_mir_eval`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_mir_eval.diff) |
| 13 | 38 | 12 | 19.5% | 407 | 2,088 | [`hir_ty/src_infer`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_infer.diff) |
| 14 | 63 | 8 | 33.7% | 504 | 1,495 | [`hir_ty/src_next_solver_ty`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_ty/src_next_solver_ty.diff) |
| 15 | 9 | 57 | 5.1% | 191 | 3,767 | [`ide/src_rename`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_rename.diff) |
| 16 | 1 | 540 | 0.1% | 12 | 19,633 | [`ide_db/src_generated_lints`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_db/src_generated_lints.diff) |
| 17 | 14 | 42 | 7.3% | 224 | 3,075 | [`ide_assists/src_handlers_generate_function`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_assists/src_handlers_generate_function.diff) |
| 18 | 90 | 7 | 47.7% | 558 | 1,169 | [`ide/src_navigation_target`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide/src_navigation_target.diff) |
| 19 | 12 | 73 | 5.2% | 171 | 3,277 | [`ide_completion/src_render`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/ide_completion/src_render.diff) |
| 20 | 97 | 11 | 39.0% | 432 | 1,109 | [`hir_def/src_expr_store`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/hir_def/src_expr_store.diff) |
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
