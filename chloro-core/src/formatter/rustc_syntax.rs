//! Syntax that rustc's parser accepts and rust-analyzer's parser rejects.
//!
//! rustfmt formats a file that rustc's parser accepts. Where rust-analyzer's parser rejects
//! such syntax but has a node for the construct rustc builds, the file is parsed again with
//! a keyword inserted where rust-analyzer needs it, and the inserted tokens are then removed
//! from the tree, so that the tree's text is the source again.
//!
//! The constructs (each checked against rustfmt 1.9.0):
//!
//! - A parenthesized first bound of a bare trait object, `Box<(Copy) + Send>` or
//!   `Box<(?Sized) + Send>`. rustc's `parse_ty_tuple_or_parens` reads a parenthesized path
//!   followed by `+` as the first bound of a trait object (`parse_remaining_bounds_path`,
//!   the bound's span includes the parentheses), and a parenthesized trait object with one
//!   bound, such as `?Sized` or `for<'a> Trait<'a>`, followed by `+` as that bound
//!   (`parse_remaining_bounds`, the bound's span excludes them). rust-analyzer parses a
//!   bare trait object only when it starts with a path. With `dyn` inserted before the
//!   `(`, rust-analyzer builds a `DYN_TRAIT_TYPE` whose first `TYPE_BOUND` holds the
//!   parentheses; without the `dyn`, it is the node rust-analyzer builds for `Send + Sync`.
//! - A type that starts with `?`, `(?Sized)`: a trait object with a "maybe" bound to rustc.
//!   rust-analyzer has no type starting with `?`; with `dyn` inserted before the `?`, it
//!   builds a `DYN_TRAIT_TYPE`.

use ra_ap_parser::{Edition, LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, T};
use rowan::{GreenNode, GreenNodeData};

use super::formatting::TreeBuilder;

/// The text inserted before a `(` or `?`: `dyn` and the space after it.
const DYN: &str = "dyn ";

/// The parse of a source text: the root and the offsets of the parser's errors. `None` when
/// the lexer reports an error.
pub(crate) fn build_tree(source: &str, edition: Edition) -> Option<(SyntaxNode, Vec<usize>)> {
    let lexed = LexedStr::new(edition, source);
    if lexed.errors().next().is_some() {
        return None;
    }
    let input = lexed.to_input(edition);
    let output = TopEntryPoint::SourceFile.parse(&input, edition);
    let mut builder = TreeBuilder::default();
    let mut errors = Vec::new();
    lexed.intersperse_trivia(&output, &mut |step| match step {
        StrStep::Token { kind, text } => builder.token(kind, text),
        StrStep::Enter { kind } => builder.start_node(kind),
        StrStep::Exit => builder.finish_node(),
        StrStep::Error { pos, .. } => errors.push(pos),
    });
    Some((builder.finish(), errors))
}

/// Parses `source`, which rust-analyzer's parser rejects, with the insertions described in
/// the module documentation. `None` if the parse still fails.
pub(crate) fn reparse(
    source: &str,
    edition: Edition,
    tree: &SyntaxNode,
    errors: &[usize],
) -> Option<SyntaxNode> {
    // Offsets in `source` before which `DYN` is inserted, sorted.
    let mut inserts: Vec<usize> = Vec::new();
    let mut found = insertion(tree, errors);
    // Each round adds the insertion for the first error. The tree after an error is the
    // parser's recovery, so only the first error is trusted.
    for _ in 0..64 {
        let original = to_original(&inserts, found?)?;
        match inserts.binary_search(&original) {
            Ok(_) => return None,
            Err(i) => inserts.insert(i, original),
        }
        let text = with_insertions(source, &inserts);
        let (tree, errors) = build_tree(&text, edition)?;
        if errors.is_empty() {
            let removed: Vec<usize> = inserts
                .iter()
                .enumerate()
                .flat_map(|(i, &at)| {
                    let start = at + i * DYN.len();
                    [start, start + DYN.len() - 1]
                })
                .collect();
            let mut offset = 0;
            let green = without_tokens(&tree.green(), &mut offset, &removed);
            let root = SyntaxNode::new_root(green);
            return (root.text() == source).then_some(root);
        }
        found = insertion(&tree, &errors);
    }
    None
}

/// `source` with `DYN` before each offset of `inserts`.
fn with_insertions(source: &str, inserts: &[usize]) -> String {
    let mut text = String::with_capacity(source.len() + inserts.len() * DYN.len());
    let mut last = 0;
    for &at in inserts {
        text.push_str(&source[last..at]);
        text.push_str(DYN);
        last = at;
    }
    text.push_str(&source[last..]);
    text
}

/// The offset in the source of `pos`, an offset in the text with `inserts` applied that
/// does not fall in inserted text.
fn to_original(inserts: &[usize], pos: usize) -> Option<usize> {
    let before = inserts
        .iter()
        .enumerate()
        .take_while(|&(i, &at)| at + i * DYN.len() < pos)
        .count();
    let original = pos.checked_sub(before * DYN.len())?;
    (before == inserts.len() || original <= inserts[before]).then_some(original)
}

/// A copy of `node` without the tokens that start at the offsets in `removed` (sorted).
fn without_tokens(node: &GreenNodeData, offset: &mut usize, removed: &[usize]) -> GreenNode {
    let mut children = Vec::new();
    for child in node.children() {
        match child {
            NodeOrToken::Token(t) => {
                let start = *offset;
                *offset += t.text().len();
                if removed.binary_search(&start).is_err() {
                    children.push(NodeOrToken::Token(t.to_owned()));
                }
            }
            NodeOrToken::Node(n) => {
                children.push(NodeOrToken::Node(without_tokens(n, offset, removed)));
            }
        }
    }
    GreenNode::new(node.kind(), children)
}

fn next_significant(token: &SyntaxToken) -> Option<SyntaxToken> {
    let mut t = token.next_token()?;
    while t.kind().is_trivia() {
        t = t.next_token()?;
    }
    Some(t)
}

/// The `)` that closes the `(` token `open`.
fn closing_paren(open: &SyntaxToken) -> Option<SyntaxToken> {
    let mut depth = 0usize;
    let mut t = open.clone();
    loop {
        match t.kind() {
            T!['('] => depth += 1,
            T![')'] => {
                depth -= 1;
                if depth == 0 {
                    return Some(t);
                }
            }
            _ => {}
        }
        t = t.next_token()?;
    }
}

/// rustc parses a type with `+` after it here (`AllowPlus::Yes`).
fn plus_allowed(parent: Option<SyntaxNode>) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    match parent.kind() {
        SyntaxKind::TYPE_ARG
        | SyntaxKind::LET_STMT
        | SyntaxKind::TYPE_ALIAS
        | SyntaxKind::PARAM
        | SyntaxKind::RECORD_FIELD
        | SyntaxKind::TUPLE_FIELD
        | SyntaxKind::CONST
        | SyntaxKind::STATIC
        | SyntaxKind::PAREN_TYPE
        | SyntaxKind::TUPLE_TYPE
        | SyntaxKind::SLICE_TYPE
        | SyntaxKind::ARRAY_TYPE => true,
        SyntaxKind::RET_TYPE => parent.parent().is_some_and(|p| p.kind() == SyntaxKind::FN),
        _ => false,
    }
}

/// A parenthesized type rustc reads as the first bound of a trait object when `+` follows:
/// a path without a qualified self type (`TyKind::Path(None, ..)`), or `for<..>` and a path
/// (a trait object with one bound).
fn bound_like(inner: &SyntaxNode) -> bool {
    let path_type = |n: &SyntaxNode| {
        n.kind() == SyntaxKind::PATH_TYPE && n.first_token().is_some_and(|t| t.kind() != T![<])
    };
    match inner.kind() {
        SyntaxKind::PATH_TYPE => path_type(inner),
        SyntaxKind::FOR_TYPE => inner
            .children()
            .find(|c| c.kind() != SyntaxKind::FOR_BINDER)
            .is_some_and(|c| path_type(&c)),
        _ => false,
    }
}

/// The offset (in the parsed text) where `DYN` makes rust-analyzer's parser build the node
/// for a construct rustc accepts, at the first error of a failed parse: the candidate
/// closest before that error.
fn insertion(tree: &SyntaxNode, errors: &[usize]) -> Option<usize> {
    let first_error = errors.iter().copied().min()?;
    insertions(tree, errors)
        .into_iter()
        .filter(|&at| at <= first_error)
        .max()
}

/// Offsets (in the parsed text) where `DYN` makes rust-analyzer's parser build the node for
/// a construct rustc accepts, found from a failed parse.
fn insertions(tree: &SyntaxNode, errors: &[usize]) -> Vec<usize> {
    let mut found = Vec::new();
    for paren in tree
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::PAREN_TYPE)
    {
        let Some(open) = paren.first_token().filter(|t| t.kind() == T!['(']) else {
            continue;
        };
        let start = usize::from(paren.text_range().start());
        let Some(first) = next_significant(&open) else {
            continue;
        };
        if first.kind() == T![?] {
            // `(?Bound..)`: the first bound of a trait object if `+` follows the `)`,
            // otherwise a parenthesized trait object.
            let Some(close) = closing_paren(&open) else {
                continue;
            };
            let plus_follows = next_significant(&close).is_some_and(|t| t.kind() == T![+]);
            if plus_follows && plus_allowed(paren.parent()) {
                found.push(start);
            } else {
                found.push(usize::from(first.text_range().start()));
            }
            continue;
        }
        let end = usize::from(paren.text_range().end());
        let closed = paren.last_token().is_some_and(|t| t.kind() == T![')']);
        let Some(inner) = paren.children().next() else {
            continue;
        };
        let plus_follows = paren
            .last_token()
            .and_then(|t| next_significant(&t))
            .is_some_and(|t| t.kind() == T![+]);
        if closed
            && plus_follows
            && errors.contains(&end)
            && plus_allowed(paren.parent())
            && bound_like(&inner)
        {
            found.push(start);
        }
    }
    found
}
