# chloro

rustfmt's output in a fraction of the time.

chloro formats Rust several times faster than rustfmt, in a fraction of the memory, and writes the same bytes as `rustfmt --edition 2024`. It is a module-by-module port of rustfmt 1.9.0's formatting code onto the syntax trees of rust-analyzer's parser, so it does not need the Rust compiler. It comes as a 2 MB command-line tool and as a library.

1220/1220 rust-analyzer files formatted identically to rustfmt 1.9.0, and to rustfmt 1.8.0 default options, none non-idempotent; [rustfmt compatibility](<https://lmmx.github.io/chloro/compatibility/index.md>)

Controlled benchmark

Nine crates vs rustfmt 1.9.0

|  | wall time, speedup | peak memory |
| --- | --- | --- |
| rustfmt | 1× | 43–86 MiB |
| chloro, 1 core | 2.9–4.7× | 4–10 MiB |
| chloro, 4 cores | 6.1–13× | 5–23 MiB |

hir, hir-ty, ide-db, syntax and parser from rust-analyzer, and tokio, regex-syntax, clap\_builder and winnow from crates.io: 215 KB–2.9 MB each. An idle four-core Linux VM, 30 runs per cell, one-core runs pinned. Ranges span the nine crates.

A large real-world workspace

ruff vs `cargo fmt`, rustfmt 1.10.0

|  | wall time, speedup | peak memory |
| --- | --- | --- |
| `cargo fmt` | 1× | 194 MiB |
| chloro, 1 thread | 3.5× | 16 MiB |
| chloro, 20 threads | 26.5× | 90 MiB |

The whole [ruff](<https://github.com/astral-sh/ruff>) repository: 2,008 files in one run of each tool, so one figure per row. A 20-core workstation, 5 runs each (`just vs-ruff`).

The speedup is rustfmt's wall time divided by chloro's, checking already-formatted files. rustfmt formats on one thread, so the one-core and one-thread rows compare the two formatters like for like; the last row of each adds chloro's parallelism. Each bar is wall time, the baseline's as the full bar; the nine crates' bars show their spread. The panels compare against different rustfmt releases: 1.9.0, the one chloro reproduces, and 1.10.0, the one ruff's toolchain pins. The method, every workload and the limits of these numbers are on the [Performance](<https://lmmx.github.io/chloro/performance/index.md>) page.

## Getting started

```sh
cargo binstall chloro          # or: cargo install chloro --features cli
chloro --check src/            # exit 1 if any file would change
chloro --write src/            # format in place
```

```rust
let formatted = chloro::format_source("fn main(){let x=1;}");
assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");
```

## Compatibility with rustfmt

chloro's formatting modules follow rustfmt's source files (`items`, `expr`, `chains`, `lists`, `comment` and so on), so each formatting decision can be checked against the rustfmt code it comes from. Any output that differs from rustfmt's is a bug; the [known differences](<https://lmmx.github.io/chloro/compatibility/#known-differences>) are listed.

It takes rustfmt's stable options under their rustfmt names (`max_width`, `tab_spaces`, `use_small_heuristics`, `edition`, `style_edition` and the rest), either as `--config key=value` on the command line or on a `Config` in the library. Unstable options keep their default values. By default it reproduces rustfmt 1.9.0; with `rustfmt_version = "1.8.0"` it reproduces rustfmt 1.8.0, the release shipped with Rust 1.88.

The command line is not a replacement for the `rustfmt` binary: chloro does not read `rustfmt.toml`, has no `--emit` modes, and does not format standard input. [rustfmt compatibility](<https://lmmx.github.io/chloro/compatibility/#what-is-not-covered>) lists the other differences.

## Documentation

[**Install**Pre-built binaries, building from source, and the two crates.](<https://lmmx.github.io/chloro/getting-started/installation/index.md>) [**Quickstart**Check a project, format it, and use it in CI.](<https://lmmx.github.io/chloro/getting-started/quickstart/index.md>) [**Command line**Sources, modes, exit codes, options, threads and caches.](<https://lmmx.github.io/chloro/guide/cli/index.md>) [**Library**Format strings from Rust, in parallel or with a cache of items.](<https://lmmx.github.io/chloro/guide/library/index.md>) [**rustfmt compatibility**What is compared, against which rustfmt, and what is known to differ.](<https://lmmx.github.io/chloro/compatibility/index.md>) [**How it works**From source text to rustfmt's bytes: lexer, flat syntax tree, ported formatter.](<https://lmmx.github.io/chloro/architecture/overview/index.md>)

chloro is open source under MIT or Apache-2.0, at [github.com/lmmx/chloro](<https://github.com/lmmx/chloro>). The Rust API is documented on [docs.rs/chloro](<https://docs.rs/chloro>) and [docs.rs/chloro-core](<https://docs.rs/chloro-core>).
