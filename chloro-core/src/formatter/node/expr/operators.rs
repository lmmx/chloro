use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, RangeItem};

use super::try_format_expr_inner;

pub fn format_bin_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let bin = ast::BinExpr::cast(node.clone())?;
    let lhs = bin.lhs()?;
    let rhs = bin.rhs()?;
    let op = bin.op_token()?;

    let lhs_str = try_format_expr_inner(lhs.syntax(), indent)?;
    let rhs_str = try_format_expr_inner(rhs.syntax(), indent)?;

    Some(format!("{} {} {}", lhs_str, op.text(), rhs_str))
}

pub fn format_range_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let range = ast::RangeExpr::cast(node.clone())?;

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

    match (start, end) {
        (Some(s), Some(e)) => Some(format!("{}{}{}", s, op, e)),
        (Some(s), None) => Some(format!("{}{}", s, op)),
        (None, Some(e)) => Some(format!("{}{}", op, e)),
        (None, None) => Some(op.to_string()),
    }
}

pub fn format_cast_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let cast = ast::CastExpr::cast(node.clone())?;
    let expr = cast.expr()?;
    let ty = cast.ty()?;

    let expr_str = try_format_expr_inner(expr.syntax(), indent)?;
    Some(format!("{} as {}", expr_str, ty.syntax().text()))
}

pub fn format_field_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let field = ast::FieldExpr::cast(node.clone())?;
    let base = field.expr()?;
    let name = field.name_ref()?;

    let base_str = try_format_expr_inner(base.syntax(), indent)?;
    Some(format!("{}.{}", base_str, name.text()))
}
