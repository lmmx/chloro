//! Syntax that rust-analyzer's parser accepts and rustc's parser rejects.
//!
//! rustfmt formats only files that rustc's parser accepts; on a parse error it leaves the
//! file as it is. rust-analyzer's parser is more permissive: it accepts experimental and
//! removed syntax, and leaves some rustc parse errors to a later validation pass, which
//! also reports semantic errors that rustc's parser does not (and that rustfmt therefore
//! formats through). [`rejected_by_rustc`] reproduces the rustc parse errors that matter
//! for formatting. Each rule below was checked against rustfmt 1.9.0.

use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{Edition, NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, T};
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
/// a match guard, and accepts a `let` chained with `&&` in an `if`/`while` condition only
/// from edition 2024. `path` holds the ancestors of the `let`, innermost last.
fn let_expr_allowed(path: &[(&GreenNodeData, usize)], edition: Edition) -> bool {
    let mut chained = false;
    for &(parent, index) in path.iter().rev() {
        match kind(parent.kind()) {
            SyntaxKind::BIN_EXPR if has_token(parent, T![&&]) => chained = true,
            // A `let` in a branch block is reached through the block, not directly.
            SyntaxKind::IF_EXPR | SyntaxKind::WHILE_EXPR => {
                return condition_index(parent) == Some(index)
                    && (!chained || edition.at_least_2024());
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

/// A binary expression whose operator is a comparison.
fn is_comparison(node: &GreenNodeData) -> bool {
    kind(node.kind()) == SyntaxKind::BIN_EXPR
        && node.children().any(|c| {
            matches!(
                child_kind(&c),
                T![==] | T![!=] | T![<] | T![>] | T![<=] | T![>=]
            )
        })
}

/// `a < b > c`, `a == b == c`: comparison operators do not chain (they share one precedence
/// level and have no associativity in rustc).
fn chained_comparison(node: &GreenNodeData) -> bool {
    is_comparison(node)
        && node
            .children()
            .any(|c| matches!(c, NodeOrToken::Node(n) if is_comparison(n)))
}

/// The kind of the first token under `child`, trivia included.
fn first_token_kind(child: &Child<'_>) -> Option<SyntaxKind> {
    match child {
        NodeOrToken::Token(t) => Some(kind(t.kind())),
        NodeOrToken::Node(n) => n.children().next().and_then(|c| first_token_kind(&c)),
    }
}

/// `a <- b`: rustc's lexer glues `<` and an adjacent `-` into one token, `<-` (`LArrow`),
/// which is not a binary operator ("unexpected token: `<-`"); rust-analyzer reads `a < -b`.
fn larrow(bin: &GreenNodeData) -> bool {
    let mut children = bin.children();
    while let Some(c) = children.next() {
        if child_kind(&c) == T![<] {
            return children
                .next()
                .is_some_and(|rhs| first_token_kind(&rhs) == Some(T![-]));
        }
    }
    false
}

fn is_assignment(bin: &GreenNodeData) -> bool {
    kind(bin.kind()) == SyntaxKind::BIN_EXPR
        && bin.children().any(|c| {
            matches!(
                child_kind(&c),
                T![=]
                    | T![+=]
                    | T![-=]
                    | T![*=]
                    | T![/=]
                    | T![%=]
                    | T![&=]
                    | T![|=]
                    | T![^=]
                    | T![<<=]
                    | T![>>=]
            )
        })
}

/// A range without a start: `..`, `..b`, `..=b`.
fn is_prefix_range(child: &Child<'_>) -> bool {
    let NodeOrToken::Node(n) = child else {
        return false;
    };
    kind(n.kind()) == SyntaxKind::RANGE_EXPR
        && n.children()
            .find(|c| !child_kind(c).is_trivia())
            .is_some_and(|c| c.as_node().is_none())
}

/// `..a = b`: rustc's `parse_expr_assoc_with` returns a range that starts an expression
/// without looking for a binary operator after it, so the `=` is unexpected. Where an outer
/// operator loop takes over after the range, the `=` is that loop's: on the right of an
/// assignment (`x = ..a = b` reads `(x = ..a) = b`), and as the operand of a closure,
/// `return`, `break`, `yield`, `become` or `do yeet` (`|x| ..a = b` reads `(|x| ..a) = b`).
fn prefix_range_assigned(bin: &GreenNodeData, path: &[(&GreenNodeData, usize)]) -> bool {
    let lhs_is_prefix_range = bin
        .children()
        .find(|c| c.as_node().is_some())
        .is_some_and(|c| is_prefix_range(&c));
    if !is_assignment(bin) || !lhs_is_prefix_range {
        return false;
    }
    match path.last() {
        Some(&(parent, index)) if is_assignment(parent) => {
            // The right operand: some node precedes it in the parent.
            !parent.children().take(index).any(|c| c.as_node().is_some())
        }
        Some(&(parent, _)) => !matches!(
            kind(parent.kind()),
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
fn inclusive_without_end(range: &GreenNodeData) -> bool {
    let mut children = range.children();
    for c in children.by_ref() {
        if matches!(child_kind(&c), T![..=] | T![...]) {
            return !children.any(|c| c.as_node().is_some());
        }
    }
    false
}

/// `&dyn A + B`, `*const impl A + B`, `x as dyn A + B`, `fn() -> dyn A + B`,
/// `Fn() -> impl A + B`: rustc parses these types with `AllowPlus::No` and reports
/// "ambiguous `+` in a type" for a `dyn` or `impl` type with more than one bound or a
/// trailing `+` (`maybe_report_ambiguous_plus`).
fn ambiguous_plus(ty: &GreenNodeData, path: &[(&GreenNodeData, usize)]) -> bool {
    let k = kind(ty.kind());
    let is_multi = (k == SyntaxKind::IMPL_TRAIT_TYPE || has_token(ty, T![dyn]))
        && ty.children().any(|c| {
            matches!(c, NodeOrToken::Node(n)
                if kind(n.kind()) == SyntaxKind::TYPE_BOUND_LIST && has_token(n, T![+]))
        });
    if !is_multi {
        return false;
    }
    let mut ancestors = path.iter().rev().map(|(p, _)| kind(p.kind()));
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
fn offset_of_rejected(node: &GreenNodeData, edition: Edition) -> bool {
    let mut prev = None;
    let mut parens = 0;
    for c in node.children() {
        let k = child_kind(&c);
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
fn const_param_default_rejected(param: &GreenNodeData) -> bool {
    let Some(arg) = param.children().find_map(|c| match c {
        NodeOrToken::Node(n) if kind(n.kind()) == SyntaxKind::CONST_ARG => Some(n),
        _ => None,
    }) else {
        return false;
    };
    let Some(expr) = arg.children().find_map(|c| c.into_node()) else {
        return false;
    };
    let first_node = |n: &GreenNodeData| {
        n.children()
            .find_map(|c| c.into_node().map(|n| n.to_owned()))
    };
    match kind(expr.kind()) {
        SyntaxKind::BLOCK_EXPR | SyntaxKind::LITERAL => false,
        SyntaxKind::PREFIX_EXPR => {
            !(has_token(expr, T![-])
                && first_node(expr).is_some_and(|n| kind(n.kind()) == SyntaxKind::LITERAL))
        }
        SyntaxKind::PATH_EXPR => !first_node(expr).is_some_and(|path| {
            // A single segment: the path has no qualifier path and its segment no
            // generic arguments.
            !has_node(&path, SyntaxKind::PATH)
                && path.children().all(|c| match c {
                    NodeOrToken::Node(seg) => !has_node(seg, SyntaxKind::GENERIC_ARG_LIST),
                    NodeOrToken::Token(_) => true,
                })
        }),
        _ => true,
    }
}

/// A parameter pattern that rustc's `is_named_param` accepts: an optional `&`, `&&` or
/// `mut`, then an identifier (or `_`).
fn is_named_param_pat(pat: &GreenNodeData) -> bool {
    let simple_ident = |n: &GreenNodeData| match kind(n.kind()) {
        SyntaxKind::WILDCARD_PAT => true,
        SyntaxKind::IDENT_PAT => {
            !has_token(n, T![ref]) && !has_token(n, T![mut]) && !has_token(n, T![@])
        }
        _ => false,
    };
    match kind(pat.kind()) {
        SyntaxKind::WILDCARD_PAT => true,
        SyntaxKind::IDENT_PAT => !has_token(pat, T![ref]) && !has_token(pat, T![@]),
        SyntaxKind::REF_PAT if !has_token(pat, T![mut]) => {
            let mut children = pat.children().skip_while(|c| child_kind(c) != T![&]);
            children.next();
            match children.next() {
                Some(NodeOrToken::Node(inner)) if simple_ident(inner) => true,
                // `&&x`: the two `&` are one token to rustc only when adjacent.
                Some(NodeOrToken::Node(inner))
                    if kind(inner.kind()) == SyntaxKind::REF_PAT
                        && !has_token(inner, T![mut])
                        && first_token_kind(&NodeOrToken::Node(inner)) == Some(T![&]) =>
                {
                    let mut rest = inner.children().skip_while(|c| child_kind(c) != T![&]);
                    rest.next();
                    matches!(rest.next(), Some(NodeOrToken::Node(n)) if simple_ident(n))
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Edition 2015 trait methods take anonymous parameters (`fn f(u8);`), so rustc parses a
/// parameter that does not start like `name:` as a type, and a pattern such as `(a, b): T`
/// is a parse error (E0642).
fn trait_param_rejected(
    param: &GreenNodeData,
    path: &[(&GreenNodeData, usize)],
    edition: Edition,
) -> bool {
    if edition != Edition::Edition2015 {
        return false;
    }
    let ancestors: Vec<_> = path
        .iter()
        .rev()
        .take(4)
        .map(|(p, _)| kind(p.kind()))
        .collect();
    if ancestors
        != [
            SyntaxKind::PARAM_LIST,
            SyntaxKind::FN,
            SyntaxKind::ASSOC_ITEM_LIST,
            SyntaxKind::TRAIT,
        ]
    {
        return false;
    }
    // A parameter with a pattern has a `:`.
    if !has_token(param, T![:]) {
        return false;
    }
    // The pattern: the first node after the parameter's attributes.
    let pat = param
        .children()
        .filter_map(|c| c.into_node())
        .find(|n| kind(n.kind()) != SyntaxKind::ATTR);
    pat.is_some_and(|pat| !is_named_param_pat(pat))
}

fn node_rejected(node: &GreenNodeData, path: &[(&GreenNodeData, usize)], edition: Edition) -> bool {
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
        SyntaxKind::LET_EXPR => !let_expr_allowed(path, edition),
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
        SyntaxKind::RANGE_EXPR => chained_range(node) || inclusive_without_end(node),
        SyntaxKind::RANGE_PAT => inclusive_without_end(node),
        SyntaxKind::BIN_EXPR => {
            chained_comparison(node) || larrow(node) || prefix_range_assigned(node, path)
        }
        SyntaxKind::DYN_TRAIT_TYPE | SyntaxKind::IMPL_TRAIT_TYPE => ambiguous_plus(node, path),
        // `builtin # asm(..)`, `builtin # format_args(..)`, `builtin # global_asm(..)`
        SyntaxKind::ASM_EXPR | SyntaxKind::FORMAT_ARGS_EXPR => true,
        SyntaxKind::OFFSET_OF_EXPR => offset_of_rejected(node, edition),
        SyntaxKind::CONST_PARAM => const_param_default_rejected(node),
        SyntaxKind::PARAM => trait_param_rejected(node, path, edition),
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
    edition: Edition,
) -> bool {
    if node_rejected(node, path, edition) {
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
                let rejected = walk(n, path, prev_colon, edition);
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
pub(crate) fn rejected_by_rustc(file: &ast::SourceFile, edition: Edition) -> bool {
    subtree_rejected_by_rustc(file.syntax(), edition)
}

/// `true` if rustc's parser rejects the syntax under `root` (a file or a macro argument).
///
/// Rules that look at ancestors (`let` placement, `if` blocks) only see ancestors within
/// `root`; for a file, that is all of them.
pub(crate) fn subtree_rejected_by_rustc(root: &SyntaxNode, edition: Edition) -> bool {
    walk(&root.green(), &mut Vec::new(), &mut false, edition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::SourceFile;

    fn rejected(src: &str) -> bool {
        rejected_in(src, Edition::Edition2024)
    }

    fn rejected_in(src: &str, edition: Edition) -> bool {
        rejected_by_rustc(&SourceFile::parse(src, edition).tree(), edition)
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
        ] {
            assert!(!rejected(src), "{src}");
        }
    }

    #[test]
    fn edition_dependent_rules() {
        // `builtin#` is a reserved prefix from edition 2021.
        let builtin = "fn f() { builtin#offset_of(Foo, bar); }";
        assert!(!rejected_in(builtin, Edition::Edition2018), "{builtin}");
        assert!(rejected_in(builtin, Edition::Edition2021), "{builtin}");
        // Edition 2015 trait methods take anonymous parameters: a pattern must look like
        // `name:`, `&name:`, `&&name:` or `mut name:`.
        for src in [
            "trait T { fn f((a, b): (u8, u8)) {} }",
            "trait T { fn f(&mut a: u8) {} }",
            "trait T { fn f(ref a: u8) {} }",
            "trait T { fn f(& &a: u8) {} }",
        ] {
            assert!(rejected_in(src, Edition::Edition2015), "{src}");
            assert!(!rejected_in(src, Edition::Edition2018), "{src}");
        }
        for src in [
            "trait T { fn f(a: u8, mut b: u8, &c: u8, &&d: u8, _: u8, &self) {} }",
            "impl X { fn f((a, b): (u8, u8)) {} }",
            "trait T { fn f(#[a] _: u64, #[a] mut x: i32); }",
        ] {
            assert!(!rejected_in(src, Edition::Edition2015), "{src}");
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
        ] {
            assert!(rejected(src), "{src}");
        }
    }
}
