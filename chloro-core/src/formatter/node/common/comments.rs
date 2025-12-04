use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

/// Collect comments immediately before an item at the given index in a children list.
///
/// This includes ALL comment types (// and ///), but higher-level callers should
/// skip doc comments if they are handled via HasDocComments.
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
                    // Skip doc comments (they're normally handled by HasDocComments at item-level)
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        comments.push(text);
                    }
                } else if t.kind() == SyntaxKind::WHITESPACE {
                    // Stop at blank lines (2+ newlines)
                    if t.text().matches('\n').count() >= 2 {
                        break;
                    }
                    // else continue
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

/// Check if there should be a blank line before a comment at the given index
pub fn should_have_blank_line_before_comment(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
) -> bool {
    for i in (0..comment_idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() == SyntaxKind::COMMENT {
                    continue;
                } else {
                    return false;
                }
            }
            NodeOrToken::Node(_) => return false,
        }
    }
    false
}

/// Generic check whether a comment at the given index is attached to the next item
/// where attachment is defined by the `item_kinds` argument.
///
/// Returns `true` when the comment is immediately (no blank line) followed by an item
/// whose SyntaxKind is included in `item_kinds`.
pub fn is_comment_attached_to_next_item(
    children: &[NodeOrToken<SyntaxNode, SyntaxToken>],
    comment_idx: usize,
    item_kinds: &[SyntaxKind],
) -> bool {
    for child_item in children.iter().skip(comment_idx + 1) {
        match &child_item {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    // If there's a blank line, the comment is not attached
                    if t.text().matches('\n').count() >= 2 {
                        return false;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    // Some other token between comment and node (e.g. comma) -> not attached
                    return false;
                }
            }
            NodeOrToken::Node(n) => {
                // Found a node — decide whether its kind is one of the target kinds
                return item_kinds.contains(&n.kind());
            }
        }
    }
    false
}
