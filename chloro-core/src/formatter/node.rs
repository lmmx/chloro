mod block;
mod enumdef;
mod function;
mod implblock;
mod module;
mod structdef;
mod useitem;

use ra_ap_syntax::{SyntaxKind, SyntaxNode};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
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
            for child in node.children() {
                format_node(&child, buf, indent);
                // Add spacing between top-level items
                if matches!(
                    child.kind(),
                    SyntaxKind::FN | SyntaxKind::STRUCT | SyntaxKind::ENUM | SyntaxKind::IMPL
                ) {
                    buf.push('\n');
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

        // Skip whitespace and comments in first pass (simplified)
        SyntaxKind::WHITESPACE => {}
        SyntaxKind::COMMENT => {
            crate::formatter::write_indent(buf, indent);
            buf.push_str(&node.text().to_string());
            buf.push('\n');
        }

        _ => {
            // Default: recurse on children
            for child in node.children() {
                format_node(&child, buf, indent);
            }
        }
    }
}
