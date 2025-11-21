use ra_ap_syntax::{
    ast::{self, HasAttrs, HasGenericParams, HasName, HasVisibility},
    AstNode, SyntaxNode,
};

use super::format_preceding_docs_and_attrs;
use crate::formatter::write_indent;

pub fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    // Format preceding doc comments and attributes
    format_preceding_docs_and_attrs(node, buf, indent);

    // Format attributes from the AST (like #[derive(...)])
    for attr in enum_.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

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
            // Format variant doc comments and attributes
            format_preceding_docs_and_attrs(variant.syntax(), buf, indent + 4);

            // Format variant attributes
            for attr in variant.attrs() {
                write_indent(buf, indent + 4);
                buf.push_str(&attr.syntax().text().to_string());
                buf.push('\n');
            }

            write_indent(buf, indent + 4);
            if let Some(name) = variant.name() {
                buf.push_str(&name.text());
            }
            if let Some(field_list) = variant.field_list() {
                match field_list {
                    ast::FieldList::RecordFieldList(fields) => {
                        buf.push_str(" {\n");
                        for field in fields.fields() {
                            // Format field doc comments
                            format_preceding_docs_and_attrs(field.syntax(), buf, indent + 8);

                            // Format field attributes
                            for attr in field.attrs() {
                                write_indent(buf, indent + 8);
                                buf.push_str(&attr.syntax().text().to_string());
                                buf.push('\n');
                            }

                            write_indent(buf, indent + 8);
                            if let Some(vis) = field.visibility() {
                                buf.push_str(&vis.syntax().text().to_string());
                                buf.push(' ');
                            }
                            if let Some(name) = field.name() {
                                buf.push_str(&name.text());
                            }
                            buf.push_str(": ");
                            if let Some(ty) = field.ty() {
                                buf.push_str(&ty.syntax().text().to_string());
                            }
                            buf.push_str(",\n");
                        }
                        write_indent(buf, indent + 4);
                        buf.push('}');
                    }
                    ast::FieldList::TupleFieldList(fields) => {
                        buf.push('(');
                        for (idx, field) in fields.fields().enumerate() {
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
