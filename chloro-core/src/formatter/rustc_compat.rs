//! Syntax that rust-analyzer's parser accepts and rustc's parser rejects.
//!
//! rustfmt formats only files that rustc's parser accepts; on a parse error it leaves the
//! file as it is. rust-analyzer's parser is more permissive: it accepts experimental and
//! removed syntax, and leaves some rustc parse errors to a later validation pass, which
//! also reports semantic errors that rustc's parser does not (and that rustfmt therefore
//! formats through). [`rejected_by_rustc`] reproduces the rustc parse errors that matter
//! for formatting. Each rule below was checked against `rustfmt --edition 2024` (1.9.0).

use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, T};
use rowan::{GreenNodeData, GreenTokenData, Language};

// The rules run on every file, so they read the green tree (no allocation per node) and
// keep the path from the root as `(node, index of the child on the path)` pairs.

type Child<'a> = NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>;

fn kind(raw: rowan::SyntaxKind) -> SyntaxKind {
    RustLanguage::kind_from_raw(raw)
}

fn child_kind(child: &Child<'_>) -> SyntaxKind {
    match child {
        NodeOrToken::Node(n) => kind(n.kind()),
        NodeOrToken::Token(t) => kind(t.kind()),
    }
}

fn has_token(node: &GreenNodeData, token_kind: SyntaxKind) -> bool {
    node.children()
        .any(|c| matches!(c, NodeOrToken::Token(t) if kind(t.kind()) == token_kind))
}

fn has_node(node: &GreenNodeData, node_kind: SyntaxKind) -> bool {
    node.children()
        .any(|c| matches!(c, NodeOrToken::Node(n) if kind(n.kind()) == node_kind))
}

/// An inner attribute: `#![...]`, `//! ...` or `/*! ... */`.
fn has_inner_attr(stmt_list: &GreenNodeData) -> bool {
    stmt_list.children().any(|c| match c {
        NodeOrToken::Node(n) => kind(n.kind()) == SyntaxKind::ATTR && has_token(n, T![!]),
        NodeOrToken::Token(t) => {
            kind(t.kind()) == SyntaxKind::COMMENT
                && (t.text().starts_with("//!") || t.text().starts_with("/*!"))
        }
    })
}

/// The index of the first child node that is not an attribute or a label: the condition
/// of an `if` or `while`.
fn condition_index(node: &GreenNodeData) -> Option<usize> {
    node.children().position(|c| {
        matches!(c, NodeOrToken::Node(n)
            if !matches!(kind(n.kind()), SyntaxKind::ATTR | SyntaxKind::LABEL))
    })
}

/// rustc accepts `let` only as an operand of a `&&` chain in an `if`/`while` condition or
/// a match guard. `path` holds the ancestors of the `let`, innermost last.
fn let_expr_allowed(path: &[(&GreenNodeData, usize)]) -> bool {
    for &(parent, index) in path.iter().rev() {
        match kind(parent.kind()) {
            SyntaxKind::BIN_EXPR if has_token(parent, T![&&]) => {}
            // A `let` in a branch block is reached through the block, not directly.
            SyntaxKind::IF_EXPR | SyntaxKind::WHILE_EXPR => {
                return condition_index(parent) == Some(index);
            }
            SyntaxKind::MATCH_GUARD => return true,
            _ => return false,
        }
    }
    false
}

/// `a..b..c`: range operators do not chain. A prefix range (`..`, `..=b`) may be the end
/// of another range (`..=..`), as it parses as a single operand.
fn chained_range(range: &GreenNodeData) -> bool {
    let is_op = |c: &Child<'_>| matches!(child_kind(c), T![..] | T![..=] | T![...]);
    let has_start = |n: &GreenNodeData| {
        n.children()
            .find(|c| !child_kind(c).is_trivia())
            .is_some_and(|c| c.as_node().is_some())
    };
    let mut before_op = true;
    for child in range.children() {
        if is_op(&child) {
            before_op = false;
            continue;
        }
        let NodeOrToken::Node(n) = child else {
            continue;
        };
        if kind(n.kind()) != SyntaxKind::RANGE_EXPR {
            continue;
        }
        if before_op || has_start(n) {
            return true;
        }
    }
    false
}

fn node_rejected(node: &GreenNodeData, path: &[(&GreenNodeData, usize)]) -> bool {
    let parent_kind = path.last().map(|(p, _)| kind(p.kind()));
    match kind(node.kind()) {
        // `const mut X: T = ..;`
        SyntaxKind::CONST => has_token(node, T![mut]),
        // `unsafe impl Foo {}`, `default impl Foo {}`, `impl !Foo {}`: only trait impls
        // take these.
        SyntaxKind::IMPL => {
            !has_token(node, T![for])
                && (has_token(node, T![unsafe])
                    || has_token(node, T![default])
                    || has_token(node, T![!]))
        }
        SyntaxKind::LET_EXPR => !let_expr_allowed(path),
        // `static async || {}`
        SyntaxKind::CLOSURE_EXPR => has_token(node, T![static]) && has_token(node, T![async]),
        // `&raw place` without `const` or `mut`
        SyntaxKind::REF_EXPR => {
            has_token(node, T![raw]) && !has_token(node, T![const]) && !has_token(node, T![mut])
        }
        // Inline `const { .. }` patterns.
        SyntaxKind::CONST_BLOCK_PAT => true,
        // `for<'a> ?Sized`
        SyntaxKind::TYPE_BOUND => has_node(node, SyntaxKind::FOR_BINDER) && has_token(node, T![?]),
        // `S { x, #[attr] .. }`
        SyntaxKind::REST_PAT => {
            parent_kind == Some(SyntaxKind::RECORD_PAT_FIELD_LIST)
                && has_node(node, SyntaxKind::ATTR)
        }
        // Inner attributes in the blocks of an `if` or `else`.
        SyntaxKind::BLOCK_EXPR => {
            parent_kind == Some(SyntaxKind::IF_EXPR)
                && node.children().any(|c| {
                    matches!(c, NodeOrToken::Node(n)
                        if kind(n.kind()) == SyntaxKind::STMT_LIST && has_inner_attr(n))
                })
        }
        SyntaxKind::RANGE_EXPR => chained_range(node),
        _ => false,
    }
}

/// Walks the tree under `node` in source order. `prev_colon` tracks whether the previous
/// token was a `:` — rustc lexes `:::` as `::` followed by `:`, rust-analyzer as `:`
/// followed by `::`.
fn walk<'a>(
    node: &'a GreenNodeData,
    path: &mut Vec<(&'a GreenNodeData, usize)>,
    prev_colon: &mut bool,
) -> bool {
    if node_rejected(node, path) {
        return true;
    }
    for (index, child) in node.children().enumerate() {
        match child {
            NodeOrToken::Token(t) => {
                let k = kind(t.kind());
                if k == T![::] && *prev_colon {
                    return true;
                }
                *prev_colon = k == T![:];
            }
            NodeOrToken::Node(n) => {
                path.push((node, index));
                let rejected = walk(n, path, prev_colon);
                path.pop();
                if rejected {
                    return true;
                }
            }
        }
    }
    false
}

/// `true` if rustc's parser rejects `file`, which rust-analyzer's parser accepted.
pub(crate) fn rejected_by_rustc(file: &ast::SourceFile) -> bool {
    subtree_rejected_by_rustc(file.syntax())
}

/// `true` if rustc's parser rejects the syntax under `root` (a file or a macro argument).
///
/// Rules that look at ancestors (`let` placement, `if` blocks) only see ancestors within
/// `root`; for a file, that is all of them.
pub(crate) fn subtree_rejected_by_rustc(root: &SyntaxNode) -> bool {
    walk(&root.green(), &mut Vec::new(), &mut false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::{Edition, SourceFile};

    fn rejected(src: &str) -> bool {
        rejected_by_rustc(&SourceFile::parse(src, Edition::Edition2024).tree())
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
            "fn f() { ..=..=..; .. ..; }",
        ] {
            assert!(!rejected(src), "{src}");
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
        ] {
            assert!(rejected(src), "{src}");
        }
    }
}
