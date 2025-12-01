use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode};

use super::try_format_expr_inner;

pub fn format_return_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let ret = ast::ReturnExpr::cast(node.clone())?;
    Some(match ret.expr() {
        Some(e) => format!("return {}", try_format_expr_inner(e.syntax(), indent)?),
        None => "return".into(),
    })
}

pub fn format_break_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let brk = ast::BreakExpr::cast(node.clone())?;
    let mut result = "break".to_string();
    if let Some(lt) = brk.lifetime() {
        result.push(' ');
        result.push_str(lt.text().as_ref());
    }
    if let Some(e) = brk.expr() {
        result.push(' ');
        result.push_str(&try_format_expr_inner(e.syntax(), indent)?);
    }
    Some(result)
}

pub fn format_continue_expr(node: &SyntaxNode, _indent: usize) -> Option<String> {
    let cont = ast::ContinueExpr::cast(node.clone())?;
    Some(match cont.lifetime() {
        Some(lt) => format!("continue {}", lt.text()),
        None => "continue".into(),
    })
}

pub fn format_yield_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let yld = ast::YieldExpr::cast(node.clone())?;
    Some(match yld.expr() {
        Some(e) => format!("yield {}", try_format_expr_inner(e.syntax(), indent)?),
        None => "yield".into(),
    })
}

pub fn format_yeet_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let yeet = ast::YeetExpr::cast(node.clone())?;
    Some(match yeet.expr() {
        Some(e) => format!("do yeet {}", try_format_expr_inner(e.syntax(), indent)?),
        None => "do yeet".into(),
    })
}

pub fn format_become_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let become_expr = ast::BecomeExpr::cast(node.clone())?;
    Some(format!(
        "become {}",
        try_format_expr_inner(become_expr.expr()?.syntax(), indent)?
    ))
}

pub fn format_let_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let let_expr = ast::LetExpr::cast(node.clone())?;
    let expr_str = try_format_expr_inner(let_expr.expr()?.syntax(), indent)?;
    Some(format!(
        "let {} = {}",
        let_expr.pat()?.syntax().text(),
        expr_str
    ))
}
