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
//! path of chloro (and so of rustfmt, which it reproduces) removes or shortens it; a
//! punctuation class only in the node kinds whose rewrite writes it on every path:
//!
//! | token class | counted | why (the module that writes it) |
//! |---|---|---|
//! | identifiers (`IDENT`) | yes, less 2 for a leading `r#` | written from the source (`types.rs` path segments, `patterns.rs`, `chains.rs` fields, `rewrite_ident`); none is dropped except the field names below. The `r#` is not counted because rustfmt 1.8.0 drops it from labels (`jump_label_text`, F1) |
//! | lifetimes (`LIFETIME_IDENT`) | yes, less 2 for `'r#` | copied as written (`types.rs`, `jump_label_text`); 1.8.0 writes `'r#a` as `'a` (F1) |
//! | keywords (`SELF_TYPE_KW` ..= `YEET_KW`) | yes | every keyword a rewrite reads is written back: closure `for`, `const`, `static`, `async`, `gen`, `move`, `use` (`rewrite_closure_fn_decl`, each tested with `has_token`); `&raw const`, `&raw mut`, `&mut` (`format_expr`); `if`, `else`, `match`, `loop`, `while`, `for`, `in`, `let` (`ControlFlow`, `rewrite_let`, `matches.rs`); `as` (`CastExpr`); `return`, `break`, `continue`, `yield`, `become`, `do yeet`; `true`, `false` (literals); `self`, `super`, `crate`, `Self` (path segments); `dyn`, `impl`, `unsafe`, `extern`, `fn` (`types.rs`; `force_explicit_abi` only adds `"C"`). The one keyword the formatter drops, the `in` of `pub(in crate)` (`format_visibility`), is in a visibility, which only items and macro arguments formatted as items have: both are uncounted subtrees |
//! | literals (`INT_NUMBER`, `FLOAT_NUMBER`, `CHAR`, `BYTE`, `STRING`, `BYTE_STRING`, `C_STRING`) | yes | copied from the source (`rewrite_literal`, `rewrite_string_lit`): `format_strings` is off and `hex_literal_case` and `float_literal_trailing_zero` are fixed at `Preserve` (`config.rs`) |
//! | the parentheses of an argument list (`ARG_LIST`) | yes, 2 | `rewrite_call` (a call) and `rewrite_method_call` (`chains.rs`) go through `overflow::rewrite_with_parens`, whose `Context::rewrite` writes `(` and `wrap_items` writes `)` on every path |
//! | the pipes of a closure | yes, 2 | `rewrite_closure_fn_decl` writes `"|" + params + "|"` (an empty list is `||`) for every closure, on every path of `rewrite_closure` and `rewrite_last_closure` |
//! | the `.` of a method call, field access or `.await` | yes, 1 | the chain items write `"." + name`, `".await"` (`ChainItem::rewrite`) |
//! | the `?` of a try | yes, 1 | the chain items write one `?` per try (`ChainItem::rewrite`, `tries`) |
//! | the `&` of a reference, the operator of a prefix expression | yes, 1 | `format_expr`: `"&"`, `"&mut "`, `"&raw const "`, `"&raw mut "`; `op_token().text()` (`!`, `-`, `*`) |
//! | the operator of a binary expression | yes, its length | `bin_op_text` is the operator token's text, written by `rewrite_pair`, `rewrite_all_pairs` and `rewrite_assignment` |
//! | the brackets of an index, a tuple, an array | yes, 2 | `rewrite_index` writes `[` and `]`; `rewrite_tuple` and `rewrite_array` go through `overflow` (`(`/`)`, `[`/`]`), a repeat array through `rewrite_pair` with `[` and `]` |
//! | other punctuation and delimiters, `_` | no | one-line lists drop trailing commas (`lists.rs`), `remove_nested_parens` drops parentheses, closure, match-arm and `use`-list braces are removed (`closures.rs`, `matches.rs`, `imports.rs`), `...` is written `..=`, `try!(e)` is written `e?`, a leading match-arm `|` follows `match_arm_leading_pipes` |
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
//! # Renderings that are never one line
//!
//! The bound decides an attempt by width. An attempt can also fail on structure: the node
//! has no one-line rendering at any shape. [`renders_match_arm`] holds for a node in which a
//! `match` with at least one arm is rendered, and which has no comment, no attribute and
//! no `asm!`:
//!
//! - `rewrite_match` (`matches.rs`; rustfmt matches.rs:72) writes `"{\n"` after the
//!   condition whenever the arm list is not empty, on every path that returns text; the
//!   one path that returns the source (`match x { /* c */ }`) has no arm;
//! - every rewrite of an expression contains the rewrite of each subexpression that the
//!   node writes (calls and method calls their arguments, closures their bodies with or
//!   without the block, pairs, prefixes, parentheses, index, field and `return`/`break`
//!   values, struct literal fields): the formatter drops delimiters, labels, field names
//!   (`use_field_init_shorthand`, which needs a path value) and attributes, never a
//!   subexpression; a block is written either through its expression on one line or
//!   through a nested visitor, whose result has a line break before the closing brace
//!   whenever the block has a statement;
//! - the rewrites that return source text instead (so a one-line `match` in the input
//!   would come back on one line) are excluded by the conditions: `recover_comment_removed`
//!   swaps in the source only when the source has a comment (`changed_comment_content`),
//!   `#[rustfmt::skip]` needs an attribute, `asm!` is returned as written
//!   (`format_expr`). A `match` in a macro call is in the token tree, not in the node's
//!   tree, and is not seen; a `match` inside a pattern's `const` block, a generic argument
//!   or an item is not counted (the 2021 slice-pattern fallback and type rewrites write
//!   source text there).
//!
//! So every rewrite of such a node that returns text has a line break, at every shape and
//! under every option: an attempt that keeps only a one-line result fails whatever it
//! renders.
//!
//! A chain wider than `chain_width` is never one line either ([`chain_too_wide`]), under
//! these conditions (`chains.rs`; rustfmt chains.rs):
//!
//! - the chain has at least two items after its parent (method calls, fields, `.await`; a
//!   `?` belongs to the item before it), so `ChainFormatterShared::child_count` is above one
//!   and `format_last_child` measures against `min(shape.width, chain_width)`;
//! - the receiver of the last item has a bound above `tab_spaces`: `format_root` moves items
//!   into the root only while the root is at most `tab_spaces - shape.offset` bytes, so the
//!   last item stays a child and the chain is not a pure root (which `wrap_str` alone
//!   measures);
//! - the parent is a path, an index expression or a call of a path, so a one-line root never
//!   ends in `"#` or consists of closing delimiters only, and `last_line_extendable` is false
//!   for it: the budget is then `min(shape.width, chain_width)` less the width of every
//!   rewrite but the last (and the last item's `?`s), and `fits_single_line` is set only if
//!   the last item's first line fits the rest; otherwise `join_rewrites` puts every item on a
//!   line of its own, or returns `None` under `force_one_line_chain`;
//! - the node has no comment (no chain item is a comment, `recover_comment_removed` keeps the
//!   rewrite, and the bound is not 0) and no attribute of its own (`#[rustfmt::skip]` would
//!   return the source).
//!
//! A one-line result is the items joined with nothing, so its width is at most the budget's
//! `chain_width`, and at least the node's bound: a bound above `chain_width` rules it out.
//! The options this reads are `chain_width` (with `use_small_heuristics` and `max_width`,
//! through `Settings::chain_width`) and `tab_spaces`, taken from the run's options at the
//! site.
//!
//! None of the predicates looks at the shape; the chain rule reads two options, the others
//! none.
//!
//! Debug builds check these facts on every expression rewrite (`format_expr`), and
//! render every attempt the bound or [`never_one_line`] skips and check that it fails
//! (`closures.rs`, `expr.rs`).

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
    for i in start..end {
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
            width += written_punctuation(elems, i);
            if width > limit {
                return width;
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

/// The punctuation every rendering of node `i` writes itself, as counted in the module
/// documentation: the parentheses of an argument list, the pipes of a closure, the `.` of a
/// method call, field or `.await`, the `?` of a try, the `&` of a reference, the operator
/// of a prefix or binary expression, the brackets of an index, a tuple or an array.
#[inline]
fn written_punctuation(elems: &[crate::cst::Elem], i: usize) -> usize {
    match elems[i].kind {
        ARG_LIST | CLOSURE_EXPR | INDEX_EXPR | TUPLE_EXPR | ARRAY_EXPR => 2,
        METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR | TRY_EXPR | REF_EXPR | PREFIX_EXPR => 1,
        BIN_EXPR => bin_op_len(elems, i),
        _ => 0,
    }
}

/// The length of a binary expression's operator: its first direct child token other than
/// whitespace and comments (a comment gives the node no bound anyway).
fn bin_op_len(elems: &[crate::cst::Elem], parent: usize) -> usize {
    let end = elems[parent].end as usize;
    let mut i = parent + 1;
    while i < end {
        let e = &elems[i];
        if !e.is_node && e.kind != WHITESPACE && e.kind != COMMENT {
            return e.len as usize;
        }
        i = e.end as usize;
    }
    0
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

/// Whether every rewrite of `node` that returns text has a line break, under options with
/// these `chain_width` and `tab_spaces`: [`renders_match_arm`] or [`chain_too_wide`] (see the
/// module documentation).
#[inline]
pub(crate) fn never_one_line(node: Node<'_>, chain_width: usize, tab_spaces: usize) -> bool {
    renders_match_arm(node) || chain_too_wide(node, chain_width, tab_spaces)
}

/// A `match` with an arm is rendered in `node`, and it has no comment, attribute or `asm!`.
/// One pass over the node's elements, made only when its source has a `=>`, which every arm
/// has (a byte search for the `>` first, the cheapest test).
fn renders_match_arm(node: Node<'_>) -> bool {
    let text = node.text();
    if !text.as_bytes().contains(&b'>') || !text.contains("=>") {
        return false;
    }
    let tree = node.tree();
    let elems = tree.elems();
    let start = node.index() as usize;
    let end = elems[start].end as usize;
    let mut arm = false;
    // Elements below `skip_end` belong to a subtree whose `match` arms are not counted.
    let mut skip_end = 0;
    for (i, e) in elems.iter().enumerate().take(end).skip(start) {
        if e.is_node {
            match e.kind {
                ATTR | ASM_EXPR => return false,
                MATCH_ARM if i >= skip_end => arm = true,
                CONST_BLOCK_PAT | GENERIC_ARG_LIST | FN | STRUCT | ENUM | UNION | TRAIT | IMPL
                | TYPE_ALIAS | CONST | STATIC | MODULE | MACRO_RULES | MACRO_DEF | EXTERN_BLOCK
                    if i >= skip_end =>
                {
                    skip_end = e.end as usize;
                }
                _ => {}
            }
        } else if e.kind == COMMENT {
            return false;
        }
    }
    arm
}

/// `node` is a chain (`chains.rs`) of at least two items after its parent, the parent a
/// path, an index expression or a call of a path, without attributes, whose receiver of the
/// last item has a bound above `tab_spaces` and whose own bound exceeds `chain_width` (so
/// it has no comment either). The checks run cheapest first: the kind, the source length
/// (which bounds the bound from above), the walk down the receivers, the receiver's bound
/// with an early exit, and the node's bound, with an early exit when its source has no `/`
/// (and so no comment, the one case in which a later token lowers the bound).
fn chain_too_wide(node: Node<'_>, chain_width: usize, tab_spaces: usize) -> bool {
    if !matches!(
        node.kind(),
        METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR | TRY_EXPR
    ) || node.text_range().len() as usize <= chain_width
    {
        return false;
    }
    let mut items = 0;
    let mut last_receiver = None;
    let mut cur = node;
    let parent = loop {
        let receiver = match cur.first_child() {
            Some(c) if c.kind() != ATTR => c,
            _ => return false,
        };
        match cur.kind() {
            TRY_EXPR => {}
            METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR => {
                items += 1;
                last_receiver.get_or_insert(receiver);
            }
            _ => break cur,
        }
        cur = receiver;
    };
    let parent_ok = match parent.kind() {
        PATH_EXPR | INDEX_EXPR => true,
        CALL_EXPR => parent.first_child().is_some_and(|c| c.kind() == PATH_EXPR),
        _ => false,
    };
    parent_ok
        && items >= 2
        && last_receiver.is_some_and(|r| min_width_up_to(r, tab_spaces) > tab_spaces)
        && if node.text().as_bytes().contains(&b'/') {
            one_line_min_width(node) > chain_width
        } else {
            min_width_up_to(node, chain_width) > chain_width
        }
}

/// Debug builds: checks the facts of the module documentation on a rewrite of `node` at
/// `shape`, under options with these `chain_width` and `tab_spaces`, that returned `result`.
#[cfg(debug_assertions)]
pub(crate) fn debug_check(
    node: Node<'_>,
    shape: super::shape::Shape,
    chain_width: usize,
    tab_spaces: usize,
    result: &str,
) {
    if result.contains('\n') {
        return;
    }
    assert!(
        !renders_match_arm(node),
        "a node with a rendered `match` arm has a one-line rendering: {node:?} -> {result:?}"
    );
    assert!(
        !chain_too_wide(node, chain_width, tab_spaces),
        "a chain wider than chain_width {chain_width} has a one-line rendering: \
         {node:?} -> {result:?}"
    );
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
        // `run`, `move`, `x`, `a`, `x`, `0`: 3 + 4 + 1 + 1 + 1 + 1, and the two argument
        // lists' parentheses and the closure's pipes: 3 x 2.
        assert_eq!(bound("run(move |x| a(x, 0))"), (17, true));
        // Field names are not counted (`use_field_init_shorthand`), their values are.
        assert_eq!(bound("S { x: y, z }").0, 3);
        // Operators, dots, `?`, `&`, brackets; not parentheses, which may be removed.
        assert_eq!(
            bound("&a.b()? + !c[d] && (e)").0,
            5 + 1 + 1 + 2 + 1 + 1 + 1 + 2 + 2
        );
        // Attributes, labels, macro calls and a raw prefix are not counted.
        assert_eq!(bound("r#match + 'r#a: loop {}").0, 5 + 1 + 4);
        assert_eq!(bound("f(#[cfg(x)] a)").0, 2 + 2);
        assert_eq!(bound("g(vec![a, b])").0, 1 + 2);
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
    fn never_one_line_needs_a_rendered_match_arm() {
        let nl = |expr: &str| {
            let src = format!("fn f() {{ {expr}; }}");
            let tree = Tree::parse(&src, Edition::Edition2024)
                .unwrap_or_else(|| panic!("{src} does not parse"));
            let node = tree
                .root()
                .descendants()
                .find(|n| n.kind() == EXPR_STMT)
                .and_then(|n| n.first_child())
                .unwrap();
            renders_match_arm(node)
        };
        assert!(nl("a.map(|x| match x { _ => 1 })"));
        assert!(nl("f(S { a: match x { A => 1, B => 2 } })"));
        // No arm: `match x {}` is one line.
        assert!(!nl("a.map(|x| match x {})"));
        // A comment or an attribute anywhere: the source may come back.
        assert!(!nl("a.map(|x| match x { _ => 1 /* c */ })"));
        assert!(!nl("f(#[rustfmt::skip] match x { _ => 1 })"));
        // A `match` in a macro call, a generic argument or an item is not counted.
        assert!(!nl("a.map(|x| m!(match x { _ => 1 }))"));
        assert!(!nl("f::<{ match N { _ => 1 } }>()"));
        assert!(!nl("f({ fn g() { match x { _ => 1 } } })"));
        assert!(!nl("a.map(|x| x + 1)"));
    }

    #[test]
    fn chain_too_wide_needs_two_items_a_long_receiver_and_a_wide_bound() {
        let ctw = |expr: &str, chain_width: usize| {
            let src = format!("fn f() {{ {expr}; }}");
            let tree = Tree::parse(&src, Edition::Edition2024).unwrap();
            let node = tree
                .root()
                .descendants()
                .find(|n| n.kind() == EXPR_STMT)
                .and_then(|n| n.first_child())
                .unwrap();
            chain_too_wide(node, chain_width, 4)
        };
        // `items.iter().map(f).count()`: bound 27, every byte.
        assert!(ctw("items.iter().map(f).count()", 26));
        assert!(!ctw("items.iter().map(f).count()", 27));
        // The `?`s belong to the items; `.await` and fields are items.
        assert!(ctw("items.iter()?.map(f)?.count()?", 25));
        assert!(ctw("state.inner.wait().await", 10));
        // One item after the parent: `chain_width` does not apply.
        assert!(!ctw("items.iter_with_a_long_name()", 10));
        // The receiver of the last item no longer than `tab_spaces` may be absorbed.
        assert!(!ctw("a.b.into_iter_with_a_long_name()", 10));
        // Parents: a path, an index, a call of a path; not a literal or a closure.
        assert!(ctw("f(x).iter().count()", 10));
        assert!(ctw("v[0].iter().count()", 10));
        assert!(!ctw("r#\"abcdef\"#.trim().len()", 10));
        assert!(!ctw("(|| x)().iter().count()", 10));
        // A comment: no bound; a division is not one.
        assert!(!ctw("items.iter() /* c */ .map(f).count()", 10));
        assert!(ctw("items.iter().map(a / b).count()", 10));
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
        assert_eq!(one_line_min_width(call), 8);
        // A one-line rewrite may be one column wider than its shape.
        assert!(one_line_fits_checked_and_exceeds(call, 6));
        assert!(!one_line_fits_checked_and_exceeds(call, 7));
        assert!(!one_line_fits_checked_and_exceeds(call, 9));
        // The scan stops once the limit is passed.
        assert_eq!(min_width_up_to(call, 2), 3);
    }
}
