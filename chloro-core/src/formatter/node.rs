mod block;
mod const_static;
mod debug;
mod enumdef;
mod function;
mod implblock;
mod imports;
mod module;
mod structdef;
mod traitdef;
mod typealias;
mod useitem;

use ra_ap_syntax::{ast, AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

pub use block::{format_block, format_block_expr_contents, format_stmt_list};
pub use const_static::format_const_or_static;
#[allow(unused_imports)]
pub use debug::{debug_children_with_tokens, debug_node_siblings};
pub use enumdef::format_enum;
pub use function::format_function;
pub use implblock::format_impl;
pub use imports::sort_and_format_imports;
pub use module::format_module;
pub use structdef::format_struct;
pub use traitdef::format_trait;
pub use typealias::format_type_alias;
pub use useitem::format_use;

/// Determine if a blank line should be added between two items
fn should_add_blank_line(prev_kind: Option<SyntaxKind>, curr_kind: SyntaxKind) -> bool {
    let Some(prev) = prev_kind else {
        return false;
    };

    // No blank line between consecutive uses
    if prev == SyntaxKind::USE && curr_kind == SyntaxKind::USE {
        return false;
    }

    // Blank line between different top-level items
    matches!(
        prev,
        SyntaxKind::FN
            | SyntaxKind::STRUCT
            | SyntaxKind::ENUM
            | SyntaxKind::IMPL
            | SyntaxKind::MODULE
            | SyntaxKind::CONST
            | SyntaxKind::STATIC
            | SyntaxKind::TYPE_ALIAS
            | SyntaxKind::TRAIT
            | SyntaxKind::MACRO_RULES
            | SyntaxKind::MACRO_DEF
    ) && matches!(
        curr_kind,
        SyntaxKind::FN
            | SyntaxKind::STRUCT
            | SyntaxKind::ENUM
            | SyntaxKind::IMPL
            | SyntaxKind::MODULE
            | SyntaxKind::CONST
            | SyntaxKind::STATIC
            | SyntaxKind::TYPE_ALIAS
            | SyntaxKind::USE
            | SyntaxKind::TRAIT
            | SyntaxKind::MACRO_RULES
            | SyntaxKind::MACRO_DEF
    )
}

/// Main node formatting dispatcher
pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            // Separate items by type, preserving order within each type
            let mut inner_attrs = Vec::new();
            let mut module_inner_docs = Vec::new();
            let mut extern_crates = Vec::new();
            let mut mod_decls = Vec::new();
            let mut uses = Vec::new();
            let mut other_items = Vec::new();

            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => {
                        match n.kind() {
                            SyntaxKind::ATTR => {
                                // Check if it's an inner attribute (#![...])
                                if let Some(attr) = ast::Attr::cast(n.clone()) {
                                    if attr.excl_token().is_some() {
                                        inner_attrs.push(attr);
                                    } else {
                                        // Outer attributes belong to the next item
                                        other_items.push(n);
                                    }
                                }
                            }
                            SyntaxKind::EXTERN_CRATE => {
                                extern_crates.push(n);
                            }
                            SyntaxKind::MODULE => {
                                // Only mod declarations (not inline mod blocks)
                                if let Some(module) = ast::Module::cast(n.clone()) {
                                    if module.item_list().is_none() {
                                        mod_decls.push(n);
                                        continue;
                                    }
                                }
                                other_items.push(n);
                            }
                            SyntaxKind::USE => {
                                if let Some(use_) = ast::Use::cast(n) {
                                    uses.push(use_);
                                }
                            }
                            _ => {
                                other_items.push(n);
                            }
                        }
                    }
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::COMMENT {
                            if let Some(comment) = ast::Comment::cast(t) {
                                // Inner doc comments (//! or /*! ... */)
                                if comment.is_inner() && comment.kind().doc.is_some() {
                                    module_inner_docs.push(comment);
                                }
                            }
                        }
                    }
                }
            }

            // Format in strict order:
            // 1. Inner attributes
            for attr in &inner_attrs {
                buf.push_str(attr.syntax().text().to_string().as_str());
                buf.push('\n');
            }

            // 2. Module-level inner doc comments
            for doc in &module_inner_docs {
                buf.push_str(doc.text());
                buf.push('\n');
            }

            // Blank line after preamble attributes/docs if we have more content
            let has_preamble = !inner_attrs.is_empty() || !module_inner_docs.is_empty();
            let has_content = !extern_crates.is_empty()
                || !mod_decls.is_empty()
                || !uses.is_empty()
                || !other_items.is_empty();

            if has_preamble && has_content {
                buf.push('\n');
            }

            // 3. Extern crate declarations
            for extern_crate in &extern_crates {
                format_node(extern_crate, buf, indent);
            }
            if !extern_crates.is_empty()
                && (!mod_decls.is_empty() || !uses.is_empty() || !other_items.is_empty())
            {
                buf.push('\n');
            }

            // 4. Module declarations (without blank lines between them)
            for mod_decl in mod_decls.iter() {
                format_node(mod_decl, buf, indent);
                // No blank line between consecutive mod declarations
            }
            if !mod_decls.is_empty() && (!uses.is_empty() || !other_items.is_empty()) {
                buf.push('\n');
            }

            // 5. Use statements (sorted and grouped)
            if !uses.is_empty() {
                sort_and_format_imports(&uses, buf, indent);
                if !other_items.is_empty() {
                    buf.push('\n');
                }
            }

            // 6. Everything else with appropriate blank lines
            let mut last_kind: Option<SyntaxKind> = if !uses.is_empty() {
                Some(SyntaxKind::USE)
            } else if !mod_decls.is_empty() {
                Some(SyntaxKind::MODULE)
            } else {
                None
            };

            for node in other_items {
                let current_kind = node.kind();

                // Add blank line if needed
                if should_add_blank_line(last_kind, current_kind) {
                    buf.push('\n');
                }

                format_node(&node, buf, indent);
                last_kind = Some(current_kind);
            }
        }

        SyntaxKind::FN => format_function(node, buf, indent),
        SyntaxKind::STRUCT => format_struct(node, buf, indent),
        SyntaxKind::ENUM => format_enum(node, buf, indent),
        SyntaxKind::IMPL => format_impl(node, buf, indent),
        SyntaxKind::TRAIT => format_trait(node, buf, indent),
        SyntaxKind::USE => format_use(node, buf, indent),
        SyntaxKind::MODULE => format_module(node, buf, indent),
        SyntaxKind::TYPE_ALIAS => format_type_alias(node, buf, indent),
        SyntaxKind::CONST => format_const_or_static(node, buf, indent),
        SyntaxKind::STATIC => format_const_or_static(node, buf, indent),

        SyntaxKind::BLOCK_EXPR => format_block(node, buf, indent),
        SyntaxKind::STMT_LIST => format_stmt_list(node, buf, indent),

        SyntaxKind::MACRO_RULES | SyntaxKind::MACRO_DEF => {
            // Preserve macro definitions as-is for now
            crate::formatter::write_indent(buf, indent);
            buf.push_str(&node.text().to_string());
            buf.push('\n');
        }

        SyntaxKind::ATTR => {
            // Handle standalone attributes
            if let Some(attr) = ast::Attr::cast(node.clone()) {
                crate::formatter::write_indent(buf, indent);
                buf.push_str(attr.syntax().text().to_string().as_str());
                buf.push('\n');
            }
        }

        _ => {
            // Default: recurse through children
            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(n) => format_node(&n, buf, indent),
                    NodeOrToken::Token(t) => format_token(&t, buf, indent),
                }
            }
        }
    }
}

/// Format a single token (comments, whitespace, keywords, etc.)
fn format_token(token: &SyntaxToken, buf: &mut String, _indent: usize) {
    match token.kind() {
        SyntaxKind::COMMENT => {
            // Handle comments specially - preserve them exactly
            if let Some(comment) = ast::Comment::cast(token.clone()) {
                buf.push_str(comment.text());
                buf.push('\n');
            }
        }
        SyntaxKind::WHITESPACE => {
            // Skip whitespace - let the formatter control all spacing
        }
        _ => {
            // Other tokens shouldn't appear in top-level context
        }
    }
}
