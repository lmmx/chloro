use crate::formatter::printer::expr_attrs_prefix;
use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, RangeItem};

use super::try_format_expr_inner;

pub fn format_bin_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let bin = ast::BinExpr::cast(node.clone())?;
    let lhs = try_format_expr_inner(bin.lhs()?.syntax(), indent)?;
    let rhs = try_format_expr_inner(bin.rhs()?.syntax(), indent)?;
    Some(format!(
        "{}{} {} {}",
        expr_attrs_prefix(&bin),
        lhs,
        bin.op_token()?.text(),
        rhs
    ))
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
    Some(format!("{}{}", expr_attrs_prefix(&range), range_str))
}

pub fn format_cast_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let cast = ast::CastExpr::cast(node.clone())?;
    let expr = try_format_expr_inner(cast.expr()?.syntax(), indent)?;
    Some(format!(
        "{}{} as {}",
        expr_attrs_prefix(&cast),
        expr,
        cast.ty()?.syntax().text()
    ))
}

pub fn format_field_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let field = ast::FieldExpr::cast(node.clone())?;
    let base = try_format_expr_inner(field.expr()?.syntax(), indent)?;
    Some(format!(
        "{}{}.{}",
        expr_attrs_prefix(&field),
        base,
        field.name_ref()?.text()
    ))
}
