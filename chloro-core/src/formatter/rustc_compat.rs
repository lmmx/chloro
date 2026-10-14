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
use crate::cst::{Element, Node, NodeOrToken};

// The rules run on every file, walking the tree once and keeping the path from the root as
// `(node, index of the child on the path)` pairs.

fn has_token(node: Node<'_>, token_kind: SyntaxKind) -> bool {
    node.children_with_tokens()
        .any(|c| matches!(c, NodeOrToken::Token(t) if t.kind() == token_kind))
}

fn has_node(node: Node<'_>, node_kind: SyntaxKind) -> bool {
    node.children().any(|n| n.kind() == node_kind)
}

/// An inner attribute: `#![...]`, `//! ...` or `/*! ... */`.
fn has_inner_attr(stmt_list: Node<'_>) -> bool {
    stmt_list.children_with_tokens().any(|c| match c {
        NodeOrToken::Node(n) => n.kind() == SyntaxKind::ATTR && has_token(n, T![!]),
        NodeOrToken::Token(t) => {
            t.kind() == SyntaxKind::COMMENT
                && (t.text().starts_with("//!") || t.text().starts_with("/*!"))
        }
    })
}

/// The index of the first child node that is not an attribute or a label: the condition
/// of an `if` or `while`.
fn condition_index(node: Node<'_>) -> Option<usize> {
    node.children_with_tokens().position(|c| {
        matches!(c, NodeOrToken::Node(n)
            if !matches!(n.kind(), SyntaxKind::ATTR | SyntaxKind::LABEL))
    })
}

/// The parser being reproduced: the edition and the rustc release of the rustfmt build.
#[derive(Clone, Copy)]
struct Toolchain {
    edition: Edition,
    rustfmt_version: RustfmtVersion,
}

/// The state of one walk: the toolchain, the path from the root and the `:` tracker, so
/// that the recursion passes one reference.
struct Walker<'a> {
    toolchain: Toolchain,
    /// The ancestors of the current node as `(node, index of the child on the path)`.
    path: Vec<(Node<'a>, usize)>,
    /// Whether the previous token was a `:` — rustc lexes `:::` as `::` followed by `:`,
    /// rust-analyzer as `:` followed by `::`.
    prev_colon: bool,
}

/// rustc accepts `let` only as an operand of a `&&` chain in an `if`/`while` condition or
/// a match guard, and accepts a `let` chained with `&&` in an `if`/`while` condition only
/// from edition 2024. `path` holds the ancestors of the `let`, innermost last.
fn let_expr_allowed(path: &[(Node<'_>, usize)], toolchain: Toolchain) -> bool {
    // P1: rustc 1.97.0 rejects a chained `let` before edition 2024 (`let chains are only
    // allowed in Rust 2024 or later`, stabilised in Rust 1.88.0 for edition 2024 only);
    // rustc 1.88.0's parser accepts the chain in every edition and leaves the gate to a
    // later pass, so rustfmt 1.8.0 formats it.
    let chain_allowed =
        toolchain.edition.at_least_2024() || toolchain.rustfmt_version < RustfmtVersion::V1_9_0;
    let mut chained = false;
    for &(parent, index) in path.iter().rev() {
        match parent.kind() {
            SyntaxKind::BIN_EXPR if has_token(parent, T![&&]) => chained = true,
            // A `let` in a branch block is reached through the block, not directly.
            SyntaxKind::IF_EXPR | SyntaxKind::WHILE_EXPR => {
                return condition_index(parent) == Some(index) && (!chained || chain_allowed);
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

/// A binary expression whose operator is a comparison.
fn is_comparison(node: Node<'_>) -> bool {
    node.kind() == SyntaxKind::BIN_EXPR
        && node
            .children_with_tokens()
            .any(|c| matches!(c.kind(), T![==] | T![!=] | T![<] | T![>] | T![<=] | T![>=]))
}

/// `a < b > c`, `a == b == c`: comparison operators do not chain (they share one precedence
/// level and have no associativity in rustc).
fn chained_comparison(node: Node<'_>) -> bool {
    is_comparison(node) && node.children().any(is_comparison)
}

fn node_rejected(node: Node<'_>, path: &[(Node<'_>, usize)], toolchain: Toolchain) -> bool {
    let rustfmt_version = toolchain.rustfmt_version;
    let parent_kind = path.last().map(|(p, _)| p.kind());
    match node.kind() {
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
        SyntaxKind::LET_EXPR => !let_expr_allowed(path, toolchain),
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
            parent_kind == Some(SyntaxKind::RECORD_PAT_FIELD_LIST)
                && has_node(node, SyntaxKind::ATTR)
        }
        // Inner attributes in the blocks of an `if` or `else`.
        SyntaxKind::BLOCK_EXPR => {
            parent_kind == Some(SyntaxKind::IF_EXPR)
                && node
                    .children()
                    .any(|n| n.kind() == SyntaxKind::STMT_LIST && has_inner_attr(n))
        }
        SyntaxKind::RANGE_EXPR => chained_range(node),
        SyntaxKind::BIN_EXPR => chained_comparison(node),
        _ => false,
    }
}

/// Walks the tree under `node` in source order.
fn walk<'a>(node: Node<'a>, w: &mut Walker<'a>) -> bool {
    if node_rejected(node, &w.path, w.toolchain) {
        return true;
    }
    for (index, child) in node.children_with_tokens().enumerate() {
        match child {
            NodeOrToken::Token(t) => {
                let k = t.kind();
                if k == T![::] && w.prev_colon {
                    return true;
                }
                w.prev_colon = k == T![:];
            }
            NodeOrToken::Node(n) => {
                w.path.push((node, index));
                let rejected = walk(n, w);
                w.path.pop();
                if rejected {
                    return true;
                }
            }
        }
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
    let mut walker = Walker {
        toolchain: Toolchain {
            edition,
            rustfmt_version,
        },
        path: Vec::new(),
        prev_colon: false,
    };
    walk(root, &mut walker)
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
            "fn f() { let x = a < b > c; }",
            "fn f() { let x = a == b != c; }",
        ] {
            assert!(rejected(src), "{src}");
        }
    }
}
