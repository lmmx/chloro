---
title: chloro
description: A Rust code formatter that reproduces rustfmt's output, byte for byte, without the compiler.
---

<h1 id="chloro-home">chloro</h1>

<div class="chl-hero" markdown>
![The chloro mark: a leaf whose veins branch like a syntax tree](assets/images/chloro.svg){ width="132" height="132" }
</div>

<p class="chl-tagline">rustfmt's output, byte for byte, in a fraction of rustfmt's time.</p>

chloro is a Rust code formatter. It reproduces what `rustfmt --edition 2024` writes, but it
does not link the Rust compiler: it is a module-for-module port of rustfmt 1.9.0's
formatting logic onto the syntax trees of rust-analyzer's parser. The result is a 2 MB binary and a library you can call from Rust, both of which give the same bytes rustfmt
would, faster and in less memory.

<div class="chl-stats">
<div class="chl-stat">
<span class="chl-figure">1220/1220</span>
<span class="chl-label">rust-analyzer files identical to rustfmt 1.9.0</span>
<span class="chl-note">and to rustfmt 1.8.0; default options, 0 non-idempotent</span>
</div>
<div class="chl-stat">
<span class="chl-figure">0.21–0.35</span>
<span class="chl-label">of rustfmt's wall time, one core</span>
<span class="chl-note">nine crates of 215 KB–2.9 MB, <code>--check</code> on formatted input</span>
</div>
<div class="chl-stat">
<span class="chl-figure">0.08–0.17</span>
<span class="chl-label">of rustfmt's wall time, four cores</span>
<span class="chl-note">chloro one thread per core, rustfmt as it runs (one thread)</span>
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

## Try it

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

## What it is, and what it is not

- **A reproduction, not a reinterpretation.** chloro's modules mirror rustfmt's source files
  (`items`, `expr`, `chains`, `lists`, `comment`, …) so that each formatting decision can be
  traced to the rustfmt code it reproduces. Where chloro's output differs from rustfmt's, that
  is a bug, and the [known cases](compatibility.md#known-differences) are listed.
- **rustfmt's stable options, by rustfmt's names.** `max_width`, `tab_spaces`,
  `use_small_heuristics`, `edition`, `style_edition` and the rest, passed as
  `--config key=value` or set on a `Config`. Unstable options keep their defaults.
- **Two rustfmt releases.** 1.9.0 by default, and 1.8.0 (the one shipped with Rust 1.88) with
  `rustfmt_version = "1.8.0"`.
- **Not a drop-in `rustfmt` binary.** chloro does not read `rustfmt.toml`, has no `--emit`
  modes, and does not format from standard input on the command line. See
  [rustfmt compatibility](compatibility.md#what-is-not-covered) for the full list.

## Where to go next

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
