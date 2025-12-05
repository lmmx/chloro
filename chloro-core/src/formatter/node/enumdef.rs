use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasName, HasVisibility},
};

use crate::formatter::node::common::{fields, header};
use crate::formatter::printer::Printer;

/// Information about a variant for formatting
struct VariantInfo {
    variant: ast::Variant,
    leading_comments: Vec<String>,
    trailing_comment: Option<String>,
    has_blank_line_before: bool,
}

/// Pre-scan all variants to correctly assign trailing comments.
///
/// Due to how rust-analyzer parses, a trailing comment like:
///   `Foo, // comment`
/// is actually attached as a leading child of the NEXT variant, not as a
/// sibling after the current variant. We detect this by checking if the
/// first token in a variant is a comment with no preceding newline.
fn collect_variant_info(variants: &ast::VariantList) -> Vec<VariantInfo> {
    let mut result: Vec<VariantInfo> = Vec::new();
    let variant_list: Vec<_> = variants.variants().collect();

    for (idx, variant) in variant_list.iter().enumerate() {
        let mut leading_comments = Vec::new();
        let mut trailing_comment_for_prev: Option<String> = None;
        let mut seen_newline = false;

        // Collect comments from inside the variant node (before the name)
        for child in variant.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::COMMENT {
                        let text = t.text().to_string();
                        // Skip doc comments (handled by HasDocComments)
                        if !text.starts_with("///") && !text.starts_with("//!") {
                            if !seen_newline && trailing_comment_for_prev.is_none() {
                                // First comment before any newline - trailing for previous
                                trailing_comment_for_prev = Some(text);
                            } else {
                                leading_comments.push(text);
                            }
                        }
                    } else if t.kind() == SyntaxKind::WHITESPACE && t.text().contains('\n') {
                        seen_newline = true;
                        // Don't move trailing_comment_for_prev to leading - it stays as trailing for previous variant
                    }
                }
                NodeOrToken::Node(n) => {
                    if n.kind() == SyntaxKind::NAME {
                        break;
                    }
                }
            }
        }

        // If we still have a trailing_comment_for_prev and there's a previous variant,
        // attach it to that variant
        if let Some(comment) = trailing_comment_for_prev {
            if !result.is_empty() {
                result.last_mut().unwrap().trailing_comment = Some(comment);
            } else {
                // No previous variant, treat as leading comment
                leading_comments.insert(0, comment);
            }
        }

        // Check for blank line before
        let has_blank_line_before = if idx > 0 {
            has_blank_line_before_variant(variant.syntax())
        } else {
            false
        };

        // Check for trailing comment as sibling (after comma) - this handles
        // comments that ARE siblings, like the last variant's trailing comment
        let trailing_from_sibling = get_trailing_comment_sibling(variant.syntax());

        result.push(VariantInfo {
            variant: variant.clone(),
            leading_comments,
            trailing_comment: trailing_from_sibling,
            has_blank_line_before,
        });
    }

    result
}

/// Get a trailing comment on the same line as a variant (checking siblings after comma)
fn get_trailing_comment_sibling(node: &SyntaxNode) -> Option<String> {
    let mut next = node.next_sibling_or_token();
    while let Some(item) = next {
        match &item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    return Some(t.text().to_string());
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().contains('\n') {
                        return None;
                    }
                } else if t.kind() == SyntaxKind::COMMA {
                    // Continue past the comma to look for trailing comment
                } else {
                    return None;
                }
                next = t.next_sibling_or_token();
            }
            NodeOrToken::Node(_) => return None,
        }
    }
    None
}

/// Check if there's a blank line before this variant node
fn has_blank_line_before_variant(node: &SyntaxNode) -> bool {
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
        buf.open_brace();

        // Pre-scan to correctly assign trailing comments
        let variant_infos = collect_variant_info(&variants);

        for (idx, info) in variant_infos.iter().enumerate() {
            let variant = &info.variant;

            // Check for blank line before this variant (to preserve spacing)
            if idx > 0 && info.has_blank_line_before {
                buf.blank();
            }

            // Output leading comments
            for comment in &info.leading_comments {
                buf.line(indent + 4, comment);
            }

            // Variant doc comments (///)
            for doc_comment in variant.doc_comments() {
                buf.line(indent + 4, doc_comment.text().trim());
            }

            // Variant attributes
            for attr in variant.attrs() {
                buf.line(indent + 4, &attr.syntax().text().to_string());
            }

            buf.indent(indent + 4);
            if let Some(name) = variant.name() {
                buf.push_str(&name.text());
            }
            if let Some(field_list) = variant.field_list() {
                match field_list {
                    ast::FieldList::RecordFieldList(record_fields) => {
                        buf.open_brace();
                        fields::format_record_fields(&record_fields, buf, indent + 8);
                        buf.close_brace(indent + 4);
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

            // Check for trailing comment on same line
            if let Some(ref trailing) = info.trailing_comment {
                buf.push_str(", ");
                buf.newline(trailing);
            } else {
                buf.newline(",");
            }
        }

        buf.close_brace(indent);
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}
