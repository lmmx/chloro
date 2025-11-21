use ra_ap_syntax::{
    ast::{self, HasVisibility},
    AstNode, SyntaxNode,
};

use crate::formatter::write_indent;

pub fn format_use(node: &SyntaxNode, buf: &mut String, indent: usize) {
    write_indent(buf, indent);

    let use_ = match ast::Use::cast(node.clone()) {
        Some(u) => u,
        None => return,
    };

    if let Some(vis) = use_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("use ");

    if let Some(use_tree) = use_.use_tree() {
        buf.push_str(&use_tree.syntax().text().to_string());
    }

    buf.push_str(";\n");
}
