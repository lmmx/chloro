use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasVisibility},
};

use crate::formatter::node::common::{fields, header};
use crate::formatter::printer::Printer;

pub fn format_struct(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let strukt = match ast::Struct::cast(node.clone()) {
        Some(s) => s,
        None => return,
    };

    // Header
    header::format_item_header(&strukt, "struct", buf, indent);

    if let Some(field_list) = strukt.field_list() {
        match field_list {
            ast::FieldList::RecordFieldList(record_fields) => {
                let fields_vec: Vec<_> = record_fields.fields().collect();

                // Check if any field has a default initializer
                let has_default_initializer = fields_vec.iter().any(|field| field.expr().is_some());

                // Check if there are any comments in the field list or inside fields
                let has_comments_in_list = record_fields
                    .syntax()
                    .children_with_tokens()
                    .any(|child| matches!(child, NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT));

                let has_comments_in_fields = fields_vec.iter().any(|field| {
                    field
                        .syntax()
                        .descendants_with_tokens()
                        .any(|child| matches!(child, NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT))
                });

                let has_comments = has_comments_in_list || has_comments_in_fields;

                // Single-line if has default initializers and no comments
                if has_default_initializer && !has_comments {
                    let fields_str: Vec<_> = fields_vec
                        .iter()
                        .map(|f| f.syntax().text().to_string())
                        .collect();
                    buf.push_str(&format!(" {{ {} }}", fields_str.join(", ")));
                    buf.push('\n');
                    return;
                }

                // Multi-line format
                buf.open_brace();
                fields::format_record_fields(&record_fields, buf, indent + 4);
                buf.close_brace_ln(indent);
            }
            ast::FieldList::TupleFieldList(tuple_fields) => {
                buf.push('(');
                for (idx, field) in tuple_fields.fields().enumerate() {
                    if idx > 0 {
                        buf.push_str(", ");
                    }
                    if let Some(vis) = field.visibility() {
                        buf.push_str(&vis.syntax().text().to_string());
                        buf.push(' ');
                    }
                    if let Some(ty) = field.ty() {
                        buf.push_str(&ty.syntax().text().to_string());
                    }
                }
                buf.newline(");");
            }
        }
    } else {
        buf.newline(";");
    }
}
