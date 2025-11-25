// src/formatter/node/fields.rs
use ra_ap_syntax::ast::{HasAttrs, HasDocComments, HasName, HasVisibility};
use ra_ap_syntax::{AstNode, AstToken, NodeOrToken, SyntaxKind, ast};

use crate::formatter::node::common::comments;
use crate::formatter::write_indent;

/// Format record fields with their comments (used by struct and enum variants).
pub fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    let children: Vec<_> = fields.syntax().children_with_tokens().collect();

    for (idx, child) in children.iter().enumerate() {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                // Check if this comment is attached to the next field
                if !comments::is_comment_attached_to_next_field(&children, idx) {
                    // Preserve blank line spacing before standalone comment
                    if comments::should_have_blank_line_before_comment(&children, idx) {
                        buf.push('\n');
                    }
                    write_indent(buf, indent);
                    buf.push_str(t.text());
                    buf.push('\n');
                }
            }
            NodeOrToken::Node(n) if n.kind() == SyntaxKind::RECORD_FIELD => {
                if let Some(field) = ast::RecordField::cast(n.clone()) {
                    // Collect comments immediately before this field
                    let preceding = comments::collect_preceding_comments_in_list(&children, idx);
                    for comment in &preceding {
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
            _ => {}
        }
    }
}
