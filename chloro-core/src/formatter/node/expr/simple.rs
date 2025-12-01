use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs};

use super::try_format_expr_inner;

/// Format attributes that may precede an expression.
fn format_expr_attrs(node: &impl HasAttrs) -> String {
    let mut result = String::new();
    for attr in node.attrs() {
        result.push_str(&attr.syntax().text().to_string());
        result.push(' ');
    }
    result
}

pub fn format_paren_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let paren = ast::ParenExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&paren);
    let inner = paren.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}({})", attrs, formatted))
}

pub fn format_try_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let try_expr = ast::TryExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&try_expr);
    let inner = try_expr.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}{}?", attrs, formatted))
}

pub fn format_await_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let await_expr = ast::AwaitExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&await_expr);
    let inner = await_expr.expr()?;
    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}{}.await", attrs, formatted))
}

pub fn format_ref_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let ref_expr = ast::RefExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&ref_expr);
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
    Some(format!("{}{}{}", attrs, prefix, formatted))
}

pub fn format_prefix_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let prefix = ast::PrefixExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&prefix);
    let inner = prefix.expr()?;
    let op = prefix.op_kind()?;

    let op_str = match op {
        ast::UnaryOp::Deref => "*",
        ast::UnaryOp::Not => "!",
        ast::UnaryOp::Neg => "-",
    };

    let formatted = try_format_expr_inner(inner.syntax(), indent)?;
    Some(format!("{}{}{}", attrs, op_str, formatted))
}
