use ra_ap_syntax::{
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_trait(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let trait_ = match ast::Trait::cast(node.clone()) {
        Some(t) => t,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in trait_.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in trait_.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = trait_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    // `trait` keyword and name
    buf.push_str("trait ");
    if let Some(name) = trait_.name() {
        buf.push_str(&name.text());
    }

    // Generic parameters
    if let Some(generics) = trait_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    // Where clause
    if let Some(where_clause) = trait_.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    // Trait body
    if let Some(item_list) = trait_.assoc_item_list() {
        buf.push_str(" {\n");

        let mut first_item = true;
        for child in item_list.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => {
                    // Add blank line between items, skip first
                    if !first_item {
                        buf.push('\n');
                    }
                    first_item = false;

                    // Recursively format items inside the trait
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
        buf.push_str("}\n");
    } else {
        buf.push_str(" {}\n");
    }
}
