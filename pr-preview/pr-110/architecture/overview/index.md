# How it works

chloro is rustfmt's formatting code ported onto a parser that is available as an ordinary library, then optimised. This page gives an overview of how a file goes from source text to formatted output.

## Background

rustfmt is built on the Rust compiler: it parses a file with rustc's parser into rustc's abstract syntax tree, then rewrites each node, trying candidate layouts in a fixed order and keeping the first that fits the width. Its output is the product of a few hundred interacting width heuristics and fallbacks. chloro's first versions tried to reproduce that output rule by rule from observation, and matched rustfmt on 233 of the 1220 conformance files. Reproducing a few hundred interacting heuristics one rule at a time from their output did not come close to rustfmt.

So chloro ports rustfmt instead. The formatter in `chloro-core` follows rustfmt 1.9.0's source module by module, keeping its control flow, names and comments, so that every formatting decision can be checked against the rustfmt code it reproduces. The difference is the parser: the syntax comes from rust-analyzer's parser (`ra_ap_parser`), which is published as a library, rather than from rustc, which is not. A thin adaptation layer presents rust-analyzer's syntax in the shape rustfmt's code expects.

The port matched rustfmt on 1219 of the 1220 files, and later on all 1220. The performance work that followed left every output byte unchanged and made the formatter 2.7 times as fast.

## The pipeline

From source text to formatted text. Each box names the module that does the step.

1. **Normalise.** A leading byte order mark is dropped and `\r\n` reads as `\n`, as rustc's source map does.
2. **Lex and parse.** chloro's own lexer front end turns the text into rust-analyzer's tokens in one pass over `rustc_lexer`, and rust-analyzer's parser turns them into a stream of events. A text with a lexer error goes through rust-analyzer's own lexer instead, so errors are reported the same way.
3. **Build the tree.** The parser's events become one flat array of nodes and tokens over the source text, with comments and whitespace attached where rust-analyzer attaches them. [Parsing and the syntax tree](<https://lmmx.github.io/chloro/pr-preview/pr-110/architecture/parsing/index.md>) describes the tree.
4. **Match rustc's parser.** rustfmt formats exactly the files rustc's parser accepts, and the two parsers do not accept exactly the same syntax. Where rust-analyzer rejects syntax rustc accepts, the tree is rebuilt with the node rustc would build (`rustc_syntax`); where rust-analyzer accepts syntax rustc rejects, the file is returned unchanged (`rustc_compat`). A file that neither parser accepts is returned unchanged, as rustfmt returns it.
5. **Visit and rewrite.** The visitor walks items and statements in source order, copying the comments and blank lines between them from the source, and the ported rewrite modules format each node. [The formatting engine](<https://lmmx.github.io/chloro/pr-preview/pr-110/architecture/formatting/index.md>) describes this part.
6. **Finish.** Trailing newlines are trimmed to one and line endings set by `newline_style`, as rustfmt's `format_lines` and `apply_newline_style` do.

For a file of 128 KiB or more, step 5 runs on several threads at once, one top-level item per task, while the text between items is written in order on one thread. [Parallelism and caches](<https://lmmx.github.io/chloro/pr-preview/pr-110/architecture/concurrency/index.md>) covers that, and the caches that let chloro skip work it has done before.

## Where the time goes

On the 49-file instruction-count sample, after the performance work:

| part | share of instructions |
| --- | --- |
| lexing and parsing, including building the tree | 34% (42% on a crates.io sample) |
| the visitor and rewrites | most of the rest |
| memory allocation | about 12%, spread across both |

The parser is rust-analyzer's, used as published. Its remaining cost could be cut further only with a fork of `ra_ap_parser` (sizing its buffers up front and writing chloro's tree directly from its events), which would save at most about 6–7% of the total; chloro does not maintain a fork.

## The crates

| crate | what is in it |
| --- | --- |
| `chloro-core` | the lexer front end and flat tree (`cst`), the formatter (`formatter`), and the public entry points |
| `chloro` | the command line (`src/cli/`: argument parsing, file discovery, the worker pool, the caches) and a library facade over `chloro-core` |

`chloro-core`'s runtime dependency tree is 18 crates as `cargo tree` counts them (itself included), rust-analyzer's parser and its lexer among them; `syn` is not one of them. `rowan` and `ra_ap_syntax`, the tree types rust-analyzer itself uses, are only development dependencies: the tests use them to check that chloro builds the same tree as rust-analyzer.
