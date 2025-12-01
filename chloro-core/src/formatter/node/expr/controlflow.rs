use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasLoopBody};

use super::try_format_expr_inner;
use crate::formatter::write_indent;

fn format_expr_attrs(node: &impl HasAttrs) -> String {
    let mut result = String::new();
    for attr in node.attrs() {
        result.push_str(&attr.syntax().text().to_string());
        result.push(' ');
    }
    result
}

/// Format a block expression body, returning the contents between braces.
fn format_block_contents(block: &ast::BlockExpr, indent: usize) -> String {
    let mut buf = String::new();

    if let Some(stmt_list) = block.stmt_list() {
        let stmts: Vec<_> = stmt_list.statements().collect();
        let tail = stmt_list.tail_expr();

        for stmt in &stmts {
            write_indent(&mut buf, indent);
            buf.push_str(&stmt.syntax().text().to_string());
            buf.push('\n');
        }

        if let Some(tail_expr) = tail {
            write_indent(&mut buf, indent);
            match try_format_expr_inner(tail_expr.syntax(), indent) {
                Some(s) => buf.push_str(&s),
                None => buf.push_str(&tail_expr.syntax().text().to_string()),
            }
            buf.push('\n');
        }
    }

    buf
}

/// Format a block expression including braces.
fn format_block_with_braces(block: &ast::BlockExpr, indent: usize) -> String {
    let mut buf = String::from("{\n");
    buf.push_str(&format_block_contents(block, indent + 4));
    write_indent(&mut buf, indent);
    buf.push('}');
    buf
}

pub fn format_if_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let if_expr = ast::IfExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&if_expr);

    let condition = if_expr.condition()?;
    let then_branch = if_expr.then_branch()?;

    let mut buf = String::new();
    buf.push_str(&attrs);
    buf.push_str("if ");

    // Format condition
    match try_format_expr_inner(condition.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&condition.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&then_branch, indent));

    // Handle else branch
    if let Some(else_branch) = if_expr.else_branch() {
        buf.push_str(" else ");
        match else_branch {
            ast::ElseBranch::IfExpr(else_if) => {
                // Recursive: else if ...
                match try_format_expr_inner(else_if.syntax(), indent) {
                    Some(s) => buf.push_str(&s),
                    None => buf.push_str(&else_if.syntax().text().to_string()),
                }
            }
            ast::ElseBranch::Block(else_block) => {
                buf.push_str(&format_block_with_braces(&else_block, indent));
            }
        }
    }

    Some(buf)
}

pub fn format_match_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let match_expr = ast::MatchExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&match_expr);

    let scrutinee = match_expr.expr()?;
    let arm_list = match_expr.match_arm_list()?;

    let mut buf = String::new();
    buf.push_str(&attrs);
    buf.push_str("match ");

    match try_format_expr_inner(scrutinee.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&scrutinee.syntax().text().to_string()),
    }

    buf.push_str(" {\n");

    for arm in arm_list.arms() {
        write_indent(&mut buf, indent + 4);

        // Arm attributes
        for attr in arm.attrs() {
            buf.push_str(&attr.syntax().text().to_string());
            buf.push(' ');
        }

        // Pattern
        if let Some(pat) = arm.pat() {
            buf.push_str(&pat.syntax().text().to_string());
        }

        // Guard
        if let Some(guard) = arm.guard() {
            buf.push_str(" if ");
            if let Some(cond) = guard.condition() {
                match try_format_expr_inner(cond.syntax(), indent + 4) {
                    Some(s) => buf.push_str(&s),
                    None => buf.push_str(&cond.syntax().text().to_string()),
                }
            }
        }

        buf.push_str(" => ");

        // Arm expression
        if let Some(expr) = arm.expr() {
            match try_format_expr_inner(expr.syntax(), indent + 4) {
                Some(s) => buf.push_str(&s),
                None => buf.push_str(&expr.syntax().text().to_string()),
            }
        }

        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push('}');

    Some(buf)
}

pub fn format_loop_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let loop_expr = ast::LoopExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&loop_expr);

    let body = loop_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = loop_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("loop ");
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_while_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let while_expr = ast::WhileExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&while_expr);

    let condition = while_expr.condition()?;
    let body = while_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = while_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("while ");

    match try_format_expr_inner(condition.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&condition.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_for_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let for_expr = ast::ForExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&for_expr);

    let pat = for_expr.pat()?;
    let iterable = for_expr.iterable()?;
    let body = for_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = for_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("for ");
    buf.push_str(&pat.syntax().text().to_string());
    buf.push_str(" in ");

    match try_format_expr_inner(iterable.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&iterable.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_block_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let block = ast::BlockExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&block);

    // Check for label
    let label = block.label();

    // Check for async/unsafe/try/const
    let is_async = block.async_token().is_some();
    let is_unsafe = block.unsafe_token().is_some();
    let is_try = block.try_token().is_some();
    let is_const = block.const_token().is_some();

    let mut buf = String::new();
    buf.push_str(&attrs);

    if let Some(label) = label {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    if is_try {
        buf.push_str("try ");
    }
    if is_const {
        buf.push_str("const ");
    }
    if is_async {
        buf.push_str("async ");
    }
    if is_unsafe {
        buf.push_str("unsafe ");
    }

    buf.push_str(&format_block_with_braces(&block, indent));

    Some(buf)
}

pub fn format_closure_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let closure = ast::ClosureExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&closure);

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Modifiers
    if let Some(for_binder) = closure.for_binder() {
        buf.push_str(&for_binder.syntax().text().to_string());
        buf.push(' ');
    }

    if closure.const_token().is_some() {
        buf.push_str("const ");
    }
    if closure.static_token().is_some() {
        buf.push_str("static ");
    }
    if closure.async_token().is_some() {
        buf.push_str("async ");
    }
    if closure.move_token().is_some() {
        buf.push_str("move ");
    }

    // Parameter list
    if let Some(param_list) = closure.param_list() {
        buf.push('|');
        let params: Vec<_> = param_list.params().collect();
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(&param.syntax().text().to_string());
        }
        buf.push('|');
    }

    // Return type
    if let Some(ret_type) = closure.ret_type() {
        buf.push(' ');
        buf.push_str(&ret_type.syntax().text().to_string());
    }

    // Body
    if let Some(body) = closure.body() {
        buf.push(' ');
        match try_format_expr_inner(body.syntax(), indent) {
            Some(s) => buf.push_str(&s),
            None => buf.push_str(&body.syntax().text().to_string()),
        }
    }

    Some(buf)
}
