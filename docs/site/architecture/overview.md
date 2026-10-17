---
description: "How chloro turns Rust source into rustfmt's output: lexer, flat syntax tree, and a module-by-module port of rustfmt."
---

# How it works

<p class="lede">chloro is rustfmt's formatting code ported onto a parser that is available as
an ordinary library, then optimised. This page gives an overview of how a file goes from
source text to formatted output.</p>

## Background

rustfmt is built on the Rust compiler: it parses a file with rustc's parser into rustc's
abstract syntax tree, then rewrites each node, trying candidate layouts in a fixed order and
keeping the first that fits the width. Its output is the product of a few hundred
interacting width heuristics and fallbacks. chloro's first versions tried to reproduce that
output rule by rule from observation, and matched rustfmt on 233 of the 1220 conformance
files. Reproducing a few hundred interacting heuristics one rule at a time from their
output did not come close to rustfmt.

So chloro ports rustfmt instead. The formatter in `chloro-core` follows rustfmt 1.9.0's
source module by module, keeping its control flow, names and comments, so that every
formatting decision can be checked against the rustfmt code it reproduces. The difference
is the parser: the syntax comes from rust-analyzer's parser (`ra_ap_parser`), which is
published as a library, rather than from rustc, which is not. A thin adaptation layer presents
rust-analyzer's syntax in the shape rustfmt's code expects.

The port matched rustfmt on 1219 of the 1220 files, and later on all 1220. The performance
work that followed left every output byte unchanged and made the formatter 2.7 times as
fast.

## The pipeline

<div class="chl-diagram" markdown="0">
<svg viewBox="0 0 780 244" role="img" aria-labelledby="pipe-title pipe-desc">
<title id="pipe-title">The formatting pipeline</title>
<desc id="pipe-desc">Source text is normalised, lexed, parsed by rust-analyzer's parser into a flat syntax tree, checked and repaired against what rustc's parser accepts, then walked by the visitor, whose ported rustfmt rewrites produce the output, which gets rustfmt's trailing newline and line endings.</desc>
<defs><marker id="pipe-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" class="arrowhead"/></marker></defs>
<rect x="4" y="12" width="177" height="84" rx="6" class="box quiet"/>
<text x="16" y="35" class="title">Source text</text>
<text x="16" y="53" class="sub">drop BOM, CRLF → LF</text>
<text x="16" y="71" class="code">normalize_src</text>
<rect x="205" y="12" width="177" height="84" rx="6" class="box"/>
<text x="217" y="35" class="title">Lexer</text>
<text x="217" y="53" class="sub">rustc_lexer, one pass</text>
<text x="217" y="71" class="code">cst::lex</text>
<rect x="406" y="12" width="177" height="84" rx="6" class="box"/>
<text x="418" y="35" class="title">Parser</text>
<text x="418" y="53" class="sub">rust-analyzer's grammar</text>
<text x="418" y="71" class="code">ra_ap_parser</text>
<rect x="607" y="12" width="177" height="84" rx="6" class="box"/>
<text x="619" y="35" class="title">Flat syntax tree</text>
<text x="619" y="53" class="sub">one array, Copy handles</text>
<text x="619" y="71" class="code">cst::Tree</text>
<rect x="4" y="148" width="177" height="84" rx="6" class="box"/>
<text x="16" y="171" class="title">rustc's view</text>
<text x="16" y="189" class="sub">repair or reject</text>
<text x="16" y="207" class="code">rustc_syntax</text>
<text x="16" y="221" class="code">rustc_compat</text>
<rect x="205" y="148" width="177" height="84" rx="6" class="box"/>
<text x="217" y="171" class="title">Visitor</text>
<text x="217" y="189" class="sub">items and gaps, in order</text>
<text x="217" y="207" class="code">visitor</text>
<text x="217" y="221" class="code">missed_spans</text>
<rect x="406" y="148" width="177" height="84" rx="6" class="box"/>
<text x="418" y="171" class="title">Rewrites</text>
<text x="418" y="189" class="sub">rustfmt's modules, ported</text>
<text x="418" y="207" class="code">expr, items, chains,</text>
<text x="418" y="221" class="code">lists, overflow, …</text>
<rect x="607" y="148" width="177" height="84" rx="6" class="box quiet"/>
<text x="619" y="171" class="title">Output</text>
<text x="619" y="189" class="sub">newlines, line endings</text>
<text x="619" y="207" class="code">apply_newline_style</text>
<path d="M181,54.0 H203" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M382,54.0 H404" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M583,54.0 H605" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M695.5,96 V122.0 H92.5 V146" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M181,190.0 H203" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M382,190.0 H404" class="arrow" marker-end="url(#pipe-arrow)"/>
<path d="M583,190.0 H605" class="arrow" marker-end="url(#pipe-arrow)"/>
</svg>
</div>
<p class="caption">From source text to formatted text. Each box names the module that does
the step.</p>

1. **Normalise.** A leading byte order mark is dropped and `\r\n` reads as `\n`, as rustc's
   source map does.
2. **Lex and parse.** chloro's own lexer front end turns the text into rust-analyzer's
   tokens in one pass over `rustc_lexer`, and rust-analyzer's parser turns them into a
   stream of events. A text with a lexer error goes through rust-analyzer's own lexer instead,
   so errors are reported the same way.
3. **Build the tree.** The parser's events become one flat array of nodes and tokens over the
   source text, with comments and whitespace attached where rust-analyzer attaches them.
   [Parsing and the syntax tree](parsing.md) describes the tree.
4. **Match rustc's parser.** rustfmt formats exactly the files rustc's parser accepts, and
   the two parsers do not accept exactly the same syntax. Where rust-analyzer rejects syntax rustc accepts, the
   tree is rebuilt with the node rustc would build (`rustc_syntax`); where rust-analyzer
   accepts syntax rustc rejects, the file is returned unchanged (`rustc_compat`). A file that
   neither parser accepts is returned unchanged, as rustfmt returns it.
5. **Visit and rewrite.** The visitor walks items and statements in source order, copying
   the comments and blank lines between them from the source, and the ported rewrite
   modules format each node. [The formatting engine](formatting.md) describes this part.
6. **Finish.** Trailing newlines are trimmed to one and line endings set by
   `newline_style`, as rustfmt's `format_lines` and `apply_newline_style` do.

For a file of 128 KiB or more, step 5 runs on several threads at once, one top-level item
per task, while the text between items is written in order on one thread.
[Parallelism and caches](concurrency.md) covers that, and the caches that let chloro skip
work it has done before.

## Where the time goes

On the 49-file instruction-count sample, after the performance work:

| part | share of instructions |
|---|---|
| lexing and parsing, including building the tree | 34% (42% on a crates.io sample) |
| the visitor and rewrites | most of the rest |
| memory allocation | about 12%, spread across both |

The parser is rust-analyzer's, used as published. Its remaining cost could be cut further
only with a fork of `ra_ap_parser` (sizing its buffers up front and writing chloro's tree
directly from its events), which would save at most about 6–7% of the total; chloro does
not maintain a fork.

## The crates

| crate | what is in it |
|---|---|
| `chloro-core` | the lexer front end and flat tree (`cst`), the formatter (`formatter`), and the public entry points |
| `chloro` | the command line (`src/cli/`: argument parsing, file discovery, the worker pool, the caches) and a library facade over `chloro-core` |

`chloro-core`'s runtime dependency tree is 18 crates as `cargo tree` counts them (itself
included), rust-analyzer's parser and its lexer among them; `syn` is not one of them.
`rowan` and `ra_ap_syntax`, the tree types rust-analyzer itself uses, are only development
dependencies: the tests use them to check that chloro builds the same tree as
rust-analyzer.
