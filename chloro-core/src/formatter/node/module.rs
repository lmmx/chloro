use crate::formatter::printer::Printer;
use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasName},
};

use super::format_node;

pub fn format_module(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let Some(module) = ast::Module::cast(node.clone()) else {
        return;
    };

    // Collect and output regular comments that appear before the visibility/mod keyword
    for child in node.children_with_tokens() {
        match &child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text();
                    // Skip doc comments - they're handled by item_preamble via HasDocComments
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        buf.line(indent, text);
                    }
                } else if t.kind() != SyntaxKind::WHITESPACE {
                    // Hit a non-comment, non-whitespace token - stop collecting comments
                    break;
                }
            }
            NodeOrToken::Node(_) => {
                // Hit a node (like VISIBILITY) - stop collecting comments
                break;
            }
        }
    }

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
