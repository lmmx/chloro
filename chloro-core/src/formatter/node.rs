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

use ra_ap_syntax::ast::{Attr, Comment, Module, Use};
use ra_ap_syntax::{AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
pub use const_static::format_const_or_static;
#[allow(unused_imports)]
pub use debug::{debug_children_with_tokens, debug_node_siblings};
pub use enumdef::format_enum;
pub use function::format_function;
pub use implblock::format_impl;
pub use imports::sort_and_format_imports;
pub use macrocall::format_macro_call;
pub use module::format_module;
pub use structdef::format_struct;
pub use traitdef::format_trait;
pub use typealias::format_type_alias;
pub use useitem::format_use;

use super::printer::Printer;

/// Determine if a blank line should be added between two items
fn should_add_blank_line(prev_kind: Option<SyntaxKind>, curr_kind: SyntaxKind) -> bool {
    let result = {
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
                | SyntaxKind::USE
                | SyntaxKind::TRAIT
                | SyntaxKind::MACRO_RULES
                | SyntaxKind::MACRO_DEF
                | SyntaxKind::MACRO_CALL
        )
    };

    result
}

/// An item with its associated preceding comments and whether there's a blank line before it
#[derive(Clone)]
struct ItemWithComments {
    comments: Vec<Comment>,
    node: NodeOrToken<SyntaxNode, SyntaxToken>,
    blank_line_before: bool,
}

/// Main node formatting dispatcher
pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            let mut module_inner_docs = Vec::new();
            let mut inner_attrs: Vec<(Vec<Comment>, Attr)> = Vec::new();
            let mut extern_crates: Vec<(Vec<Comment>, SyntaxNode)> = Vec::new();
            let mut mod_decls: Vec<(Vec<Comment>, SyntaxNode)> = Vec::new();
            let mut use_items_with_comments = Vec::new();
            let mut other_items: Vec<ItemWithComments> = Vec::new();

            let mut pending_comments: Vec<Comment> = Vec::new();
            let mut pending_blank_line = false;

            let children: Vec<_> = node.children_with_tokens().collect();

            for (idx, child) in children.iter().enumerate() {
                match child {
                    NodeOrToken::Node(n) => {
                        match n.kind() {
                            SyntaxKind::ATTR => {
                                if let Some(attr) = Attr::cast(n.clone()) {
                                    if attr.excl_token().is_some() {
                                        inner_attrs
                                            .push((std::mem::take(&mut pending_comments), attr));
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
                            SyntaxKind::EXTERN_CRATE => {
                                extern_crates
                                    .push((std::mem::take(&mut pending_comments), n.clone()));
                                pending_blank_line = false;
                            }
                            SyntaxKind::MODULE => {
                                if let Some(module) = Module::cast(n.clone())
                                    && module.item_list().is_none()
                                {
                                    // Only group mod declarations at the top, before any use statements
                                    // Once we've seen use statements, preserve the original order
                                    if use_items_with_comments.is_empty() {
                                        mod_decls.push((
                                            std::mem::take(&mut pending_comments),
                                            n.clone(),
                                        ));
                                        pending_blank_line = false;
                                        continue;
                                    }
                                }
                                other_items.push(ItemWithComments {
                                    comments: std::mem::take(&mut pending_comments),
                                    node: NodeOrToken::Node(n.clone()),
                                    blank_line_before: pending_blank_line,
                                });
                                pending_blank_line = false;
                            }
                            SyntaxKind::USE => {
                                if let Some(use_) = Use::cast(n.clone()) {
                                    // Collect comments before this use
                                    let before: Vec<_> = std::mem::take(&mut pending_comments);

                                    // Now collect trailing comments/content after this use
                                    let mut trailing = Vec::new();
                                    let mut next_idx = idx + 1;

                                    while next_idx < children.len() {
                                        match &children[next_idx] {
                                            NodeOrToken::Token(t)
                                                if t.kind() == SyntaxKind::COMMENT =>
                                            {
                                                if let Some(comment) = Comment::cast(t.clone()) {
                                                    trailing.push(comment);
                                                }
                                                next_idx += 1;
                                            }
                                            NodeOrToken::Token(t)
                                                if t.kind() == SyntaxKind::WHITESPACE =>
                                            {
                                                // Check if this is just a newline or contains multiple newlines
                                                let text = t.text();
                                                if text.matches('\n').count() >= 2 {
                                                    // Double newline = end of comment block
                                                    break;
                                                }
                                                next_idx += 1;
                                            }
                                            _ => break,
                                        }
                                    }

                                    use_items_with_comments.push((before, use_, trailing));
                                    pending_blank_line = false;
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
                        }
                    }
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::COMMENT {
                            if let Some(comment) = Comment::cast(t.clone()) {
                                if comment.is_inner() && comment.kind().doc.is_some() {
                                    module_inner_docs.push(comment);
                                } else {
                                    pending_comments.push(comment);
                                }
                            }
                        } else if t.kind() == SyntaxKind::WHITESPACE {
                            // Check for blank lines
                            if t.text().matches('\n').count() >= 2 {
                                pending_blank_line = true;
                            }
                        }
                    }
                }
            }

            // If there are leftover pending comments (at the end of file), add them to other_items
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

            // 1. Module-level inner doc comments
            for doc in &module_inner_docs {
                buf.push_str(doc.text());
                buf.push('\n');
            }

            // 2. Inner attributes
            if !module_inner_docs.is_empty() && !inner_attrs.is_empty() {
                buf.push('\n');
            }
            for (comments, attr) in &inner_attrs {
                for comment in comments {
                    buf.push_str(comment.text());
                    buf.push('\n');
                }
                buf.push_str(attr.syntax().text().to_string().as_str());
                buf.push('\n');
            }

            let has_preamble = !inner_attrs.is_empty() || !module_inner_docs.is_empty();
            let has_content = !extern_crates.is_empty()
                || !mod_decls.is_empty()
                || !use_items_with_comments.is_empty()
                || !other_items.is_empty();

            if has_preamble && has_content {
                buf.push('\n');
            }

            // 3. Extern crate declarations
            for (comments, extern_crate) in &extern_crates {
                for comment in comments {
                    buf.push_str(comment.text());
                    buf.push('\n');
                }
                format_node(extern_crate, buf, indent);
            }
            if !extern_crates.is_empty()
                && (!mod_decls.is_empty()
                    || !use_items_with_comments.is_empty()
                    || !other_items.is_empty())
            {
                buf.push('\n');
            }

            // 4. Module declarations
            for (comments, mod_decl) in &mod_decls {
                for comment in comments {
                    buf.push_str(comment.text());
                    buf.push('\n');
                }
                format_node(mod_decl, buf, indent);
            }
            if !mod_decls.is_empty()
                && (!use_items_with_comments.is_empty() || !other_items.is_empty())
            {
                buf.push('\n');
            }

            // 5. Use statements with their trailing comments (SORTED)
            if !use_items_with_comments.is_empty() {
                sort_and_format_imports(&use_items_with_comments, buf, indent);
                if !other_items.is_empty() {
                    buf.push('\n');
                }
            }

            // 6. Everything else
            let mut last_kind: Option<SyntaxKind> = if !use_items_with_comments.is_empty() {
                Some(SyntaxKind::USE)
            } else if !mod_decls.is_empty() {
                Some(SyntaxKind::MODULE)
            } else {
                None
            };

            for item in other_items {
                // Output preceding comments
                for comment in &item.comments {
                    // Add blank line before comment block if there was one in original
                    if item.blank_line_before && last_kind.is_some() {
                        buf.blank();
                    }
                    buf.newline(comment.text());
                }

                match item.node {
                    NodeOrToken::Node(n) => {
                        let current_kind = n.kind();
                        // Add blank line between major items (but comments already added their blank line)
                        if item.comments.is_empty()
                            && should_add_blank_line(last_kind, current_kind)
                        {
                            buf.blank();
                        }
                        format_node(&n, buf, indent);
                        last_kind = Some(current_kind);
                    }
                    NodeOrToken::Token(t) => {
                        // Standalone token (like a trailing comment)
                        if t.kind() == SyntaxKind::COMMENT {
                            buf.newline(t.text());
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
