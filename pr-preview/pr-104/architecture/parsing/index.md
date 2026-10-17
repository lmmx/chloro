# Parsing and the syntax tree

chloro parses with rust-analyzer's parser and stores the result as one flat array, so that reading a node of the tree is an array lookup that allocates nothing.

## Why rust-analyzer's parser

rustfmt parses with rustc's parser, which is only available inside the compiler. Of the parsers available as ordinary crates, rust-analyzer's (`ra_ap_parser`) is the one written to the same grammar and kept current with it, and it is lossless: every byte of the source, comments and whitespace included, belongs to a token in the tree. rustfmt depends on that information, because it recovers comments and blank lines by slicing the source between nodes.

The parser is used as published, pinned to one version (0.0.307 today) in `chloro-core/Cargo.toml`. It produces a stream of events (start node, token, finish node, error); chloro does everything after that, from building the tree to formatting.

## The lexer front end

rust-analyzer's own lexing step (`LexedStr`) restarts `rustc_lexer` at every token, converts each token kind, collects error messages, and then walks the tokens a second time to build the parser's input. chloro's `cst::lex` does both in one loop, into buffers sized from the text, and recognises the most common tokens (whitespace, ASCII identifiers, one-character punctuation, line comments) with a byte scan, handing the rest to `rustc_lexer`'s cursor.

It must produce exactly the tokens `LexedStr` produces, and two checks ensure that it does:

- Any text with a lexer error, a shebang or front matter goes to `LexedStr` as before, so error handling is rust-analyzer's own.
- `examples/lex_equality.rs` compares every token, every parser input call and the parser's output with `LexedStr`'s, over both corpora in two editions, 10,000 random texts and 10,000 mutated corpus files; and debug builds compare the two on every text they lex.

With this lexer, formatting the crates.io sample takes 15% fewer instructions.

## The flat tree

`cst::Tree` is the whole file as one `Vec` of 16-byte elements in preorder: each element is a node or a token, with its kind, its byte range in the source, and the index just past its subtree. Children are found by skipping subtrees; a token's text is a slice of the source, so no text is copied.

```text
     source: fn main() {}

index  kind              range   end
0      SOURCE_FILE       0..12   14
1        FN              0..12   14
2          FN_KW         0..2    3
3          WHITESPACE    2..3    4
4          NAME          3..7    6
5            IDENT       3..7    6
6          PARAM_LIST    7..9    9
7            L_PAREN     7..8    8
8            R_PAREN     8..9    9
9          WHITESPACE    9..10   10
10         BLOCK_EXPR    10..12  14
11           STMT_LIST   10..12  14
12             L_CURLY   10..11  13
13             R_CURLY   11..12  14
```

The tree of `fn main() {}`, as `cst::Tree::parse` builds it. `end` is the index after the element's subtree, so the next sibling of `NAME` (index 4) is at index 6.

A handle to a node or token (`Node`, `Token`) is a reference to the tree and an index. It is `Copy`, and making one allocates nothing. In `rowan`, the tree library rust-analyzer itself uses, reading a node creates a reference-counted "red" node on the heap. Replacing rowan's trees with this array cut the formatter's instructions by a quarter and its heap allocations by 43%.

On top of the array, a typed layer (`cst::ast`) has the same names as rust-analyzer's `ra_ap_syntax::ast`: `ast::Fn`, `expr.lhs()`, `item.name()`, `block.stmt_list()`. Its 501 accessors are generated from rust-analyzer's grammar file (`cst/rust.ungram`) by `cst/gen_ast.py`, and a test checks that the generated file is up to date.

The tree is built in one pass from the parser's events, attaching comments and whitespace to nodes by the rules rust-analyzer uses. Tests compare the result element by element with the tree rust-analyzer's own code builds through rowan, on every corpus file; debug builds make the same comparison on every tree they build.

## Macro arguments

rust-analyzer keeps the body of a macro call as an opaque token tree. rustfmt formats a call like `foo!(a + b, Vec<u8>)` by parsing each comma-separated argument as an expression, a type, a pattern or an item, in that order. chloro does the same with rust-analyzer's parser, and lays each argument's tree out at its original offset in the file, so spans, comments and source slices work inside macro arguments exactly as elsewhere. These fragment trees live in an arena that lasts for one formatting run.

## Where the two parsers disagree

rustfmt formats only what rustc's parser accepts, and leaves everything else alone. The two parsers accept almost the same syntax, with differences in both directions:

- **rust-analyzer accepts, rustc rejects.** rust-analyzer is deliberately permissive (it parses broken code in an editor) and accepts some removed or experimental syntax. `formatter/rustc_compat.rs` lists the rustc parse errors that matter, such as `<-` as an operator, `a..=` without an end, or misplaced doc comments, and returns such a file unchanged, as rustfmt would. Some of these rules depend on the rustfmt release being reproduced, because rustc 1.88's parser and rustc 1.97's differ.
- **rustc accepts, rust-analyzer rejects.** For a few constructs, such as a parenthesised first bound in a trait object (`Box<(Copy) + Send>`), a float literal used as a field name, or a `const` block as an item, `formatter/rustc_syntax.rs` rebuilds the tree with the node rustc would build, so the file is formatted as rustfmt formats it.

Two constructs remain where rust-analyzer's parser has no node to build; they are listed as [known differences](<https://lmmx.github.io/chloro/pr-preview/pr-104/compatibility/#known-differences>).
