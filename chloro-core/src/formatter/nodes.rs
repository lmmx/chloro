//! rustc-shaped views of rust-analyzer syntax.
//!
//! rustfmt is written against rustc's AST, where doc comments are attributes, a block is a
//! list of statements (its tail expression included), a missing `where` clause still has a
//! span, and so on. rust-analyzer's lossless tree encodes the same information differently.
//! The types and functions here present rowan nodes in the shape the ported rustfmt logic
//! expects, so that the formatting modules can follow rustfmt's control flow closely.

use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
use rowan::Language;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Attribute {
    Doc(SyntaxToken),
    Normal(ast::Attr),
}

impl Attribute {
    pub(crate) fn span(&self) -> Span {
        match self {
            Attribute::Doc(t) => token_span(t),
            Attribute::Normal(a) => node_range_span(a.syntax()),
        }
    }

    pub(crate) fn is_doc_comment(&self) -> bool {
        matches!(self, Attribute::Doc(_))
    }

    pub(crate) fn style(&self) -> AttrStyle {
        match self {
            Attribute::Doc(t) if is_inner_doc_comment(t) => AttrStyle::Inner,
            Attribute::Doc(_) => AttrStyle::Outer,
            Attribute::Normal(a) if a.excl_token().is_some() => AttrStyle::Inner,
            Attribute::Normal(_) => AttrStyle::Outer,
        }
    }

    /// The attribute's path with whitespace removed, e.g. `derive` or `rustfmt::skip`.
    pub(crate) fn path_text(&self) -> Option<String> {
        match self {
            Attribute::Doc(_) => None,
            Attribute::Normal(a) => {
                let path = a.meta()?.path()?;
                Some(
                    path.syntax()
                        .descendants_with_tokens()
                        .filter_map(|e| e.into_token())
                        .filter(|t| !t.kind().is_trivia())
                        .map(|t| t.text().to_string())
                        .collect(),
                )
            }
        }
    }

    pub(crate) fn has_name(&self, name: &str) -> bool {
        self.path_text().is_some_and(|p| p == name)
    }

    pub(crate) fn as_attr(&self) -> Option<&ast::Attr> {
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
                let meta = a.meta();
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
pub(crate) fn split_top_level_commas(tt: &SyntaxNode) -> Vec<Vec<SyntaxToken>> {
    let mut parts: Vec<Vec<SyntaxToken>> = vec![vec![]];
    let children: Vec<_> = tt.children_with_tokens().collect();
    let inner = children
        .get(1..children.len().saturating_sub(1))
        .unwrap_or(&[]);
    for child in inner {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMA => parts.push(vec![]),
            NodeOrToken::Token(t) => parts.last_mut().unwrap().push(t.clone()),
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

/// `true` if `node` starts with an attribute or an outer doc comment. Reads the green tree,
/// which avoids creating cursor nodes for the common case of a node without attributes.
fn has_leading_attrs(node: &SyntaxNode) -> bool {
    for child in node.green().children() {
        match child {
            NodeOrToken::Node(n) => {
                return RustLanguage::kind_from_raw(n.kind()) == SyntaxKind::ATTR;
            }
            NodeOrToken::Token(t) => match RustLanguage::kind_from_raw(t.kind()) {
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
pub(crate) fn outer_attributes(node: &SyntaxNode) -> Vec<Attribute> {
    if !has_leading_attrs(node) {
        return Vec::new();
    }
    let mut attrs = Vec::new();
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::WHITESPACE => {}
                SyntaxKind::COMMENT => {
                    if is_outer_doc_comment(&t) {
                        attrs.push(Attribute::Doc(t));
                    }
                }
                _ => break,
            },
            NodeOrToken::Node(n) => match ast::Attr::cast(n) {
                Some(attr) if attr.excl_token().is_none() => attrs.push(Attribute::Normal(attr)),
                Some(_) => {}
                None => break,
            },
        }
    }
    attrs
}

/// Inner attributes (`#![...]`, `//!`) that are direct children of a container node such as
/// a source file, an item list or a statement list.
pub(crate) fn inner_attributes(container: &SyntaxNode) -> Vec<Attribute> {
    container
        .children_with_tokens()
        .filter_map(|child| match child {
            NodeOrToken::Token(t) if is_inner_doc_comment(&t) => Some(Attribute::Doc(t)),
            NodeOrToken::Node(n) => ast::Attr::cast(n)
                .filter(|a| a.excl_token().is_some())
                .map(Attribute::Normal),
            _ => None,
        })
        .collect()
}

pub(crate) fn contains_skip(attrs: &[Attribute]) -> bool {
    attrs.iter().any(Attribute::is_skip)
}

/// The span of `node` without its outer attributes and doc comments.
pub(crate) fn span_without_attrs(node: &SyntaxNode) -> Span {
    let full = node_range_span(node);
    let first = node.children_with_tokens().find(|el| match el {
        NodeOrToken::Token(t) => !t.kind().is_trivia(),
        NodeOrToken::Node(n) => n.kind() != SyntaxKind::ATTR,
    });
    match first {
        Some(el) => full.with_lo(el.text_range().start().into()),
        None => full,
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
pub(crate) struct Block {
    pub(crate) stmts: Vec<Stmt>,
    pub(crate) span: Span,
    pub(crate) rules: BlockRules,
    pub(crate) stmt_list: Option<ast::StmtList>,
}

impl Block {
    /// The block of a block expression (`{}`, `unsafe {}`, `'a: {}`, `async {}`, ...).
    pub(crate) fn from_block_expr(block: &ast::BlockExpr) -> Option<Block> {
        let stmt_list = block.stmt_list()?;
        let list_span = node_range_span(stmt_list.syntax());
        let (rules, span) = match block.unsafe_token() {
            Some(t) => (BlockRules::Unsafe, list_span.with_lo(token_span(&t).lo())),
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
    pub(crate) fn synthetic(expr: &ast::Expr) -> Block {
        let span = expr.span();
        Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(expr.clone()),
                span,
            }],
            span,
            rules: BlockRules::Default,
            stmt_list: None,
        }
    }

    pub(crate) fn inner_attrs(&self) -> Vec<Attribute> {
        self.stmt_list
            .as_ref()
            .map(|l| inner_attributes(l.syntax()))
            .unwrap_or_default()
    }

    pub(crate) fn is_unsafe(&self) -> bool {
        self.rules == BlockRules::Unsafe
    }
}

#[derive(Clone, Debug)]
pub(crate) enum StmtKind {
    Let(ast::LetStmt),
    Item(ast::Item),
    /// An expression without a trailing semicolon (a tail expression or a block-like
    /// expression statement).
    Expr(ast::Expr),
    /// An expression followed by a semicolon.
    Semi(ast::Expr),
    /// A macro invocation in statement position (`foo!();`, `foo! { .. }`).
    MacCall(ast::MacroCall),
    /// A lone `;`.
    Empty,
}

#[derive(Clone, Debug)]
pub(crate) struct Stmt {
    pub(crate) kind: StmtKind,
    /// rustc's statement span: attributes included, trailing `;` included.
    pub(crate) span: Span,
}

impl Stmt {
    pub(crate) fn attrs(&self) -> Vec<Attribute> {
        match &self.kind {
            StmtKind::Let(l) => outer_attributes(l.syntax()),
            StmtKind::Item(i) => outer_attributes(i.syntax()),
            StmtKind::Expr(e) | StmtKind::Semi(e) => outer_attributes(e.syntax()),
            // `#[attr] foo!();`: the attributes belong to the `MacroExpr` around the call.
            StmtKind::MacCall(m) => match m.syntax().parent() {
                Some(p) if p.kind() == SyntaxKind::MACRO_EXPR => outer_attributes(&p),
                _ => outer_attributes(m.syntax()),
            },
            StmtKind::Empty => vec![],
        }
    }

    pub(crate) fn expr(&self) -> Option<&ast::Expr> {
        match &self.kind {
            StmtKind::Expr(e) => Some(e),
            _ => None,
        }
    }

    pub(crate) fn as_item(&self) -> Option<&ast::Item> {
        match &self.kind {
            StmtKind::Item(i) => Some(i),
            _ => None,
        }
    }
}

/// A macro call in statement position is a statement macro (rather than an expression) if it
/// is followed by `;` or uses braces, mirroring rustc's `parse_stmt_mac`.
fn macro_stmt(expr: &ast::Expr, has_semi: bool) -> Option<ast::MacroCall> {
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
pub(crate) fn stmts_of(list: &ast::StmtList) -> Vec<Stmt> {
    let tail = list.tail_expr();
    let mut stmts = Vec::new();
    for child in list.syntax().children_with_tokens() {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::SEMICOLON => stmts.push(Stmt {
                kind: StmtKind::Empty,
                span: token_span(&t),
            }),
            NodeOrToken::Token(_) => {}
            NodeOrToken::Node(n) => {
                if let Some(l) = ast::LetStmt::cast(n.clone()) {
                    stmts.push(Stmt {
                        span: rustc_span(&n),
                        kind: StmtKind::Let(l),
                    });
                } else if let Some(es) = ast::ExprStmt::cast(n.clone()) {
                    let Some(expr) = es.expr() else { continue };
                    let has_semi = es.semicolon_token().is_some();
                    let kind = match macro_stmt(&expr, has_semi) {
                        Some(call) => StmtKind::MacCall(call),
                        None if has_semi => StmtKind::Semi(expr),
                        None => StmtKind::Expr(expr),
                    };
                    stmts.push(Stmt {
                        span: rustc_span(&n),
                        kind,
                    });
                } else if let Some(item) = ast::Item::cast(n.clone()) {
                    stmts.push(Stmt {
                        span: rustc_span(&n),
                        kind: StmtKind::Item(item),
                    });
                } else if let Some(expr) = tail.as_ref().filter(|t| t.syntax() == &n) {
                    let kind = match macro_stmt(expr, false) {
                        Some(call) => StmtKind::MacCall(call),
                        None => StmtKind::Expr(expr.clone()),
                    };
                    stmts.push(Stmt {
                        span: rustc_span(&n),
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
pub(crate) fn child_token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::ast::HasModuleItem;
    use ra_ap_syntax::{Edition, SourceFile};

    fn block_of(src: &str) -> Block {
        let file = SourceFile::parse(src, Edition::CURRENT).tree();
        let f = file.syntax().descendants().find_map(ast::Fn::cast).unwrap();
        Block::from_block_expr(&f.body().unwrap()).unwrap()
    }

    #[test]
    fn statements_follow_rustc_classification() {
        let block = block_of("fn f() { let a = 1; a;; foo!(); bar! {} if x {} vec![1] }");
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
        let src = "/// a\n// plain\n#[x]\nfn f() {}";
        let file = SourceFile::parse(src, Edition::CURRENT).tree();
        let item = file.items().next().unwrap();
        let attrs = outer_attributes(item.syntax());
        assert_eq!(attrs.len(), 2);
        assert!(attrs[0].is_doc_comment());
        assert!(attrs[1].has_name("x"));
    }

    #[test]
    fn skip_attribute_forms() {
        let src =
            "#[rustfmt::skip] #[cfg_attr(test, rustfmt::skip)] #[cfg_attr(a, b, c)] fn f() {}";
        let file = SourceFile::parse(src, Edition::CURRENT).tree();
        let item = file.items().next().unwrap();
        let skips: Vec<_> = outer_attributes(item.syntax())
            .iter()
            .map(Attribute::is_skip)
            .collect();
        assert_eq!(skips, [true, true, false]);
    }
}
