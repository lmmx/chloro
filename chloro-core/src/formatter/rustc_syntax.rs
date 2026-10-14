//! Syntax that rustc's parser accepts and rust-analyzer's parser rejects.
//!
//! rustfmt formats a file that rustc's parser accepts. Where rust-analyzer's parser rejects
//! such syntax but has a node for the construct rustc builds, the file's tree is rebuilt
//! from the parser's steps with that node. The steps are collected into an owned tree
//! ([`ONode`]), edited, and laid out as a flat [`Tree`] over the source, so every token's
//! text is the source slice at its offset, as for a file that parses.
//!
//! One construct needs a second parse, of the source with a keyword inserted where
//! rust-analyzer needs it; the inserted tokens are left out of the tree:
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
//!   A type that starts with `?`, `(?Sized)`, is a trait object with a "maybe" bound to
//!   rustc; rust-analyzer has no type starting with `?`, and with `dyn` inserted before the
//!   `?` builds a `DYN_TRAIT_TYPE`.
//!
//! Three constructs need only an edit of the tree:
//!
//! - A float literal without `.` as a field name, `s.1e0`: rustc's `break_up_float` reads
//!   `1e0` as a single component (`DestructuredFloat::Single`) and the field name is `1e0`.
//!   rust-analyzer's tree builder reports "illegal float literal" and wraps the literal in
//!   an `ERROR` node; the edit makes it a `NAME_REF`, as for `s.0`.
//! - A const block item, `const { .. }` in a file or module (rustc's `ItemKind::ConstBlock`,
//!   the unstable `const_block_items`; rustc 1.88.0's parser, and so rustfmt 1.8.0, rejects
//!   it). rust-analyzer reports "expected an item" and keeps the `const` keyword and the
//!   block (parsed as block contents) in two `ERROR` nodes; the edit makes them a `CONST`
//!   item without a name whose body is a `BLOCK_EXPR`, which `nodes::const_block_item`
//!   recognises.
//! - An `impl Trait` type as the self type of an impl, `impl impl Trait {}` or
//!   `impl Tr for impl Trait {}`: rustc's parser accepts it (AST validation rejects it
//!   later). rust-analyzer's `impl_type` reports "expected trait or type" and ends the
//!   impl, then parses `impl Trait {}` as a second impl; the edit merges the two, with the
//!   second one's type in an `IMPL_TRAIT_TYPE`.

use ra_ap_parser::{Edition, LexedStr, Output, StrStep, SyntaxKind, T, TopEntryPoint};

use crate::cst::{Node, Tree, TreeBuilder};

use super::config::RustfmtVersion;

/// The text inserted before a `(` or `?`: `dyn` and the space after it.
const DYN: &str = "dyn ";

/// rust-analyzer's message for a float literal without `.` after a `.` (`s.1e0`); its
/// position is a token index, not a text offset.
const FLOAT_FIELD: &str = "illegal float literal";

/// `true` if rustc's lexer rejects the text `lexed` holds. rust-analyzer reports an
/// identifier directly followed by `#`, a string or a character literal (`foo#bar`,
/// `foo"x"`) as an unknown literal prefix in every edition; rustc reserves these prefixes
/// from edition 2021, and before reads an identifier and the next token.
pub(crate) fn lexer_rejects(lexed: &LexedStr<'_>, edition: Edition) -> bool {
    lexed
        .errors()
        .any(|(_, msg)| edition.at_least_2021() || !msg.starts_with("unknown literal prefix"))
}

/// An element of an owned tree: a node, or a token of a kind and a length.
#[derive(Clone)]
enum El {
    Node(ONode),
    Token(SyntaxKind, u32),
}

/// A node of an owned tree; offsets follow from the lengths of the tokens before.
#[derive(Clone)]
struct ONode {
    kind: SyntaxKind,
    children: Vec<El>,
}

impl El {
    fn kind(&self) -> SyntaxKind {
        match self {
            El::Node(n) => n.kind,
            El::Token(kind, _) => *kind,
        }
    }

    fn len(&self) -> u32 {
        match self {
            El::Node(n) => n.len(),
            El::Token(_, len) => *len,
        }
    }

    fn as_node(&self) -> Option<&ONode> {
        match self {
            El::Node(n) => Some(n),
            El::Token(..) => None,
        }
    }

    /// The kind of the first token under the element.
    fn first_token(&self) -> Option<SyntaxKind> {
        match self {
            El::Node(n) => n.children.iter().find_map(El::first_token),
            El::Token(kind, _) => Some(*kind),
        }
    }

    fn last_token(&self) -> Option<SyntaxKind> {
        match self {
            El::Node(n) => n.children.iter().rev().find_map(El::last_token),
            El::Token(kind, _) => Some(*kind),
        }
    }
}

impl ONode {
    fn new(kind: SyntaxKind, children: Vec<El>) -> ONode {
        ONode { kind, children }
    }

    fn len(&self) -> u32 {
        self.children.iter().map(El::len).sum()
    }

    /// Whether a node of `kind` is in the tree under `self`.
    fn contains(&self, kind: SyntaxKind) -> bool {
        self.kind == kind
            || self
                .children
                .iter()
                .any(|c| matches!(c, El::Node(n) if n.contains(kind)))
    }
}

/// The parse of a text as an owned tree.
struct Parse {
    root: ONode,
    /// Text offsets of the parser's errors, other than float field names.
    errors: Vec<usize>,
    /// The number of float literals without `.` used as field names.
    float_fields: usize,
}

/// Lexes and parses `text` into an owned tree, errors included. `None` when rustc's lexer
/// rejects the text.
fn parse_owned(text: &str, edition: Edition) -> Option<Parse> {
    let lexed = LexedStr::new(edition, text);
    if lexer_rejects(&lexed, edition) {
        return None;
    }
    let output = TopEntryPoint::SourceFile.parse(&lexed.to_input(edition), edition);
    owned_tree(&lexed, &output)
}

/// The owned tree of the parser's steps `output` over `lexed`, errors included.
fn owned_tree(lexed: &LexedStr<'_>, output: &Output) -> Option<Parse> {
    let mut stack: Vec<ONode> = Vec::new();
    let mut root = None;
    let mut errors = Vec::new();
    let mut float_fields = 0;
    lexed.intersperse_trivia(output, &mut |step| match step {
        StrStep::Token { kind, text } => {
            if let Some(top) = stack.last_mut() {
                top.children.push(El::Token(kind, text.len() as u32));
            }
        }
        StrStep::Enter { kind } => stack.push(ONode::new(kind, Vec::new())),
        StrStep::Exit => {
            if let Some(node) = stack.pop() {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(El::Node(node)),
                    None => root = Some(node),
                }
            }
        }
        StrStep::Error { msg, .. } if msg == FLOAT_FIELD => float_fields += 1,
        StrStep::Error { pos, .. } => errors.push(pos),
    });
    Some(Parse {
        root: root?,
        errors,
        float_fields,
    })
}

fn emit(builder: &mut TreeBuilder<'_>, el: &El) {
    match el {
        El::Node(n) => {
            builder.start_node(n.kind);
            for c in &n.children {
                emit(builder, c);
            }
            builder.finish_node();
        }
        El::Token(kind, len) => builder.token(*kind, *len),
    }
}

/// The flat tree of `root` over `src`.
fn to_tree<'s>(src: &'s str, root: ONode) -> Tree<'s> {
    let mut builder = TreeBuilder::new(src, 0, 0, src.len() / 2 + 16);
    emit(&mut builder, &El::Node(root));
    builder.finish()
}

/// The tree rustc's parser would accept for `source`, which rust-analyzer's lexer or parser
/// rejected, built as the module documentation describes. `None` if there is none.
///
/// `lexed` is `source` lexed, and `output` the parser's steps over it when the lexer
/// reported no error (`None`: not parsed yet).
pub(crate) fn reparse<'s>(
    source: &'s str,
    lexed: &LexedStr<'_>,
    output: Option<&Output>,
    edition: Edition,
    rustfmt_version: RustfmtVersion,
) -> Option<Tree<'s>> {
    if lexer_rejects(lexed, edition) {
        return None;
    }
    let parse = match output {
        Some(output) => owned_tree(lexed, output)?,
        None => owned_tree(
            lexed,
            &TopEntryPoint::SourceFile.parse(&lexed.to_input(edition), edition),
        )?,
    };
    if parse.errors.is_empty() {
        let root = float_fields_as_names(&parse.root, source, parse.float_fields)?;
        return Some(to_tree(source, root));
    }
    // The edits explain errors only where there is a `const` token, or two `impl` tokens.
    // rustc 1.88.0's parser (rustfmt 1.8.0) has no const block items.
    let const_blocks = rustfmt_version >= RustfmtVersion::V1_9_0 && source.contains("const");
    let impl_self_types = source.matches("impl").nth(1).is_some();
    if const_blocks || impl_self_types {
        let mut explained = vec![false; parse.errors.len()];
        let const_blocks = const_blocks
            .then(|| rebuild_const_blocks(&parse.root, 0, source, &parse.errors, &mut explained));
        let root = rebuild_impl_self_types(
            const_blocks.as_ref().unwrap_or(&parse.root),
            0,
            &parse.errors,
            &mut explained,
        );
        if explained.iter().all(|&e| e) {
            let root = float_fields_as_names(&root, source, parse.float_fields)?;
            return Some(to_tree(source, root));
        }
    }
    // Only a parenthesized type takes an inserted `DYN`.
    if !parse.root.contains(SyntaxKind::PAREN_TYPE) {
        return None;
    }
    with_inserted_dyn(source, edition, parse)
}

/// The parse of `source` with `DYN` inserted, one insertion per round at the first error
/// (the tree after an error is the parser's recovery, so only the first error is trusted),
/// and the inserted tokens left out.
fn with_inserted_dyn<'s>(source: &'s str, edition: Edition, parse: Parse) -> Option<Tree<'s>> {
    // Offsets in `source` before which `DYN` is inserted, sorted.
    let mut inserts: Vec<usize> = Vec::new();
    let mut text = source.to_owned();
    let mut parse = parse;
    for _ in 0..64 {
        let found = {
            let errors = parse.errors.clone();
            let tree = to_tree(&text, parse.root);
            insertion(&tree, &errors)?
        };
        let original = to_original(&inserts, found)?;
        match inserts.binary_search(&original) {
            Ok(_) => return None,
            Err(i) => inserts.insert(i, original),
        }
        text = with_insertions(source, &inserts);
        parse = parse_owned(&text, edition)?;
        if parse.errors.is_empty() {
            let removed: Vec<usize> = inserts
                .iter()
                .enumerate()
                .flat_map(|(i, &at)| {
                    let start = at + i * DYN.len();
                    [start, start + DYN.len() - 1]
                })
                .collect();
            let mut offset = 0;
            let mut kept = String::with_capacity(source.len());
            let root = without_tokens(&parse.root, &mut offset, &removed, &text, &mut kept);
            if kept != source {
                return None;
            }
            let root = float_fields_as_names(&root, source, parse.float_fields)?;
            return Some(to_tree(source, root));
        }
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

/// A copy of `node` (at `offset` in `text`) without the tokens that start at the offsets in
/// `removed` (sorted); the text of the tokens kept is appended to `kept`.
fn without_tokens(
    node: &ONode,
    offset: &mut usize,
    removed: &[usize],
    text: &str,
    kept: &mut String,
) -> ONode {
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        match child {
            El::Token(kind, len) => {
                let start = *offset;
                *offset += *len as usize;
                if removed.binary_search(&start).is_err() {
                    kept.push_str(&text[start..*offset]);
                    children.push(El::Token(*kind, *len));
                }
            }
            El::Node(n) => children.push(El::Node(without_tokens(n, offset, removed, text, kept))),
        }
    }
    ONode::new(node.kind, children)
}

/// The next token after element `index` of `tree` that is not trivia: its kind and offset.
fn next_significant(tree: &Tree<'_>, index: u32) -> Option<(SyntaxKind, usize)> {
    (index + 1..tree.len() as u32)
        .map(|i| tree.elem(i))
        .find(|e| !e.is_node && !e.kind.is_trivia())
        .map(|e| (e.kind, e.start as usize))
}

/// The index of the `)` token that closes the `(` token at `open`.
fn closing_paren(tree: &Tree<'_>, open: u32) -> Option<u32> {
    let mut depth = 0usize;
    for i in open..tree.len() as u32 {
        let e = tree.elem(i);
        if e.is_node {
            continue;
        }
        match e.kind {
            T!['('] => depth += 1,
            T![')'] => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// rustc parses a type with `+` after it here (`AllowPlus::Yes`).
fn plus_allowed(parent: Option<Node<'_>>) -> bool {
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
fn bound_like(inner: Node<'_>) -> bool {
    let path_type = |n: Node<'_>| {
        n.kind() == SyntaxKind::PATH_TYPE && n.first_token().is_some_and(|t| t.kind() != T![<])
    };
    match inner.kind() {
        SyntaxKind::PATH_TYPE => path_type(inner),
        SyntaxKind::FOR_TYPE => inner
            .children()
            .find(|c| c.kind() != SyntaxKind::FOR_BINDER)
            .is_some_and(path_type),
        _ => false,
    }
}

/// The offset (in the parsed text) where `DYN` makes rust-analyzer's parser build the node
/// for a construct rustc accepts, at the first error of a failed parse: the candidate
/// closest before that error.
fn insertion(tree: &Tree<'_>, errors: &[usize]) -> Option<usize> {
    let first_error = errors.iter().copied().min()?;
    insertions(tree, errors)
        .into_iter()
        .filter(|&at| at <= first_error)
        .max()
}

/// Offsets (in the parsed text) where `DYN` makes rust-analyzer's parser build the node for
/// a construct rustc accepts, found from a failed parse.
fn insertions(tree: &Tree<'_>, errors: &[usize]) -> Vec<usize> {
    let mut found = Vec::new();
    for paren in tree.nodes().filter(|n| n.kind() == SyntaxKind::PAREN_TYPE) {
        let Some(open) = paren.first_token().filter(|t| t.kind() == T!['(']) else {
            continue;
        };
        let start = paren.text_range().start() as usize;
        let Some((first, first_at)) = next_significant(tree, open.index()) else {
            continue;
        };
        if first == T![?] {
            // `(?Bound..)`: the first bound of a trait object if `+` follows the `)`,
            // otherwise a parenthesized trait object.
            let Some(close) = closing_paren(tree, open.index()) else {
                continue;
            };
            let plus_follows = next_significant(tree, close).is_some_and(|(k, _)| k == T![+]);
            if plus_follows && plus_allowed(paren.parent()) {
                found.push(start);
            } else {
                found.push(first_at);
            }
            continue;
        }
        let end = paren.text_range().end() as usize;
        let Some(last) = paren.last_token() else {
            continue;
        };
        let Some(inner) = paren.children().next() else {
            continue;
        };
        let plus_follows = next_significant(tree, last.index()).is_some_and(|(k, _)| k == T![+]);
        if last.kind() == T![')']
            && plus_follows
            && errors.contains(&end)
            && plus_allowed(paren.parent())
            && bound_like(inner)
        {
            found.push(start);
        }
    }
    found
}

/// `1e0` as rustc's `break_up_float` reads it: a single component (digits, an exponent
/// without sign, no suffix) is a field name; anything else is an error.
fn float_field_name(text: &str) -> bool {
    let Some((mantissa, exponent)) = text.split_once(['e', 'E']) else {
        return false;
    };
    mantissa.starts_with(|c: char| c.is_ascii_digit())
        && mantissa.chars().all(|c| c.is_ascii_digit() || c == '_')
        && exponent.chars().any(|c| c.is_ascii_digit())
        && exponent.chars().all(|c| c.is_ascii_digit() || c == '_')
}

/// `root` (over `text`) with each float literal field name in a `NAME_REF` instead of an
/// `ERROR` node, and without the `FIELD_EXPR` that rust-analyzer opens around it for a
/// second field (`s.1.2`) and leaves with that single child. `None` unless there are
/// `expected` such field names, all of rustc's single-component form, and no other `ERROR`
/// node.
fn float_fields_as_names(root: &ONode, text: &str, expected: usize) -> Option<ONode> {
    let mut fixed = 0;
    let mut offset = 0;
    let root = rebuild_field_names(root, &mut offset, text, &mut fixed)?;
    (fixed == expected).then_some(root)
}

fn rebuild_field_names(
    node: &ONode,
    offset: &mut usize,
    text: &str,
    fixed: &mut usize,
) -> Option<ONode> {
    let mut children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        match child {
            El::Token(kind, len) => {
                *offset += *len as usize;
                children.push(El::Token(*kind, *len));
            }
            El::Node(n) if n.kind == SyntaxKind::ERROR => {
                // `ERROR(FLOAT_NUMBER)` as the field of a `FIELD_EXPR`.
                let literal = match n.children.as_slice() {
                    [El::Token(SyntaxKind::FLOAT_NUMBER, len)] => *len,
                    _ => return None,
                };
                let literal_text = &text[*offset..*offset + literal as usize];
                if node.kind != SyntaxKind::FIELD_EXPR || !float_field_name(literal_text) {
                    return None;
                }
                *offset += literal as usize;
                *fixed += 1;
                children.push(El::Node(ONode::new(
                    SyntaxKind::NAME_REF,
                    vec![El::Token(SyntaxKind::FLOAT_NUMBER, literal)],
                )));
            }
            El::Node(n) => children.push(El::Node(rebuild_field_names(n, offset, text, fixed)?)),
        }
    }
    // The `FIELD_EXPR` left around a repaired one, with no other child.
    if node.kind == SyntaxKind::FIELD_EXPR
        && let [El::Node(inner)] = children.as_slice()
        && inner.kind == SyntaxKind::FIELD_EXPR
    {
        return Some(inner.clone());
    }
    Some(ONode::new(node.kind, children))
}

fn is_error_with(element: &El, first: SyntaxKind) -> bool {
    element.kind() == SyntaxKind::ERROR && element.first_token() == Some(first)
}

/// An `ERROR` node holding only outer attributes, a visibility and trivia: what
/// rust-analyzer keeps of `#[a] pub` when no item follows. rustfmt prints the attributes and
/// drops the visibility (`ast::ConstBlockItem` has none).
fn is_item_prefix(element: &El) -> bool {
    element.as_node().is_some_and(|n| {
        n.kind == SyntaxKind::ERROR
            && n.children.iter().all(|c| {
                c.kind().is_trivia()
                    || matches!(c.kind(), SyntaxKind::ATTR | SyntaxKind::VISIBILITY)
            })
    })
}

/// A copy of `node` (at `start` in `text`) with each const block item of a file or module
/// as a `CONST` node; marks in `explained` the errors (offsets in `errors`) that are theirs.
fn rebuild_const_blocks(
    node: &ONode,
    start: usize,
    text: &str,
    errors: &[usize],
    explained: &mut [bool],
) -> ONode {
    let elements = &node.children;
    let mut starts = Vec::with_capacity(elements.len());
    let mut at = start;
    for e in elements {
        starts.push(at);
        at += e.len() as usize;
    }
    let in_module = matches!(node.kind, SyntaxKind::SOURCE_FILE | SyntaxKind::ITEM_LIST);
    let mut children: Vec<El> = Vec::with_capacity(elements.len());
    let mut i = 0;
    while i < elements.len() {
        if in_module && is_error_with(&elements[i], T![const]) {
            // `const`, trivia, then the block.
            let mut j = i + 1;
            while j < elements.len() && elements[j].kind().is_trivia() {
                j += 1;
            }
            let block = elements
                .get(j)
                .filter(|e| is_error_with(e, T!['{']) && e.last_token() == Some(T!['}']))
                .and_then(El::as_node);
            if let Some(block) = block {
                // Attributes and a visibility before `const`, then the trivia in between.
                let mut first = i;
                let mut k = i;
                while k > 0 && elements[k - 1].kind().is_trivia() {
                    k -= 1;
                }
                if k > 0 && is_item_prefix(&elements[k - 1]) {
                    first = k - 1;
                }
                // Comments right before the item belong to it, as rust-analyzer attaches
                // them to items, up to a blank line.
                let mut attached = first;
                while attached > 0 {
                    let prev = &elements[attached - 1];
                    let prev_text = &text[starts[attached - 1]..starts[attached]];
                    let blank_line =
                        prev.kind() == SyntaxKind::WHITESPACE && prev_text.contains("\n\n");
                    if !prev.kind().is_trivia() || blank_line {
                        break;
                    }
                    attached -= 1;
                }
                while attached < first && elements[attached].kind() == SyntaxKind::WHITESPACE {
                    attached += 1;
                }
                children.truncate(children.len() - (i - attached));
                let (lo, hi) = (starts[first], starts[j]);
                for (e, done) in errors.iter().zip(explained.iter_mut()) {
                    if (lo..=hi).contains(e) {
                        *done = true;
                    }
                }
                let mut item = Vec::new();
                for e in &elements[attached..i] {
                    match e {
                        El::Node(prefix) if prefix.kind == SyntaxKind::ERROR => {
                            item.extend(prefix.children.iter().cloned())
                        }
                        e => item.push(e.clone()),
                    }
                }
                if let El::Node(const_kw) = &elements[i] {
                    item.extend(const_kw.children.iter().cloned());
                }
                item.extend(elements[i + 1..j].iter().cloned());
                let stmt_list = ONode::new(SyntaxKind::STMT_LIST, block.children.clone());
                let block_expr = ONode::new(SyntaxKind::BLOCK_EXPR, vec![El::Node(stmt_list)]);
                item.push(El::Node(block_expr));
                children.push(El::Node(ONode::new(SyntaxKind::CONST, item)));
                i = j + 1;
                continue;
            }
        }
        children.push(match &elements[i] {
            El::Node(n) => El::Node(rebuild_const_blocks(n, starts[i], text, errors, explained)),
            token => token.clone(),
        });
        i += 1;
    }
    ONode::new(node.kind, children)
}

/// The type, its bounds and the elements after it of an impl that rust-analyzer parsed
/// from `impl Trait {}` after an impl it ended at a second `impl`: `impl`, trivia, a path
/// type or a bare trait object, no `for`, then an optional where clause and the body.
fn impl_trait_parts(second: &ONode) -> Option<(usize, ONode, Vec<El>)> {
    let mut significant = second
        .children
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind().is_trivia());
    if significant.next()?.1.kind() != T![impl] {
        return None;
    }
    let (position, ty) = significant.next()?;
    let ty = ty.as_node()?;
    // The bounds of the `impl Trait` type.
    let bounds = match ty.kind {
        SyntaxKind::PATH_TYPE => {
            let bound = ONode::new(SyntaxKind::TYPE_BOUND, vec![El::Node(ty.clone())]);
            ONode::new(SyntaxKind::TYPE_BOUND_LIST, vec![El::Node(bound)])
        }
        SyntaxKind::DYN_TRAIT_TYPE if El::Node(ty.clone()).first_token() != Some(T![dyn]) => {
            ty.children.iter().find_map(El::as_node)?.clone()
        }
        _ => return None,
    };
    let rest: Vec<SyntaxKind> = significant.map(|(_, e)| e.kind()).collect();
    let body_ok = matches!(
        rest.as_slice(),
        [SyntaxKind::ASSOC_ITEM_LIST] | [SyntaxKind::WHERE_CLAUSE, SyntaxKind::ASSOC_ITEM_LIST]
    );
    body_ok.then(|| (position, bounds, second.children[position + 1..].to_vec()))
}

/// A copy of `node` (at `start`) with each impl whose self type is `impl Trait` merged as
/// described in the module documentation; marks in `explained` the errors that are theirs.
fn rebuild_impl_self_types(
    node: &ONode,
    start: usize,
    errors: &[usize],
    explained: &mut [bool],
) -> ONode {
    let elements = &node.children;
    let mut starts = Vec::with_capacity(elements.len());
    let mut at = start;
    for e in elements {
        starts.push(at);
        at += e.len() as usize;
    }
    let mut children = Vec::with_capacity(elements.len());
    let mut i = 0;
    while i < elements.len() {
        if let El::Node(first) = &elements[i]
            && first.kind == SyntaxKind::IMPL
            && !first
                .children
                .iter()
                .any(|c| c.kind() == SyntaxKind::ASSOC_ITEM_LIST)
            // `impl impl A for B` is an error to rustc: the trait must be a path.
            && matches!(elements[i].last_token(), Some(T![impl] | T![for] | T![>]))
        {
            let mut j = i + 1;
            while j < elements.len() && elements[j].kind().is_trivia() {
                j += 1;
            }
            let second = elements
                .get(j)
                .and_then(El::as_node)
                .filter(|n| n.kind == SyntaxKind::IMPL);
            if let Some((second, (position, bounds, rest))) =
                second.and_then(|s| Some((s, impl_trait_parts(s)?)))
            {
                let end = starts[i] + first.len() as usize;
                for (e, done) in errors.iter().zip(explained.iter_mut()) {
                    if *e == end {
                        *done = true;
                    }
                }
                let mut item: Vec<El> = first.children.clone();
                item.extend(elements[i + 1..j].iter().cloned());
                let mut impl_trait: Vec<El> = second.children[..position].to_vec();
                impl_trait.push(El::Node(bounds));
                item.push(El::Node(ONode::new(
                    SyntaxKind::IMPL_TRAIT_TYPE,
                    impl_trait,
                )));
                item.extend(rest);
                children.push(El::Node(ONode::new(SyntaxKind::IMPL, item)));
                i = j + 1;
                continue;
            }
        }
        children.push(match &elements[i] {
            El::Node(n) => El::Node(rebuild_impl_self_types(n, starts[i], errors, explained)),
            token => token.clone(),
        });
        i += 1;
    }
    ONode::new(node.kind, children)
}
