# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 33](https://img.shields.io/badge/cargo%20tree-33-blue)](https://crates.io/crates/chloro)
[![Binary Size: 2.5M](https://img.shields.io/badge/build%20size-2.5M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

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

// the rustfmt release to reproduce (a chloro option; 1.9.0 by default)
config.set("rustfmt_version", "1.8.0").unwrap();
let formatted = chloro::format_source_with_config("fn main(){}", &config);
```

Source that does not parse, or that rustc's parser would reject, is returned unchanged,
as rustfmt leaves such files alone. As in rustfmt, a byte order mark is dropped and the
output's line endings follow `newline_style` (`\n` by default, whatever the input used).

The CLI takes the same options as rustfmt's `--config`:

```sh
chloro --config max_width=80,tab_spaces=2 --write src/
```

The CLI keeps a cache of the files it has found already formatted, so that `--check` and
`--write` on an unchanged tree hash the files and run the formatter on none of them. The
cache records the BLAKE3 digest of each such file's content, in a file named by the digest
of the chloro binary and of the `--config` options, under `--cache-dir`, `$CHLORO_CACHE_DIR`,
or the user cache directory (`~/.cache/chloro`; `$XDG_CACHE_HOME/chloro` when set). The
output and exit code of a run are the same with and without the cache; `--no-cache` turns
it off.

## How it works

The formatter follows rustfmt's source module by module (`visitor`, `items`, `expr`,
`chains`, `overflow`, `lists`, `comment`, ...), so each formatting decision can be traced to
the rustfmt code it reproduces. A thin adaptation layer presents rust-analyzer's lossless
tree in the shape rustfmt expects: rustc's spans, doc comments as attributes, rustc's
classification of statements, and macro arguments re-parsed as expressions, types,
patterns or items. The [journal entries](docs/journal/) record the design, the
measurements and the known gaps.

`Config` has one field per stable rustfmt option, with rustfmt's names and defaults.
Unstable rustfmt options keep their default behaviour. `rustfmt_version` is chloro's own
option: it selects the rustfmt release whose output is reproduced (see below).

## Rustfmt Conformance

Formatting rust-analyzer's [crates][ra-crates] (1220 files, 13.8 MB) with chloro and with
rustfmt 1.9.0 gives identical output for 1219 files; the remaining file is a parser
error-recovery test case. Every output is a fixed point of chloro. To reproduce:

```sh
cargo run --release -p chloro-core --example conform -- -i
```

The expected outputs are rustfmt 1.9.0's, stored as
`chloro-core/tests/conformance/snapshots/ra/<crate>/<case>.rustfmt.rs` (the input itself
where rustfmt rejects it). The batch tests regenerate them with the `rustfmt` on `PATH`,
together with chloro's output and a diff for each case:

```sh
cargo test -p chloro-core --test conformance batch
```

For the `parser` and `syntax` crates (581 small cases) only the `.rustfmt.rs` files are
tracked; the chloro outputs and diffs are written locally.

Non-default values of the stable options are compared with `rustfmt --config` by
`cargo run --release -p chloro-core --example config_matrix`. The same example run over a
local cargo registry (`-- --root ~/.cargo/registry/src -n 6000 default`), a corpus chloro
was not developed against, gave identical output for 5384 of 5386 files.

### Supported rustfmt releases

`rustfmt_version` selects the release: `1.9.0` (the default, above) or `1.8.0`, the build
shipped with Rust 1.88.0 (`rustfmt 1.8.0-stable (6b00bc3880)`; the Rust 1.85.0 build
formats every file both accept the same way and differs only in what its parser accepts).
The two releases differ in six formatting classes (raw labels in `break`/`continue`,
labelled blocks as closure bodies, `use self::self`, `super let`, the order of `_`-prefixed
imports before style edition 2024, and a panic on Unicode whitespace in a blank-line gap)
and in three parser-acceptance classes (`let` chains before edition 2024, `[const]`
bounds, negative inherent impls); each is one branch in the formatter next to the rustfmt
code it reproduces, listed with reproducers in
[docs/journal/2026-10-14-autoresearch/experiments/EXP-008-rustfmt-version.md](docs/journal/2026-10-14-autoresearch/experiments/EXP-008-rustfmt-version.md).
With `rustfmt_version=1.8.0`, chloro gives rustfmt 1.8.0's output on 1219 of the 1220
rust-analyzer files at edition 2024 and at edition 2021 (the same file differs as for
1.9.0), including the files that release rejects, which it returns unchanged. The reference
outputs are `chloro-core/tests/conformance/snapshots/ra-1.8.0/<crate>/<case>.rustfmt.rs`
for the cases whose 1.8.0 output differs from the 1.9.0 snapshot (one file on the current
fixtures), generated by `rustup run 1.88.0 rustfmt` when the batch tests run with
`CHLORO_RUSTFMT_VERSIONS=1.8.0=1.88.0`; the reproducers under
`chloro-core/tests/conformance/rustfmt-versions/` are checked against the output of each
release at both editions the same way.

```sh
cargo run --release -p chloro-core --example conform -- -V 1.8.0          # against the snapshots
cargo run --release -p chloro-core --example config_matrix -- rustfmt_version=1.8.0,edition=2021
```

rustfmt 1.7.0 (Rust 1.80.0) is not supported. It has no `style_edition` option and no
Style Guide 2024 rules: with `--edition 2024` it formats under its Version One rules, and
its Version Two (`version = "Two"`) is refused on stable from a `rustfmt.toml`. On the
rust-analyzer files, 1.7.0 at `--edition 2024` differs from 1.9.0 on 540 of the 981 files
both accept (528 of them in `use` items alone, from the version sort of style edition
2024), so its output is not reachable from the 2024 rules by a few branch points.
`Config::set("rustfmt_version", "1.7.0")` returns an error that says so.

[ra-crates]: https://github.com/rust-lang/rust-analyzer/tree/master/crates

<!-- just: conf-md -->
**Summary:** +2 / -2

1219 of 1220 files identical; the one differing file is
[`parser/test_data/parser/err/0024_many_type_parens.rs`](https://github.com/lmmx/chloro/blob/master/chloro-core/tests/conformance/fixtures/rust-analyzer/parser/test_data/parser/err/0024_many_type_parens.rs).
<!-- /just: conf-md -->

## Performance

rustfmt chooses a layout by trying several and keeping the first that fits, so chloro does
the same work rustfmt does; it memoizes rewrites to keep nested code from multiplying it,
and reads rust-analyzer's green tree where the typed API would allocate. Single-threaded and
in process it formats about 4.1 MB/s on the rust-analyzer fixtures and 5.7 MB/s on a local
cargo registry; a `chloro` process on one large file takes 0.28x–0.56x the time of a
`rustfmt` process. The CLI formats files in parallel.

```sh
cargo run --release -p chloro-core --example bench -- -w 10   # throughput, 10 slowest files
```

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
