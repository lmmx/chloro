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

Diff 'leaderboard' for how well the output of chloro conforms to that of rustfmt, as tested on the
[crates][ra-crates] of the rust-analyzer repo itself:

[ra-crates]: https://github.com/rust-lang/rust-analyzer/blob/master/crates/syntax/src/ast/generated.rs

<!-- just: conf-md -->
**Summary:** +152,221 / -16,336

| **Top 5 Removed Lines** | **Top 5 Added Lines** |
|---|---|
| `- }` (154)<br>`- {` (135)<br>`- },` (93)<br>`- );` (87)<br>`- ///` (72)<br> | (1,157) `+ &self,`<br>(652) `+ &mut self,`<br>(518) `+ self,`<br>(463) `+ ) {`<br>(351) `+ db: &dyn HirDatabase,`<br> |

### Top 20 Most Impacted Files

| Rank | Size Rank | Diff Rank | Impact | Added | Removed | File |
|------|-----------|-----------|--------|-------|---------|------|
| 1 | 3 | 1 | 28.2% | 2,118 | 7,503 | `hir/src_lib` |
| 2 | 7 | 2 | 47.0% | 1,819 | 3,870 | `rust_analyzer/src_config` |
| 3 | 4 | 11 | 7.5% | 462 | 6,130 | `ide_assists/src_handlers_extract_function` |
| 4 | 2 | 30 | 2.5% | 278 | 11,104 | `ide/src_hover_tests` |
| 5 | 18 | 6 | 25.2% | 689 | 2,729 | `hir/src_semantics` |
| 6 | 27 | 4 | 31.2% | 771 | 2,472 | `hir_ty/src_next_solver_interner` |
| 7 | 16 | 9 | 16.3% | 484 | 2,961 | `rust_analyzer/src_lsp_to_proto` |
| 8 | 64 | 3 | 56.4% | 838 | 1,487 | `hir_expand/src_builtin_derive_macro` |
| 9 | 11 | 18 | 9.5% | 339 | 3,579 | `hir_def/src_expr_store_lower` |
| 10 | 8 | 36 | 6.6% | 255 | 3,866 | `ide/src_goto_definition` |
| 11 | 65 | 5 | 48.7% | 712 | 1,463 | `hir_def/src_lib` |
| 12 | 13 | 38 | 7.7% | 251 | 3,268 | `hir_ty/src_mir_eval` |
| 13 | 38 | 13 | 21.0% | 440 | 2,099 | `hir_ty/src_infer` |
| 14 | 63 | 8 | 33.7% | 504 | 1,495 | `hir_ty/src_next_solver_ty` |
| 15 | 9 | 60 | 5.1% | 191 | 3,767 | `ide/src_rename` |
| 16 | 1 | 545 | 0.1% | 12 | 19,633 | `ide_db/src_generated_lints` |
| 17 | 14 | 43 | 7.3% | 224 | 3,075 | `ide_assists/src_handlers_generate_function` |
| 18 | 90 | 7 | 47.7% | 558 | 1,169 | `ide/src_navigation_target` |
| 19 | 12 | 56 | 6.0% | 196 | 3,284 | `ide_completion/src_render` |
| 20 | 100 | 10 | 43.8% | 476 | 1,086 | `ide/src_lib` |
<!-- /just: conf-md -->

## Installation

Add chloro to your `Cargo.toml`:
```toml
[dependencies]
chloro = "0.5"
```

### Migration

The CLI automatically migrates code from doc comments to chloro `#[omnidoc]` attributes.

#### CLI Installation

- pre-built binary: `cargo binstall chloro` (requires [cargo-binstall][cargo-binstall]),
- build from source: `cargo install chloro --features cli`

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall

## License

This project is licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
