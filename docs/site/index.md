---
title: chloro
description: A Rust code formatter that reproduces rustfmt's output, byte for byte, without the compiler.
---

<h1 id="chloro-home">chloro</h1>

<div class="chl-hero" markdown>
![The chloro mark: a leaf whose veins branch like a syntax tree](assets/images/chloro.svg){ width="132" height="132" }
</div>

<p class="chl-tagline">A Rust code formatter that produces the same output as rustfmt.</p>

chloro is a module-by-module port of rustfmt 1.9.0's formatting code onto the syntax trees
of rust-analyzer's parser. It writes the same bytes as `rustfmt --edition 2024` without
needing the Rust compiler. It comes as a 2 MB command-line tool and as a library, and it
runs several times faster than rustfmt and uses less memory.

<div class="chl-stats">
<div class="chl-stat">
<span class="chl-figure">1220/1220</span>
<span class="chl-label">rust-analyzer files identical to rustfmt 1.9.0</span>
<span class="chl-note">and to rustfmt 1.8.0; default options, 0 non-idempotent</span>
</div>
<div class="chl-stat">
<span class="chl-figure">21–35%</span>
<span class="chl-label">of rustfmt's wall time, one core</span>
<span class="chl-note">nine crates of 215 KB–2.9 MB, <code>--check</code> on formatted input</span>
</div>
<div class="chl-stat">
<span class="chl-figure">8–17%</span>
<span class="chl-label">of rustfmt's wall time, four cores</span>
<span class="chl-note">chloro with one thread per core; rustfmt runs on one thread</span>
</div>
<div class="chl-stat">
<span class="chl-figure">4–23 MiB</span>
<span class="chl-label">peak memory on the same crates</span>
<span class="chl-note">4–10 MiB on one core, 5–23 MiB on four; rustfmt 43–86 MiB</span>
</div>
</div>

<p class="chl-stats-caption">Measured on an idle four-core x86-64 Linux VM against
<code>rustfmt 1.9.0-stable</code>, 30 timed runs per cell. The method, every workload and the
limits of these numbers are on the <a href="performance/">Performance</a> page; the
conformance matrix is on <a href="compatibility/">rustfmt compatibility</a>.</p>

## Getting started

=== "Command line"

    ```sh
    cargo binstall chloro          # or: cargo install chloro --features cli
    chloro --check src/            # exit 1 if any file would change
    chloro --write src/            # format in place
    ```

=== "Library"

    ```rust
    let formatted = chloro::format_source("fn main(){let x=1;}");
    assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");
    ```

## Compatibility with rustfmt

chloro's formatting modules follow rustfmt's source files (`items`, `expr`, `chains`,
`lists`, `comment` and so on), so each formatting decision can be checked against the rustfmt
code it comes from. Any output that differs from rustfmt's is a bug; the
[known differences](compatibility.md#known-differences) are listed.

It takes rustfmt's stable options under their rustfmt names (`max_width`, `tab_spaces`,
`use_small_heuristics`, `edition`, `style_edition` and the rest), either as
`--config key=value` on the command line or on a `Config` in the library. Unstable options
keep their default values. By default it reproduces rustfmt 1.9.0; with
`rustfmt_version = "1.8.0"` it reproduces rustfmt 1.8.0, the release shipped with Rust 1.88.

The command line is not a replacement for the `rustfmt` binary: chloro does not read
`rustfmt.toml`, has no `--emit` modes, and does not format standard input.
[rustfmt compatibility](compatibility.md#what-is-not-covered) lists the other differences.

## Documentation

<div class="chl-cards">
<a href="getting-started/installation/"><strong>Install</strong><span>Pre-built binaries, building from source, and the two crates.</span></a>
<a href="getting-started/quickstart/"><strong>Quickstart</strong><span>Check a project, format it, and use it in CI.</span></a>
<a href="guide/cli/"><strong>Command line</strong><span>Sources, modes, exit codes, options, threads and caches.</span></a>
<a href="guide/library/"><strong>Library</strong><span>Format strings from Rust, in parallel or with a cache of items.</span></a>
<a href="compatibility/"><strong>rustfmt compatibility</strong><span>What is compared, against which rustfmt, and what is known to differ.</span></a>
<a href="architecture/overview/"><strong>How it works</strong><span>From source text to rustfmt's bytes: lexer, flat syntax tree, ported formatter.</span></a>
</div>

chloro is open source under MIT or Apache-2.0, at
[github.com/lmmx/chloro](https://github.com/lmmx/chloro). The Rust API is documented on
[docs.rs/chloro](https://docs.rs/chloro) and [docs.rs/chloro-core](https://docs.rs/chloro-core).
