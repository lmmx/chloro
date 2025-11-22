use ra_ap_syntax::{
    ast::{self, HasVisibility},
    AstNode, SyntaxNode,
};

pub mod grouping;
pub mod sort;

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

/// Check if an item contains nested braces (e.g., "SyntaxKind::{self, *}")
fn has_nested_braces(item: &str) -> bool {
    item.contains('{')
}

pub fn format_use(node: &SyntaxNode, buf: &mut String, indent: usize) {
    write_indent(buf, indent);
    let use_ = match ast::Use::cast(node.clone()) {
        Some(u) => u,
        None => return,
    };

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

    // If it fits on one line, write it directly
    if single_line_len < MAX_WIDTH {
        // NOTE: Should be <= but there's an off-by-one bug, so use <
        // See: https://github.com/rust-lang/rustfmt/issues/6727
        buf.push_str(&vis_text);
        buf.push_str("use ");
        buf.push_str(&use_tree_text);
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

                // Sort items using standard lexicographic ordering
                let mut sorted_items = items;
                sorted_items.sort_by(|a, b| sort::sort_key(a).cmp(&sort::sort_key(b)));

                // Only group by submodule if no items have nested braces
                // If any item has nested braces, treat all items as one group
                let has_any_nested = sorted_items.iter().any(|item| has_nested_braces(item));

                let groups = if has_any_nested {
                    // Don't group - put all items in one group
                    vec![sorted_items]
                } else {
                    // Group items by their submodule prefix
                    grouping::group_by_submodule(sorted_items)
                };

                // Write out each group
                let line_indent = indent + 4;

                for (group_idx, group) in groups.iter().enumerate() {
                    // Check if this is a root-level group (items without ::)
                    let is_root_group = group.iter().all(|item| !item.contains("::"));

                    if is_root_group {
                        // Root-level items can be packed on lines
                        let mut current_line = String::new();

                        for item in group.iter() {
                            let item_with_comma = format!("{},", item);
                            let potential_line_len =
                                line_indent + current_line.len() + item_with_comma.len();

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
                    } else {
                        // Submodule items: one per line
                        for item in group.iter() {
                            write_indent(buf, line_indent);
                            buf.push_str(item);
                            buf.push_str(",\n");
                        }
                    }

                    // Add blank line between groups (except after the last group)
                    if group_idx < groups.len() - 1 {
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
                // Only split on commas at the top level
                let trimmed = current_item.trim().to_string();
                if !trimmed.is_empty() {
                    items.push(trimmed);
                }
                current_item.clear();
            }
            _ => {
                current_item.push(ch);
            }
        }
    }

    // Don't forget the last item
    let trimmed = current_item.trim().to_string();
    if !trimmed.is_empty() {
        items.push(trimmed);
    }

    items
}

#[cfg(test)]
mod tests;
