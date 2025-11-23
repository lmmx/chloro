use ra_ap_syntax::{
    AstNode, AstToken, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasName, HasVisibility},
};

use crate::formatter::write_indent;

pub fn format_const_or_static(node: &SyntaxNode, buf: &mut String, indent: usize) {
    if let Some(c) = ast::Const::cast(node.clone()) {
        // Format doc comments using HasDocComments trait
        for doc_comment in c.doc_comments() {
            write_indent(buf, indent);
            buf.push_str(doc_comment.text().trim());
            buf.push('\n');
        }

        // Format attributes using HasAttrs trait
        for attr in c.attrs() {
            write_indent(buf, indent);
            buf.push_str(&attr.syntax().text().to_string());
            buf.push('\n');
        }

        write_indent(buf, indent);

        // Visibility
        if let Some(vis) = c.visibility() {
            buf.push_str(&vis.syntax().text().to_string());
            buf.push(' ');
        }

        buf.push_str("const ");

        // Name
        if let Some(name) = c.name() {
            buf.push_str(name.text().as_ref());
            buf.push_str(": ");
        }

        // Type
        if let Some(ty) = c.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }

        // Initializer / body
        if let Some(expr) = c.body() {
            buf.push_str(" = ");
            buf.push_str(&expr.syntax().text().to_string());
        }

        buf.push_str(";\n");
    } else if let Some(s) = ast::Static::cast(node.clone()) {
        // Format doc comments using HasDocComments trait
        for doc_comment in s.doc_comments() {
            write_indent(buf, indent);
            buf.push_str(doc_comment.text().trim());
            buf.push('\n');
        }

        // Format attributes using HasAttrs trait
        for attr in s.attrs() {
            write_indent(buf, indent);
            buf.push_str(&attr.syntax().text().to_string());
            buf.push('\n');
        }

        write_indent(buf, indent);

        // Visibility
        if let Some(vis) = s.visibility() {
            buf.push_str(&vis.syntax().text().to_string());
            buf.push(' ');
        }

        // Static / static mut
        if s.mut_token().is_some() {
            buf.push_str("static mut ");
        } else {
            buf.push_str("static ");
        }

        // Name
        if let Some(name) = s.name() {
            buf.push_str(name.text().as_ref());
            buf.push_str(": ");
        }

        // Type
        if let Some(ty) = s.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }

        // Initializer / body
        if let Some(expr) = s.body() {
            buf.push_str(" = ");
            buf.push_str(&expr.syntax().text().to_string());
        }

        buf.push_str(";\n");
    }
}
