use ra_ap_syntax::{
    AstNode, AstToken, SyntaxNode,
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
                for field in fields.fields() {
                    // Format field doc comments
                    for doc_comment in field.doc_comments() {
                        write_indent(buf, indent + 4);
                        buf.push_str(doc_comment.text().trim());
                        buf.push('\n');
                    }

                    // Format field attributes
                    for attr in field.attrs() {
                        write_indent(buf, indent + 4);
                        buf.push_str(&attr.syntax().text().to_string());
                        buf.push('\n');
                    }

                    write_indent(buf, indent + 4);
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
