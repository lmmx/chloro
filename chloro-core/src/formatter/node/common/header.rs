use ra_ap_syntax::ast::{
    AstNode, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility,
};

use crate::formatter::printer::Printer;

/// Format common header: doc comments, attrs, visibility, keyword, name, generics.
pub fn format_item_header<T>(item: &T, keyword: &str, buf: &mut String, indent: usize)
where
    T: HasDocComments + HasAttrs + HasVisibility + HasName + HasGenericParams,
{
    buf.item_preamble(item, indent);
    buf.push_str(keyword);
    buf.push(' ');
    if let Some(name) = item.name() {
        buf.push_str(&name.text());
    }
    if let Some(generics) = item.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }
}
