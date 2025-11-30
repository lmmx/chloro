// chloro-core/src/formatter/node/block.rs
use ra_ap_syntax::ast::HasArgList;
use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind, SyntaxNode, ast};

use super::try_format_record_expr;
use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

pub fn format_block(node: &SyntaxNode, buf: &mut String, indent: usize) {
    buf.push_str("{\n");
    format_block_expr_contents(node, buf, indent + 4);
    write_indent(buf, indent);
    buf.push('}');
}

pub fn format_stmt_list(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let children: Vec<_> = node.children_with_tokens().collect();
    let last_node_idx = children
        .iter()
        .rposition(|child| matches!(child, NodeOrToken::Node(_)));

    let mut prev_was_item = false;
    let mut prev_was_comment = false;

    for (idx, child) in children.iter().enumerate() {
        match child {
            NodeOrToken::Node(n) => {
                let is_last_node = Some(idx) == last_node_idx;

                // Check for blank line before this node
                if prev_was_item && should_have_blank_line_before(&children, idx) {
                    buf.push('\n');
                }

                match n.kind() {
                    SyntaxKind::WHITESPACE => continue,

                    SyntaxKind::RECORD_EXPR if !is_last_node => {
                        // Non-tail record expression - try to format, fall back to default
                        write_indent(buf, indent);
                        if let Some(record_expr) = ast::RecordExpr::cast(n.clone())
                            && try_format_record_expr(&record_expr, buf, indent)
                        {
                            buf.push_str(";\n");
                            prev_was_item = true;
                            prev_was_comment = false;
                            continue;
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push_str(";\n");
                        prev_was_item = true;
                        prev_was_comment = false;
                    }

                    SyntaxKind::RECORD_EXPR if is_last_node => {
                        // Tail expression record - format without semicolon
                        write_indent(buf, indent);
                        if let Some(record_expr) = ast::RecordExpr::cast(n.clone())
                            && try_format_record_expr(&record_expr, buf, indent)
                        {
                            buf.push('\n');
                            prev_was_item = true;
                            prev_was_comment = false;
                            continue;
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                        prev_was_item = true;
                        prev_was_comment = false;
                    }

                    SyntaxKind::EXPR_STMT => {
                        write_indent(buf, indent);
                        if let Some(expr_stmt) = ast::ExprStmt::cast(n.clone()) {
                            if let Some(expr) = expr_stmt.expr() {
                                format_expr(&expr, buf, indent);
                            } else {
                                buf.push_str(&n.text().to_string().trim_end());
                            }
                            if expr_stmt.semicolon_token().is_some() {
                                buf.push(';');
                            }
                        } else {
                            buf.push_str(&n.text().to_string().trim_end());
                        }
                        buf.push('\n');
                        prev_was_item = true;
                        prev_was_comment = false;
                    }

                    SyntaxKind::LET_STMT => {
                        write_indent(buf, indent);
                        if let Some(let_stmt) = ast::LetStmt::cast(n.clone()) {
                            format_let_stmt(&let_stmt, buf, indent);
                        } else {
                            buf.push_str(&n.text().to_string().trim_end());
                        }
                        buf.push('\n');
                        prev_was_item = true;
                        prev_was_comment = false;
                    }

                    _ => {
                        // Everything else: preserve exactly as-is
                        write_indent(buf, indent);
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                        prev_was_item = true;
                        prev_was_comment = false;
                    }
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::COMMENT => {
                    // Check for blank line before comment
                    if (prev_was_item || prev_was_comment)
                        && should_have_blank_line_before(&children, idx)
                    {
                        buf.push('\n');
                    }
                    write_indent(buf, indent);
                    buf.push_str(t.text());
                    buf.push('\n');
                    prev_was_item = false;
                    prev_was_comment = true;
                }
                SyntaxKind::WHITESPACE => continue,
                _ => {}
            },
        }
    }
}

/// Format an expression, handling method chains and other complex expressions
fn format_expr(expr: &ast::Expr, buf: &mut String, indent: usize) {
    match expr {
        ast::Expr::MethodCallExpr(method_call) => {
            format_method_call_chain(method_call, buf, indent);
        }
        _ => {
            buf.push_str(&expr.syntax().text().to_string().trim_end());
        }
    }
}

/// Format a method call chain, breaking across lines if needed
fn format_method_call_chain(method_call: &ast::MethodCallExpr, buf: &mut String, indent: usize) {
    // Collect all segments of the chain
    let mut segments: Vec<MethodSegment> = Vec::new();
    let mut current: ast::Expr = ast::Expr::MethodCallExpr(method_call.clone());
    let mut receiver_text = String::new();

    loop {
        match &current {
            ast::Expr::MethodCallExpr(mc) => {
                let method_name = mc
                    .name_ref()
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                let args = mc
                    .arg_list()
                    .map(|a| a.syntax().text().to_string())
                    .unwrap_or_else(|| "()".to_string());

                segments.push(MethodSegment { method_name, args });

                if let Some(receiver) = mc.receiver() {
                    current = receiver;
                } else {
                    break;
                }
            }
            _ => {
                receiver_text = current.syntax().text().to_string();
                break;
            }
        }
    }

    segments.reverse();

    // Calculate single-line length
    let single_line = format_chain_single_line(&receiver_text, &segments);
    let line_len = indent + single_line.len();

    if line_len <= MAX_WIDTH {
        buf.push_str(&single_line);
    } else {
        // Multi-line format
        buf.push_str(&receiver_text);
        for segment in &segments {
            buf.push('\n');
            write_indent(buf, indent + 4);
            buf.push('.');
            buf.push_str(&segment.method_name);
            buf.push_str(&segment.args);
        }
    }
}

struct MethodSegment {
    method_name: String,
    args: String,
}

fn format_chain_single_line(receiver: &str, segments: &[MethodSegment]) -> String {
    let mut result = receiver.to_string();
    for segment in segments {
        result.push('.');
        result.push_str(&segment.method_name);
        result.push_str(&segment.args);
    }
    result
}

/// Format a let statement
fn format_let_stmt(let_stmt: &ast::LetStmt, buf: &mut String, indent: usize) {
    // If this is a let-else statement, preserve it exactly as-is for now
    // since let-else has complex formatting requirements
    if let_stmt.let_else().is_some() {
        buf.push_str(&let_stmt.syntax().text().to_string().trim_end());
        return;
    }

    buf.push_str("let ");

    if let Some(pat) = let_stmt.pat() {
        buf.push_str(&pat.syntax().text().to_string());
    }

    if let Some(ty) = let_stmt.ty() {
        buf.push_str(": ");
        buf.push_str(&ty.syntax().text().to_string());
    }

    if let Some(init) = let_stmt.initializer() {
        buf.push_str(" = ");
        format_expr(&init, buf, indent);
    }

    if let_stmt.semicolon_token().is_some() {
        buf.push(';');
    }
}

/// Check if there should be a blank line before the item at the given index
fn should_have_blank_line_before(
    children: &[NodeOrToken<SyntaxNode, ra_ap_syntax::SyntaxToken>],
    idx: usize,
) -> bool {
    // Look backwards for whitespace with 2+ newlines
    for i in (0..idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    break;
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }
    false
}

pub fn format_block_expr_contents(node: &SyntaxNode, buf: &mut String, indent: usize) {
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Node(n) => match n.kind() {
                SyntaxKind::STMT_LIST => format_stmt_list(&n, buf, indent),
                SyntaxKind::WHITESPACE => continue,
                _ => {
                    write_indent(buf, indent);
                    buf.push_str(&n.text().to_string());
                    buf.push('\n');
                }
            },
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::COMMENT => {
                    write_indent(buf, indent);
                    buf.push_str(t.text());
                    buf.push('\n');
                }
                SyntaxKind::WHITESPACE => continue,
                _ => {}
            },
        }
    }
}
