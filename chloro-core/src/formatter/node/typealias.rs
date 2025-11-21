use ra_ap_syntax::{
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
    AstNode, AstToken, SyntaxNode,
};

use crate::formatter::write_indent;

pub fn format_type_alias(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let type_alias = match ast::TypeAlias::cast(node.clone()) {
        Some(t) => t,
        None => return,
    };

    // Format doc comments
    for doc_comment in type_alias.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.syntax().text().to_string().trim());
        buf.push('\n');
    }

    // Format attributes
    for attr in type_alias.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = type_alias.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("type ");

    // Name
    if let Some(name) = type_alias.name() {
        buf.push_str(&name.text());
    }

    // Generic params
    if let Some(generics) = type_alias.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    // Where clause
    if let Some(where_clause) = type_alias.where_clause() {
        buf.push(' ');
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    // Type
    if let Some(ty) = type_alias.ty() {
        buf.push_str(" = ");
        buf.push_str(&ty.syntax().text().to_string());
    }

    buf.push_str(";\n");
}
