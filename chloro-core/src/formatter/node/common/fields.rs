// chloro-core/src/formatter/node/common/fields.rs
use ra_ap_syntax::ast::{HasAttrs, HasDocComments, HasName, HasVisibility};
use ra_ap_syntax::{AstNode, AstToken, NodeOrToken, SyntaxKind, ast};

use crate::formatter::write_indent;

/// Collect non-doc comments from inside a node (before the name)
fn collect_inner_comments(node: &ra_ap_syntax::SyntaxNode) -> Vec<String> {
    let mut comments = Vec::new();

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments (handled by HasDocComments)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                }
            }
            NodeOrToken::Node(n) => {
                // Stop when we hit the NAME node
                if n.kind() == SyntaxKind::NAME {
                    break;
                }
            }
        }
    }

    comments
}

/// Format record fields with their comments (used by struct and enum variants).
pub fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    for field in fields.fields() {
        // Collect comments from inside the field node (before the name)
        let comments_before = collect_inner_comments(field.syntax());
        for comment in &comments_before {
            write_indent(buf, indent);
            buf.push_str(comment);
            buf.push('\n');
        }

        // Format field doc comments
        for comment in field.doc_comments() {
            let text = comment.text();
            write_indent(buf, indent);
            buf.push_str(text.trim());
            buf.push('\n');
        }

        // Format field attributes
        for attr in field.attrs() {
            write_indent(buf, indent);
            buf.push_str(&attr.syntax().text().to_string());
            buf.push('\n');
        }

        write_indent(buf, indent);
        if let Some(vis) = field.visibility() {
            buf.push_str(&vis.syntax().text().to_string());
            buf.push(' ');
        }
        if let Some(name) = field.name() {
            buf.push_str(&name.text());
        }
        buf.push_str(": ");
        if let Some(ty) = field.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }
        buf.push_str(",\n");
    }
}
