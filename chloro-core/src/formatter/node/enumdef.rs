// chloro-core/src/formatter/node/enumdef.rs
use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasName, HasVisibility},
};

use crate::formatter::node::common::{comments, fields, header};
use crate::formatter::write_indent;

pub fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    // Header: docs, attrs, visibility, "enum", name, generics
    header::format_item_header(&enum_, "enum", buf, indent);

    if let Some(variants) = enum_.variant_list() {
        buf.push_str(" {\n");

        // Process children to handle comments between variants
        let children: Vec<_> = variants.syntax().children_with_tokens().collect();

        for (idx, child) in children.iter().enumerate() {
            match child {
                NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                    // If not attached to next variant, print standalone comment
                    if !comments::is_comment_attached_to_next_variant(&children, idx) {
                        if comments::should_have_blank_line_before_comment(&children, idx) {
                            buf.push('\n');
                        }
                        write_indent(buf, indent + 4);
                        buf.push_str(t.text());
                        buf.push('\n');
                    }
                }
                NodeOrToken::Node(n) if n.kind() == SyntaxKind::VARIANT => {
                    if let Some(variant) = ast::Variant::cast(n.clone()) {
                        // Comments immediately before this variant
                        let comments_before =
                            comments::collect_preceding_comments_in_list(&children, idx);
                        for comment in &comments_before {
                            write_indent(buf, indent + 4);
                            buf.push_str(comment);
                            buf.push('\n');
                        }

                        // Variant doc comments (///)
                        for doc_comment in variant.doc_comments() {
                            write_indent(buf, indent + 4);
                            buf.push_str(doc_comment.text().trim());
                            buf.push('\n');
                        }

                        // Variant attributes
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
                                ast::FieldList::RecordFieldList(record_fields) => {
                                    buf.push_str(" {\n");
                                    fields::format_record_fields(&record_fields, buf, indent + 8);
                                    write_indent(buf, indent + 4);
                                    buf.push('}');
                                }
                                ast::FieldList::TupleFieldList(fields_tuple) => {
                                    buf.push('(');
                                    for (i, field) in fields_tuple.fields().enumerate() {
                                        if i > 0 {
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

                        // Check for trailing comment on same line as variant
                        if let Some(trailing) = comments::get_trailing_comment(&children, idx) {
                            if buf.ends_with(",\n") {
                                buf.pop(); // remove \n
                                buf.push(' ');
                                buf.push_str(&trailing);
                                buf.push('\n');
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}
