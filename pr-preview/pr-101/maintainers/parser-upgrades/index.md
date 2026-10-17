# Upgrading the parser

chloro reimplements two pieces of rust-analyzer's parser crate for speed: its lexing step and its tree building. Both are pinned to the behaviour of one `ra_ap_parser` release, so after a version bump they must be checked against the new release.

## What is pinned

| dependency | version | why chloro depends on its exact behaviour |
| --- | --- | --- |
| `ra_ap_parser` | 0.0.307 | the grammar; `cst::lex` reproduces its `LexedStr::new` and `to_input`, and `TreeBuilder::steps_fused` reproduces its `intersperse_trivia` |
| `ra-ap-rustc_lexer` | 0.137.0 | the tokeniser `cst::lex` drives directly, the same version `ra_ap_parser` uses |
| `rustc-literal-escaper` | 0.0.4 | literal validation, as in `LexedStr` |
| `ra_ap_syntax`, `rowan` (dev) | 0.0.307, 0.15.15 | the reference the equality tests compare chloro's tree with |
| `cst/rust.ungram` | copied from the `ra_ap_syntax` 0.0.307 crate | the grammar the typed layer is generated from |

The three runtime crates are upgraded together: `ra-ap-rustc_lexer` and `rustc-literal-escaper` are the versions `ra_ap_parser` itself depends on.

## The equality harnesses

Each reimplementation has a harness that compares it with the code it replaces. Release builds do not run these comparisons; debug builds also run them inline, on every input.

| what | harness | what it compares |
| --- | --- | --- |
| the lexer front end (`cst/lex.rs`) | `examples/lex_equality.rs` | every token's kind and offset, every call that builds the parser's input, and the parser's output, against `LexedStr` and `to_input`; over the corpus in editions 2021 and 2024, 10,000 random texts and 10,000 mutated corpus files in all four editions |
| the lexer front end, inline | debug builds of `cst::lex` | the same comparison on every text a debug build lexes |
| the fused tree build (`TreeBuilder::steps_fused`) | `cst` tests `fused_build_equals_the_interspersed_build*` | the fused build against the build through rust-analyzer's `intersperse_trivia`, on sample sources and every corpus file |
| the flat tree as a whole | `cst` tests `flat_tree_equals_the_rowan_tree*` | chloro's tree against the rowan tree `ra_ap_syntax` builds, node by node |
| the fused build, inline | debug builds of `TreeBuilder::steps` | the fused build against `intersperse_trivia` on every tree a debug build makes |
| the typed layer | `cst::ast` test `generated_layer_is_up_to_date` | the checked-in `generated.rs` against `gen_ast.py`'s output |

## Bumping `ra_ap_parser`

1. **Bump the versions together.** In `chloro-core/Cargo.toml`, set `ra_ap_parser` and the development dependencies `ra_ap_syntax` and `rowan` to the new release's versions, and `ra-ap-rustc_lexer` and `rustc-literal-escaper` to the versions the new `ra_ap_parser` depends on (`cargo tree -p ra_ap_parser`).
2. **Regenerate the typed layer.** Copy `rust.ungram` from the root of the new `ra_ap_syntax` crate (in the cargo registry once the dependency is fetched) over `chloro-core/src/cst/rust.ungram`, and run the generator, passing `ra_ap_syntax`'s own generated nodes so the script checks that the accessor set matches:

   ```sh
   python3 chloro-core/src/cst/gen_ast.py chloro-core/src/cst/rust.ungram \
       chloro-core/src/cst/generated.rs \
       ~/.cargo/registry/src/index.crates.io-*/ra_ap_syntax-<version>/src/ast/generated/nodes.rs
   ```

   It prints any difference and exits with 1 if there is one.
3. **Read the diffs of the reimplemented code.** In the new `ra_ap_parser`, compare `lexed_str.rs` (`LexedStr::new`), `shortcuts.rs` (`to_input`, `intersperse_trivia`) and the trivia attachment rules with the previous release. A change there needs the same change in `cst/lex.rs` or `cst/mod.rs`; the module documentation of each names the source it follows.
4. **Run the harnesses.**

   ```sh
   cargo test -p chloro-core --lib cst                         # tree equality, generated layer
   cargo run --release -p chloro-core --example lex_equality   # 0 mismatches required
   cargo run --release -p chloro-core --example lex_equality -- --root ~/.cargo/registry/src
   cargo run -p chloro-core --example bench -- -n 1            # debug build: every inline check, on the corpus
   ```

   Each prints its mismatches and exits with 1 if there is any. The debug `bench` run is slow; it is the one that exercises the inline checks on every file.
5. **Run the conformance checks.** A parser change can change what parses. Record output hashes on the old version, then compare on the new one, and rerun `conform` and `config_matrix` as in [Development](<https://lmmx.github.io/chloro/pr-preview/pr-101/maintainers/development/#checks-for-a-change>). Any output that changes needs to be explained by a change in rust-analyzer's grammar, and compared with rustfmt.
6. **Check the rustc boundary.** `formatter/rustc_compat.rs` and `formatter/rustc_syntax.rs` encode where rust-analyzer's parser and rustc's disagree. A new release may close a gap (a `rustc_syntax` rebuild is no longer needed) or open one; the conformance ledger lists the cases each rule covers.

> [!WARNING]
>
> **Release builds do not check**
>
> The inline comparisons run only in debug builds. A release build on a new parser version that skipped step 4 would silently produce whatever the stale reimplementation produces.
