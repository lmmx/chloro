use ra_ap_syntax::{
    AstNode, SyntaxNode,
    ast::{self, HasVisibility},
};

use crate::formatter::node::common::{fields, header};
use crate::formatter::write_indent;

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
                buf.push_str(" {\n");
                fields::format_record_fields(&record_fields, buf, indent + 4);
                write_indent(buf, indent);
                buf.push('}');
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
                buf.push_str(");");
            }
        }
    } else {
        buf.push(';');
    }
    buf.push('\n');
}
