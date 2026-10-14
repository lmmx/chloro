//! The hand-written accessors of `ra_ap_syntax::ast` (`expr_ext.rs`, `node_ext.rs`,
//! `traits.rs`) that the formatter calls (audit table C), ported over the flat tree with the
//! same names and the same child-selection rules. Token-level helpers (`Comment::kind`,
//! `Literal::kind`, `IsString::is_raw`, `HasDocComments::doc_comments`) are not ported here.

#![allow(dead_code)]

use ra_ap_syntax::SyntaxKind::{self, *};
use ra_ap_syntax::T;

use super::ast::*;
use super::cst::{AstNode, Node, Token, support};

/// `expr_ext.rs`: the `else` branch of an `if`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElseBranch<'t> {
    Block(BlockExpr<'t>),
    IfExpr(IfExpr<'t>),
}

impl<'t> AstNode<'t> for ElseBranch<'t> {
    fn can_cast(kind: SyntaxKind) -> bool {
        BlockExpr::can_cast(kind) || IfExpr::can_cast(kind)
    }
    fn cast(syntax: Node<'t>) -> Option<Self> {
        if let Some(block) = BlockExpr::cast(syntax) {
            Some(Self::Block(block))
        } else {
            IfExpr::cast(syntax).map(Self::IfExpr)
        }
    }
    fn syntax(&self) -> Node<'t> {
        match self {
            ElseBranch::Block(b) => b.syntax(),
            ElseBranch::IfExpr(i) => i.syntax(),
        }
    }
}

impl<'t> IfExpr<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
    pub fn then_branch(&self) -> Option<BlockExpr<'t>> {
        match support::children::<Expr<'t>>(self.syntax()).nth(1)? {
            Expr::BlockExpr(block) => Some(block),
            _ => None,
        }
    }
    pub fn else_branch(&self) -> Option<ElseBranch<'t>> {
        match support::children::<Expr<'t>>(self.syntax()).nth(2)? {
            Expr::BlockExpr(block) => Some(ElseBranch::Block(block)),
            Expr::IfExpr(elif) => Some(ElseBranch::IfExpr(elif)),
            _ => None,
        }
    }
}

impl<'t> PrefixExpr<'t> {
    pub fn op_token(&self) -> Option<Token<'t>> {
        self.syntax().first_child_or_token()?.into_token()
    }
}

/// `expr_ext.rs`: the operator of a binary expression (the kind only; rust-analyzer's
/// `BinaryOp` value is a function of the kind).
pub fn is_bin_op(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        T![||]
            | T![&&]
            | T![==]
            | T![!=]
            | T![<=]
            | T![>=]
            | T![<]
            | T![>]
            | T![+]
            | T![*]
            | T![-]
            | T![/]
            | T![%]
            | T![<<]
            | T![>>]
            | T![^]
            | T![|]
            | T![&]
            | T![=]
            | T![+=]
            | T![*=]
            | T![-=]
            | T![/=]
            | T![%=]
            | T![<<=]
            | T![>>=]
            | T![^=]
            | T![|=]
            | T![&=]
    )
}

impl<'t> BinExpr<'t> {
    pub fn op_token(&self) -> Option<Token<'t>> {
        self.syntax()
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| is_bin_op(t.kind()))
    }
    pub fn op_kind(&self) -> Option<SyntaxKind> {
        self.op_token().map(|t| t.kind())
    }
    pub fn lhs(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).next()
    }
    pub fn rhs(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).nth(1)
    }
}

/// `ast.rs`: the parts of a range expression or pattern.
pub trait RangeItem<'t>: AstNode<'t> {
    type Bound: AstNode<'t>;
    fn op_token(&self) -> Option<Token<'t>>;
    fn start(&self) -> Option<Self::Bound>;
    fn end(&self) -> Option<Self::Bound>;
    fn op_kind(&self) -> Option<SyntaxKind> {
        self.op_token().map(|t| t.kind())
    }
}

fn range_op_details<'t>(node: Node<'t>) -> Option<(usize, Token<'t>)> {
    node.children_with_tokens()
        .enumerate()
        .find_map(|(ix, child)| {
            let token = child.into_token()?;
            matches!(token.kind(), T![..] | T![..=]).then_some((ix, token))
        })
}

impl<'t> RangeItem<'t> for RangeExpr<'t> {
    type Bound = Expr<'t>;
    fn op_token(&self) -> Option<Token<'t>> {
        range_op_details(self.syntax()).map(|t| t.1)
    }
    fn start(&self) -> Option<Expr<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .take(op_ix)
            .find_map(|it| Expr::cast(it.into_node()?))
    }
    fn end(&self) -> Option<Expr<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .skip(op_ix + 1)
            .find_map(|it| Expr::cast(it.into_node()?))
    }
}

impl<'t> RangeItem<'t> for RangePat<'t> {
    type Bound = Pat<'t>;
    fn op_token(&self) -> Option<Token<'t>> {
        range_op_details(self.syntax()).map(|t| t.1)
    }
    fn start(&self) -> Option<Pat<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .take(op_ix)
            .find_map(|it| Pat::cast(it.into_node()?))
    }
    fn end(&self) -> Option<Pat<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .skip(op_ix + 1)
            .find_map(|it| Pat::cast(it.into_node()?))
    }
}

impl<'t> IndexExpr<'t> {
    pub fn base(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).next()
    }
    pub fn index(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).nth(1)
    }
}

impl<'t> Literal<'t> {
    pub fn token(&self) -> Token<'t> {
        self.syntax()
            .children_with_tokens()
            .find(|e| e.kind() != ATTR && !e.kind().is_trivia())
            .and_then(|e| e.into_token())
            .unwrap()
    }
}

impl<'t> ClosureExpr<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}
impl<'t> Const<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}
impl<'t> Fn<'t> {
    pub fn body(&self) -> Option<BlockExpr<'t>> {
        support::child(self.syntax())
    }
}
impl<'t> Static<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}
impl<'t> ArgList<'t> {
    pub fn args(&self) -> super::cst::AstChildren<'t, Expr<'t>> {
        support::children(self.syntax())
    }
}
impl<'t> MacroDef<'t> {
    pub fn args(&self) -> Option<TokenTree<'t>> {
        support::children(self.syntax()).next()
    }
    pub fn body(&self) -> Option<TokenTree<'t>> {
        support::children(self.syntax()).nth(1)
    }
}
impl<'t> MatchGuard<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> BlockExpr<'t> {
    pub fn statements(&self) -> impl Iterator<Item = Stmt<'t>> + 't {
        self.stmt_list().into_iter().flat_map(|it| it.statements())
    }
    pub fn tail_expr(&self) -> Option<Expr<'t>> {
        self.stmt_list()?.tail_expr()
    }
}

impl<'t> ForExpr<'t> {
    pub fn iterable(&self) -> Option<Expr<'t>> {
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
}
impl<'t> HasLoopBody<'t> for ForExpr<'t> {
    fn loop_body(&self) -> Option<BlockExpr<'t>> {
        let mut exprs = support::children::<BlockExpr<'t>>(self.syntax());
        let first = exprs.next();
        let second = exprs.next();
        second.or(first)
    }
}
impl<'t> WhileExpr<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
}
impl<'t> HasLoopBody<'t> for WhileExpr<'t> {
    fn loop_body(&self) -> Option<BlockExpr<'t>> {
        let mut exprs = support::children::<BlockExpr<'t>>(self.syntax());
        let first = exprs.next();
        let second = exprs.next();
        second.or(first)
    }
}

impl<'t> Impl<'t> {
    pub fn self_ty(&self) -> Option<Type<'t>> {
        match self.target() {
            (Some(t), None) | (_, Some(t)) => Some(t),
            _ => None,
        }
    }
    pub fn trait_(&self) -> Option<Type<'t>> {
        match self.target() {
            (Some(t), Some(_)) => Some(t),
            _ => None,
        }
    }
    fn target(&self) -> (Option<Type<'t>>, Option<Type<'t>>) {
        let mut types = support::children(self.syntax());
        let first = types.next();
        let second = types.next();
        (first, second)
    }
}

impl<'t> Abi<'t> {
    /// The string literal token of `extern "C"`.
    pub fn abi_string(&self) -> Option<Token<'t>> {
        self.string_token()
    }
}

impl<'t> NameRef<'t> {
    pub fn text(&self) -> &'t str {
        self.syntax().text()
    }
}
impl<'t> Name<'t> {
    pub fn text(&self) -> &'t str {
        self.syntax().text()
    }
}

impl<'t> Path<'t> {
    pub fn segments(&self) -> impl Iterator<Item = PathSegment<'t>> + 't {
        let first = self.first_segment();
        let path = *self;
        std::iter::successors(first, move |p| {
            // The next segment after `p` is the segment of the path whose qualifier is p's
            // path; found by walking from the top path down, as rust-analyzer does with
            // `parent_path`.
            let mut cur = path;
            let mut last = None;
            while let Some(q) = cur.qualifier() {
                if q.segment().map(|s| s.syntax()) == Some(p.syntax()) {
                    last = cur.segment();
                    break;
                }
                cur = q;
            }
            last
        })
    }
    pub fn first_segment(&self) -> Option<PathSegment<'t>> {
        let mut cur = *self;
        while let Some(q) = cur.qualifier() {
            cur = q;
        }
        cur.segment()
    }
}
