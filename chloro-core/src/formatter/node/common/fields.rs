use ra_ap_syntax::ast::{self, HasName};
use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind};

use crate::formatter::printer::Printer;

use super::comments;

/// Collect inner comments, excluding any that should be trailing for the previous field
fn collect_inner_comments_excluding_trailing(node: &ra_ap_syntax::SyntaxNode) -> Vec<String> {
    // If there's a newline before this node, all comments are leading (none are trailing for prev)
    let newline_before_node = comments::has_newline_before_node(node);
    let mut first_comment_skipped = false;

    node.children_with_tokens()
        .take_while(|child| !matches!(child, NodeOrToken::Node(n) if n.kind() == SyntaxKind::NAME))
        .filter_map(|child| match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                let text = t.text().to_string();
                if text.starts_with("///") || text.starts_with("//!") {
                    return None;
                }
                // Skip first comment if it's trailing for previous (no newline before node)
                if !newline_before_node && !first_comment_skipped {
                    first_comment_skipped = true;
                    return None;
                }
                Some(text)
            }
            _ => None,
        })
        .collect()
}

/// Check if the first non-doc comment in a field should be a trailing comment for the previous field
/// Returns (whitespace, comment) if found
fn get_trailing_comment_for_prev(node: &ra_ap_syntax::SyntaxNode) -> Option<(String, String)> {
    // If there's a newline before this node, any comment inside is a leading comment
    if comments::has_newline_before_node(node) {
        return None;
    }

    // Get the whitespace from the sibling before this node (after the comma)
    let whitespace_before = get_whitespace_before_node(node);

    // Check if first child is a comment (before any newline)
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        return Some((whitespace_before, text));
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE && t.text().contains('\n') {
                    return None;
                }
            }
            NodeOrToken::Node(n) if n.kind() == SyntaxKind::NAME => {
                return None;
            }
            _ => {}
        }
    }
    None
}

/// Get whitespace immediately before a node (from siblings)
fn get_whitespace_before_node(node: &ra_ap_syntax::SyntaxNode) -> String {
    let mut current = node.prev_sibling_or_token();

    while let Some(item) = current {
        match &item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    return t.text().to_string();
                } else if t.kind() == SyntaxKind::COMMA {
                    current = t.prev_sibling_or_token();
                    continue;
                } else {
                    return " ".to_string();
                }
            }
            NodeOrToken::Node(_) => return " ".to_string(),
        }
    }
    " ".to_string()
}

/// Format record fields with their comments.
pub fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    let field_list: Vec<_> = fields.fields().collect();

    for (idx, field) in field_list.iter().enumerate() {
        // Check if next field has a trailing comment for us
        let trailing_comment = if idx + 1 < field_list.len() {
            get_trailing_comment_for_prev(field_list[idx + 1].syntax())
        } else {
            // Last field - check siblings
            comments::get_trailing_comment_sibling(field.syntax())
        };

        // Collect comments excluding any trailing for previous
        for comment in collect_inner_comments_excluding_trailing(field.syntax()) {
            buf.line(indent, &comment);
        }
        buf.doc_comments(field, indent);
        buf.attrs(field, indent);
        buf.indent(indent);
        buf.visibility(field);
        if let Some(name) = field.name() {
            buf.push_str(&name.text());
        }
        buf.push_str(": ");
        if let Some(ty) = field.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }

        if let Some((ref whitespace, ref comment)) = trailing_comment {
            buf.push(',');
            buf.push_str(whitespace);
            buf.newline(comment);
        } else {
            buf.push_str(",\n");
        }
    }
}
