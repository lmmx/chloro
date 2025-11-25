use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
};

use crate::formatter::write_indent;

pub fn format_struct(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let strukt = match ast::Struct::cast(node.clone()) {
        Some(s) => s,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in strukt.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in strukt.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    if let Some(vis) = strukt.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("struct ");

    if let Some(name) = strukt.name() {
        buf.push_str(&name.text());
    }

    if let Some(generics) = strukt.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    if let Some(field_list) = strukt.field_list() {
        match field_list {
            ast::FieldList::RecordFieldList(fields) => {
                buf.push_str(" {\n");
                format_record_fields(&fields, buf, indent + 4);
                write_indent(buf, indent);
                buf.push('}');
            }
            ast::FieldList::TupleFieldList(fields) => {
                buf.push('(');
                for (idx, field) in fields.fields().enumerate() {
                    if idx > 0 {
                        buf.push_str(", ");
                    }
                    if let Some(vis) = field.visibility() {
                        buf.push_str(&vis.syntax().text().to_string());
                        buf.push(' ');
                    }
                    if let Some(ty) = field.ty() {
                        buf.push_str(&ty.syntax().text().to_string());
                    }
                }
                buf.push_str(");");
            }
        }
    } else {
        buf.push(';');
    }
    buf.push('\n');
}

/// Collect comments immediately before an item at the given index in a children list
fn collect_preceding_comments_in_list(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    item_idx: usize,
) -> Vec<String> {
    let mut comments = Vec::new();

    for i in (0..item_idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments (they're handled by HasDocComments)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Stop at blank lines
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                } else {
                    break;
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }

    comments.reverse();
    comments
}

/// Check if a comment at the given index is attached to the next field
fn is_comment_attached_to_next_field(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
) -> bool {
    for child_item in children.iter().skip(comment_idx + 1) {
        match &child_item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return false;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    return false;
                }
            }
            NodeOrToken::Node(n) => {
                return n.kind() == SyntaxKind::RECORD_FIELD;
            }
        }
    }
    false
}

/// Format record fields with their comments
fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    let children: Vec<_> = fields.syntax().children_with_tokens().collect();

    for (idx, child) in children.iter().enumerate() {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                // Check if this comment is attached to the next field
                if !is_comment_attached_to_next_field(&children, idx) {
                    write_indent(buf, indent);
                    buf.push_str(t.text());
                    buf.push('\n');
                }
            }
            NodeOrToken::Node(n) if n.kind() == SyntaxKind::RECORD_FIELD => {
                if let Some(field) = ast::RecordField::cast(n.clone()) {
                    // Collect comments immediately before this field
                    let comments = collect_preceding_comments_in_list(&children, idx);
                    for comment in &comments {
                        write_indent(buf, indent);
                        buf.push_str(comment);
                        buf.push('\n');
                    }

                    // Format field doc comments
                    for doc_comment in field.doc_comments() {
                        write_indent(buf, indent);
                        buf.push_str(doc_comment.text().trim());
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
