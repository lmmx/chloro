use ra_ap_syntax::ast::{self, HasName};
use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind};

use crate::formatter::printer::Printer;

/// Collect non-doc comments from inside a node (before the name)
fn collect_inner_comments(node: &ra_ap_syntax::SyntaxNode) -> Vec<String> {
    node.children_with_tokens()
        .take_while(|child| !matches!(child, NodeOrToken::Node(n) if n.kind() == SyntaxKind::NAME))
        .filter_map(|child| match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                let text = t.text().to_string();
                (!text.starts_with("///") && !text.starts_with("//!")).then_some(text)
            }
            _ => None,
        })
        .collect()
}

/// Format record fields with their comments.
pub fn format_record_fields(fields: &ast::RecordFieldList, buf: &mut String, indent: usize) {
    for field in fields.fields() {
        for comment in collect_inner_comments(field.syntax()) {
            buf.line(indent, &comment);
        }
        buf.doc_comments(&field, indent);
        buf.attrs(&field, indent);
        buf.indent(indent);
        buf.visibility(&field);
        if let Some(name) = field.name() {
            buf.push_str(&name.text());
        }
        buf.push_str(": ");
        if let Some(ty) = field.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }
        buf.push_str(",\n");
    }
}
