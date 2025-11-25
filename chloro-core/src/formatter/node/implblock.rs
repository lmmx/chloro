use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams},
};

use super::format_node;
use crate::formatter::write_indent;

pub fn format_impl(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let impl_ = match ast::Impl::cast(node.clone()) {
        Some(i) => i,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in impl_.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in impl_.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
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

    // Check if this is a trait impl
    if let Some(trait_) = impl_.trait_() {
        buf.push_str(&trait_.syntax().text().to_string());
        buf.push_str(" for ");
    }

    if let Some(ty) = impl_.self_ty() {
        buf.push_str(&ty.syntax().text().to_string());
    }

    if let Some(where_clause) = impl_.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    if let Some(assoc_items) = impl_.assoc_item_list() {
        if impl_.where_clause().is_some() {
            buf.push('\n');
            write_indent(buf, indent);
            buf.push_str("{\n");
        } else {
            buf.push_str(" {\n");
        }

        let mut first_item = true;
        let children: Vec<_> = assoc_items.syntax().children_with_tokens().collect();

        // Item kinds we care about in impl blocks
        const IMPL_ITEM_KINDS: &[SyntaxKind] = &[
            SyntaxKind::FN,
            SyntaxKind::TYPE_ALIAS,
            SyntaxKind::CONST,
            SyntaxKind::ASSOC_ITEM_LIST,
        ];

        // Process all items AND comments within the impl block
        for (idx, child) in children.iter().enumerate() {
            match child {
                NodeOrToken::Node(n) => {
                    let is_item = IMPL_ITEM_KINDS.contains(&n.kind());

                    if is_item {
                        // Add blank line before items except the first
                        if !first_item {
                            buf.push('\n');
                        }
                        first_item = false;

                        // Check for comments immediately before this item
                        let comments = collect_preceding_comments_in_list(&children, idx);
                        for comment in comments {
                            write_indent(buf, indent + 4);
                            buf.push_str(&comment);
                            buf.push('\n');
                        }

                        format_node(n, buf, indent + 4);
                    } else {
                        format_node(n, buf, indent + 4);
                    }
                }
                NodeOrToken::Token(t) => {
                    // Only output comments that aren't attached to the next item
                    if t.kind() == SyntaxKind::COMMENT {
                        // Check if this comment is followed by an item (will be handled there)
                        let is_attached =
                            is_comment_attached_to_next_item(&children, idx, IMPL_ITEM_KINDS);
                        if !is_attached {
                            write_indent(buf, indent + 4);
                            buf.push_str(t.text());
                            buf.push('\n');
                        }
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

/// Collect comments immediately before an item at the given index
fn collect_preceding_comments_in_list(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    item_idx: usize,
) -> Vec<String> {
    let mut comments = Vec::new();

    for i in (0..item_idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Stop at blank lines
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                } else {
                    break;
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }

    comments.reverse();
    comments
}

/// Check if a comment at the given index is attached to the next item
fn is_comment_attached_to_next_item(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
    item_kinds: &[SyntaxKind],
) -> bool {
    for child_item in children.iter().skip(comment_idx + 1) {
        match &child_item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    // If there's a blank line, the comment is not attached
                    if t.text().matches('\n').count() >= 2 {
                        return false;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    return false;
                }
            }
            NodeOrToken::Node(n) => {
                // Found an item - the comment is attached
                return item_kinds.contains(&n.kind());
            }
        }
    }
    false
}
