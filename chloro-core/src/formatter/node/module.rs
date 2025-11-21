use ra_ap_syntax::{
    ast::{self, HasModuleItem, HasName, HasVisibility},
    AstNode, SyntaxNode,
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_module(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let module = match ast::Module::cast(node.clone()) {
        Some(m) => m,
        None => return,
    };

    write_indent(buf, indent);

    if let Some(vis) = module.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("mod ");

    if let Some(name) = module.name() {
        buf.push_str(&name.text());
    }

    if let Some(item_list) = module.item_list() {
        buf.push_str(" {\n");
        for item in item_list.items() {
            format_node(item.syntax(), buf, indent + 4);
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push(';');
    }
    buf.push('\n');
}
