---
title: chloro
description: A fast Rust code formatter that matches the output of rustfmt, byte for byte.
---

<h1 id="chloro-home">chloro</h1>

<div class="chl-hero" markdown>
![The chloro mark: a seedling whose leaf veins branch like a syntax tree](assets/images/chloro-hero.svg#only-light){ width="132" height="132" }
![The chloro mark: a seedling whose leaf veins branch like a syntax tree](assets/images/chloro-hero-dark.svg#only-dark){ width="132" height="132" }
</div>

<p class="chl-tagline">rustfmt's output in a fraction of the time.</p>

chloro formats Rust several times faster than rustfmt, in a fraction of the memory, and
writes the same bytes as `rustfmt --edition 2024`. It is a module-by-module port of rustfmt
1.9.0's formatting code onto the syntax trees of rust-analyzer's parser, so it does not need
the Rust compiler. It comes as a 2 MB command-line tool and as a library.

<div class="chl-conform">
<span class="chl-figure">1220/1220</span>
<span class="chl-label">rust-analyzer files formatted identically to rustfmt 1.9.0, and to rustfmt 1.8.0</span>
<span class="chl-note">default options, none non-idempotent;
<a href="compatibility/">rustfmt compatibility</a></span>
</div>

<div class="chl-benches">
<div class="chl-bench">
<p class="chl-bench-kicker">Controlled benchmark</p>
<p class="chl-bench-title">Nine crates <span>vs rustfmt 1.9.0</span></p>
<table class="chl-bench-table">
<thead><tr><th></th><th>wall time, speedup</th><th>peak memory</th></tr></thead>
<tbody>
<tr class="chl-ref"><th>rustfmt</th><td><span class="chl-meter" title="rustfmt's wall time: the baseline"><span class="chl-fill" style="width:100%"></span></span>1×</td><td>43–86 MiB</td></tr>
<tr><th>chloro, 1 core</th><td><span class="chl-meter" title="21–35% of rustfmt's wall time, per crate: 2.9–4.7× as fast"><span class="chl-fill" style="width:21.4%"></span><span class="chl-spread" style="left:21.4%;width:13.1%"></span></span>2.9–4.7×</td><td>4–10 MiB</td></tr>
<tr><th>chloro, 4 cores</th><td><span class="chl-meter" title="8–17% of rustfmt's wall time, per crate: 6.1–13× as fast"><span class="chl-fill" style="width:7.7%"></span><span class="chl-spread" style="left:7.7%;width:8.8%"></span></span>6.1–13×</td><td>5–23 MiB</td></tr>
</tbody>
</table>
<p class="chl-bench-note">hir, hir-ty, ide-db, syntax and parser from rust-analyzer, and
tokio, regex-syntax, clap_builder and winnow from crates.io: 215 KB–2.9 MB each.
An idle four-core Linux VM, 30 runs per cell, one-core runs pinned. Ranges span the
nine crates.</p>
</div>
<div class="chl-bench">
<p class="chl-bench-kicker">A large real-world workspace</p>
<p class="chl-bench-title">ruff <span>vs <code>cargo fmt</code>, rustfmt 1.10.0</span></p>
<table class="chl-bench-table">
<thead><tr><th></th><th>wall time, speedup</th><th>peak memory</th></tr></thead>
<tbody>
<tr class="chl-ref"><th><code>cargo fmt</code></th><td><span class="chl-meter" title="cargo fmt's wall time, 5.63 s: the baseline"><span class="chl-fill" style="width:100%"></span></span>1×</td><td>194 MiB</td></tr>
<tr><th>chloro, 1 thread</th><td><span class="chl-meter" title="1.60 s against 5.63 s: 3.5× as fast"><span class="chl-fill" style="width:28.4%"></span></span>3.5×</td><td>16 MiB</td></tr>
<tr><th>chloro, 20 threads</th><td><span class="chl-meter" title="0.21 s against 5.63 s: 26.5× as fast"><span class="chl-fill" style="width:3.8%"></span></span>26.5×</td><td>90 MiB</td></tr>
</tbody>
</table>
<p class="chl-bench-note">The whole <a href="https://github.com/astral-sh/ruff">ruff</a>
repository: 2,008 files in one run of each tool, so one figure per row. A 20-core
workstation, 5 runs each (<code>just vs-ruff</code>).</p>
</div>
</div>

<p class="chl-stats-caption">The speedup is rustfmt's wall time divided by chloro's, checking
already-formatted files. rustfmt formats on one thread, so the one-core and one-thread rows
compare the two formatters like for like; the last row of each adds chloro's parallelism.
Each bar is wall time, the baseline's as the full bar; the nine crates' bars show their
spread. The panels compare against different rustfmt releases: 1.9.0, the one chloro
reproduces, and 1.10.0, the one ruff's toolchain pins. The method, every workload and the limits of these numbers are on the
<a href="performance/">Performance</a> page.</p>

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
