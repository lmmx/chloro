// chloro-core/src/formatter/node/traitdef.rs
use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
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

                        // Check for comments immediately before this item
                        let comments = collect_preceding_comments_in_list(&children, idx);
                        for comment in comments {
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
                        let is_attached =
                            is_comment_attached_to_next_item(&children, idx, TRAIT_ITEM_KINDS);
                        if !is_attached {
                            // Check for blank line before standalone comment
                            if should_have_blank_line_before_comment(&children, idx) {
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

/// Check if there should be a blank line before a comment at the given index
fn should_have_blank_line_before_comment(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
) -> bool {
    for i in (0..comment_idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() == SyntaxKind::COMMENT {
                    continue;
                } else {
                    return false;
                }
            }
            NodeOrToken::Node(_) => return false,
        }
    }
    false
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
