//! Statement rewriting (rustfmt's `stmt.rs`).

use ra_ap_syntax::ast;

use super::comment::recover_comment_removed;
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{ExprType, format_expr, is_simple_block};
use super::nodes::{self, Attribute, Block, StmtKind};
use super::shape::Shape;
use super::span::Span;

/// A statement together with whether it is the last one of its block.
pub(crate) struct Stmt {
    inner: nodes::Stmt,
    is_last: bool,
}

impl Stmt {
    pub(crate) fn span(&self) -> Span {
        self.inner.span
    }

    pub(crate) fn as_ast_node(&self) -> &nodes::Stmt {
        &self.inner
    }

    pub(crate) fn to_item(&self) -> Option<&ast::Item> {
        self.inner.as_item()
    }

    pub(crate) fn from_simple_block(
        context: &RewriteContext<'_>,
        block: &Block,
        attrs: Option<&[Attribute]>,
    ) -> Option<Self> {
        if is_simple_block(context, block, attrs) {
            // Simple blocks only contain one expr and no stmts
            Some(Stmt {
                inner: block.stmts[0].clone(),
                is_last: true,
            })
        } else {
            None
        }
    }

    pub(crate) fn from_ast_node(inner: &nodes::Stmt, is_last: bool) -> Self {
        Stmt {
            inner: inner.clone(),
            is_last,
        }
    }

    pub(crate) fn from_ast_nodes(stmts: &[nodes::Stmt]) -> Vec<Self> {
        let len = stmts.len();
        stmts
            .iter()
            .enumerate()
            .map(|(i, s)| Stmt::from_ast_node(s, i + 1 == len))
            .collect()
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

impl Rewrite for Stmt {
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
