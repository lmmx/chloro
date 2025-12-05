use ra_ap_syntax::ast::HasName;
use ra_ap_syntax::{AstNode, SyntaxNode, ast};

use crate::formatter::printer::Printer;

pub fn format_extern_crate(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let Some(extern_crate) = ast::ExternCrate::cast(node.clone()) else {
        return;
    };

    buf.item_preamble(&extern_crate, indent);
    buf.push_str("extern crate ");

    if let Some(name_ref) = extern_crate.name_ref() {
        buf.push_str(&name_ref.text());
    }

    if let Some(rename) = extern_crate.rename() {
        if let Some(name) = rename.name() {
            buf.push_str(" as ");
            buf.push_str(&name.text());
        }
    }

    buf.push_str(";\n");
}
