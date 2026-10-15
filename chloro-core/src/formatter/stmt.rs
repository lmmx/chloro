//! Statement rewriting (rustfmt's `stmt.rs`).

use crate::cst::ast::{self, AstNode};

use super::comment::recover_comment_removed;
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{ExprType, format_expr, is_simple_block};
use super::nodes::{self, Attribute, Block, StmtKind};
use super::shape::Shape;
use super::span::Span;

/// A statement together with whether it is the last one of its block.
pub(crate) struct Stmt<'a> {
    inner: nodes::Stmt<'a>,
    is_last: bool,
}

impl<'a> Stmt<'a> {
    pub(crate) fn span(&self) -> Span {
        self.inner.span
    }

    pub(crate) fn as_ast_node(&self) -> &nodes::Stmt<'_> {
        &self.inner
    }

    pub(crate) fn to_item(&self) -> Option<&ast::Item<'_>> {
        self.inner.as_item()
    }

    pub(crate) fn from_simple_block(
        context: &RewriteContext<'_>,
        block: &Block<'a>,
        attrs: Option<&[Attribute<'_>]>,
    ) -> Option<Self> {
        if is_simple_block(context, block, attrs) {
            // Simple blocks only contain one expr and no stmts
            Some(Stmt {
                inner: block.stmts[0],
                is_last: true,
            })
        } else {
            None
        }
    }

    pub(crate) fn from_ast_node(inner: &nodes::Stmt<'a>, is_last: bool) -> Self {
        Stmt {
            inner: *inner,
            is_last,
        }
    }

    pub(crate) fn from_ast_nodes(stmts: &[nodes::Stmt<'a>]) -> Vec<Self> {
        let len = stmts.len();
        stmts
            .iter()
            .enumerate()
            .map(|(i, s)| Stmt::from_ast_node(s, i + 1 == len))
            .collect()
    }

    /// A lower bound on the width of a one-line rewrite of the statement: the bound of its
    /// expression ([`super::width::one_line_min_width`]), 0 for any other statement. The
    /// rewrite is the expression's rewrite with an optional `;`, or the statement's source
    /// when a comment was lost, either of which contains the expression's tokens.
    pub(crate) fn one_line_min_width(&self) -> usize {
        match &self.inner.kind {
            StmtKind::Expr(e) | StmtKind::Semi(e) => super::width::one_line_min_width(e.syntax()),
            _ => 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        matches!(self.inner.kind, StmtKind::Empty)
    }

    fn is_last_expr(&self) -> bool {
        if !self.is_last {
            return false;
        }

        match &self.inner.kind {
            StmtKind::Expr(expr) => !matches!(
                expr,
                ast::Expr::ReturnExpr(..) | ast::Expr::ContinueExpr(..) | ast::Expr::BreakExpr(..)
            ),
            _ => false,
        }
    }
}

impl Rewrite for Stmt<'_> {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let expr_type =
            if context.config.style_edition() >= StyleEdition::Edition2024 && self.is_last_expr() {
                ExprType::SubExpression
            } else {
                ExprType::Statement
            };
        format_stmt(context, shape, &self.inner, expr_type, self.is_last_expr())
    }
}

fn semicolon_for_stmt(
    context: &RewriteContext<'_>,
    stmt: &nodes::Stmt,
    is_last_expr: bool,
) -> bool {
    match &stmt.kind {
        StmtKind::Semi(expr) => match expr {
            ast::Expr::WhileExpr(..) | ast::Expr::LoopExpr(..) | ast::Expr::ForExpr(..) => false,
            ast::Expr::BreakExpr(..) | ast::Expr::ContinueExpr(..) | ast::Expr::ReturnExpr(..) => {
                // The only time we can skip the semi-colon is if the config option is set to false
                // **and** this is the last expr (even though any following exprs are unreachable)
                context.config.trailing_semicolon() || !is_last_expr
            }
            _ => true,
        },
        StmtKind::Expr(..) => false,
        _ => true,
    }
}

fn format_stmt(
    context: &RewriteContext<'_>,
    shape: Shape,
    stmt: &nodes::Stmt,
    expr_type: ExprType,
    is_last_expr: bool,
) -> Option<String> {
    let result = match &stmt.kind {
        StmtKind::Let(local) => local.rewrite(context, shape),
        StmtKind::Expr(ex) | StmtKind::Semi(ex) => {
            let suffix = if semicolon_for_stmt(context, stmt, is_last_expr) {
                ";"
            } else {
                ""
            };

            let shape = shape.sub_width(suffix.len())?;
            format_expr(ex, expr_type, context, shape).map(|s| s + suffix)
        }
        StmtKind::MacCall(..) | StmtKind::Item(..) | StmtKind::Empty => None,
    };
    result.map(|res| recover_comment_removed(res, stmt.span, context))
}
