use crate::formatter::node::common::comments;
use crate::formatter::printer::Printer;
use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasGenericParams},
};

use super::format_node;

pub fn format_impl(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let impl_ = match ast::Impl::cast(node.clone()) {
        Some(i) => i,
        None => return,
    };

    buf.doc_comments(&impl_, indent);
    buf.attrs(&impl_, indent);
    buf.indent(indent);

    if impl_.default_token().is_some() {
        buf.push_str("default ");
    }

    if impl_.unsafe_token().is_some() {
        buf.push_str("unsafe ");
    }

    buf.push_str("impl");
    if let Some(generics) = impl_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    buf.push(' ');

    if let Some(trait_) = impl_.trait_() {
        buf.push_str(&trait_.syntax().text().to_string());
        buf.push_str(" for ");
    }

    if let Some(ty) = impl_.self_ty() {
        buf.push_str(&ty.syntax().text().to_string());
    }

    if let Some(where_clause) = impl_.where_clause() {
        buf.blank();
        buf.indent(indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    if let Some(assoc_items) = impl_.assoc_item_list() {
        let has_items = assoc_items.assoc_items().next().is_some();

        if !has_items {
            buf.newline(" {}");
        } else {
            if impl_.where_clause().is_some() {
                buf.open_brace_newline(indent);
            } else {
                buf.open_brace();
            }

            let mut first_item = true;
            let children: Vec<_> = assoc_items.syntax().children_with_tokens().collect();

            const IMPL_ITEM_KINDS: &[SyntaxKind] = &[
                SyntaxKind::FN,
                SyntaxKind::TYPE_ALIAS,
                SyntaxKind::CONST,
                SyntaxKind::ASSOC_ITEM_LIST,
            ];

            for (idx, child) in children.iter().enumerate() {
                match child {
                    NodeOrToken::Node(n) => {
                        let is_item = IMPL_ITEM_KINDS.contains(&n.kind());

                        if is_item {
                            if !first_item {
                                buf.blank();
                            }
                            first_item = false;

                            let comments_before =
                                comments::collect_preceding_comments_in_list(&children, idx);
                            for comment in comments_before {
                                buf.line(indent + 4, &comment);
                            }

                            format_node(n, buf, indent + 4);
                        } else {
                            format_node(n, buf, indent + 4);
                        }
                    }
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::COMMENT {
                            let is_attached = comments::is_comment_attached_to_next_item(
                                &children,
                                idx,
                                IMPL_ITEM_KINDS,
                            );
                            if !is_attached {
                                if comments::should_have_blank_line_before_comment(&children, idx) {
                                    buf.blank();
                                }
                                buf.line(indent + 4, t.text());
                            }
                        }
                    }
                }
            }

            buf.close_brace_ln(indent);
        }
    } else {
        buf.newline(" {}");
    }
}
