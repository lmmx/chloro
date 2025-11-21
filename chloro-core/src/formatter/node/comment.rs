use crate::formatter::write_indent;
use ra_ap_syntax::ast::{self, AstNode};

/// Extract and format attributes for an item
pub fn format_attributes(attrs: impl Iterator<Item = ast::Attr>, buf: &mut String, indent: usize) {
    for attr in attrs {
        write_indent(buf, indent);
        buf.push_str(attr.syntax().text().to_string().as_str());
        buf.push('\n');
    }
}
