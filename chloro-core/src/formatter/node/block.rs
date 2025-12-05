// In chloro-core/src/formatter/node/block.rs

use crate::formatter::write_indent;
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};

use super::expr::{FormatResult, try_format_expr};

pub fn format_block(node: &SyntaxNode, buf: &mut String, indent: usize) {
    buf.push_str("{\n");
    format_block_expr_contents(node, buf, indent + 4);
    write_indent(buf, indent);
    buf.push('}');
}

/// An item in a statement list with its associated preceding comments and blank line info
struct StmtWithComments {
    comments: Vec<String>,
    node: SyntaxNode,
    blank_line_before: bool,
    is_last: bool,
}

pub fn format_stmt_list(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let children: Vec<_> = node.children_with_tokens().collect();

    // Collect all statement nodes with their preceding comments and blank line info
    let mut items: Vec<StmtWithComments> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();
    let mut pending_blank_line = false;
    let mut standalone_comments: Vec<(Vec<String>, bool)> = Vec::new(); // (comments, blank_line_before)

    // First pass: identify all nodes and their indices
    let node_indices: Vec<usize> = children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            NodeOrToken::Node(_) => Some(i),
            _ => None,
        })
        .collect();

    let last_node_idx = node_indices.last().copied();

    for (idx, child) in children.iter().enumerate() {
        match child {
            NodeOrToken::Node(n) => {
                let is_last = Some(idx) == last_node_idx;
                items.push(StmtWithComments {
                    comments: std::mem::take(&mut pending_comments),
                    node: n.clone(),
                    blank_line_before: pending_blank_line,
                    is_last,
                });
                pending_blank_line = false;
            }
            NodeOrToken::Token(t) => match t.kind() {
                SyntaxKind::COMMENT => {
                    pending_comments.push(t.text().to_string());
                }
                SyntaxKind::WHITESPACE => {
                    if t.text().matches('\n').count() >= 2 {
                        pending_blank_line = true;
                    }
                }
                _ => {}
            },
        }
    }

    // Handle any trailing comments (not attached to a node)
    if !pending_comments.is_empty() {
        standalone_comments.push((std::mem::take(&mut pending_comments), pending_blank_line));
    }

    // Output items
    let mut prev_was_item = false;

    for item in items {
        // Add blank line if needed
        if prev_was_item && item.blank_line_before {
            buf.push('\n');
        }

        // Output comments
        for comment in &item.comments {
            write_indent(buf, indent);
            buf.push_str(comment);
            buf.push('\n');
        }

        // Output the statement
        write_indent(buf, indent);
        match try_format_expr(&item.node, indent) {
            FormatResult::Formatted(s) => {
                buf.push_str(&s);
                if !item.is_last {
                    buf.push(';');
                }
                buf.push('\n');
            }
            FormatResult::Unsupported => {
                buf.push_str(&item.node.text().to_string());
                buf.push('\n');
            }
        }

        prev_was_item = true;
    }

    // Output any trailing standalone comments
    for (comments, blank_before) in standalone_comments {
        if prev_was_item && blank_before {
            buf.push('\n');
        }
        for comment in comments {
            write_indent(buf, indent);
            buf.push_str(&comment);
            buf.push('\n');
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
