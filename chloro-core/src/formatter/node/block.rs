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

    for (idx, child) in children.into_iter().enumerate() {
        match child {
            NodeOrToken::Node(n) => {
                let is_last_node = Some(idx) == last_node_idx;

                match n.kind() {
                    SyntaxKind::WHITESPACE => continue,

                    SyntaxKind::RECORD_EXPR if !is_last_node => {
                        // Non-tail record expression - try to format, fall back to default
                        write_indent(buf, indent);
                        if let Some(record_expr) = ast::RecordExpr::cast(n.clone()) {
                            if try_format_record_expr(&record_expr, buf, indent) {
                                buf.push_str(";\n");
                                continue;
                            }
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push_str(";\n");
                    }

                    SyntaxKind::RECORD_EXPR if is_last_node => {
                        // Tail expression record - format without semicolon
                        write_indent(buf, indent);
                        if let Some(record_expr) = ast::RecordExpr::cast(n.clone()) {
                            if try_format_record_expr(&record_expr, buf, indent) {
                                buf.push('\n');
                                continue;
                            }
                        }
                        // Fall through to default
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                    }

                    _ => {
                        // Everything else: preserve exactly as-is
                        write_indent(buf, indent);
                        buf.push_str(&n.text().to_string());
                        buf.push('\n');
                    }
                }
            }
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
