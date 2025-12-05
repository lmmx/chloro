use ra_ap_syntax::{AstNode, SyntaxNode, ast};

use crate::formatter::printer::Printer;

pub fn format_extern_block(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let Some(extern_block) = ast::ExternBlock::cast(node.clone()) else {
        return;
    };

    buf.indent(indent);

    if extern_block.unsafe_token().is_some() {
        buf.push_str("unsafe ");
    }

    if let Some(abi) = extern_block.abi() {
        buf.push_str(&abi.syntax().text().to_string());
    } else {
        buf.push_str("extern");
    }

    if let Some(item_list) = extern_block.extern_item_list() {
        let items: Vec<_> = item_list.extern_items().collect();
        if items.is_empty() {
            buf.newline(" {}");
        } else {
            buf.open_brace();
            for item in items {
                buf.line(indent + 4, item.syntax().text().to_string().trim());
            }
            buf.close_brace_ln(indent);
        }
    } else {
        buf.newline(" {}");
    }
}
