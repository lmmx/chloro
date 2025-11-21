mod block;
mod debug;
mod doccomment;
mod enumdef;
mod function;
mod implblock;
mod imports;
mod module;
mod structdef;
mod typealias;
mod useitem;

use ra_ap_syntax::{ast, AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
#[allow(unused_imports)]
pub use debug::{debug_children_with_tokens, debug_node_siblings};
pub use doccomment::format_preceding_docs_and_attrs;
pub use enumdef::format_enum;
pub use function::format_function;
pub use implblock::format_impl;
pub use imports::sort_and_format_imports;
pub use module::format_module;
pub use structdef::format_struct;
pub use typealias::format_type_alias;
pub use useitem::format_use;

/// Determine if a blank line should be added between two items
fn should_add_blank_line(prev_kind: Option<SyntaxKind>, curr_kind: SyntaxKind) -> bool {
    let Some(prev) = prev_kind else {
        return false;
    };

    // No blank line between consecutive uses
    if prev == SyntaxKind::USE && curr_kind == SyntaxKind::USE {
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
    )
}

/// Main node formatting dispatcher
pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            // First pass: collect all uses and other items
            let mut uses = Vec::new();
            let mut other_items = Vec::new();
            let mut module_docs = Vec::new();

            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => {
                        if n.kind() == SyntaxKind::USE {
                            if let Some(use_) = ast::Use::cast(n) {
                                uses.push(use_);
                            }
                        } else {
                            other_items.push(n);
                        }
                    }
                    NodeOrToken::Token(t) => {
                        // Check if this is a module-level doc comment
                        if t.kind() == SyntaxKind::COMMENT {
                            if let Some(comment) = ast::Comment::cast(t) {
                                if comment.is_inner() && comment.kind().doc.is_some() {
                                    module_docs.push(comment);
                                }
                            }
                        }
                    }
                }
            }

            // Format module-level docs first
            for doc in &module_docs {
                buf.push_str(doc.text());
                buf.push('\n');
            }

            let has_docs = !module_docs.is_empty();
            let has_uses = !uses.is_empty();

            // Ensure a blank line after module docs if anything follows
            if has_docs && (has_uses || !other_items.is_empty()) {
                buf.push('\n');
            }

            // Format sorted and grouped imports
            if has_uses {
                sort_and_format_imports(&uses, buf, indent);
                // If other items follow imports, ensure a blank line between
                if !other_items.is_empty() {
                    buf.push('\n');
                }
            }

            // Format other items
            let mut last_kind: Option<SyntaxKind> = if has_uses {
                Some(SyntaxKind::USE)
            } else {
                None
            };

            for node in other_items {
                let current_kind = node.kind();

                // Add blank line if needed
                if should_add_blank_line(last_kind, current_kind) {
                    buf.push('\n');
                }

                format_node(&node, buf, indent);
                last_kind = Some(current_kind);
            }
        }

        SyntaxKind::FN => format_function(node, buf, indent),
        SyntaxKind::STRUCT => format_struct(node, buf, indent),
        SyntaxKind::ENUM => format_enum(node, buf, indent),
        SyntaxKind::IMPL => format_impl(node, buf, indent),
        SyntaxKind::USE => format_use(node, buf, indent),
        SyntaxKind::MODULE => format_module(node, buf, indent),
        SyntaxKind::TYPE_ALIAS => format_type_alias(node, buf, indent),

        SyntaxKind::BLOCK_EXPR => format_block(node, buf, indent),
        SyntaxKind::STMT_LIST => format_stmt_list(node, buf, indent),

        SyntaxKind::MACRO_RULES | SyntaxKind::MACRO_DEF => {
            // Preserve macro definitions as-is for now
            crate::formatter::write_indent(buf, indent);
            buf.push_str(&node.text().to_string());
            buf.push('\n');
        }

        SyntaxKind::ATTR => {
            // Handle standalone attributes
            if let Some(attr) = ast::Attr::cast(node.clone()) {
                crate::formatter::write_indent(buf, indent);
                buf.push_str(attr.syntax().text().to_string().as_str());
                buf.push('\n');
            }
        }

        _ => {
            // Default: recurse through children AND tokens
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
            // Handle comments specially
            if let Some(comment) = ast::Comment::cast(token.clone()) {
                let text = comment.text();

                // Preserve the comment exactly (it already has //, //!, or ///)
                buf.push_str(text);
                buf.push('\n');
            }
        }
        SyntaxKind::WHITESPACE => {
            // Skip whitespace entirely - let the formatter control all spacing
        }
        _ => {
            // For other tokens, this shouldn't happen in SOURCE_FILE context
        }
    }
}
