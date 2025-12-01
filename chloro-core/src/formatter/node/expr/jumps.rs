use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode};

use super::try_format_expr_inner;

pub fn format_return_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let ret = ast::ReturnExpr::cast(node.clone())?;

    match ret.expr() {
        Some(expr) => {
            let formatted = try_format_expr_inner(expr.syntax(), indent)?;
            Some(format!("return {}", formatted))
        }
        None => Some("return".to_string()),
    }
}

pub fn format_break_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let brk = ast::BreakExpr::cast(node.clone())?;

    let mut result = "break".to_string();

    if let Some(lifetime) = brk.lifetime() {
        result.push(' ');
        result.push_str(&lifetime.text().to_string());
    }

    if let Some(expr) = brk.expr() {
        let formatted = try_format_expr_inner(expr.syntax(), indent)?;
        result.push(' ');
        result.push_str(&formatted);
    }

    Some(result)
}

pub fn format_continue_expr(node: &SyntaxNode, _indent: usize) -> Option<String> {
    let cont = ast::ContinueExpr::cast(node.clone())?;

    match cont.lifetime() {
        Some(lifetime) => Some(format!("continue {}", lifetime.text())),
        None => Some("continue".to_string()),
    }
}

pub fn format_yield_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let yld = ast::YieldExpr::cast(node.clone())?;

    match yld.expr() {
        Some(expr) => {
            let formatted = try_format_expr_inner(expr.syntax(), indent)?;
            Some(format!("yield {}", formatted))
        }
        None => Some("yield".to_string()),
    }
}

pub fn format_yeet_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let yeet = ast::YeetExpr::cast(node.clone())?;

    match yeet.expr() {
        Some(expr) => {
            let formatted = try_format_expr_inner(expr.syntax(), indent)?;
            Some(format!("do yeet {}", formatted))
        }
        None => Some("do yeet".to_string()),
    }
}

pub fn format_become_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let become_expr = ast::BecomeExpr::cast(node.clone())?;
    let expr = become_expr.expr()?;
    let formatted = try_format_expr_inner(expr.syntax(), indent)?;
    Some(format!("become {}", formatted))
}

pub fn format_let_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let let_expr = ast::LetExpr::cast(node.clone())?;
    let pat = let_expr.pat()?;
    let expr = let_expr.expr()?;

    let expr_str = try_format_expr_inner(expr.syntax(), indent)?;

    // Pattern formatting is its own beast; preserve verbatim for now
    Some(format!("let {} = {}", pat.syntax().text(), expr_str))
}
