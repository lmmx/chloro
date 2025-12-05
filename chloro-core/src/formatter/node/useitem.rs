use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasVisibility},
};

pub mod grouping;
pub mod sort;

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::printer::Printer;
use crate::formatter::write_indent;

pub fn format_use(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let use_ = match ast::Use::cast(node.clone()) {
        Some(u) => u,
        None => return,
    };

    // Output leading non-doc comments (// style) that appear before visibility/keywords
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text();
                    // Skip doc comments - they're handled by doc_comments() below
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        buf.line(indent, text);
                    }
                } else if t.kind() != SyntaxKind::WHITESPACE {
                    // Hit a non-comment, non-whitespace token - stop
                    break;
                }
            }
            NodeOrToken::Node(_) => {
                // Hit a node (like VISIBILITY) - stop
                break;
            }
        }
    }

    // Handle attributes (like #[cfg(...)])
    buf.attrs(&use_, indent);

    buf.indent(indent);

    let vis_text = if let Some(vis) = use_.visibility() {
        format!("{} ", vis.syntax().text())
    } else {
        String::new()
    };

    let use_tree_text = if let Some(use_tree) = use_.use_tree() {
        use_tree.syntax().text().to_string()
    } else {
        String::new()
    };

    // Calculate the full single-line length
    let single_line = format!("{}use {};", vis_text, use_tree_text);
    let single_line_len = indent + single_line.len();

    // Check if we need multi-line formatting due to nested groups
    let has_nested_groups = if use_tree_text.contains('{') && use_tree_text.contains('}') {
        if let Some(open_brace) = use_tree_text.find('{') {
            let rest = &use_tree_text[open_brace + 1..];
            if let Some(close_brace) = rest.rfind('}') {
                let items_str = &rest[..close_brace];
                let items = parse_items_with_nested_braces(items_str);
                items
                    .iter()
                    .any(|item| item.contains('{') && item.contains(','))
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // If it fits on one line AND has no nested groups, write it directly
    if single_line_len < MAX_WIDTH && !has_nested_groups {
        // NOTE: Should be <= but there's an off-by-one bug, so use <
        // See: https://github.com/rust-lang/rustfmt/issues/6727
        buf.push_str(&vis_text);
        buf.push_str("use ");

        // Sort even for single-line output
        if use_tree_text.contains('{') {
            let sorted = sort_nested_items(&use_tree_text);
            buf.push_str(&sorted);
        } else {
            buf.push_str(&use_tree_text);
        }

        buf.push_str(";\n");
        return;
    }

    // Multi-line formatting for use statements with braced lists
    buf.push_str(&vis_text);
    buf.push_str("use ");

    // Parse the use tree to handle multi-line formatting
    if use_tree_text.contains('{') && use_tree_text.contains('}') {
        // Extract path prefix and items
        if let Some(open_brace) = use_tree_text.find('{') {
            let prefix = &use_tree_text[..open_brace];
            let rest = &use_tree_text[open_brace + 1..];

            if let Some(close_brace) = rest.rfind('}') {
                let items_str = &rest[..close_brace];

                // Write prefix and opening brace
                buf.push_str(prefix);
                buf.push_str("{\n");

                // Parse items carefully, respecting nested braces
                let items = parse_items_with_nested_braces(items_str);

                // Sort nested brace contents
                let items: Vec<_> = items.iter().map(|item| sort_nested_items(item)).collect();

                // Sort items using standard lexicographic ordering
                let mut sorted_items = items;
                sorted_items.sort_by_key(|a| sort::sort_key(a));

                // Group items by their submodule prefix
                let mut groups = grouping::group_by_submodule(sorted_items);

                // Sort within each group to maintain order
                for group in &mut groups {
                    group.sort_by_key(|a| sort::sort_key(a));
                }

                // Write out each group
                let line_indent = indent + 4;

                for group in groups.iter() {
                    // Pack items in this group onto lines
                    let mut current_line = String::new();

                    for item in group.iter() {
                        // Use format_item_with_nested_braces for nested formatting
                        let mut item_buf = String::new();
                        format_item_with_nested_braces(item, &mut item_buf, line_indent);

                        let item_with_comma = format!("{},", item_buf);
                        let potential_line_len = if current_line.is_empty() {
                            line_indent + item_with_comma.len()
                        } else {
                            line_indent + current_line.len() + 1 + item_with_comma.len() // +1 for the space
                        };

                        if current_line.is_empty() {
                            current_line.push_str(&item_with_comma);
                        } else if potential_line_len < MAX_WIDTH {
                            current_line.push(' ');
                            current_line.push_str(&item_with_comma);
                        } else {
                            write_indent(buf, line_indent);
                            buf.push_str(&current_line);
                            buf.push('\n');
                            current_line.clear();
                            current_line.push_str(&item_with_comma);
                        }
                    }

                    if !current_line.is_empty() {
                        write_indent(buf, line_indent);
                        buf.push_str(&current_line);
                        buf.push('\n');
                    }
                }

                write_indent(buf, indent);
                buf.push_str("};\n");
                return;
            }
        }
    }

    // Fallback: just write as-is if we can't parse it
    buf.push_str(&use_tree_text);
    buf.push_str(";\n");
}

/// Format an item, handling nested braces with proper indentation
fn format_item_with_nested_braces(item: &str, buf: &mut String, indent: usize) {
    if !item.contains('{') {
        buf.push_str(item);
        return;
    }

    if let Some(open_idx) = item.find('{') {
        let prefix = &item[..open_idx];
        let rest = &item[open_idx + 1..];

        if let Some(close_idx) = rest.rfind('}') {
            let inner = &rest[..close_idx];
            let inner_items = parse_items_with_nested_braces(inner);

            // Check if it fits on one line
            let single_line = format!("{}{{{}}}", prefix, inner_items.join(", "));
            if indent + single_line.len() < MAX_WIDTH {
                // Note off-by-one error: max width=100 means at most 99 chars per line
                buf.push_str(&single_line);
                return;
            }

            // Multi-line format
            buf.push_str(prefix);
            buf.push_str("{\n");

            // Format inner items (packed if they're root-level)
            let inner_indent = indent + 4;
            let mut current_line = String::new();

            for inner_item in inner_items {
                let item_with_comma = format!("{}, ", inner_item);
                let potential_len = inner_indent + current_line.len() + item_with_comma.len();

                if current_line.is_empty() || potential_len < MAX_WIDTH {
                    current_line.push_str(&item_with_comma);
                } else {
                    write_indent(buf, inner_indent);
                    buf.push_str(current_line.trim_end_matches(", "));
                    buf.push_str(",\n");
                    current_line.clear();
                    current_line.push_str(&item_with_comma);
                }
            }

            if !current_line.is_empty() {
                write_indent(buf, inner_indent);
                buf.push_str(current_line.trim_end_matches(", "));
                buf.push_str(",\n");
            }

            write_indent(buf, indent);
            buf.push('}');
            return;
        }
    }

    buf.push_str(item);
}

/// Parse items from a use tree, respecting nested braces.
///
/// This is more sophisticated than just splitting on commas because
/// items can contain nested braces like `SyntaxKind::{self, *}`.
fn parse_items_with_nested_braces(items_str: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current_item = String::new();
    let mut brace_depth = 0;

    for ch in items_str.chars() {
        match ch {
            '{' => {
                brace_depth += 1;
                current_item.push(ch);
            }
            '}' => {
                brace_depth -= 1;
                current_item.push(ch);
            }
            ',' if brace_depth == 0 => {
                let trimmed = current_item.trim().to_string();
                if !trimmed.is_empty() {
                    // Unpack singleton braces
                    items.push(unpack_singleton_braces(&trimmed));
                }
                current_item.clear();
            }
            _ => {
                current_item.push(ch);
            }
        }
    }

    let trimmed = current_item.trim().to_string();
    if !trimmed.is_empty() {
        items.push(unpack_singleton_braces(&trimmed));
    }

    items
}

/// Unpack singleton braces: `foo::{bar}` -> `foo::bar`
fn unpack_singleton_braces(item: &str) -> String {
    if let Some(open_idx) = item.find('{') {
        let prefix = &item[..open_idx];
        let rest = &item[open_idx + 1..];

        if let Some(close_idx) = rest.rfind('}') {
            let inner = rest[..close_idx].trim();

            // Check if it's a singleton (no commas at depth 0)
            if !inner.contains(',') && !inner.contains('{') {
                return format!("{}{}", prefix, inner);
            }
        }
    }
    item.to_string()
}

/// Recursively sort items within nested braces
fn sort_nested_items(item: &str) -> String {
    if !item.contains('{') {
        return item.to_string();
    }

    // Find the opening brace
    if let Some(open_idx) = item.find('{') {
        let prefix = &item[..open_idx + 1];
        let rest = &item[open_idx + 1..];

        if let Some(close_idx) = rest.rfind('}') {
            let inner = &rest[..close_idx];
            let suffix = &rest[close_idx..];

            // Parse and sort the inner items
            let inner_items = parse_items_with_nested_braces(inner);
            let mut sorted_inner = inner_items;
            sorted_inner.sort_by_key(|a| sort::sort_key(a));

            // Reconstruct
            let sorted_inner_str = sorted_inner.join(", ");
            return format!("{}{}{}", prefix, sorted_inner_str, suffix);
        }
    }

    item.to_string()
}

#[cfg(test)]
mod tests;
