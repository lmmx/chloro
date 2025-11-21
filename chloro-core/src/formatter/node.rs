mod block;
mod comment;
mod enumdef;
mod function;
mod implblock;
mod module;
mod structdef;
mod useitem;

use ra_ap_syntax::{ast, AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
pub use comment::format_attributes;
pub use enumdef::format_enum;
pub use function::format_function;
pub use implblock::format_impl;
pub use module::format_module;
pub use structdef::format_struct;
pub use useitem::format_use;

/// Main node formatting dispatcher
pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            let mut last_was_item = false;

            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => {
                        let is_item = matches!(
                            n.kind(),
                            SyntaxKind::FN
                                | SyntaxKind::STRUCT
                                | SyntaxKind::ENUM
                                | SyntaxKind::IMPL
                                | SyntaxKind::MODULE
                                | SyntaxKind::USE
                                | SyntaxKind::TYPE_ALIAS
                                | SyntaxKind::CONST
                                | SyntaxKind::STATIC
                        );

                        // Add blank line before items (except the first one)
                        if is_item && last_was_item {
                            buf.push('\n');
                        }

                        format_node(&n, buf, indent);
                        last_was_item = is_item;
                    }
                    NodeOrToken::Token(t) => {
                        format_token(&t, buf, indent);
                        // Comments and attributes don't count as items
                        if !matches!(t.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT) {
                            last_was_item = false;
                        }
                    }
                }
            }
        }

        SyntaxKind::FN => format_function(node, buf, indent),
        SyntaxKind::STRUCT => format_struct(node, buf, indent),
        SyntaxKind::ENUM => format_enum(node, buf, indent),
        SyntaxKind::IMPL => format_impl(node, buf, indent),
        SyntaxKind::USE => format_use(node, buf, indent),
        SyntaxKind::MODULE => format_module(node, buf, indent),

        SyntaxKind::BLOCK_EXPR => format_block(node, buf, indent),
        SyntaxKind::STMT_LIST => format_stmt_list(node, buf, indent),

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
