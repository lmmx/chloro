use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind, SyntaxNode, ast};

use super::try_format_record_expr;
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
                            continue;
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push_str(";\n");
                        prev_was_item = true;
                    }

                    SyntaxKind::RECORD_EXPR if is_last_node => {
                        // Tail expression record - format without semicolon
                        write_indent(buf, indent);
                        if let Some(record_expr) = ast::RecordExpr::cast(n.clone())
                            && try_format_record_expr(&record_expr, buf, indent)
                        {
                            buf.push('\n');
                            prev_was_item = true;
                            continue;
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                        prev_was_item = true;
                    }

                    _ => {
                        // Everything else: preserve exactly as-is
                        write_indent(buf, indent);
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                        prev_was_item = true;
                    }
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::COMMENT => {
                    write_indent(buf, indent);
                    buf.push_str(t.text());
                    buf.push('\n');
                    prev_was_item = true;
                }
                SyntaxKind::WHITESPACE => continue,
                _ => {}
            },
        }
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
