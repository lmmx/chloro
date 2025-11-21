use ra_ap_syntax::{SyntaxKind, SyntaxNode};

use crate::formatter::write_indent;

pub fn format_block(node: &SyntaxNode, buf: &mut String, indent: usize) {
    buf.push_str("{\n");
    format_block_expr_contents(node, buf, indent + 4);
    write_indent(buf, indent);
    buf.push('}');
}

pub fn format_stmt_list(node: &SyntaxNode, buf: &mut String, indent: usize) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::WHITESPACE | SyntaxKind::COMMENT => continue,
            _ => {
                write_indent(buf, indent);
                let text = child.text().to_string();
                buf.push_str(&text);
                if !text.ends_with(';') && !text.ends_with('}') {
                    buf.push(';');
                }
                buf.push('\n');
            }
        }
    }
}

pub fn format_block_expr_contents(node: &SyntaxNode, buf: &mut String, indent: usize) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::STMT_LIST => format_stmt_list(&child, buf, indent),
            SyntaxKind::WHITESPACE => continue,
            _ => {
                write_indent(buf, indent);
                buf.push_str(&child.text().to_string());
                buf.push('\n');
            }
        }
    }
}
