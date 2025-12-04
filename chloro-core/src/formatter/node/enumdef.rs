use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasName, HasVisibility},
};

use crate::formatter::node::common::{fields, header};
use crate::formatter::printer::Printer;
use crate::formatter::write_indent;

/// Collect non-doc comments from inside a variant node (before the name)
fn collect_inner_comments(node: &SyntaxNode) -> Vec<String> {
    let mut comments = Vec::new();

    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments (handled by HasDocComments)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                }
            }
            NodeOrToken::Node(n) => {
                // Stop when we hit the NAME node - comments after that are trailing
                if n.kind() == SyntaxKind::NAME {
                    break;
                }
            }
        }
    }

    comments
}

fn get_trailing_comment(node: &SyntaxNode) -> Option<String> {
    let mut next = node.next_sibling_or_token();
    while let Some(item) = next {
        match &item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    return Some(t.text().to_string());
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Only continue if no newline
                    if t.text().contains('\n') {
                        return None;
                    }
                    next = t.next_sibling_or_token();
                    continue;
                } else {
                    return None;
                }
            }
            NodeOrToken::Node(_) => return None,
        }
    }
    None
}

/// Check if there's a blank line before this node (looking at preceding siblings)
fn has_blank_line_before(node: &SyntaxNode) -> bool {
    let mut current = node.prev_sibling_or_token();

    while let Some(item) = current {
        match &item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() == SyntaxKind::COMMENT || t.kind() == SyntaxKind::COMMA {
                    // Continue past comments and commas
                } else {
                    return false;
                }
                current = t.prev_sibling_or_token();
            }
            NodeOrToken::Node(_) => return false,
        }
    }
    false
}

pub fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    // Header: docs, attrs, visibility, "enum", name, generics
    header::format_item_header(&enum_, "enum", buf, indent);

    if let Some(variants) = enum_.variant_list() {
        buf.push_str(" {\n");

        let mut first_variant = true;

        for variant in variants.variants() {
            // Check for blank line before this variant (to preserve spacing)
            if !first_variant && has_blank_line_before(variant.syntax()) {
                buf.push('\n');
            }

            // Collect comments from inside the variant node (before the name)
            let comments_before = collect_inner_comments(variant.syntax());
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
            if let Some(trailing) = get_trailing_comment(variant.syntax()) {
                buf.push_str(", ");
                buf.newline(&trailing);
            } else {
                buf.newline(",");
            }

            first_variant = false;
        }

        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}
