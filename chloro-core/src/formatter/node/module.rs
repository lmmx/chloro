use ra_ap_syntax::{
    ast::{self, HasAttrs, HasDocComments, HasName, HasVisibility},
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_module(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let module = match ast::Module::cast(node.clone()) {
        Some(m) => m,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in module.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in module.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

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

        // Process all items and comments within the module
        for child in item_list.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => {
                    format_node(&n, buf, indent + 4);
                }
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::COMMENT {
                        write_indent(buf, indent + 4);
                        buf.push_str(t.text());
                        buf.push('\n');
                    }
                }
            }
        }

        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push(';');
    }
    buf.push('\n');
}
