mod block;
pub mod common;
mod const_static;
mod debug;
mod enumdef;
mod expr;
mod function;
mod implblock;
mod imports;
mod macrocall;
mod module;
mod structdef;
mod traitdef;
mod typealias;
mod useitem;

use ra_ap_syntax::ast::{Attr, Comment, Use};
use ra_ap_syntax::{AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
pub use const_static::format_const_or_static;
pub use enumdef::format_enum;
pub use function::format_function;
pub use implblock::format_impl;
pub use macrocall::format_macro_call;
pub use module::format_module;
pub use structdef::format_struct;
pub use traitdef::format_trait;
pub use typealias::format_type_alias;
pub use useitem::format_use;

use super::printer::Printer;

/// Determine if a blank line should be added between two items
fn should_add_blank_line(prev_kind: Option<SyntaxKind>, curr_kind: SyntaxKind) -> bool {
    {
        let Some(prev) = prev_kind else {
            return false;
        };

        // No blank line between consecutive uses
        if prev == SyntaxKind::USE && curr_kind == SyntaxKind::USE {
            return false;
        }

        // No blank line between consecutive mod declarations
        if prev == SyntaxKind::MODULE && curr_kind == SyntaxKind::MODULE {
            return false;
        }

        // Blank line between different top-level items
        matches!(
            prev,
            SyntaxKind::FN
                | SyntaxKind::STRUCT
                | SyntaxKind::ENUM
                | SyntaxKind::IMPL
                | SyntaxKind::MODULE
                | SyntaxKind::CONST
                | SyntaxKind::STATIC
                | SyntaxKind::TYPE_ALIAS
                | SyntaxKind::TRAIT
                | SyntaxKind::MACRO_RULES
                | SyntaxKind::MACRO_DEF
                | SyntaxKind::MACRO_CALL
        ) && matches!(
            curr_kind,
            SyntaxKind::FN
                | SyntaxKind::STRUCT
                | SyntaxKind::ENUM
                | SyntaxKind::IMPL
                | SyntaxKind::MODULE
                | SyntaxKind::CONST
                | SyntaxKind::STATIC
                | SyntaxKind::TYPE_ALIAS
                | SyntaxKind::TRAIT
                | SyntaxKind::MACRO_RULES
                | SyntaxKind::MACRO_DEF
                | SyntaxKind::MACRO_CALL
        )
    }
}

/// An item with its associated preceding comments and whether there's a blank line before it
#[derive(Clone)]
struct ItemWithComments {
    comments: Vec<Comment>,
    node: NodeOrToken<SyntaxNode, SyntaxToken>,
    blank_line_before: bool,
}

fn sort_use_groups(items: &mut Vec<ItemWithComments>) {
    let mut i = 0;
    while i < items.len() {
        if let NodeOrToken::Node(n) = &items[i].node
            && n.kind() == SyntaxKind::USE
        {
            let start = i;
            let mut end = i + 1;
            while end < items.len() {
                if let NodeOrToken::Node(n) = &items[end].node
                    && n.kind() == SyntaxKind::USE
                    && !items[end].blank_line_before
                {
                    end += 1;
                    continue;
                }
                break;
            }
            if end > start + 1 {
                items[start..end].sort_by(|a, b| {
                    let use_a = match &a.node {
                        NodeOrToken::Node(n) => Use::cast(n.clone()),
                        _ => None,
                    };
                    let use_b = match &b.node {
                        NodeOrToken::Node(n) => Use::cast(n.clone()),
                        _ => None,
                    };
                    match (use_a, use_b) {
                        (Some(a), Some(b)) => {
                            let (group_a, path_a) = imports::classify_import(&a);
                            let (group_b, path_b) = imports::classify_import(&b);
                            group_a.cmp(&group_b).then_with(|| {
                                useitem::sort::sort_key(&path_a)
                                    .cmp(&useitem::sort::sort_key(&path_b))
                            })
                        }
                        _ => std::cmp::Ordering::Equal,
                    }
                });
            }
            i = end;
            continue;
        }
        i += 1;
    }
}

/// Main node formatting dispatcher
pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            // Leftover comments
            // Sort contiguous USE groups in place
            // Output
            let mut module_inner_docs = Vec::new();
            let mut inner_attrs: Vec<(Vec<Comment>, Attr)> = Vec::new();
            let mut other_items: Vec<ItemWithComments> = Vec::new();
            let mut pending_comments: Vec<Comment> = Vec::new();
            let mut pending_blank_line = false;
            let children: Vec<_> = node.children_with_tokens().collect();
            for child in children.iter() {
                match child {
                    NodeOrToken::Node(n) => match n.kind() {
                        SyntaxKind::ATTR => {
                            if let Some(attr) = Attr::cast(n.clone()) {
                                if attr.excl_token().is_some() {
                                    inner_attrs.push((std::mem::take(&mut pending_comments), attr));
                                    pending_blank_line = false;
                                } else {
                                    other_items.push(ItemWithComments {
                                        comments: std::mem::take(&mut pending_comments),
                                        node: NodeOrToken::Node(n.clone()),
                                        blank_line_before: pending_blank_line,
                                    });
                                    pending_blank_line = false;
                                }
                            }
                        }
                        _ => {
                            other_items.push(ItemWithComments {
                                comments: std::mem::take(&mut pending_comments),
                                node: NodeOrToken::Node(n.clone()),
                                blank_line_before: pending_blank_line,
                            });
                            pending_blank_line = false;
                        }
                    },
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::COMMENT {
                            if let Some(comment) = Comment::cast(t.clone()) {
                                if comment.is_inner() && comment.kind().doc.is_some() {
                                    module_inner_docs.push(comment);
                                } else {
                                    pending_comments.push(comment);
                                }
                            }
                        } else if t.kind() == SyntaxKind::WHITESPACE
                            && t.text().matches('\n').count() >= 2
                        {
                            pending_blank_line = true;
                        }
                    }
                }
            }
            if !pending_comments.is_empty() {
                for comment in pending_comments {
                    other_items.push(ItemWithComments {
                        comments: vec![],
                        node: NodeOrToken::Token(SyntaxToken::from(comment.syntax().clone())),
                        blank_line_before: pending_blank_line,
                    });
                    pending_blank_line = false;
                }
            }
            sort_use_groups(&mut other_items);
            for doc in &module_inner_docs {
                buf.push_str(doc.text());
                buf.push('\n');
            }
            if !module_inner_docs.is_empty() && !inner_attrs.is_empty() {
                buf.blank();
            }
            for (comments, attr) in &inner_attrs {
                for comment in comments {
                    buf.newline(comment.text());
                }
                buf.newline(&attr.syntax().text().to_string());
            }
            if (!inner_attrs.is_empty() || !module_inner_docs.is_empty()) && !other_items.is_empty()
            {
                buf.blank();
            }
            let mut last_kind: Option<SyntaxKind> = None;
            let mut prev_was_standalone_comment = false;

            for item in other_items {
                // Output comments attached to this item
                for (i, comment) in item.comments.iter().enumerate() {
                    // Add blank line before first comment if needed
                    if i == 0
                        && item.blank_line_before
                        && (last_kind.is_some() || prev_was_standalone_comment)
                    {
                        buf.blank();
                    }
                    buf.newline(comment.text());
                }

                match item.node {
                    NodeOrToken::Node(n) => {
                        let current_kind = n.kind();
                        // Add blank line if needed (only when no comments preceded this)
                        if item.comments.is_empty() {
                            if item.blank_line_before
                                && (last_kind.is_some() || prev_was_standalone_comment)
                            {
                                buf.blank();
                            } else if !prev_was_standalone_comment
                                && should_add_blank_line(last_kind, current_kind)
                            {
                                buf.blank();
                            }
                        }
                        format_node(&n, buf, indent);
                        last_kind = Some(current_kind);
                        prev_was_standalone_comment = false;
                    }
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::COMMENT {
                            // Standalone comment (not attached to an item)
                            if item.blank_line_before
                                && (last_kind.is_some() || prev_was_standalone_comment)
                            {
                                buf.blank();
                            }
                            buf.newline(t.text());
                            prev_was_standalone_comment = true;
                        }
                    }
                }
            }
        }

        SyntaxKind::FN => format_function(node, buf, indent),
        SyntaxKind::STRUCT => format_struct(node, buf, indent),
        SyntaxKind::ENUM => format_enum(node, buf, indent),
        SyntaxKind::IMPL => format_impl(node, buf, indent),
        SyntaxKind::TRAIT => format_trait(node, buf, indent),
        SyntaxKind::USE => format_use(node, buf, indent),
        SyntaxKind::MODULE => format_module(node, buf, indent),
        SyntaxKind::TYPE_ALIAS => format_type_alias(node, buf, indent),
        SyntaxKind::CONST => format_const_or_static(node, buf, indent),
        SyntaxKind::STATIC => format_const_or_static(node, buf, indent),

        SyntaxKind::BLOCK_EXPR => format_block(node, buf, indent),
        SyntaxKind::STMT_LIST => format_stmt_list(node, buf, indent),

        SyntaxKind::MACRO_CALL => format_macro_call(node, buf, indent),
        SyntaxKind::MACRO_RULES | SyntaxKind::MACRO_DEF => {
            // Preserve macro definitions as-is for now
            crate::formatter::write_indent(buf, indent);
            buf.push_str(&node.text().to_string());
            buf.push('\n');
        }

        SyntaxKind::ATTR => {
            // Handle standalone attributes
            if let Some(attr) = Attr::cast(node.clone()) {
                crate::formatter::write_indent(buf, indent);
                buf.push_str(attr.syntax().text().to_string().as_str());
                buf.push('\n');
            }
        }

        _ => {
            // Default: recurse through children
            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => format_node(&n, buf, indent),
                    NodeOrToken::Token(t) => format_token(&t, buf, indent),
                }
            }
        }
    }
}

/// Format a single token (comments, whitespace, keywords, etc.)
fn format_token(token: &SyntaxToken, buf: &mut String, _indent: usize) {
    match token.kind() {
        SyntaxKind::COMMENT => {
            // Handle comments specially - preserve them exactly
            if let Some(comment) = Comment::cast(token.clone()) {
                buf.push_str(comment.text());
                buf.push('\n');
            }
        }
        SyntaxKind::WHITESPACE => {
            // Skip whitespace - let the formatter control all spacing
        }
        _ => {
            // Other tokens shouldn't appear in top-level context
        }
    }
}
