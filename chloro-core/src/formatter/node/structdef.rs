use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasVisibility},
};

use crate::formatter::config::MAX_WIDTH;
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

                // Check if we can format on a single line (single field, no comments)
                if fields_vec.len() == 1 && !has_comments {
                    let field = &fields_vec[0];
                    let field_text = field.syntax().text().to_string();

                    // Build single-line version: " { field_text }"
                    let single_line = format!(" {{ {} }}", field_text);

                    let current_line_len = buf.lines().last().map(|l| l.len()).unwrap_or(0);

                    if current_line_len + single_line.len() <= MAX_WIDTH {
                        buf.push_str(&single_line);
                        buf.push('\n');
                        return;
                    }
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
