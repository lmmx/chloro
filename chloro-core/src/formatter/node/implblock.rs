use ra_ap_syntax::{
    ast::{self, HasGenericParams},
    AstNode, SyntaxNode,
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_impl(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let impl_ = match ast::Impl::cast(node.clone()) {
        Some(i) => i,
        None => return,
    };

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
        for item in assoc_items.assoc_items() {
            format_node(item.syntax(), buf, indent + 4);
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}
