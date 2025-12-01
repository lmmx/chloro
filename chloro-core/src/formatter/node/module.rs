use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasName},
};

use super::format_node;
use crate::formatter::printer::Printer;

pub fn format_module(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let Some(module) = ast::Module::cast(node.clone()) else {
        return;
    };

    buf.item_preamble(&module, indent);
    buf.push_str("mod ");
    if let Some(name) = module.name() {
        buf.push_str(&name.text());
    }

    if let Some(item_list) = module.item_list() {
        buf.open_brace();
        for child in item_list.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => format_node(&n, buf, indent + 4),
                NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                    buf.line(indent + 4, t.text());
                }
                _ => {}
            }
        }
        buf.close_brace(indent);
    } else {
        buf.push(';');
    }
    buf.push('\n');
}
