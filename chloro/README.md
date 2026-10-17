# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![site](https://img.shields.io/badge/docs-lmmx.github.io%2Fchloro-2f9a57)](https://lmmx.github.io/chloro/)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](https://github.com/lmmx/chloro#license)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 33](https://img.shields.io/badge/cargo%20tree-33-blue)](https://crates.io/crates/chloro)
[![Binary Size: 2.5M](https://img.shields.io/badge/build%20size-2.5M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

chloro is a Rust code formatter that produces rustfmt's output, byte for byte, without
depending on the compiler. It is a module-for-module port of rustfmt 1.9.0's formatting
logic onto the syntax trees of rust-analyzer's parser, available as a command-line tool and
as a library.

The documentation is at [lmmx.github.io/chloro](https://lmmx.github.io/chloro/).

## Compatibility and performance

chloro's output is identical to rustfmt 1.9.0's on all 1220 files of the rust-analyzer
conformance corpus, on 31 non-default values of 24 stable options across four editions,
and on all 20,540 files of a 577-crate registry corpus that rustfmt formats. With
`rustfmt_version = "1.8.0"` it matches rustfmt 1.8.0 instead. The
[compatibility page](https://lmmx.github.io/chloro/compatibility/) lists what was compared
and what was not.

Checking nine crates of 215 KB to 2.9 MB, chloro took 0.21–0.35 of rustfmt's wall time on
one core and 0.08–0.17 on four, and 4–23 MiB of peak memory where rustfmt took 43–86 MiB.
With a warm cache, `--check` of 1238 unchanged files takes 16 ms. Its runtime dependency tree
is 18 crates and does not include `syn`. These figures were measured on x86-64 Linux; the
[performance page](https://lmmx.github.io/chloro/performance/) gives the method and the
commands to reproduce them.

<!-- just: conf-md -->
**Summary:** +0 / -0

1220 of 1220 files identical.
<!-- /just: conf-md -->

## Install

```sh
cargo binstall chloro                 # a pre-built binary, where the release has one
cargo install chloro --features cli   # or build it
```

As a library:

```toml
[dependencies]
chloro = "0.9"
```

## Use

```sh
chloro --check src/ tests/            # exit 1 if any file would change
chloro --write src/ tests/            # format in place
chloro --config max_width=80,edition=2021 --write src/
```

chloro walks every directory it is given, `target/` included, and reads no `rustfmt.toml`:
name your source directories and pass options with `--config`. See the
[command-line guide](https://lmmx.github.io/chloro/guide/cli/).

```rust
let formatted = chloro::format_source("fn main(){let x=1;}");
assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");

let mut config = chloro::Config::default();
config.set("max_width", "80").unwrap();
let formatted = chloro::format_source_with_config("fn main(){}", &config);
```

See the [library guide](https://lmmx.github.io/chloro/guide/library/) for the parallel and cached entry points of
`chloro-core`, and [docs.rs](https://docs.rs/chloro) for the API.

## Documentation

- [How it works](https://lmmx.github.io/chloro/architecture/overview/): the parser, the flat syntax tree, the ported
  formatter, and parallel execution.
- [Changelog](https://github.com/lmmx/chloro/blob/master/CHANGELOG.md): what changed in each
  release, and what is not released yet.
- [Maintainers' guide](https://lmmx.github.io/chloro/maintainers/development/): building, testing, conformance,
  benchmarking and releasing.
- [The journal](https://github.com/lmmx/chloro/tree/master/docs/journal): the design records,
  experiments and measurements behind all of the above;
  [the 2026-10-17 handoff](https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-17-handoff.md) summarises the performance and
  conformance programme.
- For LLMs and coding agents: [llms.txt](https://lmmx.github.io/chloro/llms.txt) indexes a
  Markdown version of every page, and [llms-full.txt](https://lmmx.github.io/chloro/llms-full.txt)
  has them all in one file; [AGENTS.md](https://github.com/lmmx/chloro/blob/master/AGENTS.md)
  covers working on the code.

## License

This project is licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/lmmx/chloro/blob/master/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/lmmx/chloro/blob/master/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
