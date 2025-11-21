mod block;
mod debug;
mod doccomment;
mod enumdef;
mod function;
mod implblock;
mod module;
mod structdef;
mod useitem;

use ra_ap_syntax::{ast, AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
pub use debug::{debug_children_with_tokens, debug_node_siblings};
pub use doccomment::format_preceding_docs_and_attrs;
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
            let mut last_kind: Option<SyntaxKind> = None;
            let mut has_seen_item = false;

            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => {
                        let current_kind = n.kind();
                        let is_item = matches!(
                            current_kind,
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

                        if is_item {
                            // Add blank line before items, except:
                            // 1. The very first item (has_seen_item is false)
                            // 2. Between consecutive USE statements
                            if has_seen_item {
                                if let Some(last) = last_kind {
                                    if !(current_kind == SyntaxKind::USE && last == SyntaxKind::USE)
                                    {
                                        buf.push('\n');
                                    }
                                }
                            }

                            format_node(&n, buf, indent);

                            has_seen_item = true;
                            last_kind = Some(current_kind);
                        } else {
                            format_node(&n, buf, indent);
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
