//! A lower bound on the width of a node's one-line rendering (chloro only: rustfmt decides
//! whether a candidate fits by rendering it and measuring the text).
//!
//! Some attempts keep their result only if it is one line, or one line that fits: the
//! closure body without a block (`closures.rs`, `rewrite_closure_expr` under
//! `veto_multiline`) and `if c { a } else { b }` on one line (`expr.rs`,
//! `ControlFlow::rewrite_single_line`). When every one-line rendering of the node is wider
//! than the attempt allows, the attempt fails whatever the rendering is, so it need not be
//! made. [`one_line_min_width`] gives a width below which no one-line rendering can be,
//! from the node's tokens and without rendering anything.
//!
//! # The bound
//!
//! The sum, over the tokens of the node that every rendering writes unchanged, of a lower
//! bound on each token's display width. A token class is counted only if no formatting
//! path of chloro (and so of rustfmt, which it reproduces) removes or shortens it:
//!
//! | token class | counted | why (the module that writes it) |
//! |---|---|---|
//! | identifiers (`IDENT`) | yes, less 2 for a leading `r#` | written from the source (`types.rs` path segments, `patterns.rs`, `chains.rs` fields, `rewrite_ident`); none is dropped except the field names below. The `r#` is not counted because rustfmt 1.8.0 drops it from labels (`jump_label_text`, F1) |
//! | lifetimes (`LIFETIME_IDENT`) | yes, less 2 for `'r#` | copied as written (`types.rs`, `jump_label_text`); 1.8.0 writes `'r#a` as `'a` (F1) |
//! | keywords (`SELF_TYPE_KW` ..= `YEET_KW`) | yes | every keyword a rewrite reads is written back: closure `for`, `const`, `static`, `async`, `gen`, `move`, `use` (`rewrite_closure_fn_decl`, each tested with `has_token`); `&raw const`, `&raw mut`, `&mut` (`format_expr`); `if`, `else`, `match`, `loop`, `while`, `for`, `in`, `let` (`ControlFlow`, `rewrite_let`, `matches.rs`); `as` (`CastExpr`); `return`, `break`, `continue`, `yield`, `become`, `do yeet`; `true`, `false` (literals); `self`, `super`, `crate`, `Self` (path segments); `dyn`, `impl`, `unsafe`, `extern`, `fn` (`types.rs`; `force_explicit_abi` only adds `"C"`). The one keyword the formatter drops, the `in` of `pub(in crate)` (`format_visibility`), is in a visibility, which only items and macro arguments formatted as items have: both are uncounted subtrees |
//! | literals (`INT_NUMBER`, `FLOAT_NUMBER`, `CHAR`, `BYTE`, `STRING`, `BYTE_STRING`, `C_STRING`) | yes | copied from the source (`rewrite_literal`, `rewrite_string_lit`): `format_strings` is off and `hex_literal_case` and `float_literal_trailing_zero` are fixed at `Preserve` (`config.rs`) |
//! | punctuation and delimiters, `_` | no | one-line lists drop trailing commas (`lists.rs`), `remove_nested_parens` drops parentheses, closure, match-arm and `use`-list braces are removed (`closures.rs`, `matches.rs`, `imports.rs`), `...` is written `..=`, `try!(e)` is written `e?` |
//! | whitespace | no | replaced by the formatter's own |
//! | comments | no bound | a node with a comment anywhere in it gets the bound 0 |
//!
//! Subtrees whose tokens are not counted:
//! - attributes (`ATTR`): `merge_derives` merges `derive` lists, and rustfmt's
//!   `normalize_doc_attributes` (fixed at `false` in chloro) writes `#[doc = ".."]` as
//!   `///..`;
//! - labels (`LABEL`): rustfmt 1.8.0 unwraps a labelled block that is a closure body and
//!   drops the label (`get_inner_expr`, F2);
//! - the name of a struct-literal or struct-pattern field (the `NAME_REF` child of
//!   `RECORD_EXPR_FIELD`, `RECORD_PAT_FIELD`): `use_field_init_shorthand` writes `x: x` as
//!   `x` (the expression side is counted, and is written as the name when it is dropped);
//! - a macro call's path and token tree: `use_try_shorthand` writes `try!(e)` as `e?`, and
//!   arguments formatted as items or statements go through the normalisations below;
//! - `use` declarations and items, which `imports.rs` and `items.rs` normalise (`a::{b}` to
//!   `a::b`, `pub(in crate)` to `pub(crate)`) and which never render on one line inside an
//!   expression.
//!
//! The display width of a counted token is at least its number of ASCII bytes other than
//! `\n` and `\r`: `unicode_str_width` (`utils.rs`, unicode-width 0.1.14) counts every other
//! ASCII character as one column and no character as less than none, and a byte length is
//! at least the display width. A one-line rendering contains each counted token whole, the
//! tokens side by side, so its display width and its length are at least the sum. A token
//! with a newline cannot be in a one-line rendering at all, which makes its count vacuous.
//!
//! # Renderings that fit their shape, to one column
//!
//! An attempt that accepts any one-line result without measuring it (the closure body
//! without a block) needs a second fact before it can be skipped: how much wider than the
//! shape it was asked for a one-line result can be. rustfmt's rewrites mostly check the
//! width, but not all: a one-line string literal in style edition 2024 is returned as
//! written (`rewrite_string_lit`), and so are `#[rustfmt::skip]` expressions, `asm!`, raw
//! strings that fail to fit, macro calls left as written, `return`, `break`, `continue`,
//! `yield` and `_`; a rewrite that writes such a child next to a prefix (`&"..."`) passes
//! the excess on. [`one_line_fits_checked`] holds for a node whose every node is of a kind
//! below and that has no string literal, no comment and no assignment operator. By
//! induction over the node, each of these rewrites returns one-line text at most one column
//! wider than its shape when its children's one-line texts are at most one column wider
//! than theirs:
//!
//! | kind | rewrite | the check | excess |
//! |---|---|---|---|
//! | `PATH_EXPR` (`PATH`, `PATH_SEGMENT`, `NAME_REF`, no generic arguments, no `<T>::`) | `types::rewrite_path` | a one-identifier path returns it only if `ident.len() <= shape.width`; otherwise each segment is written after `shrink_left` by the text so far and `offset_left(ident_len)?` (`rewrite_segment`) | 0 |
//! | `LITERAL` (not a string) | `rewrite_literal` | `wrap_str` | 0 |
//! | `CALL_EXPR` (`ARG_LIST`), `TUPLE_EXPR`, `ARRAY_EXPR` | `overflow::rewrite_with_parens`, `rewrite_with_square_brackets`; a repeat array through `rewrite_pair` | the list stays on the callee's line only if `wrap_items` finds `first_line_width(items) + 1 <= shape.width - callee` (`force_single_line`), otherwise it inserts a newline; the closing delimiter is not in that sum. The tactic is chosen against `one_line_width` = width − callee − 2, but a single overflowed item whose rewrite has one newline is replaced by its rewrite at `nested_shape` (`try_overflow_last_item`, rustfmt overflow.rs:502) without a new width test, so only `wrap_items`' test binds: `Ok(self.into_inner_unchecked())` comes out at 31 columns for a shape of 30 (arrayvec 0.7, found by the registry run of the harness) | 1 |
//! | `METHOD_CALL_EXPR`, `FIELD_EXPR`, `TRY_EXPR`, `AWAIT_EXPR` | `chains::rewrite_chain` | `wrap_str` on the joined chain; a chain without children is its parent's rewrite at `sub_width(tries)` followed by the `?`s | 0, or the parent's |
//! | `BIN_EXPR` (no assignment) | `rewrite_all_pairs`, else `rewrite_pair` | `wrap_str` (`rewrite_pairs_one_line`); `rewrite_pair`: `one_line_width <= shape.width` over the actual texts; in `rewrite_pairs_multiline` an operand kept on the line is rewritten at `offset_left` by the line so far | 0, or the last operand's |
//! | `PREFIX_EXPR`, `REF_EXPR` | `rewrite_unary_prefix` | the operand at `offset_left(prefix.len())` | the operand's |
//! | `PAREN_EXPR` | `rewrite_paren` | the inner expression at `offset_left(1)` and `sub_width(1)`; no comment, so nothing else on the line | the inner one's |
//! | `INDEX_EXPR` | `rewrite_index` | the base at the shape, the index at `offset_left(base + 1)` and `sub_width(1)`: a base wider than the shape leaves no index shape | the index's |
//! | `CLOSURE_EXPR` (`PARAM_LIST`, `PARAM`, `IDENT_PAT`, `NAME`; no types, no block body) | `rewrite_closure` | the body at `shape.offset_left(last_line_width(prefix) + 1)?`, so the prefix fits and the body is measured against the rest | the body's |
//!
//! No rule adds an excess to a child's: a list re-measures its items in `wrap_items`, a chain
//! and `rewrite_pairs_one_line` in `wrap_str`, and the others shift the child's shape by
//! what they write before it. So a one-line rewrite of such a node is at most one column
//! wider than its shape, and a node whose bound exceeds the width by more than one has no
//! one-line rewrite at that width.
//!
//! Neither predicate looks at the shape or the options, so both are functions of the node.
//!
//! Debug builds check both facts on every expression rewrite (`format_expr`), and render
//! every attempt the bound skips and check that it fails (`closures.rs`, `expr.rs`).

use ra_ap_parser::SyntaxKind::{self, *};

use crate::cst::Node;

/// A lower bound on the display width of every one-line rendering of `node`; 0 when the
/// node contains a comment.
pub(crate) fn one_line_min_width(node: Node<'_>) -> usize {
    min_width_up_to(node, usize::MAX)
}

/// [`one_line_min_width`], except that the scan stops once the sum exceeds `limit` and
/// returns the partial sum: exact for a node without comments (the only case in which a
/// later token lowers the result, to 0).
fn min_width_up_to(node: Node<'_>, limit: usize) -> usize {
    let tree = node.tree();
    let elems = tree.elems();
    let src = tree.source().as_bytes();
    let start = node.index() as usize;
    let end = elems[start].end as usize;
    let mut width = 0;
    // Elements below `skip_end` belong to a subtree whose tokens are not counted.
    let mut skip_end = 0;
    // The direct child of a field or macro call whose tokens are not counted.
    let mut skip_child = usize::MAX;
    for i in start + 1..end {
        let e = &elems[i];
        if e.is_node {
            if i < skip_end {
                continue;
            }
            if i == skip_child || is_uncounted_subtree(e.kind) {
                skip_end = e.end as usize;
                continue;
            }
            if let Some(child) = uncounted_child_kind(e.kind) {
                skip_child = direct_child(elems, i, child);
            }
        } else if e.kind == COMMENT {
            return 0;
        } else if i >= skip_end && is_counted_token(e.kind) {
            let text = &src[e.start as usize..(e.start + e.len) as usize];
            width += token_min_width(e.kind, text);
            if width > limit {
                return width;
            }
        }
    }
    width
}

/// Keywords, literals, identifiers and lifetimes: the token classes written unchanged.
#[inline]
fn is_counted_token(kind: SyntaxKind) -> bool {
    let k = kind as u16;
    (SELF_TYPE_KW as u16..=YEET_KW as u16).contains(&k)
        || (BYTE as u16..=STRING as u16).contains(&k)
        || kind == IDENT
        || kind == LIFETIME_IDENT
}

/// Subtrees whose tokens are not counted (see the module documentation).
#[inline]
fn is_uncounted_subtree(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        ATTR | LABEL
            | TOKEN_TREE
            | USE
            | FN
            | STRUCT
            | ENUM
            | UNION
            | TRAIT
            | IMPL
            | TYPE_ALIAS
            | CONST
            | STATIC
            | MODULE
            | MACRO_RULES
            | MACRO_DEF
            | EXTERN_BLOCK
            | EXTERN_CRATE
    )
}

/// The kind of the direct child of a `kind` node whose tokens are not counted.
#[inline]
fn uncounted_child_kind(kind: SyntaxKind) -> Option<SyntaxKind> {
    match kind {
        RECORD_EXPR_FIELD | RECORD_PAT_FIELD => Some(NAME_REF),
        MACRO_CALL => Some(PATH),
        _ => None,
    }
}

/// The index of the first direct child of node `parent` of kind `kind`, or `usize::MAX`.
fn direct_child(elems: &[crate::cst::Elem], parent: usize, kind: SyntaxKind) -> usize {
    let end = elems[parent].end as usize;
    let mut i = parent + 1;
    while i < end {
        let e = &elems[i];
        if e.is_node && e.kind == kind {
            return i;
        }
        i = e.end as usize;
    }
    usize::MAX
}

/// A lower bound on the display width of a token on one line: its ASCII bytes other than
/// `\n` and `\r`, less a raw prefix.
#[inline]
fn token_min_width(kind: SyntaxKind, text: &[u8]) -> usize {
    let mut w = if text.iter().all(|&b| (b' '..=b'~').contains(&b)) {
        text.len()
    } else {
        text.iter()
            .filter(|&&b| b.is_ascii() && b != b'\n' && b != b'\r')
            .count()
    };
    let raw = match kind {
        IDENT => text.starts_with(b"r#"),
        LIFETIME_IDENT => text.starts_with(b"'r#"),
        _ => false,
    };
    if raw {
        w -= 2;
    }
    w
}

/// Whether `format_expr` returns, for `node` at any shape, only one-line texts that fit
/// the shape's width (see the module documentation for the kinds and their checks).
pub(crate) fn one_line_fits_checked(node: Node<'_>) -> bool {
    let tree = node.tree();
    let elems = tree.elems();
    let start = node.index() as usize;
    let end = elems[start].end as usize;
    elems[start..end].iter().all(|e| {
        if e.is_node {
            matches!(
                e.kind,
                PATH_EXPR
                    | PATH
                    | PATH_SEGMENT
                    | NAME_REF
                    | NAME
                    | LITERAL
                    | CALL_EXPR
                    | ARG_LIST
                    | TUPLE_EXPR
                    | ARRAY_EXPR
                    | METHOD_CALL_EXPR
                    | FIELD_EXPR
                    | TRY_EXPR
                    | AWAIT_EXPR
                    | BIN_EXPR
                    | PREFIX_EXPR
                    | REF_EXPR
                    | PAREN_EXPR
                    | INDEX_EXPR
                    | CLOSURE_EXPR
                    | PARAM_LIST
                    | PARAM
                    | IDENT_PAT
            )
        } else {
            !matches!(
                e.kind,
                STRING
                    | BYTE_STRING
                    | C_STRING
                    | COMMENT
                    | EQ
                    | PLUSEQ
                    | MINUSEQ
                    | STAREQ
                    | SLASHEQ
                    | PERCENTEQ
                    | CARETEQ
                    | AMPEQ
                    | PIPEEQ
                    | SHLEQ
                    | SHREQ
            )
        }
    })
}

/// Whether no rewrite of `node` at a shape of width `width` can be one line: the node
/// passes [`one_line_fits_checked`], so a one-line rewrite is at most `width + 1` wide, and
/// every one-line rendering of it is wider than that. The node's source length bounds
/// [`one_line_min_width`] from above, so a node no longer than `width + 1` is answered
/// without a scan; the fit predicate rules out comments, so the width scan may stop as soon
/// as the sum exceeds `width + 1`.
#[inline]
pub(crate) fn one_line_fits_checked_and_exceeds(node: Node<'_>, width: usize) -> bool {
    let limit = width + 1;
    node.text_range().len() as usize > limit
        && one_line_fits_checked(node)
        && min_width_up_to(node, limit) > limit
}

/// Debug builds: checks the two facts of the module documentation on a rewrite of `node` at
/// `shape` that returned `result`.
#[cfg(debug_assertions)]
pub(crate) fn debug_check(node: Node<'_>, shape: super::shape::Shape, result: &str) {
    if result.contains('\n') {
        return;
    }
    let width = super::utils::unicode_str_width(result);
    let min = one_line_min_width(node);
    assert!(
        min <= width,
        "one-line width bound {min} above the rendering's width {width}: {node:?} -> {result:?}"
    );
    if one_line_fits_checked(node) {
        assert!(
            width <= shape.width + 1,
            "one-line rendering more than a column wider ({width}) than its shape ({}): \
             {node:?} -> {result:?}",
            shape.width
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::Tree;
    use ra_ap_parser::Edition;

    /// The bound and the fit predicate of the first expression of `f`'s body.
    fn bound(expr: &str) -> (usize, bool) {
        let src = format!("fn f() {{ {expr}; }}");
        let tree = Tree::parse(&src, Edition::Edition2024).unwrap();
        let node = tree
            .root()
            .descendants()
            .find(|n| n.kind() == EXPR_STMT)
            .and_then(|n| n.first_child())
            .unwrap();
        (one_line_min_width(node), one_line_fits_checked(node))
    }

    #[test]
    fn counts_identifiers_keywords_and_literals_only() {
        // `run`, `move`, `x`, `a`, `x`, `0`: 3 + 4 + 1 + 1 + 1 + 1.
        assert_eq!(bound("run(move |x| a(x, 0))"), (11, true));
        // Field names are not counted (`use_field_init_shorthand`), their values are.
        assert_eq!(bound("S { x: y, z }").0, 3);
        // Attributes, labels, macro calls and a raw prefix are not counted.
        assert_eq!(bound("r#match + 'r#a: loop {}").0, 5 + 4);
        assert_eq!(bound("f(#[cfg(x)] a)").0, 2);
        assert_eq!(bound("g(vec![a, b])").0, 1);
        // Non-ASCII characters count as nothing, ASCII ones as one column.
        assert_eq!(bound("\"é\"").0, 2);
        // A comment anywhere: no bound.
        assert_eq!(bound("a(b /* c */)"), (0, false));
    }

    #[test]
    fn fit_predicate_excludes_unchecked_rewrites() {
        assert!(bound("a.b(c)?.d[e] + -f * (g, [h; 2])").1);
        assert!(!bound("&\"string\"").1);
        assert!(!bound("|| return").1);
        assert!(!bound("a = b").1);
        assert!(!bound("f(m!())").1);
        assert!(!bound("|x: u8| x").1);
        assert!(!bound("Vec::<u8>::new()").1);
    }

    #[test]
    fn exceeds_answers_short_nodes_without_the_scan() {
        let src = "fn f() { abc(def); }";
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let call = tree
            .root()
            .descendants()
            .find(|n| n.kind() == CALL_EXPR)
            .unwrap();
        assert_eq!(one_line_min_width(call), 6);
        // A one-line rewrite may be one column wider than its shape.
        assert!(one_line_fits_checked_and_exceeds(call, 4));
        assert!(!one_line_fits_checked_and_exceeds(call, 5));
        assert!(!one_line_fits_checked_and_exceeds(call, 8));
        // The scan stops once the limit is passed.
        assert_eq!(min_width_up_to(call, 2), 3);
    }
}
