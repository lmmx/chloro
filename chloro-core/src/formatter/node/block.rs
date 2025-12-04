use crate::formatter::write_indent;
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};

use super::expr::{FormatResult, try_format_expr};

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
                // Check for blank line before this node
                let is_last_node = Some(idx) == last_node_idx;
                if (prev_was_item || prev_was_comment)
                    && should_have_blank_line_before(&children, idx)
                {
                    buf.push('\n');
                }
                match n.kind() {
                    SyntaxKind::WHITESPACE => continue,

                    _ => {
                        // Try to format the expression; fall back to verbatim if unsupported
                        write_indent(buf, indent);
                        match try_format_expr(n, indent) {
                            FormatResult::Formatted(s) => {
                                buf.push_str(&s);
                                if !is_last_node {
                                    buf.push(';');
                                }
                                buf.push('\n');
                            }
                            FormatResult::Unsupported => {
                                buf.push_str(&n.text().to_string());
                                buf.push('\n');
                            }
                        }
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
