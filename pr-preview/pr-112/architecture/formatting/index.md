# The formatting engine

chloro's formatter is a module-by-module port of rustfmt 1.9.0. An adaptation layer translates rust-analyzer's syntax tree for it, and a few additions make it faster without changing any formatting decision.

## Modules ported from rustfmt

The modules of `chloro-core/src/formatter/` carry the names of the rustfmt source files they reproduce, so a decision in chloro can be compared line by line with rustfmt's:

| module | role |
| --- | --- |
| `formatting` | the entry point: parse, visit, trailing newlines, newline style |
| `visitor`, `missed_spans` | walk items and statements; copy comments and blank lines between them |
| `items`, `imports`, `reorder` | items, `use` trees, the sorting of `use`, `mod` and `extern crate` |
| `expr`, `stmt`, `chains`, `closures`, `matches`, `pairs`, `patterns`, `types` | expressions and their parts |
| `macros`, `macro_args` | macro calls, with their arguments re-parsed, and `macro_rules!` bodies |
| `overflow`, `lists` | comma-separated lists and the "overflow the last item" heuristic |
| `attr`, `comment` | attributes, doc comments, re-indenting and recovering comments |
| `skip` | the names listed by `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` |
| `shape`, `context`, `utils`, `sort` | shared infrastructure, as in rustfmt |

As in rustfmt, every node implements a `rewrite` method that takes a *shape* (the width available and the current indentation) and returns the node's text, or nothing if it cannot be laid out in that shape. A caller with several possible layouts tries them in rustfmt's order and keeps the first that succeeds: a call on one line, then with its last argument overflowing, then one argument per line, and so on.

Comments are handled as rustfmt handles them with its default (stable) options: re-indented and stripped of trailing whitespace, never re-wrapped.

## The adaptation layer

rustfmt is written against rustc's abstract syntax tree; chloro reads rust-analyzer's lossless tree. Three modules translate, so that the ported code can keep rustfmt's control flow:

- **`span`** computes rustc's spans. rustfmt recovers comments, blank lines and code it gives up on by slicing the source between nodes, so spans must start and end where rustc's do. The main difference is that rust-analyzer attaches leading comments to the following item, while a rustc span starts at the first attribute.
- **`nodes`** presents rust-analyzer's nodes in rustc's shape: doc comments as attributes, a block as a list of statements with its tail expression, rustc's classification of statements.
- **`macro_args`** parses macro call arguments as rustc's parser would (expression, then type, pattern or item), laying each argument's tree out at its offset in the file.

## Performance

rustfmt chooses a layout by rendering candidates and measuring them. Each attempt rewrites every subexpression again, so the work grows with nesting, exponentially in the worst case. chloro has to make the same choices, so it cannot skip an attempt that might succeed. It saves time by not repeating work and by skipping attempts that cannot succeed.

### A memo for deep nesting

A rewrite of an expression at a given shape is stored and reused when the same expression is asked for at the same shape again. On ordinary code that rarely happens (3.2% of lookups hit), and storing every result costs about as much as the hits save, so the memo only stores rewrites nested six or more deep (`MEMO_MIN_DEPTH`), where most repeated attempts happen. The memo is cleared at each top-level item.

### A lower bound on one-line width

Some attempts succeed only if their result fits on one line: a closure body without braces, or `if c { a } else { b }` on one line. `formatter/width.rs` computes, from a node's tokens and without rendering anything, a width below which no one-line rendering can be: the sum of the tokens that every formatting path writes unchanged (identifiers, keywords, literals, and the punctuation each node kind always writes). When the bound already exceeds the space available, the attempt is skipped, because it would fail.

The bound is justified token class by token class in the module's documentation, and tested against every actual one-line rendering over both corpora, three option sets and synthetic worst cases (`tests/width_bound.rs`). Its main effect is on deeply nested closures: a 2,000-byte file of closures nested 16 deep went from 105.4 million instructions to 5.7 million.

One case is also decided by structure alone: a closure body that renders a `match` with an arm is never one line. The argument for this comes from rustfmt's `rewrite_match`, and debug builds check it on every rewrite.

### Fewer, cheaper allocations

- Lists are written into the caller's buffer instead of being assembled and copied.
- Intermediate values that never leave one module (a call's list items, a chain's entries, a path's segments) live in a per-thread `bumpalo` arena, reset at each top-level item (`formatter/scratch.rs`), instead of being separate heap allocations. The arena uses no `unsafe` code.
- Dozens of local fast paths in the comment scanner, the list formatter and string building avoid re-scanning text, each tested against the computation it replaces. One of them is `chloro-core`'s only `unsafe` block: a 16-byte SSE2 load in the comment scanner, used only where the target supports it, with a randomised test against the scalar search.

None of these changes affects what a rewrite returns. Each was checked by formatting both corpora and comparing every file's output with the output recorded before the change.

## Modules not from rustfmt

- `width` and `scratch`, above.
- `parallel` and `unit_cache`: formatting top-level items on several threads, and reusing formatted items. See [Parallelism and caches](<https://lmmx.github.io/chloro/pr-preview/pr-112/architecture/concurrency/index.md>).
- `rustc_compat` and `rustc_syntax`: the boundary between what rustc's parser and rust-analyzer's accept. See [Parsing](<https://lmmx.github.io/chloro/pr-preview/pr-112/architecture/parsing/#where-the-two-parsers-disagree>).
- `printer`: a public `Printer` trait from chloro's pre-port formatter. Nothing in the formatter calls it; it is kept for API compatibility.
