use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
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

/// Collect non-doc comments that appear immediately before a syntax node
fn collect_preceding_comments(node: &SyntaxNode) -> Vec<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling_or_token();

    while let Some(p) = prev {
        match &p {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments (they're handled by HasDocComments)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Check for blank lines - if there are 2+ newlines, stop collecting
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                } else {
                    break;
                }
                prev = t.prev_sibling_or_token();
            }
            NodeOrToken::Node(_) => break,
        }
    }

    comments.reverse();
    comments
}

/// Format record fields with their comments
fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    for field in fields.fields() {
        // Collect comments immediately before this field
        let comments = collect_preceding_comments(field.syntax());
        for comment in comments {
            write_indent(buf, indent);
            buf.push_str(&comment);
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
