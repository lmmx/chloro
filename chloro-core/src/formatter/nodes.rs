//! rustc-shaped views of rust-analyzer syntax.
//!
//! rustfmt is written against rustc's AST, where doc comments are attributes, a block is a
//! list of statements (its tail expression included), a missing `where` clause still has a
//! span, and so on. rust-analyzer's lossless tree encodes the same information differently.
//! The types and functions here present rowan nodes in the shape the ported rustfmt logic
//! expects, so that the formatting modules can follow rustfmt's control flow closely.

use ra_ap_syntax::T;
use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{
    NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize,
};
use rowan::Language;

#[cfg(test)]
use super::span::is_outer_doc_comment;
use super::span::{
    BytePos, Span, Spanned, is_inner_doc_comment, is_inner_doc_text, is_outer_doc_text, mk_sp,
    node_range_span, rustc_span, token_span,
};
use ra_ap_syntax::SyntaxElement;

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
            Attribute::Normal(a) if has_token(a.syntax(), T![!]) => AttrStyle::Inner,
            Attribute::Normal(_) => AttrStyle::Outer,
        }
    }

    /// The attribute's path with whitespace removed, e.g. `derive` or `rustfmt::skip`.
    pub(crate) fn path_text(&self) -> Option<String> {
        match self {
            Attribute::Doc(_) => None,
            Attribute::Normal(a) => attr_path_text(a),
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
                let meta = child::<ast::Meta>(a.syntax());
                let has_args = meta.as_ref().is_some_and(|m| {
                    has_child::<ast::TokenTree>(m.syntax()) || has_child::<ast::Expr>(m.syntax())
                });
                if !has_args {
                    return path == "rustfmt::skip" || path == "rustfmt_skip";
                }
                if path != "cfg_attr" {
                    return false;
                }
                let Some(tt) = meta.and_then(|m| child::<ast::TokenTree>(m.syntax())) else {
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

/// The text of the non-trivia tokens under `node`, in source order, read from the green tree
/// (no cursor node per descendant).
#[cfg(test)]
pub(crate) fn non_trivia_text(node: &SyntaxNode) -> String {
    let mut out = String::new();
    push_non_trivia_text(&node.green(), &mut out);
    out
}

fn push_non_trivia_text(green: &rowan::GreenNodeData, out: &mut String) {
    for child in green.children() {
        match child {
            NodeOrToken::Node(n) => push_non_trivia_text(n, out),
            NodeOrToken::Token(t) => {
                if !RustLanguage::kind_from_raw(t.kind()).is_trivia() {
                    out.push_str(t.text());
                }
            }
        }
    }
}

/// `Attr::meta().path()` as whitespace-free text (`derive`, `rustfmt::skip`): the path node of
/// the first meta node of `attr`, read from the green tree.
pub(crate) fn attr_path_text(attr: &ast::Attr) -> Option<String> {
    let green = attr.syntax().green();
    let meta = green_child(&green, SyntaxKind::META)?;
    let path = green_child(meta, SyntaxKind::PATH)?;
    let mut out = String::new();
    push_non_trivia_text(path, &mut out);
    Some(out)
}

/// The first child node of `green` of `kind`.
pub(crate) fn green_child(
    green: &rowan::GreenNodeData,
    kind: SyntaxKind,
) -> Option<&rowan::GreenNodeData> {
    green.children().find_map(|c| match c {
        NodeOrToken::Node(n) if RustLanguage::kind_from_raw(n.kind()) == kind => Some(n),
        _ => None,
    })
}

/// Whether `parent` has a child node that casts to `N`: a generated `x().is_some()` whose
/// `support::child` scan creates a red node per sibling, read from the green tree.
pub(crate) fn has_child<N: AstNode>(parent: &SyntaxNode) -> bool {
    parent.green().children().any(
        |c| matches!(c, NodeOrToken::Node(n) if N::can_cast(RustLanguage::kind_from_raw(n.kind()))),
    )
}

/// The range of the first child node of `parent` that casts to `N`: a generated `x()` whose
/// caller reads only the node's span, read from the green tree.
pub(crate) fn child_range<N: AstNode>(parent: &SyntaxNode) -> Option<TextRange> {
    let mut offset = parent.text_range().start();
    for child in parent.green().children() {
        let len = child.text_len();
        if let NodeOrToken::Node(n) = child
            && N::can_cast(RustLanguage::kind_from_raw(n.kind()))
        {
            return Some(TextRange::at(offset, len));
        }
        offset += len;
    }
    None
}

/// `BinExpr::op_details` on the green tree: the first child token that is a binary operator,
/// as rust-analyzer classifies it, with its text.
fn bin_op_details(expr: &ast::BinExpr) -> Option<(ast::BinaryOp, String)> {
    use ast::{ArithOp, BinaryOp, CmpOp, LogicOp, Ordering};
    expr.syntax().green().children().find_map(|c| {
        let NodeOrToken::Token(t) = c else {
            return None;
        };
        #[rustfmt::skip]
        let op = match RustLanguage::kind_from_raw(t.kind()) {
            T![||] => BinaryOp::LogicOp(LogicOp::Or),
            T![&&] => BinaryOp::LogicOp(LogicOp::And),

            T![==] => BinaryOp::CmpOp(CmpOp::Eq { negated: false }),
            T![!=] => BinaryOp::CmpOp(CmpOp::Eq { negated: true }),
            T![<=] => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less,    strict: false }),
            T![>=] => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: false }),
            T![<]  => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less,    strict: true }),
            T![>]  => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: true }),

            T![+]  => BinaryOp::ArithOp(ArithOp::Add),
            T![*]  => BinaryOp::ArithOp(ArithOp::Mul),
            T![-]  => BinaryOp::ArithOp(ArithOp::Sub),
            T![/]  => BinaryOp::ArithOp(ArithOp::Div),
            T![%]  => BinaryOp::ArithOp(ArithOp::Rem),
            T![<<] => BinaryOp::ArithOp(ArithOp::Shl),
            T![>>] => BinaryOp::ArithOp(ArithOp::Shr),
            T![^]  => BinaryOp::ArithOp(ArithOp::BitXor),
            T![|]  => BinaryOp::ArithOp(ArithOp::BitOr),
            T![&]  => BinaryOp::ArithOp(ArithOp::BitAnd),

            T![=]   => BinaryOp::Assignment { op: None },
            T![+=]  => BinaryOp::Assignment { op: Some(ArithOp::Add) },
            T![*=]  => BinaryOp::Assignment { op: Some(ArithOp::Mul) },
            T![-=]  => BinaryOp::Assignment { op: Some(ArithOp::Sub) },
            T![/=]  => BinaryOp::Assignment { op: Some(ArithOp::Div) },
            T![%=]  => BinaryOp::Assignment { op: Some(ArithOp::Rem) },
            T![<<=] => BinaryOp::Assignment { op: Some(ArithOp::Shl) },
            T![>>=] => BinaryOp::Assignment { op: Some(ArithOp::Shr) },
            T![^=]  => BinaryOp::Assignment { op: Some(ArithOp::BitXor) },
            T![|=]  => BinaryOp::Assignment { op: Some(ArithOp::BitOr) },
            T![&=]  => BinaryOp::Assignment { op: Some(ArithOp::BitAnd) },

            _ => return None,
        };
        Some((op, t.text().to_string()))
    })
}

/// `BinExpr::op_kind` read from the green tree.
pub(crate) fn bin_op_kind(expr: &ast::BinExpr) -> Option<ast::BinaryOp> {
    bin_op_details(expr).map(|(op, _)| op)
}

/// The operator of a binary expression as written, e.g. `&&` or `+=`
/// (`BinExpr::op_token().text()`), read from the green tree.
pub(crate) fn bin_op_text(expr: &ast::BinExpr) -> Option<String> {
    bin_op_details(expr).map(|(_, text)| text)
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
///
/// The leading children are classified on the green tree; a red element is created for each
/// attribute kept, and for none of the whitespace, plain comments and inner attributes
/// between them, nor for the child the scan stops at.
pub(crate) fn outer_attributes(node: &SyntaxNode) -> Vec<Attribute> {
    if !has_leading_attrs(node) {
        return Vec::new();
    }
    let mut attrs = Vec::new();
    let mut offset = node.text_range().start();
    for (index, child) in node.green().children().enumerate() {
        let len = child.text_len();
        match child {
            NodeOrToken::Token(t) => match RustLanguage::kind_from_raw(t.kind()) {
                SyntaxKind::WHITESPACE => {}
                SyntaxKind::COMMENT => {
                    if is_outer_doc_text(t.text())
                        && let Some(t) = element_at(node, index, offset, len)
                        && let Some(t) = t.into_token()
                    {
                        attrs.push(Attribute::Doc(t));
                    }
                }
                _ => break,
            },
            NodeOrToken::Node(n) => {
                if RustLanguage::kind_from_raw(n.kind()) != SyntaxKind::ATTR {
                    break;
                }
                if !green_has_token(n, T![!])
                    && let Some(attr) = child_at(node, index, offset, len).and_then(ast::Attr::cast)
                {
                    attrs.push(Attribute::Normal(attr));
                }
            }
        }
        offset += len;
    }
    attrs
}

/// `false` when no direct child of `container` can be an inner attribute: no attribute with
/// a `!` token and no comment starting `//!` or `/*!`. Reads the green tree, so a container
/// without inner attributes (nearly every block and module) costs no cursor node per child.
/// A `true` only means [`inner_attributes`] has to look.
fn may_have_inner_attrs(container: &SyntaxNode) -> bool {
    container.green().children().any(|child| match child {
        NodeOrToken::Node(n) => {
            RustLanguage::kind_from_raw(n.kind()) == SyntaxKind::ATTR
                && n.children().any(|c| {
                    matches!(c, NodeOrToken::Token(t) if RustLanguage::kind_from_raw(t.kind()) == T![!])
                })
        }
        NodeOrToken::Token(t) => {
            RustLanguage::kind_from_raw(t.kind()) == SyntaxKind::COMMENT
                && (t.text().starts_with("//!") || t.text().starts_with("/*!"))
        }
    })
}

/// Inner attributes (`#![...]`, `//!`) that are direct children of a container node such as
/// a source file, an item list or a statement list.
pub(crate) fn inner_attributes(container: &SyntaxNode) -> Vec<Attribute> {
    if !may_have_inner_attrs(container) {
        return Vec::new();
    }
    let mut attrs = Vec::new();
    let mut offset = container.text_range().start();
    for (index, child) in container.green().children().enumerate() {
        let len = child.text_len();
        match child {
            NodeOrToken::Token(t)
                if RustLanguage::kind_from_raw(t.kind()) == SyntaxKind::COMMENT
                    && is_inner_doc_text(t.text()) =>
            {
                if let Some(t) = element_at(container, index, offset, len)
                    && let Some(t) = t.into_token()
                {
                    attrs.push(Attribute::Doc(t));
                }
            }
            NodeOrToken::Node(n)
                if RustLanguage::kind_from_raw(n.kind()) == SyntaxKind::ATTR
                    && green_has_token(n, T![!]) =>
            {
                if let Some(attr) =
                    child_at(container, index, offset, len).and_then(ast::Attr::cast)
                {
                    attrs.push(Attribute::Normal(attr));
                }
            }
            _ => {}
        }
        offset += len;
    }
    attrs
}

pub(crate) fn contains_skip(attrs: &[Attribute]) -> bool {
    attrs.iter().any(Attribute::is_skip)
}

/// The span of `node` without its outer attributes and doc comments.
pub(crate) fn span_without_attrs(node: &SyntaxNode) -> Span {
    let full = node_range_span(node);
    // The first child that is neither trivia nor an attribute, found on the green tree.
    let mut lo = full.lo();
    for child in node.green().children() {
        let (kind, len) = match child {
            NodeOrToken::Token(t) => (t.kind(), t.text_len()),
            NodeOrToken::Node(n) => (n.kind(), n.text_len()),
        };
        let kind = RustLanguage::kind_from_raw(kind);
        let skipped = match child {
            NodeOrToken::Token(_) => kind.is_trivia(),
            NodeOrToken::Node(_) => kind == SyntaxKind::ATTR,
        };
        if !skipped {
            return full.with_lo(lo);
        }
        lo += BytePos::from(len);
    }
    full
}

/// Whether `node` has a direct child token of `kind`: a generated `x_token().is_some()`
/// that reads the green tree instead of creating a red node per child.
pub(crate) fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    green_has_token(&node.green(), kind)
}

/// [`has_token`] on a green node.
pub(crate) fn green_has_token(green: &rowan::GreenNodeData, kind: SyntaxKind) -> bool {
    green.children().any(|child| {
        matches!(child, NodeOrToken::Token(t) if RustLanguage::kind_from_raw(t.kind()) == kind)
    })
}

/// The start of the first direct child token of `kind`: a generated `x_token()` whose caller
/// reads only the token's position, from the green tree.
pub(crate) fn token_start(node: &SyntaxNode, kind: SyntaxKind) -> Option<BytePos> {
    let mut offset = node.text_range().start();
    for child in node.green().children() {
        if let NodeOrToken::Token(t) = child
            && RustLanguage::kind_from_raw(t.kind()) == kind
        {
            return Some(BytePos::from(offset));
        }
        offset += child.text_len();
    }
    None
}

/// The red element for the `index`-th green child of `parent`, which starts at `offset` and
/// is `len` long: a binary search on the range when the child has text, a walk to the
/// position otherwise (a range search cannot tell empty siblings apart).
pub(crate) fn element_at(
    parent: &SyntaxNode,
    index: usize,
    offset: TextSize,
    len: TextSize,
) -> Option<SyntaxElement> {
    if len == TextSize::from(0) {
        return parent.children_with_tokens().nth(index);
    }
    parent.child_or_token_at_range(TextRange::at(offset, len))
}

/// The `n`-th child node of `parent` that casts to `N`: the hand-written rust-analyzer
/// accessors `support::children(..).nth(n)` (`BinExpr::rhs`, `IndexExpr::index`, the `if`
/// branches), but a red node is created only for the child returned.
pub(crate) fn nth_child<N: AstNode>(parent: &SyntaxNode, n: usize) -> Option<N> {
    let mut offset = parent.text_range().start();
    let mut seen = 0;
    for (index, green_child) in parent.green().children().enumerate() {
        let len = green_child.text_len();
        if let NodeOrToken::Node(node) = green_child
            && N::can_cast(RustLanguage::kind_from_raw(node.kind()))
        {
            if seen == n {
                return child_at(parent, index, offset, len).and_then(N::cast);
            }
            seen += 1;
        }
        offset += len;
    }
    None
}

/// The first expression child of an `if`, `while` or `for` expression, as rust-analyzer's
/// `IfExpr::condition`, `WhileExpr::condition` and `ForExpr::iterable` answer it: a block
/// expression there counts as the condition only when another expression follows it (else
/// it is the body of a condition-less expression).
pub(crate) fn condition_expr(parent: &SyntaxNode) -> Option<ast::Expr> {
    let first = nth_child::<ast::Expr>(parent, 0)?;
    if matches!(first, ast::Expr::BlockExpr(_)) && !has_nth_child::<ast::Expr>(parent, 1) {
        return None;
    }
    Some(first)
}

/// Whether `parent` has an `n`-th child node that casts to `N`, read from the green tree.
fn has_nth_child<N: AstNode>(parent: &SyntaxNode, n: usize) -> bool {
    parent
        .green()
        .children()
        .filter(|c| {
            matches!(c, NodeOrToken::Node(node) if N::can_cast(RustLanguage::kind_from_raw(node.kind())))
        })
        .nth(n)
        .is_some()
}

/// `IfExpr::then_branch`: the second expression child, when it is a block.
pub(crate) fn then_branch(if_expr: &ast::IfExpr) -> Option<ast::BlockExpr> {
    match nth_child::<ast::Expr>(if_expr.syntax(), 1)? {
        ast::Expr::BlockExpr(block) => Some(block),
        _ => None,
    }
}

/// `IfExpr::else_branch` as an expression: the third expression child, when it is a block
/// or another `if`.
pub(crate) fn else_branch(if_expr: &ast::IfExpr) -> Option<ast::Expr> {
    nth_child::<ast::Expr>(if_expr.syntax(), 2)
        .filter(|e| matches!(e, ast::Expr::BlockExpr(_) | ast::Expr::IfExpr(_)))
}

/// `HasLoopBody::loop_body` for `while` and `for`: the second block child, else the first.
pub(crate) fn loop_body(parent: &SyntaxNode) -> Option<ast::BlockExpr> {
    nth_child::<ast::BlockExpr>(parent, 1).or_else(|| nth_child::<ast::BlockExpr>(parent, 0))
}

/// [`element_at`] for a child node.
pub(crate) fn child_at(
    parent: &SyntaxNode,
    index: usize,
    offset: TextSize,
    len: TextSize,
) -> Option<SyntaxNode> {
    element_at(parent, index, offset, len)?.into_node()
}

/// The first child node of `parent` that casts to `N`: the generated `support::child`
/// accessors (`x.expr()`, `x.path()`, ...), but a red node is created only for the child
/// found, located by a binary search on its range, instead of one per child inspected.
pub(crate) fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    let mut offset = parent.text_range().start();
    for (index, green_child) in parent.green().children().enumerate() {
        let len = green_child.text_len();
        if let NodeOrToken::Node(n) = green_child
            && N::can_cast(RustLanguage::kind_from_raw(n.kind()))
        {
            return child_at(parent, index, offset, len).and_then(N::cast);
        }
        offset += len;
    }
    None
}

/// The source text of `node`. Reads the green tree, which costs neither a red node per
/// token nor `fmt` machinery, unlike `node.text().to_string()`.
pub(crate) fn node_text(node: &SyntaxNode) -> String {
    fn push(green: &rowan::GreenNodeData, out: &mut String) {
        for child in green.children() {
            match child {
                NodeOrToken::Node(n) => push(n, out),
                NodeOrToken::Token(t) => out.push_str(t.text()),
            }
        }
    }
    let mut out = String::with_capacity(usize::from(node.text_range().len()));
    push(&node.green(), &mut out);
    out
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
pub(crate) struct Block {
    pub(crate) stmts: Vec<Stmt>,
    pub(crate) span: Span,
    pub(crate) rules: BlockRules,
    pub(crate) stmt_list: Option<ast::StmtList>,
}

impl Block {
    /// The block of a block expression (`{}`, `unsafe {}`, `'a: {}`, `async {}`, ...).
    pub(crate) fn from_block_expr(block: &ast::BlockExpr) -> Option<Block> {
        let stmt_list = child::<ast::StmtList>(block.syntax())?;
        let list_span = node_range_span(stmt_list.syntax());
        // `BlockExpr::unsafe_token`, of which only the start is used.
        let (rules, span) = match token_start(block.syntax(), T![unsafe]) {
            Some(lo) => (BlockRules::Unsafe, list_span.with_lo(lo)),
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
    let call = child::<ast::MacroCall>(m.syntax())?;
    // `MacroCall::token_tree` is the first token tree child; `TokenTree::l_curly_token` is a
    // direct `{` token of it. Both read from the green tree.
    let braces = call
        .syntax()
        .green()
        .children()
        .find_map(|c| match c {
            NodeOrToken::Node(n)
                if RustLanguage::kind_from_raw(n.kind()) == SyntaxKind::TOKEN_TREE =>
            {
                Some(green_has_token(n, T!['{']))
            }
            _ => None,
        })
        .unwrap_or(false);
    (has_semi || braces).then_some(call)
}

/// The statements of a statement list in rustc's form, tail expression included.
///
/// The list's children are classified on the green tree; a red node is created for each
/// statement kept, and for none of the whitespace, brace and semicolon tokens between them.
pub(crate) fn stmts_of(list: &ast::StmtList) -> Vec<Stmt> {
    let parent = list.syntax();
    let green = parent.green();
    // `StmtList::tail_expr`: the first child node that is an expression.
    let tail_index = green.children().position(|c| {
        matches!(c, NodeOrToken::Node(n) if ast::Expr::can_cast(RustLanguage::kind_from_raw(n.kind())))
    });
    let mut offset = parent.text_range().start();
    let mut stmts = Vec::new();
    for (index, element) in green.children().enumerate() {
        let len = element.text_len();
        match element {
            NodeOrToken::Token(t) => {
                if RustLanguage::kind_from_raw(t.kind()) == SyntaxKind::SEMICOLON {
                    stmts.push(Stmt {
                        kind: StmtKind::Empty,
                        span: mk_sp(BytePos::from(offset), BytePos::from(offset + len)),
                    });
                }
            }
            NodeOrToken::Node(n) => {
                let kind = RustLanguage::kind_from_raw(n.kind());
                let is_let = ast::LetStmt::can_cast(kind);
                let is_expr_stmt = ast::ExprStmt::can_cast(kind);
                let is_item = ast::Item::can_cast(kind);
                let is_tail = tail_index == Some(index);
                if is_let || is_expr_stmt || is_item || is_tail {
                    let Some(n) = child_at(parent, index, offset, len) else {
                        continue;
                    };
                    let span = rustc_span(&n);
                    let kind = if is_let {
                        let Some(l) = ast::LetStmt::cast(n) else {
                            continue;
                        };
                        StmtKind::Let(l)
                    } else if is_expr_stmt {
                        let Some(expr) = child::<ast::Expr>(&n) else {
                            continue;
                        };
                        let has_semi = has_token(&n, T![;]);
                        match macro_stmt(&expr, has_semi) {
                            Some(call) => StmtKind::MacCall(call),
                            None if has_semi => StmtKind::Semi(expr),
                            None => StmtKind::Expr(expr),
                        }
                    } else if is_item {
                        let Some(item) = ast::Item::cast(n) else {
                            continue;
                        };
                        StmtKind::Item(item)
                    } else {
                        let Some(expr) = ast::Expr::cast(n) else {
                            continue;
                        };
                        match macro_stmt(&expr, false) {
                            Some(call) => StmtKind::MacCall(call),
                            None => StmtKind::Expr(expr),
                        }
                    };
                    stmts.push(Stmt { kind, span });
                }
            }
        }
        offset += len;
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

    /// Sources covering every statement kind and block kind `stmts_of` and
    /// `Block::from_block_expr` classify: `let` statements, expression statements with and
    /// without `;`, block-like expression statements, statement macros with `;`, braces and
    /// neither, items in blocks, empty statements, tail expressions, blocks without a tail,
    /// empty blocks, `unsafe`, `const`, `async`, `try`, `gen` and labeled blocks, attributes
    /// and doc comments on statements, inner attributes and comments in blocks, `let`-`else`.
    const BLOCK_SOURCES: &[&str] = &[
        "fn f() {\n    let a = 1;\n    a;\n    ;\n    foo!();\n    bar! {}\n    baz![];\n    \
         if x {} else {}\n    fn g() {}\n    struct S;\n    use std::io;\n    \
         macro_rules! m { () => {} }\n    const C: u8 = 1;\n    vec![1]\n}\n",
        "fn f() { m! {} }\nfn g() { m!() }\nfn h() { m!(); }\nfn i() { m! { a }; }\n\
         fn j() { m!({}) }\nfn k() { m![{}]; m }\n",
        "fn f() {\n    #[allow(x)]\n    let a = 1;\n    /// doc\n    #[cfg(test)]\n    fn g() {}\n    \
         #[attr] foo!();\n    #[attr] bar! {}\n    #[attr] a\n}\n",
        "fn f() {\n    unsafe { x; }\n    const { 1 }\n    async { 1 }\n    async move { 1 }\n    \
         'a: { break 'a; }\n    try { 1 }\n    gen { yield 1 }\n    {}\n    { ; }\n    { x }\n    \
         { x; }\n    let b = { let c = 1; c };\n    let Some(d) = e else { return };\n    \
         loop { break; }\n    while x {}\n    unsafe {}\n}\n",
        "fn f() {\n    #![allow(x)]\n    // c\n    let a = 1; // t\n    /* b */ a\n}\n",
        "fn f() {\n    match x { _ => {} }\n    unsafe {}\n    a.b();\n    (a, b)\n}\n",
        "fn f() {\n    let x = || { 1 };\n    let y = async move { x };\n    x = 2;\n    x += 1;\n    \
         return;\n}\n",
    ];

    /// The red-cursor computation `stmts_of` replaced.
    fn stmts_by_cursor(list: &ast::StmtList) -> Vec<Stmt> {
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

    /// What a statement holds, for comparison: its kind, span and node.
    fn stmt_key(stmt: &Stmt) -> (&'static str, Span, Option<SyntaxNode>) {
        let (kind, node) = match &stmt.kind {
            StmtKind::Let(l) => ("let", Some(l.syntax().clone())),
            StmtKind::Item(i) => ("item", Some(i.syntax().clone())),
            StmtKind::Expr(e) => ("expr", Some(e.syntax().clone())),
            StmtKind::Semi(e) => ("semi", Some(e.syntax().clone())),
            StmtKind::MacCall(m) => ("mac", Some(m.syntax().clone())),
            StmtKind::Empty => ("empty", None),
        };
        (kind, stmt.span, node)
    }

    #[test]
    fn stmts_of_matches_a_cursor_walk_on_every_statement_list() {
        let mut seen = std::collections::BTreeMap::new();
        for src in BLOCK_SOURCES {
            let file = SourceFile::parse(src, Edition::Edition2024).tree();
            for list in file.syntax().descendants().filter_map(ast::StmtList::cast) {
                let green: Vec<_> = stmts_of(&list).iter().map(stmt_key).collect();
                let red: Vec<_> = stmts_by_cursor(&list).iter().map(stmt_key).collect();
                assert_eq!(green, red, "{list} in {src:?}");
                for (kind, _, node) in &green {
                    let is_tail = |n: &SyntaxNode| {
                        list.tail_expr()
                            .is_some_and(|t| t.syntax().text_range() == n.text_range())
                    };
                    let variant = match (*kind, node) {
                        ("mac", Some(n)) if is_tail(n) => "mac tail",
                        ("mac", Some(n)) if n.text().to_string().ends_with('}') => "mac braces",
                        (k, _) => k,
                    };
                    *seen.entry(variant).or_insert(0) += 1;
                }
                let tail = green.last().is_some_and(|(k, ..)| *k == "expr");
                let key = match (green.is_empty(), tail) {
                    (true, _) => "empty block",
                    (false, true) => "block with tail",
                    (false, false) => "block without tail",
                };
                *seen.entry(key).or_insert(0) += 1;
            }
        }
        for variant in [
            "let",
            "item",
            "expr",
            "semi",
            "mac",
            "mac braces",
            "mac tail",
            "empty",
            "empty block",
            "block with tail",
            "block without tail",
        ] {
            assert!(
                seen.get(variant).is_some_and(|&n| n > 0),
                "{variant} occurs: {seen:?}"
            );
        }
    }

    #[test]
    fn block_from_block_expr_matches_the_generated_accessors() {
        let mut unsafe_blocks = 0;
        let mut blocks = 0;
        for src in BLOCK_SOURCES {
            let file = SourceFile::parse(src, Edition::Edition2024).tree();
            for b in file.syntax().descendants().filter_map(ast::BlockExpr::cast) {
                let block = Block::from_block_expr(&b).unwrap();
                let list = b.stmt_list().unwrap();
                let list_span = node_range_span(list.syntax());
                let (rules, span) = match b.unsafe_token() {
                    Some(t) => (BlockRules::Unsafe, list_span.with_lo(token_span(&t).lo())),
                    None => (BlockRules::Default, list_span),
                };
                assert_eq!(block.rules, rules, "{b}");
                assert_eq!(block.span, span, "{b}");
                assert_eq!(block.stmt_list, Some(list), "{b}");
                blocks += 1;
                unsafe_blocks += usize::from(rules == BlockRules::Unsafe);
            }
        }
        assert!(
            blocks > 20 && unsafe_blocks >= 3,
            "{blocks} blocks, {unsafe_blocks} unsafe"
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

    /// The red-cursor computation `inner_attributes` replaced.
    fn inner_attributes_by_cursor(container: &SyntaxNode) -> Vec<Attribute> {
        container
            .children_with_tokens()
            .filter_map(|child| match child {
                NodeOrToken::Token(t) if is_inner_doc_comment(&t) => Some(Attribute::Doc(t)),
                NodeOrToken::Node(n) => ast::Attr::cast(n)
                    .filter(|a| has_token(a.syntax(), T![!]))
                    .map(Attribute::Normal),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn inner_attributes_match_a_cursor_walk_on_every_node() {
        let mut seen = [0usize; 2];
        for src in ATTRIBUTE_SOURCES.iter().chain(BLOCK_SOURCES) {
            let file = SourceFile::parse(src, Edition::Edition2024).tree();
            for node in file.syntax().descendants() {
                let attrs = inner_attributes(&node);
                assert_eq!(
                    attrs,
                    inner_attributes_by_cursor(&node),
                    "{node:?} in {src:?}"
                );
                for attr in &attrs {
                    seen[usize::from(attr.is_doc_comment())] += 1;
                }
            }
        }
        assert!(seen.iter().all(|&n| n > 0), "normal and doc: {seen:?}");
    }

    /// The red-cursor computation `outer_attributes` replaced.
    fn outer_attributes_by_cursor(node: &SyntaxNode) -> Vec<Attribute> {
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
                    Some(attr) if attr.excl_token().is_none() => {
                        attrs.push(Attribute::Normal(attr))
                    }
                    Some(_) => {}
                    None => break,
                },
            }
        }
        attrs
    }

    /// `outer_attributes` answers what the cursor walk answers on every node of sources with
    /// doc comments, plain comments that look like doc comments, inner attributes before
    /// outer ones, attributes on statements, and nodes without attributes.
    #[test]
    fn outer_attributes_match_a_cursor_walk_on_every_node() {
        let mut seen = [0usize; 3];
        for src in ATTRIBUTE_SOURCES.iter().chain(BLOCK_SOURCES) {
            let file = SourceFile::parse(src, Edition::Edition2024).tree();
            for node in file.syntax().descendants() {
                let attrs = outer_attributes(&node);
                assert_eq!(
                    attrs,
                    outer_attributes_by_cursor(&node),
                    "{node:?} in {src:?}"
                );
                seen[2] += usize::from(attrs.is_empty() && has_leading_attrs(&node));
                for attr in &attrs {
                    seen[usize::from(attr.is_doc_comment())] += 1;
                }
            }
        }
        assert!(
            seen.iter().all(|&n| n > 0),
            "normal, doc, leading inner attributes only: {seen:?}"
        );
    }

    /// `is_inner_doc_text` on a comment's text answers what `is_inner_doc_comment` answers
    /// on the token, on every comment of the attribute sources.
    #[test]
    fn inner_doc_text_matches_the_comment_classification() {
        let mut comments = 0;
        for src in ATTRIBUTE_SOURCES {
            let file = SourceFile::parse(src, Edition::CURRENT).tree();
            for token in file
                .syntax()
                .descendants_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|t| t.kind() == SyntaxKind::COMMENT)
            {
                assert_eq!(
                    is_inner_doc_text(token.text()),
                    is_inner_doc_comment(&token),
                    "{token}"
                );
                assert_eq!(
                    is_outer_doc_text(token.text()),
                    is_outer_doc_comment(&token),
                    "{token}"
                );
                comments += 1;
            }
        }
        assert!(comments > 10, "{comments} comments");
    }

    #[test]
    fn inner_attributes_are_found() {
        let file = SourceFile::parse(ATTRIBUTE_SOURCES[0], Edition::CURRENT).tree();
        let attrs = inner_attributes(file.syntax());
        assert_eq!(attrs.len(), 3);
        assert!(attrs.iter().all(|a| a.style() == AttrStyle::Inner));
    }

    #[test]
    fn non_trivia_text_matches_a_cursor_walk_on_every_node() {
        for src in ATTRIBUTE_SOURCES {
            let file = SourceFile::parse(src, Edition::CURRENT).tree();
            for node in file.syntax().descendants() {
                let by_cursor: String = node
                    .descendants_with_tokens()
                    .filter_map(|e| e.into_token())
                    .filter(|t| !t.kind().is_trivia())
                    .map(|t| t.text().to_string())
                    .collect();
                assert_eq!(non_trivia_text(&node), by_cursor, "{node:?} in {src:?}");
            }
        }
    }

    /// `has_child` and `child_range` answer what the generated `support::child` accessors
    /// answer, for every node type they are used with, on every node of the sources.
    #[test]
    fn has_child_and_child_range_match_the_generated_child_accessors() {
        fn check<N: AstNode>(node: &SyntaxNode, seen: &mut usize) {
            let by_cursor = node.children().find_map(N::cast);
            assert_eq!(has_child::<N>(node), by_cursor.is_some(), "{node}");
            assert_eq!(
                child_range::<N>(node),
                by_cursor.as_ref().map(|n| n.syntax().text_range()),
                "{node}"
            );
            *seen += usize::from(by_cursor.is_some());
        }
        let mut seen = [0usize; 7];
        for src in BLOCK_SOURCES.iter().chain(ATTRIBUTE_SOURCES).chain(&[
            "fn f(a: u8, (b, c): (u8, u8), ...) {}\nfn g<T: X<U>>(x: T) -> T where T: Y { x }\n\
             macro_rules! m { () => {} }\nfn h() { m!(1); m![2]; m! { 3 } }\n\
             fn i() { match x { 1 | 2 if y => a, #[attr] _ => b } }\n",
        ]) {
            let file = SourceFile::parse(src, Edition::Edition2024).tree();
            for node in file.syntax().descendants() {
                check::<ast::Expr>(&node, &mut seen[0]);
                check::<ast::BlockExpr>(&node, &mut seen[1]);
                check::<ast::Type>(&node, &mut seen[2]);
                check::<ast::Pat>(&node, &mut seen[3]);
                check::<ast::GenericArg>(&node, &mut seen[4]);
                check::<ast::TokenTree>(&node, &mut seen[5]);
                check::<ast::Path>(&node, &mut seen[6]);
            }
        }
        assert!(seen.iter().all(|&n| n > 0), "every type found: {seen:?}");
    }

    /// `bin_op_kind` and `bin_op_text` answer what `BinExpr::op_kind` and `op_token` answer
    /// on every binary operator, compound assignment included.
    #[test]
    fn bin_op_matches_the_generated_operator_accessors() {
        let ops = [
            "||", "&&", "==", "!=", "<=", ">=", "<", ">", "+", "*", "-", "/", "%", "<<", ">>", "^",
            "|", "&", "=", "+=", "*=", "-=", "/=", "%=", "<<=", ">>=", "^=", "|=", "&=",
        ];
        let body: String = ops.iter().map(|op| format!("    a {op} b;\n")).collect();
        let src = format!("fn f() {{\n{body}    a + b * c - d;\n    a = b == c;\n}}\n");
        let file = SourceFile::parse(&src, Edition::Edition2024).tree();
        let mut seen = std::collections::BTreeSet::new();
        for b in file.syntax().descendants().filter_map(ast::BinExpr::cast) {
            assert_eq!(bin_op_kind(&b), b.op_kind(), "{b}");
            assert_eq!(
                bin_op_text(&b),
                b.op_token().map(|t| t.text().to_string()),
                "{b}"
            );
            seen.insert(bin_op_text(&b).unwrap());
        }
        assert_eq!(seen.len(), ops.len(), "every operator parsed: {seen:?}");
    }

    /// `attr_path_text` answers what `Meta::path` and `non_trivia_text` answer on every
    /// attribute of the attribute sources.
    #[test]
    fn attr_path_text_matches_the_generated_meta_accessors() {
        let mut attrs = 0;
        for src in ATTRIBUTE_SOURCES {
            let file = SourceFile::parse(src, Edition::CURRENT).tree();
            for attr in file.syntax().descendants().filter_map(ast::Attr::cast) {
                let by_cursor = attr
                    .meta()
                    .and_then(|m| m.path())
                    .map(|p| non_trivia_text(p.syntax()));
                assert_eq!(attr_path_text(&attr), by_cursor, "{attr}");
                attrs += 1;
            }
        }
        assert!(attrs > 10, "{attrs} attributes");
    }

    /// `nth_child` and the control-flow accessors answer what rust-analyzer's hand-written
    /// accessors answer on `if`, `while`, `for`, index and binary expressions, including the
    /// malformed forms (a block where a condition is expected, a missing body) that the
    /// accessors special-case.
    #[test]
    fn control_flow_accessors_match_the_generated_ones() {
        use ra_ap_syntax::ast::HasLoopBody;
        let src = "fn f() {\n    if a {} else {}\n    if a {} else if b {} else {}\n    if a {}\n    \
                   if let Some(x) = y {}\n    if {a} {}\n    if {}\n    if a\n    while a {}\n    \
                   while let Some(x) = y {}\n    while {a} {}\n    while {}\n    'l: while a {}\n    \
                   for x in y {}\n    for x in {y} {}\n    for x in {}\n    'l: for x in y {}\n    \
                   loop {}\n    'l: loop {}\n    a[b]; a[b][c]; a[{b}];\n    a + b; a = b + c; a +\n}\n";
        let file = SourceFile::parse(src, Edition::Edition2024).tree();
        let mut seen = [0usize; 9];
        for node in file.syntax().descendants() {
            if let Some(i) = ast::IfExpr::cast(node.clone()) {
                assert_eq!(condition_expr(&node), i.condition(), "{node}");
                assert_eq!(then_branch(&i), i.then_branch(), "{node}");
                let by_cursor = i.else_branch().map(|b| match b {
                    ast::ElseBranch::Block(b) => ast::Expr::BlockExpr(b),
                    ast::ElseBranch::IfExpr(i) => ast::Expr::IfExpr(i),
                });
                assert_eq!(else_branch(&i), by_cursor, "{node}");
                seen[0] += 1;
                seen[1] += usize::from(i.condition().is_none());
                seen[2] += usize::from(by_cursor.is_some());
            } else if let Some(w) = ast::WhileExpr::cast(node.clone()) {
                assert_eq!(condition_expr(&node), w.condition(), "{node}");
                assert_eq!(loop_body(&node), w.loop_body(), "{node}");
                seen[3] += 1;
                seen[4] += usize::from(w.condition().is_none());
            } else if let Some(f) = ast::ForExpr::cast(node.clone()) {
                assert_eq!(condition_expr(&node), f.iterable(), "{node}");
                assert_eq!(loop_body(&node), f.loop_body(), "{node}");
                seen[5] += 1;
                seen[6] += usize::from(f.iterable().is_none());
            } else if let Some(i) = ast::IndexExpr::cast(node.clone()) {
                assert_eq!(nth_child::<ast::Expr>(&node, 0), i.base(), "{node}");
                assert_eq!(nth_child::<ast::Expr>(&node, 1), i.index(), "{node}");
                seen[7] += 1;
            } else if let Some(b) = ast::BinExpr::cast(node.clone()) {
                assert_eq!(nth_child::<ast::Expr>(&node, 0), b.lhs(), "{node}");
                assert_eq!(nth_child::<ast::Expr>(&node, 1), b.rhs(), "{node}");
                assert_eq!(nth_child::<ast::Expr>(&node, 2), None, "{node}");
                seen[8] += 1;
            }
        }
        assert!(seen.iter().all(|&n| n > 0), "every form occurs: {seen:?}");
    }

    #[test]
    fn attribute_paths_drop_whitespace_and_comments() {
        let file = SourceFile::parse(ATTRIBUTE_SOURCES[7], Edition::CURRENT).tree();
        let item = file.items().next().unwrap();
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
        let file = SourceFile::parse(src, Edition::Edition2024).tree();
        let mut seen = [0usize; 7];
        for node in file.syntax().descendants() {
            if let Some(p) = ast::IdentPat::cast(node.clone()) {
                assert_eq!(has_token(&node, T![ref]), p.ref_token().is_some(), "{node}");
                assert_eq!(has_token(&node, T![mut]), p.mut_token().is_some(), "{node}");
                seen[0] += usize::from(p.ref_token().is_some());
                seen[1] += usize::from(p.mut_token().is_some());
            } else if let Some(r) = ast::RefExpr::cast(node.clone()) {
                assert_eq!(has_token(&node, T![raw]), r.raw_token().is_some(), "{node}");
                assert_eq!(has_token(&node, T![mut]), r.mut_token().is_some(), "{node}");
                seen[2] += usize::from(r.raw_token().is_some());
            } else if let Some(c) = ast::ClosureExpr::cast(node.clone()) {
                assert_eq!(
                    has_token(&node, T![async]),
                    c.async_token().is_some(),
                    "{node}"
                );
                assert_eq!(has_token(&node, T![gen]), c.gen_token().is_some(), "{node}");
                seen[3] += usize::from(c.async_token().is_some());
                seen[4] += usize::from(c.gen_token().is_some());
            } else if let Some(l) = ast::LetStmt::cast(node.clone()) {
                assert_eq!(
                    has_token(&node, T![super]),
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
