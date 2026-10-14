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
the rustfmt code it reproduces. rust-analyzer's parser (`ra_ap_parser`) lexes and parses
the file; chloro lays the parser's events out as one flat array of nodes and tokens over
the source text (`chloro_core::cst`), with a typed accessor layer generated from
rust-analyzer's grammar that keeps `ra_ap_syntax`'s names. A thin adaptation layer
presents that tree in the shape rustfmt expects: rustc's spans, doc comments as
attributes, rustc's classification of statements, and macro arguments re-parsed as
expressions, types, patterns or items, each laid out at its offset in the file. The
[journal entries](docs/journal/) record the design, the measurements and the known gaps.

`Config` has one field per stable rustfmt option, with rustfmt's names and defaults.
Unstable rustfmt options keep their default behaviour. `rustfmt_version` is chloro's own
option: it selects the rustfmt release whose output is reproduced (see below).

## Rustfmt Conformance

Formatting rust-analyzer's [crates][ra-crates] (1220 files, 13.8 MB) with chloro and with
rustfmt 1.9.0 gives identical output for all 1220 files, under the default options and under
each of 31 non-default values of the stable options; every output is a fixed point of chloro.
To reproduce:

```sh
cargo run --release -p chloro-core --example conform -- -i
cargo run --release -p chloro-core --example config_matrix -- -n 1220
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

On a corpus of 577 crates from a cargo registry (20,850 files) that chloro was not developed
against, `config_matrix --root ~/.cargo/registry/src -n 30000 default` gives identical
output for every file both tools format (20540 of 20540). The remaining known differences
are listed in
[`docs/journal/2026-10-14-autoresearch/conformance-ledger.md`](docs/journal/2026-10-14-autoresearch/conformance-ledger.md):
they are files rustfmt itself fails on or formats non-idempotently, and two constructs
rust-analyzer's parser does not represent.

[ra-crates]: https://github.com/rust-lang/rust-analyzer/tree/master/crates

<!-- just: conf-md -->
**Summary:** +0 / -0

1220 of 1220 files identical.
<!-- /just: conf-md -->

## Performance

rustfmt chooses a layout by trying several and keeping the first that fits, so chloro does
the same work rustfmt does; it memoizes rewrites to keep nested code from multiplying it,
and reads syntax from a flat array whose handles are a reference and an index, so a query
allocates nothing. Measured against rustfmt 1.9.0 on the same files, rustfmt-formatted
input, `--check`, in an idle 4-core container (`benchmarks/compare_rustfmt.sh`, 30 runs per
cell; the record is
[`docs/journal/2026-10-14-autoresearch/measurements/m2-quiet-host.md`](docs/journal/2026-10-14-autoresearch/measurements/m2-quiet-host.md)):

| workload | one core, wall time as a share of rustfmt's | four cores (chloro one thread per core, rustfmt one) |
|---|---|---|
| nine crates of 215 KB to 2.9 MB (rust-analyzer, tokio, regex-syntax, clap_builder, winnow) | 0.44–0.61 | 0.13–0.20 |
| the same, profile-guided build (`benchmarks/pgo.sh`) | 0.37–0.51 | 0.10–0.19 |

Every chloro run was below every rustfmt run in those sweeps. Peak memory was 5–31 MiB
against rustfmt's 43–85 MiB. In process on one core the formatter runs at 7.3–7.7 MB/s on
the rust-analyzer fixtures (8.9–9.3 MB/s profile-guided). Files of 128 KiB and more have
their top-level items formatted on all cores, which takes the largest fixtures to 0.39–0.56
of their single-threaded time. A tree whose files were all formatted before is checked from
the cache in about 30 ms for 1238 files (13.9 MB).

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
