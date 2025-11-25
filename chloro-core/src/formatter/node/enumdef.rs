use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
};

use crate::formatter::write_indent;

pub fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in enum_.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in enum_.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    if let Some(vis) = enum_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("enum ");

    if let Some(name) = enum_.name() {
        buf.push_str(&name.text());
    }

    if let Some(generics) = enum_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    if let Some(variants) = enum_.variant_list() {
        buf.push_str(" {\n");

        // Process children to handle comments between variants
        let children: Vec<_> = variants.syntax().children_with_tokens().collect();

        for (idx, child) in children.iter().enumerate() {
            match child {
                NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                    // Check if this comment is attached to the next variant
                    if !is_comment_attached_to_next_variant(&children, idx) {
                        write_indent(buf, indent + 4);
                        buf.push_str(t.text());
                        buf.push('\n');
                    }
                }
                NodeOrToken::Node(n) if n.kind() == SyntaxKind::VARIANT => {
                    if let Some(variant) = ast::Variant::cast(n.clone()) {
                        // Collect comments immediately before this variant
                        let comments = collect_preceding_comments_in_list(&children, idx);
                        for comment in &comments {
                            write_indent(buf, indent + 4);
                            buf.push_str(comment);
                            buf.push('\n');
                        }

                        // Format variant doc comments
                        for doc_comment in variant.doc_comments() {
                            write_indent(buf, indent + 4);
                            buf.push_str(doc_comment.text().trim());
                            buf.push('\n');
                        }

                        // Format variant attributes
                        for attr in variant.attrs() {
                            write_indent(buf, indent + 4);
                            buf.push_str(&attr.syntax().text().to_string());
                            buf.push('\n');
                        }

                        write_indent(buf, indent + 4);
                        if let Some(name) = variant.name() {
                            buf.push_str(&name.text());
                        }
                        if let Some(field_list) = variant.field_list() {
                            match field_list {
                                ast::FieldList::RecordFieldList(fields) => {
                                    buf.push_str(" {\n");
                                    format_record_fields(&fields, buf, indent + 8);
                                    write_indent(buf, indent + 4);
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
                                    buf.push(')');
                                }
                            }
                        }
                        buf.push_str(",\n");
                    }
                }
                _ => {}
            }
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
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

/// Check if a comment at the given index is attached to the next variant
fn is_comment_attached_to_next_variant(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
) -> bool {
    for child_item in children.iter().skip(comment_idx + 1) {
        match &child_item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    // If there's a blank line, the comment is not attached
                    if t.text().matches('\n').count() >= 2 {
                        return false;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    return false;
                }
            }
            NodeOrToken::Node(n) => {
                return n.kind() == SyntaxKind::VARIANT;
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
