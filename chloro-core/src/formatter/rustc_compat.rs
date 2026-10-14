//! Syntax that rust-analyzer's parser accepts and rustc's parser rejects.
//!
//! rustfmt formats only files that rustc's parser accepts; on a parse error it leaves the
//! file as it is. rust-analyzer's parser is more permissive: it accepts experimental and
//! removed syntax, and leaves some rustc parse errors to a later validation pass, which
//! also reports semantic errors that rustc's parser does not (and that rustfmt therefore
//! formats through). [`rejected_by_rustc`] reproduces the rustc parse errors that matter
//! for formatting. Each rule below was checked against rustfmt 1.9.0 (rustc 1.97.0);
//! the rules that depend on the toolchain take the [`RustfmtVersion`] and reproduce the
//! rustc 1.88.0 parser for `RustfmtVersion::V1_8_0` (classes P1, P2 and P8 of
//! `docs/journal/2026-10-14-autoresearch/rustfmt-versions.md`).

use ra_ap_parser::{Edition, SyntaxKind, T};

use super::config::RustfmtVersion;
use crate::cst::ast::is_assign_op;
use crate::cst::{Elem, Element, Node, NodeOrToken, Tree};

// The rules run on every file, in one pass over the preorder elements of the tree. A node
// costs a dispatch on its kind, and a token a few comparisons unless it is a comment, a `{`,
// a C string or follows an outer doc comment. The rules that look at ancestors find them
// on demand ([`Ancestors`]), after their other conditions hold.

fn has_token(node: Node<'_>, token_kind: SyntaxKind) -> bool {
    node.children_with_tokens()
        .any(|c| matches!(c, NodeOrToken::Token(t) if t.kind() == token_kind))
}

fn has_node(node: Node<'_>, node_kind: SyntaxKind) -> bool {
    node.children().any(|n| n.kind() == node_kind)
}

/// The first child node that is not an attribute or a label: the condition of an `if` or
/// `while`.
fn condition(node: Node<'_>) -> Option<Node<'_>> {
    node.children()
        .find(|n| !matches!(n.kind(), SyntaxKind::ATTR | SyntaxKind::LABEL))
}

/// The ancestors of an element within the walked subtree, innermost first, each with the
/// ancestor's child on the path. A parent is found by searching backwards from its child
/// (the tree keeps no parent column), which costs the elements of the preceding siblings.
struct Ancestors<'a> {
    tree: &'a Tree<'a>,
    /// The index of the element whose parent comes next.
    child: u32,
    /// The index of the walked subtree's root: its ancestors are not the walk's.
    first: u32,
}

impl<'a> Ancestors<'a> {
    fn of(tree: &'a Tree<'a>, i: u32, first: u32) -> Ancestors<'a> {
        Ancestors {
            tree,
            child: i,
            first,
        }
    }

    fn of_node(node: Node<'a>, first: u32) -> Ancestors<'a> {
        Ancestors::of(node.tree(), node.index(), first)
    }
}

impl<'a> Iterator for Ancestors<'a> {
    /// `(ancestor, its child on the path)`; the child is a node except for the first item of
    /// a token's ancestors.
    type Item = (Node<'a>, u32);

    fn next(&mut self) -> Option<(Node<'a>, u32)> {
        let child = self.child;
        let parent = (self.first..child).rev().find(|&j| {
            let e = self.tree.elem(j);
            e.is_node && e.end > child
        })?;
        self.child = parent;
        Some((self.tree.node_at(parent), child))
    }
}

/// `if c { #![attr] }`, `if c { //! doc }`: rustc rejects inner attributes in the blocks of
/// an `if` or `else`. `i` is the index of an inner attribute or inner doc comment.
fn inner_attr_in_if_block(tree: &Tree<'_>, i: u32, first: u32) -> bool {
    let mut kinds = Ancestors::of(tree, i, first).map(|(p, _)| p.kind());
    kinds.next() == Some(SyntaxKind::STMT_LIST)
        && kinds.next() == Some(SyntaxKind::BLOCK_EXPR)
        && kinds.next() == Some(SyntaxKind::IF_EXPR)
}

/// The parser being reproduced: the edition and the rustc release of the rustfmt build.
#[derive(Clone, Copy)]
struct Toolchain {
    edition: Edition,
    rustfmt_version: RustfmtVersion,
}

/// The state of one walk: the toolchain and the token-level trackers.
struct Walker {
    toolchain: Toolchain,
    /// Whether the previous token was a `:` — rustc lexes `:::` as `::` followed by `:`,
    /// rust-analyzer as `:` followed by `::`.
    prev_colon: bool,
    /// An inner doc comment is allowed here: at the start of a file or of a module, block,
    /// `impl`/`trait`, `extern` or `match` body, after inner attributes only.
    inner_doc_ok: bool,
    /// An outer doc comment was seen and nothing it documents yet.
    pending_outer_doc: bool,
}

/// rustc accepts `let` only as an operand of a `&&` chain in an `if`/`while` condition or
/// a match guard, and accepts a `let` chained with `&&` in an `if`/`while` condition only
/// from edition 2024.
fn let_expr_allowed(let_expr: Node<'_>, first: u32, toolchain: Toolchain) -> bool {
    // P1: rustc 1.97.0 rejects a chained `let` before edition 2024 (`let chains are only
    // allowed in Rust 2024 or later`, stabilised in Rust 1.88.0 for edition 2024 only);
    // rustc 1.88.0's parser accepts the chain in every edition and leaves the gate to a
    // later pass, so rustfmt 1.8.0 formats it.
    let chain_allowed =
        toolchain.edition.at_least_2024() || toolchain.rustfmt_version < RustfmtVersion::V1_9_0;
    let mut chained = false;
    for (parent, child) in Ancestors::of_node(let_expr, first) {
        match parent.kind() {
            SyntaxKind::BIN_EXPR if has_token(parent, T![&&]) => chained = true,
            // A `let` in a branch block is reached through the block, not directly.
            SyntaxKind::IF_EXPR | SyntaxKind::WHILE_EXPR => {
                return condition(parent).is_some_and(|c| c.index() == child)
                    && (!chained || chain_allowed);
            }
            SyntaxKind::MATCH_GUARD => return true,
            _ => return false,
        }
    }
    false
}

/// `a..b..c`: range operators do not chain. A prefix range (`..`, `..=b`) may be the end
/// of another range (`..=..`), as it parses as a single operand.
fn chained_range(range: Node<'_>) -> bool {
    let is_op = |c: &Element<'_>| matches!(c.kind(), T![..] | T![..=] | T![...]);
    let has_start = |n: Node<'_>| {
        n.children_with_tokens()
            .find(|c| !c.kind().is_trivia())
            .is_some_and(|c| c.as_node().is_some())
    };
    let mut before_op = true;
    for child in range.children_with_tokens() {
        if is_op(&child) {
            before_op = false;
            continue;
        }
        let NodeOrToken::Node(n) = child else {
            continue;
        };
        if n.kind() != SyntaxKind::RANGE_EXPR {
            continue;
        }
        if before_op || has_start(n) {
            return true;
        }
    }
    false
}

fn is_comparison_op(kind: SyntaxKind) -> bool {
    matches!(kind, T![==] | T![!=] | T![<] | T![>] | T![<=] | T![>=])
}

/// A binary expression whose operator is a comparison.
fn is_comparison(node: Node<'_>) -> bool {
    node.kind() == SyntaxKind::BIN_EXPR
        && node
            .children_with_tokens()
            .any(|c| is_comparison_op(c.kind()))
}

/// The rules for a binary expression, which share one look at its operator (the first
/// token among its children that is not trivia).
fn bin_expr_rejected(bin: Node<'_>, first: u32) -> bool {
    let mut children = bin.children_with_tokens();
    let Some(op) = children.by_ref().find_map(|c| match c {
        NodeOrToken::Token(t) if !t.kind().is_trivia() => Some(t.kind()),
        _ => None,
    }) else {
        return false;
    };
    // `a < b > c`, `a == b == c`: comparison operators do not chain (they share one
    // precedence level and have no associativity in rustc).
    if is_comparison_op(op) && bin.children().any(is_comparison) {
        return true;
    }
    // `a <- b`: the element right after `<`.
    if op == T![<]
        && children
            .next()
            .is_some_and(|rhs| first_token_kind(&rhs) == Some(T![-]))
    {
        return true;
    }
    is_assign_op(op) && prefix_range_assigned(bin, first)
}

/// The kind of the first token under `child`, trivia included.
fn first_token_kind(child: &Element<'_>) -> Option<SyntaxKind> {
    match child {
        NodeOrToken::Token(t) => Some(t.kind()),
        NodeOrToken::Node(n) => n.first_token().map(|t| t.kind()),
    }
}

// `a <- b` (in `bin_expr_rejected`): rustc's lexer glues `<` and an adjacent `-` into one
// token, `<-` (`LArrow`), which is not a binary operator ("unexpected token: `<-`");
// rust-analyzer reads `a < -b`.

fn is_assignment(bin: Node<'_>) -> bool {
    bin.kind() == SyntaxKind::BIN_EXPR && bin.children_with_tokens().any(|c| is_assign_op(c.kind()))
}

/// A range without a start: `..`, `..b`, `..=b`.
fn is_prefix_range(node: Node<'_>) -> bool {
    node.kind() == SyntaxKind::RANGE_EXPR
        && node
            .children_with_tokens()
            .find(|c| !c.kind().is_trivia())
            .is_some_and(|c| c.as_node().is_none())
}

/// `..a = b`: rustc's `parse_expr_assoc_with` returns a range that starts an expression
/// without looking for a binary operator after it, so the `=` is unexpected. Where an outer
/// operator loop takes over after the range, the `=` is that loop's: on the right of an
/// assignment (`x = ..a = b` reads `(x = ..a) = b`), and as the operand of a closure,
/// `return`, `break`, `yield`, `become` or `do yeet` (`|x| ..a = b` reads `(|x| ..a) = b`).
/// `bin` is an assignment.
fn prefix_range_assigned(bin: Node<'_>, first: u32) -> bool {
    if !bin.children().next().is_some_and(is_prefix_range) {
        return false;
    }
    match Ancestors::of_node(bin, first).next() {
        // The right operand: some node precedes it in the parent.
        Some((parent, _)) if is_assignment(parent) => parent.children().next() == Some(bin),
        Some((parent, _)) => !matches!(
            parent.kind(),
            SyntaxKind::CLOSURE_EXPR
                | SyntaxKind::RETURN_EXPR
                | SyntaxKind::BREAK_EXPR
                | SyntaxKind::YIELD_EXPR
                | SyntaxKind::BECOME_EXPR
                | SyntaxKind::YEET_EXPR
        ),
        None => true,
    }
}

/// `a..=`, `a...`: an inclusive range without an end (E0586), in an expression or a pattern.
fn inclusive_without_end(range: Node<'_>) -> bool {
    let mut children = range.children_with_tokens();
    for c in children.by_ref() {
        if matches!(c.kind(), T![..=] | T![...]) {
            return !children.any(|c| c.as_node().is_some());
        }
    }
    false
}

/// `&dyn A + B`, `*const impl A + B`, `x as dyn A + B`, `fn() -> dyn A + B`,
/// `Fn() -> impl A + B`: rustc parses these types with `AllowPlus::No` and reports
/// "ambiguous `+` in a type" for a `dyn` or `impl` type with more than one bound or a
/// trailing `+` (`maybe_report_ambiguous_plus`).
fn ambiguous_plus(ty: Node<'_>, first: u32) -> bool {
    let is_multi = (ty.kind() == SyntaxKind::IMPL_TRAIT_TYPE || has_token(ty, T![dyn]))
        && ty
            .children()
            .any(|n| n.kind() == SyntaxKind::TYPE_BOUND_LIST && has_token(n, T![+]));
    if !is_multi {
        return false;
    }
    let mut ancestors = Ancestors::of_node(ty, first).map(|(p, _)| p.kind());
    match ancestors.next() {
        Some(SyntaxKind::REF_TYPE | SyntaxKind::PTR_TYPE | SyntaxKind::CAST_EXPR) => true,
        Some(SyntaxKind::RET_TYPE) => matches!(
            ancestors.next(),
            Some(SyntaxKind::FN_PTR_TYPE | SyntaxKind::PATH_SEGMENT)
        ),
        _ => false,
    }
}

/// `builtin # offset_of(T, field)`: rustc's `parse_expr_builtin` knows `offset_of` (not
/// `asm` or `format_args`), wants dot-separated field names (no parentheses), and from
/// edition 2021 `builtin#` without a space is a reserved prefix.
fn offset_of_rejected(node: Node<'_>, edition: Edition) -> bool {
    let mut prev = None;
    let mut parens = 0;
    for c in node.children_with_tokens() {
        let k = c.kind();
        if prev == Some(SyntaxKind::BUILTIN_KW) && k == T![#] && edition.at_least_2021() {
            return true;
        }
        if k == T!['('] {
            parens += 1;
        }
        prev = Some(k);
    }
    parens > 1
}

/// `const N: i32 = i32::MAX`: rustc's `expr_is_valid_const_arg` accepts only a block, a
/// literal, a negated literal or a single-segment path without generic arguments as a
/// const argument without braces.
fn const_param_default_rejected(param: Node<'_>) -> bool {
    let Some(arg) = param.children().find(|n| n.kind() == SyntaxKind::CONST_ARG) else {
        return false;
    };
    let Some(expr) = arg.children().next() else {
        return false;
    };
    match expr.kind() {
        SyntaxKind::BLOCK_EXPR | SyntaxKind::LITERAL => false,
        SyntaxKind::PREFIX_EXPR => {
            !(has_token(expr, T![-])
                && expr
                    .children()
                    .next()
                    .is_some_and(|n| n.kind() == SyntaxKind::LITERAL))
        }
        SyntaxKind::PATH_EXPR => !expr.children().next().is_some_and(|path| {
            // A single segment: the path has no qualifier path and its segment no
            // generic arguments.
            !has_node(path, SyntaxKind::PATH)
                && path
                    .children()
                    .all(|seg| !has_node(seg, SyntaxKind::GENERIC_ARG_LIST))
        }),
        _ => true,
    }
}

/// A parameter pattern that rustc's `is_named_param` accepts: an optional `&`, `&&` or
/// `mut`, then an identifier (or `_`).
fn is_named_param_pat(pat: Node<'_>) -> bool {
    let simple_ident = |n: Node<'_>| match n.kind() {
        SyntaxKind::WILDCARD_PAT => true,
        SyntaxKind::IDENT_PAT => {
            !has_token(n, T![ref]) && !has_token(n, T![mut]) && !has_token(n, T![@])
        }
        _ => false,
    };
    // The node right after the `&` of a reference pattern, if no trivia separates them.
    fn after_amp(n: Node<'_>) -> Option<Element<'_>> {
        let mut children = n.children_with_tokens().skip_while(|c| c.kind() != T![&]);
        children.next();
        children.next()
    }
    match pat.kind() {
        SyntaxKind::WILDCARD_PAT => true,
        SyntaxKind::IDENT_PAT => !has_token(pat, T![ref]) && !has_token(pat, T![@]),
        SyntaxKind::REF_PAT if !has_token(pat, T![mut]) => match after_amp(pat) {
            Some(NodeOrToken::Node(inner)) if simple_ident(inner) => true,
            // `&&x`: the two `&` are one token to rustc only when adjacent.
            Some(NodeOrToken::Node(inner))
                if inner.kind() == SyntaxKind::REF_PAT
                    && !has_token(inner, T![mut])
                    && inner.first_token().is_some_and(|t| t.kind() == T![&]) =>
            {
                matches!(after_amp(inner), Some(NodeOrToken::Node(n)) if simple_ident(n))
            }
            _ => false,
        },
        _ => false,
    }
}

/// Edition 2015 trait methods take anonymous parameters (`fn f(u8);`), so rustc parses a
/// parameter that does not start like `name:` as a type, and a pattern such as `(a, b): T`
/// is a parse error (E0642).
fn trait_param_rejected(param: Node<'_>, first: u32, edition: Edition) -> bool {
    if edition != Edition::Edition2015 {
        return false;
    }
    let mut kinds = Ancestors::of_node(param, first).map(|(p, _)| p.kind());
    let in_trait_fn = kinds.next() == Some(SyntaxKind::PARAM_LIST)
        && kinds.next() == Some(SyntaxKind::FN)
        && kinds.next() == Some(SyntaxKind::ASSOC_ITEM_LIST)
        && kinds.next() == Some(SyntaxKind::TRAIT);
    if !in_trait_fn {
        return false;
    }
    // A parameter with a pattern has a `:`.
    if !has_token(param, T![:]) {
        return false;
    }
    // The pattern: the first node after the parameter's attributes.
    let pat = param.children().find(|n| n.kind() != SyntaxKind::ATTR);
    pat.is_some_and(|pat| !is_named_param_pat(pat))
}

/// The node rules; `first` is the index of the walked subtree's root.
fn node_rejected(node: Node<'_>, kind: SyntaxKind, first: u32, toolchain: Toolchain) -> bool {
    let rustfmt_version = toolchain.rustfmt_version;
    match kind {
        // `const mut X: T = ..;`
        SyntaxKind::CONST => has_token(node, T![mut]),
        // `unsafe impl Foo {}`, `default impl Foo {}`, `impl !Foo {}`: only trait impls
        // take these.
        SyntaxKind::IMPL => {
            // P8: `impl !Foo {}` is a parse error in rustc 1.97.0 (`inherent impls cannot
            // be negative`) and accepted by rustc 1.88.0's parser, so rustfmt 1.8.0
            // formats it.
            let negative = has_token(node, T![!]) && rustfmt_version >= RustfmtVersion::V1_9_0;
            !has_token(node, T![for])
                && (has_token(node, T![unsafe]) || has_token(node, T![default]) || negative)
        }
        SyntaxKind::LET_EXPR => !let_expr_allowed(node, first, toolchain),
        // `static async || {}`
        SyntaxKind::CLOSURE_EXPR => has_token(node, T![static]) && has_token(node, T![async]),
        // `&raw place` without `const` or `mut`
        SyntaxKind::REF_EXPR => {
            has_token(node, T![raw]) && !has_token(node, T![const]) && !has_token(node, T![mut])
        }
        // Inline `const { .. }` patterns.
        SyntaxKind::CONST_BLOCK_PAT => true,
        // `for<'a> ?Sized`; P2: `[const] Trait` bounds (rust-lang/rust#140838, Rust 1.89.0)
        // are a parse error in rustc 1.88.0, so rustfmt 1.8.0 leaves the file as it is.
        SyntaxKind::TYPE_BOUND => {
            (has_node(node, SyntaxKind::FOR_BINDER) && has_token(node, T![?]))
                || (rustfmt_version < RustfmtVersion::V1_9_0
                    && has_token(node, T!['['])
                    && has_token(node, T![const]))
        }
        // `S { x, #[attr] .. }`
        SyntaxKind::REST_PAT => {
            has_node(node, SyntaxKind::ATTR)
                && Ancestors::of_node(node, first)
                    .next()
                    .is_some_and(|(p, _)| p.kind() == SyntaxKind::RECORD_PAT_FIELD_LIST)
        }
        // Inner attributes in the blocks of an `if` or `else`: `inner_attr_in_if_block`,
        // from the attribute.
        SyntaxKind::RANGE_EXPR => chained_range(node) || inclusive_without_end(node),
        SyntaxKind::RANGE_PAT => inclusive_without_end(node),
        SyntaxKind::BIN_EXPR => bin_expr_rejected(node, first),
        SyntaxKind::DYN_TRAIT_TYPE | SyntaxKind::IMPL_TRAIT_TYPE => ambiguous_plus(node, first),
        // `builtin # asm(..)`, `builtin # format_args(..)`, `builtin # global_asm(..)`
        SyntaxKind::ASM_EXPR | SyntaxKind::FORMAT_ARGS_EXPR => true,
        SyntaxKind::OFFSET_OF_EXPR => offset_of_rejected(node, toolchain.edition),
        SyntaxKind::CONST_PARAM => const_param_default_rejected(node),
        SyntaxKind::PARAM => trait_param_rejected(node, first, toolchain.edition),
        _ => false,
    }
}

/// Whether a comment is a doc comment to rustc's lexer, and of which style.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocStyle {
    /// `///`, `/** */`: an attribute of what follows.
    Outer,
    /// `//!`, `/*! */`: an attribute of the enclosing item.
    Inner,
}

/// rustc's lexer: `///` (not `////`) and `/**` (not `/***` or `/**/`) are outer doc
/// comments, `//!` and `/*!` inner ones; other comments are trivia.
pub(crate) fn doc_comment_style(text: &str) -> Option<DocStyle> {
    if text.starts_with("//!") || text.starts_with("/*!") {
        Some(DocStyle::Inner)
    } else if (text.starts_with("///") && !text.starts_with("////"))
        || (text.starts_with("/**") && !text.starts_with("/***") && text != "/**/")
    {
        Some(DocStyle::Outer)
    } else {
        None
    }
}

impl Walker {
    /// The `:::` rule, which holds in macro input too.
    #[inline]
    fn colon(&mut self, k: SyntaxKind) -> bool {
        if k == T![::] && self.prev_colon {
            return true;
        }
        self.prev_colon = k == T![:];
        false
    }

    /// `true` if rustc rejects token `e`, at index `i` outside macro input.
    #[inline]
    fn token(&mut self, tree: &Tree<'_>, i: u32, e: Elem, first: u32) -> bool {
        let k = e.kind;
        if k == SyntaxKind::WHITESPACE {
            self.prev_colon = false;
            return false;
        }
        if self.colon(k) {
            return true;
        }
        if self.pending_outer_doc
            || matches!(
                k,
                SyntaxKind::COMMENT | SyntaxKind::C_STRING | SyntaxKind::SHEBANG | T!['{']
            )
        {
            return self.doc_token(tree, i, e, first);
        }
        // What `doc_token` answers for any other token when no outer doc comment is
        // pending.
        self.inner_doc_ok = false;
        false
    }

    /// The doc comment and C string rules for a token that is not whitespace.
    fn doc_token(&mut self, tree: &Tree<'_>, i: u32, e: Elem, first: u32) -> bool {
        let k = e.kind;
        if k == SyntaxKind::SHEBANG {
            return false;
        }
        if k == SyntaxKind::COMMENT {
            let text = &tree.source()[e.start as usize..(e.start + e.len) as usize];
            match doc_comment_style(text) {
                // "expected outer doc comment"
                Some(DocStyle::Inner) => {
                    return !self.inner_doc_ok
                        || self.pending_outer_doc
                        || inner_attr_in_if_block(tree, i, first);
                }
                Some(DocStyle::Outer) => {
                    self.pending_outer_doc = true;
                    self.inner_doc_ok = false;
                }
                None => {}
            }
            return false;
        }
        // `c"..."` is a C string literal from edition 2021; before, `c` is an identifier
        // followed by a string literal.
        if k == SyntaxKind::C_STRING && !self.toolchain.edition.at_least_2021() {
            return true;
        }
        // "found a documentation comment that doesn't document anything", "expected item
        // after doc comment"
        if self.pending_outer_doc
            && matches!(
                k,
                T!['}'] | T![')'] | T![']'] | T![,] | T![;] | T![=>] | T![=] | T![>]
            )
        {
            return true;
        }
        self.pending_outer_doc = false;
        self.inner_doc_ok = k == T!['{']
            && Ancestors::of(tree, i, first).next().is_some_and(|(p, _)| {
                matches!(
                    p.kind(),
                    SyntaxKind::ITEM_LIST
                        | SyntaxKind::STMT_LIST
                        | SyntaxKind::ASSOC_ITEM_LIST
                        | SyntaxKind::EXTERN_ITEM_LIST
                        | SyntaxKind::MATCH_ARM_LIST
                )
            });
        false
    }
}

/// Walks the subtree of `root` in source order: one pass over its preorder elements.
fn walk(root: Node<'_>, w: &mut Walker) -> bool {
    let tree = root.tree();
    let first = root.index();
    let end = tree.elem(first).end;
    // The index after the inner attribute being walked, and `inner_doc_ok` before it: an
    // inner attribute leaves the inner-doc-comment state as it was.
    let mut attr_end = u32::MAX;
    let mut attr_inner_doc_ok = false;
    let mut i = first;
    while i < end {
        if i >= attr_end {
            w.inner_doc_ok = attr_inner_doc_ok;
            attr_end = u32::MAX;
        }
        let e = tree.elem(i);
        if !e.is_node {
            if w.token(tree, i, e, first) {
                return true;
            }
            i += 1;
            continue;
        }
        let node = tree.node_at(i);
        if node_rejected(node, e.kind, first, w.toolchain) {
            return true;
        }
        match e.kind {
            SyntaxKind::ATTR if has_token(node, T![!]) => {
                // After an outer doc comment, an inner attribute is an error ("expected
                // outer doc comment" has a counterpart for `#![..]`).
                if w.pending_outer_doc || inner_attr_in_if_block(tree, i, first) {
                    return true;
                }
                attr_end = e.end;
                attr_inner_doc_ok = w.inner_doc_ok;
            }
            SyntaxKind::TOKEN_TREE => {
                // The opening delimiter of a token tree is a token of the file; the rest of
                // the tree is macro input, which rustc does not parse as part of the file.
                let mut j = i + 1;
                if j < e.end && !tree.elem(j).is_node {
                    if w.token(tree, j, tree.elem(j), first) {
                        return true;
                    }
                    j += 1;
                }
                for j in j..e.end {
                    let t = tree.elem(j);
                    if !t.is_node && w.colon(t.kind) {
                        return true;
                    }
                }
                i = e.end;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// `true` if rustc's parser rejects the file whose root is `file`, which rust-analyzer's
/// parser accepted.
pub(crate) fn rejected_by_rustc(
    file: Node<'_>,
    edition: Edition,
    rustfmt_version: RustfmtVersion,
) -> bool {
    subtree_rejected_by_rustc(file, edition, rustfmt_version)
}

/// `true` if rustc's parser rejects the syntax under `root` (a file or a macro argument).
///
/// Rules that look at ancestors (`let` placement, `if` blocks) only see ancestors within
/// `root`; for a file, that is all of them.
pub(crate) fn subtree_rejected_by_rustc(
    root: Node<'_>,
    edition: Edition,
    rustfmt_version: RustfmtVersion,
) -> bool {
    let is_file = root.kind() == SyntaxKind::SOURCE_FILE;
    let mut walker = Walker {
        toolchain: Toolchain {
            edition,
            rustfmt_version,
        },
        prev_colon: false,
        inner_doc_ok: is_file,
        pending_outer_doc: false,
    };
    // A doc comment at the end of a file documents nothing.
    walk(root, &mut walker) || (is_file && walker.pending_outer_doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::Tree;

    fn rejected(src: &str) -> bool {
        rejected_in(src, Edition::Edition2024)
    }

    fn rejected_in(src: &str, edition: Edition) -> bool {
        let tree = Tree::parse(src, edition).unwrap();
        rejected_by_rustc(tree.root(), edition, RustfmtVersion::V1_9_0)
    }

    fn rejected_by_1_8_0(src: &str, edition: Edition) -> bool {
        let tree = Tree::parse(src, edition).unwrap();
        rejected_by_rustc(tree.root(), edition, RustfmtVersion::V1_8_0)
    }

    #[test]
    fn rustc_1_88_acceptance_differs_on_p1_p2_p8() {
        // P1: let chains before edition 2024.
        let chain = "fn f() { if a && let x = 1 {} }";
        assert!(rejected_in(chain, Edition::Edition2021));
        assert!(!rejected_by_1_8_0(chain, Edition::Edition2021));
        assert!(!rejected_by_1_8_0(chain, Edition::Edition2024));
        assert!(rejected_by_1_8_0(
            "fn f() { if let x = 1 || true {} }",
            Edition::Edition2021
        ));
        // P2: `[const]` bounds.
        for src in [
            "fn f<T: [const] Trait>() {}",
            "fn g() where T: [const] Trait {}",
            "fn h(x: impl [const] Trait) {}",
        ] {
            assert!(!rejected_in(src, Edition::Edition2024), "{src}");
            assert!(rejected_by_1_8_0(src, Edition::Edition2024), "{src}");
        }
        // P8: negative inherent impls.
        assert!(rejected_in("impl !Foo {}", Edition::Edition2024));
        assert!(!rejected_by_1_8_0("impl !Foo {}", Edition::Edition2024));
        assert!(!rejected_by_1_8_0(
            "impl !Send for Foo {}",
            Edition::Edition2024
        ));
        assert!(rejected_by_1_8_0(
            "unsafe impl Foo {}",
            Edition::Edition2024
        ));
    }

    #[test]
    fn let_chains_need_edition_2024() {
        for src in [
            "fn f() { if a && let x = 1 {} }",
            "fn f() { while let x = 1 && a {} }",
        ] {
            assert!(rejected_in(src, Edition::Edition2021), "{src}");
            assert!(!rejected_in(src, Edition::Edition2024), "{src}");
        }
        for src in [
            "fn f() { if let x = 1 {} }",
            "fn f() { match x { _ if let x = 1 && a => {} } }",
        ] {
            assert!(!rejected_in(src, Edition::Edition2021), "{src}");
        }
    }

    #[test]
    fn accepted_by_rustc() {
        for src in [
            "fn f() { if true && let x = 1 && let y = 2 {} }",
            "fn f() { match x { _ if let _ = None => {} } }",
            "fn f() { while let Some(x) = y { #![a] } }",
            "fn f() { 'a: while let Some(x) = y {} }",
            "fn f() { static move || {}; }",
            "fn f() { let x = &raw const y; }",
            "fn f() { let x: ::X = 1; }",
            "unsafe impl<T> Foo for T {}",
            "default impl<T> Foo for T {}",
            "fn f() { let S { #[a] x, .. } = (); }",
            "use a::self;",
            "fn f() { let x = (1..2)..3; }",
            "fn f() { let x = a < (b == c) && d > e; }",
            "fn f() { ..=..=..; .. ..; }",
            "fn f() { a < -b; foo::<-1>(); }",
            "fn f() { x = ..z = 2; let _ = |x| ..z = 2; (..z) = 2; }",
            "fn f() { return ..z = 2; }",
            "fn f() { let x = 0..; 0..=1; match x { 0.. => {} } }",
            "fn f(_: &(dyn A + B)) -> impl A + B {}",
            "impl dyn A + B {}",
            "type T = Box<dyn Fn() -> Box<dyn A + B>>;",
            "type T = &[dyn A + B];",
            "fn f() { let _ = |x| -> dyn A + B { x }; }",
            "fn f() { builtin # offset_of(Foo, bar.baz.0); }",
            "struct A<const N: i32 = MAX, const M: i32 = -1, const O: i32 = { i32::MAX }>;",
            "#!/usr/bin/env x\n//! a\n#![b]\n//! c\nfn f() {}",
            "mod m { //! a\n fn f() {} }",
            "fn f() { //! a\n match x { //! b\n _ => {} } }",
            "impl X { //! a\n }",
            "fn f() { /// a\n let x = 1; /// b\n { } }",
            "fn f() { foo(/// a\n x); }",
            "struct S(/// a\n u8);",
            "/// a\nfn f() {}\n////\n/**/\n/***/",
            "macro_rules! m { () => { /// a\n } }",
            "fn f() { m!(/// a\n); }",
            "fn f() { m!(c\"x\"); }",
        ] {
            assert!(!rejected(src), "{src}");
        }
    }

    #[test]
    fn edition_dependent_lexing_and_params() {
        // A file rust-analyzer's parser rejects is rejected too.
        let chloro_rejects = |src: &str, edition| {
            Tree::parse(src, edition)
                .is_none_or(|tree| rejected_by_rustc(tree.root(), edition, RustfmtVersion::V1_9_0))
        };
        // `c"x"` lexes as an identifier and a string before edition 2021.
        let c_string = "fn f() { let _ = c\"x\"; }";
        assert!(chloro_rejects(c_string, Edition::Edition2018), "{c_string}");
        assert!(
            !chloro_rejects(c_string, Edition::Edition2021),
            "{c_string}"
        );
        // `builtin#` is a reserved prefix from edition 2021.
        let builtin = "fn f() { builtin#offset_of(Foo, bar); }";
        assert!(!chloro_rejects(builtin, Edition::Edition2018), "{builtin}");
        assert!(chloro_rejects(builtin, Edition::Edition2021), "{builtin}");
        // Edition 2015 trait methods take anonymous parameters: a pattern must look like
        // `name:`, `&name:`, `&&name:` or `mut name:`.
        for src in [
            "trait T { fn f((a, b): (u8, u8)) {} }",
            "trait T { fn f(&mut a: u8) {} }",
            "trait T { fn f(ref a: u8) {} }",
            "trait T { fn f(& &a: u8) {} }",
        ] {
            assert!(chloro_rejects(src, Edition::Edition2015), "{src}");
            assert!(!chloro_rejects(src, Edition::Edition2018), "{src}");
        }
        for src in [
            "trait T { fn f(&self, a: u8, mut b: u8, &c: u8, &&d: u8, _: u8) {} }",
            "impl X { fn f((a, b): (u8, u8)) {} }",
            "trait T { fn f(#[a] _: u64, #[a] mut x: i32); }",
        ] {
            assert!(!chloro_rejects(src, Edition::Edition2015), "{src}");
        }
    }

    #[test]
    fn rejected_by_rustc_parser() {
        for src in [
            "const mut FOO: () = ();",
            "unsafe impl Foo {}",
            "impl !Foo {}",
            "fn f() { if (let x = 1) {} }",
            "fn f() { if let x = 1 || true {} }",
            "fn f() { let x = let y = 1; }",
            "fn f() { static async || {}; }",
            "fn f() { let _ = &raw foo; }",
            "fn f() { let const { 1 } = 1; }",
            "fn f<T: for<'a> ?Sized>() {}",
            "fn f() { let S { x, #[a] .. } = (); }",
            "fn f() { if x {} else { #![a] } }",
            "fn f() { let ():::X = (); }",
            "fn f() { let x = 1..2..3; }",
            "fn f() { let x = 1..=2..=3; }",
            "fn f() { let x = a < b > c; }",
            "fn f() { let x = a == b != c; }",
            // `<-` is one token to rustc.
            "fn f() { let x = a <- b; }",
            // A range that starts an expression ends it.
            "fn f() { ..z = 2; }",
            "fn f() { ..=z += 2; }",
            "fn f() { foo(..z = 2); }",
            // Inclusive ranges need an end.
            "fn f() { 0..=; }",
            "fn f() { ..=; }",
            "fn f() { match x { 0..= => {} } }",
            // Ambiguous `+` in a type parsed without `+`.
            "fn f(_: &impl A + B) {}",
            "type T = *const dyn A + B;",
            "type T = &'a dyn A + 'a;",
            "type T = &dyn A +;",
            "type T = fn() -> dyn A + B;",
            "fn f(_: impl Fn() -> impl A + B) {}",
            "fn f() { x as dyn A + B; }",
            // `builtin #` constructs rustc does not have, and a parenthesized field.
            "fn f() { builtin # asm(\"\"); }",
            "fn f() { builtin # format_args(\"\", 1); }",
            "fn f() { builtin # offset_of(Foo, (bar)); }",
            // Const parameter defaults without braces.
            "struct A<const N: i32 = i32::MAX>;",
            "struct A<const N: i32 = MAX::<u8>>;",
            // Doc comments that document nothing.
            "struct S { a: u8, /// a\n }",
            "fn f() { foo(a, /// a\n); }",
            "fn f() { a(); /** a */ }",
            "fn f() { let x /// a\n = 1; }",
            "fn f<T, /// a\n>() {}",
            "fn f() {}\n/// a",
            // Inner doc comments after an item or an outer attribute.
            "fn f() {} //! a",
            "fn f() { a(); //! a\n }",
            "struct S { //! a\n a: u8 }",
            "#[a]\n//! a\nfn f() {}",
            "/// a\n//! b\nfn f() {}",
        ] {
            assert!(rejected(src), "{src}");
        }
    }
}
