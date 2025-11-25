//! Shared comment handling utilities for the formatter.

use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

/// Collect non-doc comments that appear immediately before a syntax node.
/// Stops at blank lines (2+ newlines in whitespace).
pub fn collect_preceding_comments(node: &SyntaxNode) -> Vec<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling_or_token();

    while let Some(p) = prev {
        match &p {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments (they're handled by HasDocComments)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Check for blank lines - if there are 2+ newlines, stop collecting
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                } else {
                    break;
                }
                prev = t.prev_sibling_or_token();
            }
            NodeOrToken::Node(_) => break,
        }
    }

    comments.reverse();
    comments
}

/// Collect comments immediately before an item at the given index in a list of children.
pub fn collect_preceding_comments_in_list(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    item_idx: usize,
) -> Vec<String> {
    let mut comments = Vec::new();

    for i in (0..item_idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text().to_string();
                    // Skip doc comments
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Stop at blank lines
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                } else {
                    break;
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }

    comments.reverse();
    comments
}

/// Check if a comment at the given index is attached to the next item.
pub fn is_comment_attached_to_next_item(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
    item_kinds: &[SyntaxKind],
) -> bool {
    for i in (comment_idx + 1)..children.len() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    // If there's a blank line, the comment is not attached
                    if t.text().matches('\n').count() >= 2 {
                        return false;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    return false;
                }
            }
            NodeOrToken::Node(n) => {
                // Found a node - check if it's one of the item kinds we care about
                return item_kinds.contains(&n.kind());
            }
        }
    }
    false
}

/// Check if there should be a blank line before the item at the given index.
pub fn should_have_blank_line_before(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    idx: usize,
) -> bool {
    // Look backwards for whitespace with 2+ newlines
    for i in (0..idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    break;
                }
            }
            NodeOrToken::Node(_) => break,
        }
    }
    false
}
