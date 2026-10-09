# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 32](https://img.shields.io/badge/cargo%20tree-32-blue)](https://crates.io/crates/chloro)
[![Binary Size: 1.7M](https://img.shields.io/badge/build%20size-1.7M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

chloro is a Rust code formatter that reproduces `rustfmt --edition 2024` without depending
on the compiler: it is a port of rustfmt 1.9.0's formatting logic onto the syntax trees of
rust-analyzer's parser ([rowan][rowan]).

[rowan]: https://github.com/rust-analyzer/rowan

## Usage

```rust
// rustfmt's defaults (`rustfmt --edition 2024` without a rustfmt.toml)
let formatted = chloro::format_source("fn main(){let x=1;}");
assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");

// rustfmt's stable options, by their rustfmt names
let mut config = chloro::Config::default();
config.set("max_width", "80").unwrap();
config.set("use_small_heuristics", "Max").unwrap();
let formatted = chloro::format_source_with_config("fn main(){}", &config);
```

Source that does not parse, or that rustc's parser would reject, is returned unchanged,
as rustfmt leaves such files alone. As in rustfmt, a byte order mark is dropped and the
output's line endings follow `newline_style` (`\n` by default, whatever the input used).

The CLI takes the same options as rustfmt's `--config`:

```sh
chloro --config max_width=80,tab_spaces=2 --write src/
```

## How it works

The formatter follows rustfmt's source module by module (`visitor`, `items`, `expr`,
`chains`, `overflow`, `lists`, `comment`, ...), so each formatting decision can be traced to
the rustfmt code it reproduces. A thin adaptation layer presents rust-analyzer's lossless
tree in the shape rustfmt expects: rustc's spans, doc comments as attributes, rustc's
classification of statements, and macro arguments re-parsed as expressions, types,
patterns or items. See [docs/JOURNAL.md](docs/JOURNAL.md) for the design, the
measurements and the known gaps.

`Config` has one field per stable rustfmt option, with rustfmt's names and defaults.
Unstable rustfmt options keep their default behaviour.

## Rustfmt Conformance

Formatting rust-analyzer's [crates][ra-crates] (1220 files, 13.8 MB) with chloro and with
rustfmt 1.9.0 gives identical output for 1219 files; the remaining file is a parser
error-recovery test case. Every output is a fixed point of chloro. To reproduce:

```sh
cargo run --release -p chloro-core --example conform -- -i
```

Non-default values of the stable options are compared with `rustfmt --config` by
`cargo run --release -p chloro-core --example config_matrix`. The same example run over a
local cargo registry (`-- --root ~/.cargo/registry/src -n 6000 default`), a corpus chloro
was not developed against, gave identical output for 5384 of 5386 files.

[ra-crates]: https://github.com/rust-lang/rust-analyzer/tree/master/crates

<!-- just: conf-md -->
**Summary:** +2 / -2

1219 of 1220 files identical; the one differing file is
[`parser/test_data_parser_err_0024_many_type_parens`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/snapshots/ra/parser/test_data_parser_err_0024_many_type_parens.diff).
<!-- /just: conf-md -->

## Installation

Add chloro to your `Cargo.toml`:
```toml
[dependencies]
chloro = "0.7"
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
