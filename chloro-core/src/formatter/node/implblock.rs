use ra_ap_syntax::{
    ast::{self, HasDocComments, HasGenericParams},
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_impl(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let impl_ = match ast::Impl::cast(node.clone()) {
        Some(i) => i,
        None => return,
    };

    // Format impl doc comments if any
    for doc_comment in impl_.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.syntax().text().to_string().trim());
        buf.push('\n');
    }

    write_indent(buf, indent);

    if impl_.unsafe_token().is_some() {
        buf.push_str("unsafe ");
    }

    buf.push_str("impl");

    if let Some(generics) = impl_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    buf.push(' ');

    if let Some(ty) = impl_.self_ty() {
        buf.push_str(&ty.syntax().text().to_string());
    }

    if let Some(where_clause) = impl_.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    if let Some(assoc_items) = impl_.assoc_item_list() {
        buf.push_str(" {\n");

        let mut first_item = true;

        // Process all items AND comments within the impl block
        for child in assoc_items.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => {
                    let is_item = matches!(
                        n.kind(),
                        SyntaxKind::FN
                            | SyntaxKind::TYPE_ALIAS
                            | SyntaxKind::CONST
                            | SyntaxKind::ASSOC_ITEM_LIST
                    );

                    if is_item {
                        // Add blank line before items except the first
                        if !first_item {
                            buf.push('\n');
                        }
                        first_item = false;

                        format_node(&n, buf, indent + 4);
                    } else {
                        format_node(&n, buf, indent + 4);
                    }
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
        buf.push_str(" {}");
    }
    buf.push('\n');
}
