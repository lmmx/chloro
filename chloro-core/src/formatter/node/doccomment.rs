use crate::formatter::write_indent;
use ra_ap_syntax::{ast, AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode};

/// Collect and format doc comments that precede an item
/// (This does NOT include #[...] attributes - those are handled separately)
pub fn format_preceding_docs_and_attrs(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let mut docs = Vec::new();

    // Walk backwards from this node to collect preceding comments and attributes
    let mut prev = node.prev_sibling_or_token();

    while let Some(sibling) = prev {
        match sibling {
            NodeOrToken::Token(t) => {
                match t.kind() {
                    SyntaxKind::WHITESPACE => {
                        // Skip whitespace but keep looking
                        prev = t.prev_sibling_or_token();
                        continue;
                    }
                    SyntaxKind::COMMENT => {
                        if let Some(comment) = ast::Comment::cast(t.clone()) {
                            // Only collect outer doc comments (/// or /** ... */)
                            // Inner doc comments (//! or /*! ... */) belong to the parent
                            if comment.kind().doc.is_some() && !comment.is_inner() {
                                docs.push(comment.text().to_string());
                                prev = t.prev_sibling_or_token();
                                continue;
                            }
                        }
                        // Non-doc comment, stop here
                        break;
                    }
                    _ => break,
                }
            }
            NodeOrToken::Node(n) => {
                // Check if it's an outer attribute (#[...])
                if n.kind() == SyntaxKind::ATTR {
                    if let Some(attr) = ast::Attr::cast(n.clone()) {
                        // Only collect outer attributes, not inner ones (#![...])
                        let attr_text = attr.syntax().text().to_string();
                        if !attr_text.starts_with("#![") {
                            docs.push(attr_text);
                            prev = n.prev_sibling_or_token();
                            continue;
                        }
                    }
                }
                // Any other node, stop
                break;
            }
        }
    }

    // Docs were collected in reverse order, so reverse them back
    for doc in docs.iter().rev() {
        write_indent(buf, indent);
        buf.push_str(doc);
        buf.push('\n');
    }
}
