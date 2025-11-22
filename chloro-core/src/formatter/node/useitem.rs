use ra_ap_syntax::{
    ast::{self, HasVisibility},
    AstNode, SyntaxNode,
};

pub mod grouping;
pub mod sort;

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

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
        // NOTE: rustfmt max_width implementation is (unfortunately) off by one.
        // This should be `<=` but make it `<` to match behaviour of rustfmt
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

                // Parse and sort items lexicographically
                let mut items: Vec<String> = items_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                // Sort items using standard lexicographic ordering
                items.sort_by(|a, b| sort::sort_key(a).cmp(&sort::sort_key(b)));

                // Group items by their submodule prefix
                let groups = grouping::group_by_submodule(items);

                // Write out each group
                let line_indent = indent + 4;

                for (group_idx, group) in groups.iter().enumerate() {
                    let mut current_line = String::new();

                    for item in group.iter() {
                        let item_with_comma = format!("{},", item);

                        // Check if adding this item would exceed MAX_WIDTH
                        // NOTE: technically whether it would *reach* MAX_WIDTH (rustfmt bug)
                        let potential_line_len =
                            line_indent + current_line.len() + item_with_comma.len();

                        if current_line.is_empty() {
                            // First item on the line
                            current_line.push_str(&item_with_comma);
                        } else if potential_line_len < MAX_WIDTH {
                            // NOTE: `<` not `<=` to match behaviour of rustfmt
                            // Add to current line with a space
                            current_line.push(' ');
                            current_line.push_str(&item_with_comma);
                        } else {
                            // Write current line and start new one
                            write_indent(buf, line_indent);
                            buf.push_str(&current_line);
                            buf.push('\n');
                            current_line.clear();
                            current_line.push_str(&item_with_comma);
                        }
                    }

                    // Write any remaining items
                    if !current_line.is_empty() {
                        write_indent(buf, line_indent);
                        buf.push_str(&current_line);
                        buf.push('\n');
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

#[cfg(test)]
mod tests;
