use crate::formatter::printer::expr_attrs_prefix;
use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode};

use super::try_format_expr_inner;

pub fn format_paren_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let paren = ast::ParenExpr::cast(node.clone())?;
    let inner = try_format_expr_inner(paren.expr()?.syntax(), indent)?;
    Some(format!("{}({})", expr_attrs_prefix(&paren), inner))
}

pub fn format_try_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let try_expr = ast::TryExpr::cast(node.clone())?;
    let inner = try_format_expr_inner(try_expr.expr()?.syntax(), indent)?;
    Some(format!("{}{}?", expr_attrs_prefix(&try_expr), inner))
}

pub fn format_await_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let await_expr = ast::AwaitExpr::cast(node.clone())?;
    let inner = try_format_expr_inner(await_expr.expr()?.syntax(), indent)?;
    Some(format!("{}{}.await", expr_attrs_prefix(&await_expr), inner))
}

pub fn format_ref_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let ref_expr = ast::RefExpr::cast(node.clone())?;
    let prefix = match (
        ref_expr.raw_token().is_some(),
        ref_expr.mut_token().is_some(),
    ) {
        (true, true) => "&raw mut ",
        (true, false) => "&raw const ",
        (false, true) => "&mut ",
        (false, false) => "&",
    };
    let inner = try_format_expr_inner(ref_expr.expr()?.syntax(), indent)?;
    Some(format!(
        "{}{}{}",
        expr_attrs_prefix(&ref_expr),
        prefix,
        inner
    ))
}

pub fn format_prefix_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let prefix = ast::PrefixExpr::cast(node.clone())?;
    let op = match prefix.op_kind()? {
        ast::UnaryOp::Deref => "*",
        ast::UnaryOp::Not => "!",
        ast::UnaryOp::Neg => "-",
    };
    let inner = try_format_expr_inner(prefix.expr()?.syntax(), indent)?;
    Some(format!("{}{}{}", expr_attrs_prefix(&prefix), op, inner))
}
