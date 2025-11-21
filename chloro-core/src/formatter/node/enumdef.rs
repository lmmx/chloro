use ra_ap_syntax::{
    ast::{self, HasGenericParams, HasName, HasVisibility},
    AstNode, SyntaxNode,
};

use crate::formatter::write_indent;

pub fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    write_indent(buf, indent);

    if let Some(vis) = enum_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("enum ");

    if let Some(name) = enum_.name() {
        buf.push_str(&name.text());
    }

    if let Some(generics) = enum_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    if let Some(variants) = enum_.variant_list() {
        buf.push_str(" {\n");
        for variant in variants.variants() {
            write_indent(buf, indent + 4);
            if let Some(name) = variant.name() {
                buf.push_str(&name.text());
            }
            if let Some(field_list) = variant.field_list() {
                match field_list {
                    ast::FieldList::RecordFieldList(_) => {
                        buf.push_str(" { ... }");
                    }
                    ast::FieldList::TupleFieldList(fields) => {
                        buf.push('(');
                        for (idx, field) in fields.fields().enumerate() {
                            if idx > 0 {
                                buf.push_str(", ");
                            }
                            if let Some(ty) = field.ty() {
                                buf.push_str(&ty.syntax().text().to_string());
                            }
                        }
                        buf.push(')');
                    }
                }
            }
            buf.push_str(",\n");
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}
