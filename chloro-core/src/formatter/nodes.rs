//! rustc-shaped views of rust-analyzer syntax.
//!
//! rustfmt is written against rustc's AST, where doc comments are attributes, a block is a
//! list of statements (its tail expression included), a missing `where` clause still has a
//! span, and so on. rust-analyzer's lossless tree encodes the same information differently.
//! The types and functions here present syntax nodes in the shape the ported rustfmt logic
//! expects, so that the formatting modules can follow rustfmt's control flow closely.

use ra_ap_parser::{SyntaxKind, T};

use crate::cst::{AstNode, Node, NodeOrToken, Token, Tree, ast, support};

use super::span::{
    BytePos, Span, Spanned, is_inner_doc_comment, is_outer_doc_comment, is_outer_doc_text, mk_sp,
    node_range_span, rustc_span, token_span,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum AttrStyle {
    Outer,
    Inner,
}

/// An attribute as rustc sees it: either a `#[...]`/`#![...]` attribute or a sugared doc
/// comment (`///`, `//!`, `/** */`, `/*! */`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attribute<'a> {
    Doc(Token<'a>),
    Normal(ast::Attr<'a>),
}

impl<'a> Attribute<'a> {
    pub(crate) fn span(&self) -> Span {
        match self {
            Attribute::Doc(t) => token_span(*t),
            Attribute::Normal(a) => node_range_span(a.syntax()),
        }
    }

    pub(crate) fn is_doc_comment(&self) -> bool {
        matches!(self, Attribute::Doc(_))
    }

    pub(crate) fn style(&self) -> AttrStyle {
        match self {
            Attribute::Doc(t) if is_inner_doc_comment(*t) => AttrStyle::Inner,
            Attribute::Doc(_) => AttrStyle::Outer,
            Attribute::Normal(a) if has_token(a.syntax(), T![!]) => AttrStyle::Inner,
            Attribute::Normal(_) => AttrStyle::Outer,
        }
    }

    /// The attribute's path with whitespace removed, e.g. `derive` or `rustfmt::skip`.
    pub(crate) fn path_text(&self) -> Option<String> {
        match self {
            Attribute::Doc(_) => None,
            Attribute::Normal(a) => {
                let path = child::<ast::Meta>(a.syntax())?.path()?;
                Some(non_trivia_text(path.syntax()))
            }
        }
    }

    pub(crate) fn has_name(&self, name: &str) -> bool {
        self.path_text().is_some_and(|p| p == name)
    }

    pub(crate) fn as_attr(&self) -> Option<&ast::Attr<'a>> {
        match self {
            Attribute::Normal(a) => Some(a),
            Attribute::Doc(_) => None,
        }
    }

    /// `#[rustfmt::skip]`, `#[rustfmt_skip]` or `#[cfg_attr(<pred>, rustfmt::skip)]`.
    pub(crate) fn is_skip(&self) -> bool {
        match self {
            Attribute::Doc(_) => false,
            Attribute::Normal(a) => {
                let Some(path) = self.path_text() else {
                    return false;
                };
                let meta = child::<ast::Meta>(a.syntax());
                let has_args = meta
                    .as_ref()
                    .is_some_and(|m| m.token_tree().is_some() || m.expr().is_some());
                if !has_args {
                    return path == "rustfmt::skip" || path == "rustfmt_skip";
                }
                if path != "cfg_attr" {
                    return false;
                }
                let Some(tt) = meta.and_then(|m| m.token_tree()) else {
                    return false;
                };
                // `cfg_attr(<pred>, rustfmt::skip)`: exactly two top-level elements, the
                // second of which is the skip path.
                let parts = split_top_level_commas(tt.syntax());
                parts.len() == 2 && {
                    let second: String = parts[1]
                        .iter()
                        .filter(|t| !t.kind().is_trivia())
                        .map(|t| t.text().to_string())
                        .collect();
                    second == "rustfmt::skip" || second == "rustfmt_skip"
                }
            }
        }
    }
}

/// Splits the tokens of a token tree (excluding its delimiters) at top-level commas.
pub(crate) fn split_top_level_commas<'t>(tt: Node<'t>) -> Vec<Vec<Token<'t>>> {
    let mut parts: Vec<Vec<Token<'t>>> = vec![vec![]];
    let children: Vec<_> = tt.children_with_tokens().collect();
    let inner = children
        .get(1..children.len().saturating_sub(1))
        .unwrap_or(&[]);
    for child in inner {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMA => parts.push(vec![]),
            NodeOrToken::Token(t) => parts.last_mut().unwrap().push(*t),
            NodeOrToken::Node(n) => parts
                .last_mut()
                .unwrap()
                .extend(n.descendants_with_tokens().filter_map(|e| e.into_token())),
        }
    }
    if parts
        .last()
        .is_some_and(|p| p.iter().all(|t| t.kind().is_trivia()))
    {
        parts.pop();
    }
    parts
}

/// The text of the non-trivia tokens under `node`, in source order.
pub(crate) fn non_trivia_text(node: Node<'_>) -> String {
    let mut out = String::new();
    for t in node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
    {
        if !t.kind().is_trivia() {
            out.push_str(t.text());
        }
    }
    out
}

/// `true` if `node` starts with an attribute or an outer doc comment.
fn has_leading_attrs(node: Node<'_>) -> bool {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(n) => return n.kind() == SyntaxKind::ATTR,
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::WHITESPACE => {}
                SyntaxKind::COMMENT if is_outer_doc_text(t.text()) => return true,
                SyntaxKind::COMMENT => {}
                _ => return false,
            },
        }
    }
    false
}

/// Outer attributes (including sugared doc comments) at the start of `node`, in source order.
pub(crate) fn outer_attributes<'t>(node: Node<'t>) -> Vec<Attribute<'t>> {
    if !has_leading_attrs(node) {
        return Vec::new();
    }
    let mut attrs = Vec::new();
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::WHITESPACE => {}
                SyntaxKind::COMMENT => {
                    if is_outer_doc_comment(t) {
                        attrs.push(Attribute::Doc(t));
                    }
                }
                _ => break,
            },
            NodeOrToken::Node(n) => match ast::Attr::cast(n) {
                Some(attr) if !has_token(attr.syntax(), T![!]) => {
                    attrs.push(Attribute::Normal(attr))
                }
                Some(_) => {}
                None => break,
            },
        }
    }
    attrs
}

/// `false` when no direct child of `container` can be an inner attribute: no attribute with
/// a `!` token and no comment starting `//!` or `/*!`. A `true` only means
/// [`inner_attributes`] has to look.
fn may_have_inner_attrs(container: Node<'_>) -> bool {
    container.children_with_tokens().any(|child| match child {
        NodeOrToken::Node(n) => n.kind() == SyntaxKind::ATTR && has_token(n, T![!]),
        NodeOrToken::Token(t) => {
            t.kind() == SyntaxKind::COMMENT
                && (t.text().starts_with("//!") || t.text().starts_with("/*!"))
        }
    })
}

/// Inner attributes (`#![...]`, `//!`) that are direct children of a container node such as
/// a source file, an item list or a statement list.
pub(crate) fn inner_attributes<'t>(container: Node<'t>) -> Vec<Attribute<'t>> {
    if !may_have_inner_attrs(container) {
        return Vec::new();
    }
    container
        .children_with_tokens()
        .filter_map(|child| match child {
            NodeOrToken::Token(t) if is_inner_doc_comment(t) => Some(Attribute::Doc(t)),
            NodeOrToken::Node(n) => ast::Attr::cast(n)
                .filter(|a| has_token(a.syntax(), T![!]))
                .map(Attribute::Normal),
            _ => None,
        })
        .collect()
}

pub(crate) fn contains_skip(attrs: &[Attribute<'_>]) -> bool {
    attrs.iter().any(Attribute::is_skip)
}

/// The span of `node` without its outer attributes and doc comments.
pub(crate) fn span_without_attrs(node: Node<'_>) -> Span {
    let full = node_range_span(node);
    // The first child that is neither trivia nor an attribute.
    for child in node.children_with_tokens() {
        let skipped = match child {
            NodeOrToken::Token(t) => t.kind().is_trivia(),
            NodeOrToken::Node(n) => n.kind() == SyntaxKind::ATTR,
        };
        if !skipped {
            return full.with_lo(child.text_range().start());
        }
    }
    full
}

/// Whether `node` has a direct child token of `kind`: a generated `x_token().is_some()`.
pub(crate) fn has_token(node: Node<'_>, kind: SyntaxKind) -> bool {
    node.children_with_tokens()
        .any(|child| matches!(child, NodeOrToken::Token(t) if t.kind() == kind))
}

/// The first child node of `parent` that casts to `N`: the generated `support::child`
/// accessors (`x.expr()`, `x.path()`, ...) for a child the generated layer has no accessor
/// for.
pub(crate) fn child<'t, N: AstNode<'t>>(parent: Node<'t>) -> Option<N> {
    support::child(parent)
}

/// The source text of `node`: a slice of the source.
pub(crate) fn node_text<'t>(node: Node<'t>) -> &'t str {
    node.text()
}

/// rustc's expression kind for a block expression. rust-analyzer has one `BlockExpr` for all
/// of them; rustfmt treats them differently (overflow, closure bodies, match arms).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum BlockExprKind {
    /// `{}`, `unsafe {}`, `'a: {}`: `ExprKind::Block`.
    Block,
    /// `async {}`, `gen {}`, `async gen {}`: `ExprKind::Gen`.
    Gen,
    /// `try {}`: `ExprKind::TryBlock`.
    TryBlock,
    /// `const {}`: `ExprKind::ConstBlock`.
    ConstBlock,
}

pub(crate) fn block_expr_kind(b: &ast::BlockExpr) -> BlockExprKind {
    if has_token(b.syntax(), T![const]) {
        BlockExprKind::ConstBlock
    } else if has_token(b.syntax(), T![try]) {
        BlockExprKind::TryBlock
    } else if has_token(b.syntax(), T![async]) || has_token(b.syntax(), T![gen]) {
        BlockExprKind::Gen
    } else {
        BlockExprKind::Block
    }
}

/// The rustc kind of `expr` if it is a block expression.
pub(crate) fn block_kind_of(expr: &ast::Expr) -> Option<BlockExprKind> {
    match expr {
        ast::Expr::BlockExpr(b) => Some(block_expr_kind(b)),
        _ => None,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum BlockRules {
    Default,
    Unsafe,
}

/// A brace-delimited block, possibly synthesised around a single expression (rustfmt wraps a
/// closure body in a block this way).
#[derive(Clone, Debug)]
pub(crate) struct Block<'a> {
    pub(crate) stmts: Vec<Stmt<'a>>,
    pub(crate) span: Span,
    pub(crate) rules: BlockRules,
    pub(crate) stmt_list: Option<ast::StmtList<'a>>,
}

impl<'a> Block<'a> {
    /// The block of a block expression (`{}`, `unsafe {}`, `'a: {}`, `async {}`, ...).
    pub(crate) fn from_block_expr(block: &ast::BlockExpr<'a>) -> Option<Block<'a>> {
        let stmt_list = block.stmt_list()?;
        let list_span = node_range_span(stmt_list.syntax());
        let (rules, span) = match block.unsafe_token() {
            Some(t) => (BlockRules::Unsafe, list_span.with_lo(token_span(t).lo())),
            None => (BlockRules::Default, list_span),
        };
        Some(Block {
            stmts: stmts_of(&stmt_list),
            span,
            rules,
            stmt_list: Some(stmt_list),
        })
    }

    /// A block without braces containing a single expression statement.
    pub(crate) fn synthetic(expr: &ast::Expr<'a>) -> Block<'a> {
        let span = expr.span();
        Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(*expr),
                span,
            }],
            span,
            rules: BlockRules::Default,
            stmt_list: None,
        }
    }

    /// The block without handles, for the run's block cache.
    pub(crate) fn to_cached(&self) -> CachedBlock {
        CachedBlock {
            stmts: self
                .stmts
                .iter()
                .map(|s| {
                    let (tag, index) = match &s.kind {
                        StmtKind::Let(l) => (StmtTag::Let, l.syntax().index()),
                        StmtKind::Item(i) => (StmtTag::Item, i.syntax().index()),
                        StmtKind::Expr(e) => (StmtTag::Expr, e.syntax().index()),
                        StmtKind::Semi(e) => (StmtTag::Semi, e.syntax().index()),
                        StmtKind::MacCall(m) => (StmtTag::MacCall, m.syntax().index()),
                        StmtKind::Empty => (StmtTag::Empty, 0),
                    };
                    (tag, index, s.span)
                })
                .collect(),
            span: self.span,
            rules: self.rules,
            stmt_list: self.stmt_list.map(|l| l.syntax().index()),
        }
    }

    /// The block `cached` describes, over `tree` (the tree it was built from).
    pub(crate) fn from_cached(cached: &CachedBlock, tree: &'a Tree<'a>) -> Block<'a> {
        let cast = |i: u32| tree.node_at(i);
        Block {
            stmts: cached
                .stmts
                .iter()
                .map(|&(tag, index, span)| {
                    let kind = match tag {
                        StmtTag::Let => StmtKind::Let(ast::LetStmt {
                            syntax: cast(index),
                        }),
                        StmtTag::Item => StmtKind::Item(
                            ast::Item::cast(cast(index)).expect("an item was cached"),
                        ),
                        StmtTag::Expr => StmtKind::Expr(
                            ast::Expr::cast(cast(index)).expect("an expression was cached"),
                        ),
                        StmtTag::Semi => StmtKind::Semi(
                            ast::Expr::cast(cast(index)).expect("an expression was cached"),
                        ),
                        StmtTag::MacCall => StmtKind::MacCall(ast::MacroCall {
                            syntax: cast(index),
                        }),
                        StmtTag::Empty => StmtKind::Empty,
                    };
                    Stmt { kind, span }
                })
                .collect(),
            span: cached.span,
            rules: cached.rules,
            stmt_list: cached.stmt_list.map(|i| ast::StmtList { syntax: cast(i) }),
        }
    }

    pub(crate) fn inner_attrs(&self) -> Vec<Attribute<'a>> {
        self.stmt_list
            .as_ref()
            .map(|l| inner_attributes(l.syntax()))
            .unwrap_or_default()
    }

    pub(crate) fn is_unsafe(&self) -> bool {
        self.rules == BlockRules::Unsafe
    }
}

/// A [`Block`] without handles, as the run's block cache keeps it: every statement as its
/// kind, the index of its node in the tree and its span. A cached block is rebuilt for the
/// caller's tree lifetime by [`Block::from_cached`], so the cache carries no lifetime.
#[derive(Clone, Debug)]
pub(crate) struct CachedBlock {
    stmts: Vec<(StmtTag, u32, Span)>,
    span: Span,
    rules: BlockRules,
    stmt_list: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
enum StmtTag {
    Let,
    Item,
    Expr,
    Semi,
    MacCall,
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StmtKind<'a> {
    Let(ast::LetStmt<'a>),
    Item(ast::Item<'a>),
    /// An expression without a trailing semicolon (a tail expression or a block-like
    /// expression statement).
    Expr(ast::Expr<'a>),
    /// An expression followed by a semicolon.
    Semi(ast::Expr<'a>),
    /// A macro invocation in statement position (`foo!();`, `foo! { .. }`).
    MacCall(ast::MacroCall<'a>),
    /// A lone `;`.
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Stmt<'a> {
    pub(crate) kind: StmtKind<'a>,
    /// rustc's statement span: attributes included, trailing `;` included.
    pub(crate) span: Span,
}

impl<'a> Stmt<'a> {
    pub(crate) fn attrs(&self) -> Vec<Attribute<'a>> {
        match &self.kind {
            StmtKind::Let(l) => outer_attributes(l.syntax()),
            StmtKind::Item(i) => outer_attributes(i.syntax()),
            StmtKind::Expr(e) | StmtKind::Semi(e) => outer_attributes(e.syntax()),
            // `#[attr] foo!();`: the attributes belong to the `MacroExpr` around the call.
            // The parent search stops at the `MacroExpr`, a few elements back at most.
            StmtKind::MacCall(m) => match m.syntax().parent() {
                Some(p) if p.kind() == SyntaxKind::MACRO_EXPR => outer_attributes(p),
                _ => outer_attributes(m.syntax()),
            },
            StmtKind::Empty => vec![],
        }
    }

    pub(crate) fn expr(&self) -> Option<&ast::Expr<'a>> {
        match &self.kind {
            StmtKind::Expr(e) => Some(e),
            _ => None,
        }
    }

    pub(crate) fn as_item(&self) -> Option<&ast::Item<'a>> {
        match &self.kind {
            StmtKind::Item(i) => Some(i),
            _ => None,
        }
    }
}

/// A macro call in statement position is a statement macro (rather than an expression) if it
/// is followed by `;` or uses braces, mirroring rustc's `parse_stmt_mac`.
fn macro_stmt<'a>(expr: &ast::Expr<'a>, has_semi: bool) -> Option<ast::MacroCall<'a>> {
    let ast::Expr::MacroExpr(m) = expr else {
        return None;
    };
    let call = m.macro_call()?;
    let braces = call
        .token_tree()
        .and_then(|tt| tt.l_curly_token())
        .is_some();
    (has_semi || braces).then_some(call)
}

/// The statements of a statement list in rustc's form, tail expression included.
pub(crate) fn stmts_of<'a>(list: &ast::StmtList<'a>) -> Vec<Stmt<'a>> {
    let tail = list.tail_expr();
    let mut stmts = Vec::new();
    for child in list.syntax().children_with_tokens() {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::SEMICOLON => stmts.push(Stmt {
                kind: StmtKind::Empty,
                span: token_span(t),
            }),
            NodeOrToken::Token(_) => {}
            NodeOrToken::Node(n) => {
                if let Some(l) = ast::LetStmt::cast(n) {
                    stmts.push(Stmt {
                        span: rustc_span(n),
                        kind: StmtKind::Let(l),
                    });
                } else if let Some(es) = ast::ExprStmt::cast(n) {
                    let Some(expr) = es.expr() else { continue };
                    let has_semi = has_token(es.syntax(), T![;]);
                    let kind = match macro_stmt(&expr, has_semi) {
                        Some(call) => StmtKind::MacCall(call),
                        None if has_semi => StmtKind::Semi(expr),
                        None => StmtKind::Expr(expr),
                    };
                    stmts.push(Stmt {
                        span: rustc_span(n),
                        kind,
                    });
                } else if let Some(item) = ast::Item::cast(n) {
                    stmts.push(Stmt {
                        span: rustc_span(n),
                        kind: StmtKind::Item(item),
                    });
                } else if let Some(expr) = tail.filter(|t| t.syntax() == n) {
                    let kind = match macro_stmt(&expr, false) {
                        Some(call) => StmtKind::MacCall(call),
                        None => StmtKind::Expr(expr),
                    };
                    stmts.push(Stmt {
                        span: rustc_span(n),
                        kind,
                    });
                }
            }
        }
    }
    stmts
}

/// rustc's generics span: the `<...>` list, or an empty span right after the item name.
pub(crate) fn generics_span(params: Option<&ast::GenericParamList>, name_hi: BytePos) -> Span {
    match params {
        Some(p) => node_range_span(p.syntax()),
        None => mk_sp(name_hi, name_hi),
    }
}

/// rustc's where-clause span: the clause, or an empty span at `fallback`.
pub(crate) fn where_clause_span(wc: Option<&ast::WhereClause>, fallback: BytePos) -> Span {
    match wc {
        Some(wc) => node_range_span(wc.syntax()),
        None => mk_sp(fallback, fallback),
    }
}

/// Returns the first token of the given kind among `node`'s direct children.
pub(crate) fn child_token(node: Node<'_>, kind: SyntaxKind) -> Option<Token<'_>> {
    support::token(node, kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::Tree;
    use crate::cst::ast::HasModuleItem;
    use ra_ap_parser::Edition;

    fn parse(src: &str) -> Tree<'_> {
        Tree::parse_with_errors(src, Edition::CURRENT)
    }

    fn block_of<'t>(tree: &'t Tree<'t>) -> Block<'t> {
        let f = tree.root().descendants().find_map(ast::Fn::cast).unwrap();
        Block::from_block_expr(&f.body().unwrap()).unwrap()
    }

    fn first_item<'t>(tree: &'t Tree<'t>) -> ast::Item<'t> {
        ast::SourceFile::cast(tree.root())
            .unwrap()
            .items()
            .next()
            .unwrap()
    }

    #[test]
    fn statements_follow_rustc_classification() {
        let tree = parse("fn f() { let a = 1; a;; foo!(); bar! {} if x {} vec![1] }");
        let block = block_of(&tree);
        let kinds: Vec<_> = block
            .stmts
            .iter()
            .map(|s| match s.kind {
                StmtKind::Let(_) => "let",
                StmtKind::Item(_) => "item",
                StmtKind::Expr(_) => "expr",
                StmtKind::Semi(_) => "semi",
                StmtKind::MacCall(_) => "mac",
                StmtKind::Empty => "empty",
            })
            .collect();
        assert_eq!(
            kinds,
            ["let", "semi", "empty", "mac", "mac", "expr", "expr"]
        );
    }

    #[test]
    fn doc_comments_are_attributes() {
        let tree = parse("/// a\n// plain\n#[x]\nfn f() {}");
        let item = first_item(&tree);
        let attrs = outer_attributes(item.syntax());
        assert_eq!(attrs.len(), 2);
        assert!(attrs[0].is_doc_comment());
        assert!(attrs[1].has_name("x"));
    }

    #[test]
    fn skip_attribute_forms() {
        let tree = parse(
            "#[rustfmt::skip] #[cfg_attr(test, rustfmt::skip)] #[cfg_attr(a, b, c)] fn f() {}",
        );
        let item = first_item(&tree);
        let skips: Vec<_> = outer_attributes(item.syntax())
            .iter()
            .map(Attribute::is_skip)
            .collect();
        assert_eq!(skips, [true, true, false]);
    }

    /// Sources covering inner and outer attributes and doc comments in files, modules, blocks
    /// and item lists, plain comments that look like doc comments, and attribute paths with
    /// whitespace and comments inside.
    const ATTRIBUTE_SOURCES: &[&str] = &[
        "#![allow(dead_code)]\n//! inner line doc\n/*! inner block doc */\n// plain\nfn f() {}\n",
        "/// outer doc\n//// plain, four slashes\n/*** plain block */\n/**/\n#[test]\nfn f() {}\n",
        "fn f() {\n    #![allow(unused)]\n    //! inner doc in a block\n    let a = 1;\n}\n",
        "fn f() {\n    /// outer doc on a statement\n    let a = 1;\n    // plain\n    a\n}\n",
        "mod m {\n    #![cfg(test)]\n    fn f() {}\n}\nmod n {\n    #[test]\n    fn f() {}\n}\n",
        "impl S {\n    #![doc = \"x\"]\n    fn f() {}\n}\ntrait T {\n    //! trait doc\n    fn g();\n}\n",
        "extern \"C\" {\n    #![allow(x)]\n    fn h();\n}\nfn f() { if a { #![allow(x)] } }\n",
        "#[rustfmt /* c */ :: skip]\n#[ derive ( Debug ) ]\n#[cfg_attr(rustfmt, rustfmt::skip)]\nstruct S;\n",
        "#[r#raw::path]\n#[a::b::<c>]\nfn f() {}\n",
    ];

    /// `inner_attributes` without its `may_have_inner_attrs` pre-check.
    fn inner_attributes_by_cursor<'t>(container: Node<'t>) -> Vec<Attribute<'t>> {
        container
            .children_with_tokens()
            .filter_map(|child| match child {
                NodeOrToken::Token(t) if is_inner_doc_comment(t) => Some(Attribute::Doc(t)),
                NodeOrToken::Node(n) => ast::Attr::cast(n)
                    .filter(|a| has_token(a.syntax(), T![!]))
                    .map(Attribute::Normal),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn inner_attributes_match_a_cursor_walk_on_every_node() {
        for src in ATTRIBUTE_SOURCES {
            let tree = parse(src);
            for node in tree.root().descendants() {
                assert_eq!(
                    inner_attributes(node),
                    inner_attributes_by_cursor(node),
                    "{node:?} in {src:?}"
                );
            }
        }
    }

    #[test]
    fn inner_attributes_are_found() {
        let tree = parse(ATTRIBUTE_SOURCES[0]);
        let attrs = inner_attributes(tree.root());
        assert_eq!(attrs.len(), 3);
        assert!(attrs.iter().all(|a| a.style() == AttrStyle::Inner));
    }

    #[test]
    fn non_trivia_text_matches_a_cursor_walk_on_every_node() {
        for src in ATTRIBUTE_SOURCES {
            let tree = parse(src);
            for node in tree.root().descendants() {
                let by_cursor: String = node
                    .descendants_with_tokens()
                    .filter_map(|e| e.into_token())
                    .filter(|t| !t.kind().is_trivia())
                    .map(|t| t.text().to_string())
                    .collect();
                assert_eq!(non_trivia_text(node), by_cursor, "{node:?} in {src:?}");
            }
        }
    }

    #[test]
    fn attribute_paths_drop_whitespace_and_comments() {
        let tree = parse(ATTRIBUTE_SOURCES[7]);
        let item = first_item(&tree);
        let paths: Vec<_> = outer_attributes(item.syntax())
            .iter()
            .map(Attribute::path_text)
            .collect();
        assert_eq!(
            paths,
            [
                Some("rustfmt::skip".to_string()),
                Some("derive".to_string()),
                Some("cfg_attr".to_string())
            ]
        );
    }

    /// `has_token` answers what the generated `x_token().is_some()` accessors answer, on every
    /// node whose keyword tokens the formatter checks this way.
    #[test]
    fn has_token_matches_the_generated_token_accessors() {
        let src = "fn f() {\n    let x = 1;\n    let ref a = x;\n    let mut b = x;\n    \
                   let ref mut c = x;\n    super let d = 1;\n    let e = &raw const x;\n    \
                   let g = &raw mut b;\n    let h = &mut b;\n    let i = &x;\n    \
                   let j = async || 1;\n    let k = gen || 1;\n    let l = async gen || 1;\n    \
                   let m = move || 1;\n    match x { ref n @ 1 => {}, _ => {} }\n}\n";
        let tree = parse(src);
        let mut seen = [0usize; 7];
        for node in tree.root().descendants() {
            if let Some(p) = ast::IdentPat::cast(node) {
                assert_eq!(has_token(node, T![ref]), p.ref_token().is_some(), "{node}");
                assert_eq!(has_token(node, T![mut]), p.mut_token().is_some(), "{node}");
                seen[0] += usize::from(p.ref_token().is_some());
                seen[1] += usize::from(p.mut_token().is_some());
            } else if let Some(r) = ast::RefExpr::cast(node) {
                assert_eq!(has_token(node, T![raw]), r.raw_token().is_some(), "{node}");
                assert_eq!(has_token(node, T![mut]), r.mut_token().is_some(), "{node}");
                seen[2] += usize::from(r.raw_token().is_some());
            } else if let Some(c) = ast::ClosureExpr::cast(node) {
                assert_eq!(
                    has_token(node, T![async]),
                    c.async_token().is_some(),
                    "{node}"
                );
                assert_eq!(has_token(node, T![gen]), c.gen_token().is_some(), "{node}");
                seen[3] += usize::from(c.async_token().is_some());
                seen[4] += usize::from(c.gen_token().is_some());
            } else if let Some(l) = ast::LetStmt::cast(node) {
                assert_eq!(
                    has_token(node, T![super]),
                    l.super_token().is_some(),
                    "{node}"
                );
                seen[5] += usize::from(l.super_token().is_some());
                seen[6] += 1;
            }
        }
        assert!(
            seen.iter().all(|&n| n > 0),
            "every keyword occurs: {seen:?}"
        );
    }
}
