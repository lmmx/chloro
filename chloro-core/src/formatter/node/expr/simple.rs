use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode};

use super::try_format_expr_inner;

pub fn format_paren_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let paren = ast::ParenExpr::cast(node.clone())?;
    let inner = paren.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("({})", formatted))
}

pub fn format_try_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let try_expr = ast::TryExpr::cast(node.clone())?;
    let inner = try_expr.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}?", formatted))
}

pub fn format_await_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let await_expr = ast::AwaitExpr::cast(node.clone())?;
    let inner = await_expr.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}.await", formatted))
}

pub fn format_ref_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let ref_expr = ast::RefExpr::cast(node.clone())?;
    let inner = ref_expr.expr()?;

    let prefix = if ref_expr.raw_token().is_some() {
        if ref_expr.mut_token().is_some() {
            "&raw mut "
        } else {
            "&raw const "
        }
    } else if ref_expr.mut_token().is_some() {
        "&mut "
    } else {
        "&"
    };

    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}{}", prefix, formatted))
}

pub fn format_prefix_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let prefix = ast::PrefixExpr::cast(node.clone())?;
    let inner = prefix.expr()?;
    let op = prefix.op_kind()?;

    let op_str = match op {
        ast::UnaryOp::Deref => "*",
        ast::UnaryOp::Not => "!",
        ast::UnaryOp::Neg => "-",
    };

    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}{}", op_str, formatted))
}
