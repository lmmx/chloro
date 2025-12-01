use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, RangeItem};

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

pub fn format_bin_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let bin = ast::BinExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&bin);
    let lhs = bin.lhs()?;
    let rhs = bin.rhs()?;
    let op = bin.op_token()?;

    let lhs_str = try_format_expr_inner(lhs.syntax(), indent)?;
    let rhs_str = try_format_expr_inner(rhs.syntax(), indent)?;

    Some(format!("{}{} {} {}", attrs, lhs_str, op.text(), rhs_str))
}

pub fn format_range_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let range = ast::RangeExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&range);

    let start = match range.start() {
        Some(e) => Some(try_format_expr_inner(e.syntax(), indent)?),
        None => None,
    };
    let end = match range.end() {
        Some(e) => Some(try_format_expr_inner(e.syntax(), indent)?),
        None => None,
    };

    // Determine operator: .. or ..=
    let op = if range.op_token().is_some_and(|t| t.text() == "..=") {
        "..="
    } else {
        ".."
    };

    let range_str = match (start, end) {
        (Some(s), Some(e)) => format!("{}{}{}", s, op, e),
        (Some(s), None) => format!("{}{}", s, op),
        (None, Some(e)) => format!("{}{}", op, e),
        (None, None) => op.to_string(),
    };

    Some(format!("{}{}", attrs, range_str))
}

pub fn format_cast_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let cast = ast::CastExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&cast);
    let expr = cast.expr()?;
    let ty = cast.ty()?;

    let expr_str = try_format_expr_inner(expr.syntax(), indent)?;
    Some(format!("{}{} as {}", attrs, expr_str, ty.syntax().text()))
}

pub fn format_field_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let field = ast::FieldExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&field);
    let base = field.expr()?;
    let name = field.name_ref()?;

    let base_str = try_format_expr_inner(base.syntax(), indent)?;
    Some(format!("{}{}.{}", attrs, base_str, name.text()))
}
