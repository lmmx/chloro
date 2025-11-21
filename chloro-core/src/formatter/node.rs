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
            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => {
                        format_node(&n, buf, indent);
                        // Add spacing between top-level items
                        if matches!(
                            n.kind(),
                            SyntaxKind::FN
                                | SyntaxKind::STRUCT
                                | SyntaxKind::ENUM
                                | SyntaxKind::IMPL
                                | SyntaxKind::MODULE
                        ) {
                            buf.push('\n');
                        }
                    }
                    NodeOrToken::Token(t) => {
                        format_token(&t, buf, indent);
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
fn format_token(token: &SyntaxToken, buf: &mut String, indent: usize) {
    match token.kind() {
        SyntaxKind::COMMENT => {
            // Handle comments specially
            if let Some(comment) = ast::Comment::cast(token.clone()) {
                let text = comment.text();

                // Preserve the comment exactly
                crate::formatter::write_indent(buf, indent);
                buf.push_str(text);
                buf.push('\n');
            }
        }
        SyntaxKind::WHITESPACE => {
            // Normalize whitespace - preserve newlines but not excessive ones
            let text = token.text();
            if text.contains('\n') {
                let newline_count = text.matches('\n').count();
                // Preserve up to 2 newlines (one blank line)
                for _ in 0..newline_count.min(2) {
                    buf.push('\n');
                }
            }
            // Don't add spaces - let the formatter handle spacing
        }
        _ => {
            // For other tokens, just append them as-is
            // (This shouldn't happen much since we handle nodes specially)
        }
    }
}
