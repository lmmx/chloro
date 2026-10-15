# chloro

[![crates.io](https://img.shields.io/crates/v/chloro.svg)](https://crates.io/crates/chloro)
[![documentation](https://docs.rs/chloro/badge.svg)](https://docs.rs/chloro)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/chloro.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/chloro/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/chloro/master)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)<!-- blazon -->
[![Dependencies: 33](https://img.shields.io/badge/cargo%20tree-33-blue)](https://crates.io/crates/chloro)
[![Binary Size: 2.5M](https://img.shields.io/badge/build%20size-2.5M-green)](https://crates.io/crates/chloro)<!-- /blazon -->

chloro is a Rust code formatter that produces rustfmt's output without depending on the
compiler: a port of rustfmt 1.9.0's formatting logic onto the syntax trees of
rust-analyzer's parser. Its output is rustfmt's, byte for byte, across the tested
compatibility matrix below (two rustfmt releases, four editions, 31 option values, 1220
rust-analyzer files and 20,540 files from 577 registry crates), and it formats the same
files in 0.32–0.54 of rustfmt's wall time on one core and 0.11–0.20 on four. Both claims
are measured, with the method and the commands to reproduce them in the two sections that
follow; neither is a claim of equivalence beyond that matrix.

## Conformance

chloro's output is compared with the output of a named rustfmt binary on the same source,
byte for byte. The matrix, with every cell's result and the record it comes from:

| what varies | values | corpus | identical | record |
|---|---|---|---|---|
| rustfmt release | 1.9.0 (`rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; the default) | 1220 rust-analyzer files, default options | 1220 of 1220, 0 non-idempotent | [ledger][ledger] |
| rustfmt release | 1.8.0 (`rustfmt 1.8.0-stable (6b00bc3880 2025-06-23)`, Rust 1.88; `rustfmt_version = "1.8.0"`) | the same | 1220 of 1220, 0 non-idempotent | [ledger][ledger], [rustfmt-versions][versions] |
| stable options | 31 non-default values of 24 options (listed below), rustfmt 1.9.0 | 1220 files, every file rustfmt accepts under the option | every file: 1022 of 1022 for the 28 values at edition 2024; see editions for the other 3 | [ledger][ledger] |
| edition | 2015, 2018, 2021, 2024 (`edition`; rustfmt's `--edition`) | the same | 838/838, 843/843, 844/844, 1022/1022 | [ledger][ledger] |
| style edition | 2015, 2021, 2024 (`style_edition`) | the same | 1022/1022 each | [ledger][ledger] |
| code chloro was not developed on | default options, rustfmt 1.9.0 | 20,850 files of 577 crates from a cargo registry ([manifest][manifest]) | 20,540 of the 20,540 files rustfmt formats | [ledger][ledger] |
| the same at rustfmt 1.8.0 | default options, rustfmt 1.8.0 (measured before the last conformance fixes) | 20,019 files of 559 crates | 20,007 of 20,019; the 12: 9 files the fixes since then target, 3 files both rustfmt releases reject | [EXP-008][exp8] |

Two denominators appear above. `conform` compares every file: where rustfmt rejects a file
(a parse error, 216 of the 1220), the expected output is the input itself, which chloro
returns unchanged. `config_matrix` counts the files rustfmt accepts under the option, so an
edition that rejects more files has a smaller denominator. "Non-idempotent" counts files
whose chloro output is not a fixed point of chloro.

The option values compared: `max_width` 80 and 120, `hard_tabs`, `tab_spaces` 2,
`use_small_heuristics` Max and Off, `fn_call_width` 40, `chain_width` 40, `struct_lit_width`
0, `array_width` 30, `single_line_if_else_max_width` 0, `single_line_let_else_max_width` 0,
`reorder_imports` false, `reorder_modules` false, `remove_nested_parens` false,
`short_array_element_width_threshold` 4, `match_arm_leading_pipes` Always and Preserve,
`fn_params_layout` Vertical and Compressed, `match_block_trailing_comma`, `merge_derives`
false, `use_try_shorthand`, `use_field_init_shorthand`, `force_explicit_abi` false,
`newline_style` Windows, `style_edition` 2021 and 2015, `edition` 2021, 2018 and 2015.
`Config` also takes `attr_fn_like_width`, `struct_variant_width` and
`disable_all_formatting`, which the matrix does not compare.

What the matrix does not cover, and the known differences:

- rustfmt's unstable options are not configurable; chloro behaves as rustfmt does with each
  at its default, and `Config::set` refuses them by name. rustfmt 1.7.0 and earlier are
  refused (`rustfmt_version = "1.7.0"` is an error); nightly rustfmt is not compared.
- `rustfmt.toml` is not read. The CLI takes options through `--config`; the library through
  `Config`.
- Source that does not parse, or that rustc's parser would reject, is returned unchanged,
  as rustfmt leaves such files alone. Where rust-analyzer's parser rejects syntax rustc
  accepts, chloro rebuilds the tree for the known cases; two constructs remain
  (edition-2015 anonymous trait-method parameters; a trailing comma in `builtin # offset_of`).
- Six differences are known and recorded in the [conformance ledger][ledger] (cases 19–24):
  a doc comment before an expression in an argument list, array or `let` (rustc treats it as
  an attribute; no registry file is affected); the two parser gaps above; one rustyline file
  on which rustfmt reports an internal error and still writes the output chloro writes; one
  generated pest_meta file on which rustfmt takes nine minutes and chloro 0.2 s (identical
  output); and eight files whose rustfmt output is itself not a fixed point, where chloro
  matches rustfmt pass for pass.

To reproduce (the `rustfmt` on `PATH` is the 1.9.0 reference; `rustup run 1.88.0 rustfmt`
is the 1.8.0 one):

```sh
cargo run --release -p chloro-core --example conform -- -i                  # 1220 files, rustfmt 1.9.0 snapshots
cargo run --release -p chloro-core --example conform -- -i -V 1.8.0         # the same at rustfmt_version = 1.8.0
cargo run --release -p chloro-core --example config_matrix -- -n 1220       # 31 option values against rustfmt
cargo run --release -p chloro-core --example config_matrix -- --root ~/.cargo/registry/src -n 30000 default
cargo test -p chloro-core --test conformance batch                          # regenerate the snapshots
```

The expected outputs are rustfmt's, stored as
`chloro-core/tests/conformance/snapshots/ra/<crate>/<case>.rustfmt.rs` (1.9.0) and
`snapshots/ra-1.8.0/` (1.8.0), with the input itself where rustfmt rejects it. The batch
test regenerates them with the `rustfmt` on `PATH`, together with chloro's output and a diff
for each case. For the `parser` and `syntax` crates (581 small cases) only the `.rustfmt.rs`
files are tracked; the chloro outputs and diffs are written locally. The where-the-releases-
differ record, [rustfmt-versions.md][versions], names each class of 1.8.0/1.9.0 difference
with its reproducer under `chloro-core/tests/conformance/rustfmt-versions/`.

[ledger]: docs/journal/2026-10-14-autoresearch/conformance-ledger.md
[versions]: docs/journal/2026-10-14-autoresearch/rustfmt-versions.md
[manifest]: docs/journal/2026-10-14-autoresearch/corpus/registry-manifest.toml
[exp8]: docs/journal/2026-10-14-autoresearch/experiments/EXP-008-rustfmt-version.md

<!-- just: conf-md -->
**Summary:** +0 / -0

1220 of 1220 files identical.
<!-- /just: conf-md -->

## Performance

Measured against rustfmt 1.9.0 on identical file sets, rustfmt-formatted input, `--check`,
in an idle 4-core KVM guest (2.80 GHz host class), with `benchmarks/compare_rustfmt.sh`:
hyperfine 2.0, 15 runs per command per round, 2 rounds with the command order reversed
(n = 30 per cell), 2 warm-up runs, wall time, CPU time and peak RSS recorded per run. The
record is [measurements/m3-quiet-host.md][m3], taken on `4867896`, whose formatter code is
the current one (the commits since change the CLI's file list, the cache file's header and
the parallel replay's checks, with the instruction count on the profile sample inside the
run-to-run spread and every output identical). Ratios are of means; "every run below"
means the slowest chloro run was faster than the fastest rustfmt run.

| workload | chloro / rustfmt wall time, one core (both pinned) | four cores (chloro one thread per core, rustfmt one) |
|---|---|---|
| nine crates of 215 KB to 2.9 MB (rust-analyzer `hir`, `hir-ty`, `ide-db`, `syntax`, `parser`; tokio, regex-syntax, clap_builder, winnow) | 0.32–0.54, every run below on all nine | 0.11–0.20, every run below on all nine |
| the same, profile-guided build (`benchmarks/pgo.sh`) | 0.28–0.44 | 0.10–0.19 |
| 287 small files as one invocation (`parser/test_data`) | 0.31 | 0.16 |
| one 13-byte file (process start-up) | 1.7 ms against 9.8 ms | 2.4 ms against 11.1 ms |

Other measured quantities from the same record:

| quantity | chloro | rustfmt 1.9.0 |
|---|---|---|
| in-process throughput, one core, 1238 files (13.9 MB), best of 5 | 9.4–9.5 MB/s (11.3–11.6 profile-guided) | not measured in process |
| peak RSS on the nine crates (`memory_peak_resident`, max of 30 runs) | 5–31 MiB on four cores, 4–14 MiB on one | 43–85 MiB |
| `--check` of an unchanged 1238-file tree, warm cache | 28.9 ms on four cores, 37.1 ms on one | no cache |
| the same, cache directory removed before each run (cold) | 446 ms | — |
| the same, `--no-cache` | 434 ms on four cores, 1564 ms on one | — |
| a single file of 128 KiB or more, four threads against one | 1.4x–1.8x faster on the five largest fixture files (means of 30 runs), at 1.03x–1.19x the CPU | single-threaded |

Not measured, and not claimed: the share of process time spent in the allocator (only its
share of instructions on the profile sample, about 12%); memory bandwidth; any figure on
macOS or Windows; rustfmt's in-process throughput. Absolute times are this host's; the
ratios hold within one measurement window, and chloro and rustfmt are always timed in the
same window, interleaved.

Build configuration: `cargo build --release -p chloro --features cli` with the workspace's
`[profile.release]` (`lto = "fat"`, `codegen-units = 1`), which is what `cargo install` uses
and what the published binaries start from. The profile-guided rows come from
`benchmarks/pgo.sh` (`-Cprofile-generate`, training on the conformance fixtures,
`-Cprofile-use`). The release workflow runs the script's training phase on the targets
whose runner executes them natively (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`x86_64-pc-windows-msvc`) and builds those binaries with the profile; the two `aarch64`
targets are cross-compiled and keep the plain build, as does a target whose training fails.
That step has been validated by running its commands locally on `x86_64-unknown-linux-gnu`
(the same output as the plain build on every gate), not yet by a release run.

Why it is faster: rustfmt chooses a layout by trying several and keeping the first that
fits, and chloro does the same work rustfmt does. It memoizes rewrites so nested code does
not multiply that work, reads syntax from a flat array whose handles are a reference and an
index (a query allocates nothing), formats the top-level items of large files on all cores
with the text between items kept sequential, and hands files to one worker per core from a
largest-first queue. On the 49-file profile sample the formatter executes 47% fewer
instructions than the release before this work (587.99 M to 312.73 M, callgrind,
`benchmarks/profile.sh`; 244.71 M profile-guided).

The cache: the CLI records the BLAKE3 digest of each file it found already formatted, in a
file named by the digest of the chloro binary and of the `--config` options, under
`--cache-dir`, `$CHLORO_CACHE_DIR`, or the user cache directory (`~/.cache/chloro`;
`$XDG_CACHE_HOME/chloro` when set). A later `--check` or `--write` on an unchanged tree
hashes the files and runs the formatter on none of them. The output and exit code of a run
are the same with and without the cache; `--no-cache` turns it off.

To reproduce (needs `hyperfine` 2.0, `taskset`, `jq`, `python3`; `valgrind` and `setarch` for
the instruction count; rustfmt 1.9.0 on `PATH`):

```sh
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
benchmarks/compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3     # wall, CPU, RSS per workload, tables in report.md
benchmarks/profile.sh --core 3                                   # callgrind Ir on the 49-file sample
cargo run --release -p chloro-core --example bench -- -n 5       # in-process MB/s on the fixtures
```

`benchmarks/README.md` describes the workloads, what is held equal between the two tools,
and the known confounders (rustfmt's `--check` builds a diff of every changed file, which is
why the inputs are rustfmt-formatted copies; rustfmt loads `librustc_driver` at start-up).

[m3]: docs/journal/2026-10-14-autoresearch/measurements/m3-quiet-host.md

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
chloro --check src/              # exit 1 if a file would change
chloro --threads 1 --no-cache src/lib.rs
```

A file reachable twice (named twice, or through a symlink) is formatted once. Files of
128 KiB and more have their top-level items formatted on all threads; `--threads N` sets the
count (default: one per core).

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
[journal entries](docs/journal/) record the design, the measurements and the known gaps;
[docs/journal/2026-10-14-autoresearch.md](docs/journal/2026-10-14-autoresearch.md) is the
record of the conformance and performance programme behind the two sections above.

`Config` has one field per stable rustfmt option, with rustfmt's names and defaults.
Unstable rustfmt options keep their default behaviour. `rustfmt_version` is chloro's own
option: it selects the rustfmt release whose output is reproduced.

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
