// src/formatter/node/header.rs
use ra_ap_syntax::ast::{
    AstNode, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility,
};

use crate::formatter::write_indent;

/// Format common header for items: doc comments, attrs, visibility, keyword, name, generics.
///
/// Generic over AST node types that implement the Has* traits used here.
pub fn format_item_header<T>(item: &T, keyword: &str, buf: &mut String, indent: usize)
where
    T: HasDocComments + HasAttrs + HasVisibility + HasName + HasGenericParams,
{
    // Doc comments (///)
    for comment in item.doc_comments() {
        if let Some(text) = comment.doc_comment() {
            write_indent(buf, indent);
            buf.push_str(text.trim());
            buf.push('\n');
        }
    }

    // Attributes
    for attr in item.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = item.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    // Keyword & name
    buf.push_str(keyword);
    buf.push(' ');
    if let Some(name) = item.name() {
        buf.push_str(&name.text());
    }

    // Generics (if any)
    if let Some(generics) = item.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }
}
