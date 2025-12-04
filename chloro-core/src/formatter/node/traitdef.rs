use crate::formatter::node::common::{comments, header};
use crate::formatter::write_indent;
use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasGenericParams},
};

use super::format_node;

pub fn format_trait(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let trait_ = match ast::Trait::cast(node.clone()) {
        Some(t) => t,
        None => return,
    };

    // Header (docs, attrs, vis, "trait", name, generics)
    header::format_item_header(&trait_, "trait", buf, indent);

    // Where clause
    if let Some(where_clause) = trait_.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    // Trait body
    if let Some(item_list) = trait_.assoc_item_list() {
        if trait_.where_clause().is_some() {
            buf.push('\n');
            write_indent(buf, indent);
            buf.push_str("{\n");
        } else {
            buf.push_str(" {\n");
        }
        let mut first_item = true;
        let children: Vec<_> = item_list.syntax().children_with_tokens().collect();
        // Item kinds we care about in trait blocks
        const TRAIT_ITEM_KINDS: &[SyntaxKind] =
            &[SyntaxKind::FN, SyntaxKind::TYPE_ALIAS, SyntaxKind::CONST];
        for (idx, child) in children.iter().enumerate() {
            match child {
                NodeOrToken::Node(n) => {
                    let is_item = TRAIT_ITEM_KINDS.contains(&n.kind());

                    if is_item {
                        // Add blank line between items, skip first
                        if !first_item {
                            buf.push('\n');
                        }
                        first_item = false;

                        // Comments immediately before this item
                        let comments_before =
                            comments::collect_preceding_comments_in_list(&children, idx);
                        for comment in comments_before {
                            write_indent(buf, indent + 4);
                            buf.push_str(&comment);
                            buf.push('\n');
                        }

                        // Recursively format items inside the trait
                        format_node(n, buf, indent + 4);
                    } else {
                        format_node(n, buf, indent + 4);
                    }
                }
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::COMMENT {
                        // Only output comments that aren't attached to the next item
                        let is_attached = comments::is_comment_attached_to_next_item(
                            &children,
                            idx,
                            TRAIT_ITEM_KINDS,
                        );
                        if !is_attached {
                            if comments::should_have_blank_line_before_comment(&children, idx) {
                                buf.push('\n');
                            }
                            write_indent(buf, indent + 4);
                            buf.push_str(t.text());
                            buf.push('\n');
                        }
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
