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

```
Score  SizeRank  DiffRank    Impact       +   / -       File
────────────────────────────────────────────────────────────────────────────
    3         3         1     28.2%   2,118  /  7,503   ra/hir/src_lib
   14         7         2     47.0%   1,819  /  3,870   ra/rust_analyzer/src_config
   44         4        11      7.5%     462  /  6,130   ra/ide_assists/src_handlers_extract_function
   60         2        30      2.5%     278  /  11,104  ra/ide/src_hover_tests
  108        18         6     25.2%     689  /  2,729   ra/hir/src_semantics
  108        27         4     31.2%     771  /  2,472   ra/hir_ty/src_next_solver_interner
  144        16         9     16.3%     484  /  2,961   ra/rust_analyzer/src_lsp_to_proto
  192        64         3     56.4%     838  /  1,487   ra/hir_expand/src_builtin_derive_macro
  198        11        18      9.5%     339  /  3,579   ra/hir_def/src_expr_store_lower
  288         8        36      6.6%     255  /  3,866   ra/ide/src_goto_definition
  325        65         5     48.7%     712  /  1,463   ra/hir_def/src_lib
  494        13        38      7.7%     251  /  3,268   ra/hir_ty/src_mir_eval
  494        38        13     21.0%     440  /  2,099   ra/hir_ty/src_infer
  504        63         8     33.7%     504  /  1,495   ra/hir_ty/src_next_solver_ty
  540         9        60      5.1%     191  /  3,767   ra/ide/src_rename
  545         1       545      0.1%      12  /  19,633  ra/ide_db/src_generated_lints
  602        14        43      7.3%     224  /  3,075   ra/ide_assists/src_handlers_generate_function
  630        90         7     47.7%     558  /  1,169   ra/ide/src_navigation_target
  672        12        56      6.0%     196  /  3,284   ra/ide_completion/src_render
 1000       100        10     43.8%     476  /  1,086   ra/ide/src_lib
```

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
