#![allow(dead_code)]

use ra_ap_syntax::{NodeOrToken, SyntaxNode};

#[cfg(debug_assertions)]
pub fn debug_node_siblings(node: &SyntaxNode, label: &str, max_depth: usize) {
    eprintln!("=== {} ===", label);
    eprintln!("Node prev_sibling_or_token:");
    let mut prev = node.prev_sibling_or_token();
    let mut i = 0;
    while let Some(p) = prev {
        match &p {
            NodeOrToken::Token(t) => {
                eprintln!("  [{}] Token: {:?} = {:?}", i, t.kind(), t.text());
            }
            NodeOrToken::Node(n) => {
                eprintln!("  [{}] Node: {:?}", i, n.kind());
            }
        }
        prev = match p {
            NodeOrToken::Token(t) => t.prev_sibling_or_token(),
            NodeOrToken::Node(n) => n.prev_sibling_or_token(),
        };
        i += 1;
        if i >= max_depth {
            break;
        }
    }
    eprintln!("=== END {} ===", label);
}

#[cfg(debug_assertions)]
pub fn debug_children_with_tokens(node: &SyntaxNode, label: &str, max_depth: usize) {
    eprintln!("=== {} ===", label);
    for (i, child) in node.children_with_tokens().enumerate() {
        match &child {
            NodeOrToken::Token(t) => {
                eprintln!("  [{}] Token: {:?} = {:?}", i, t.kind(), t.text());
            }
            NodeOrToken::Node(n) => {
                eprintln!("  [{}] Node: {:?}", i, n.kind());
                if n.kind() == ra_ap_syntax::SyntaxKind::VARIANT {
                    eprintln!("    Variant prev_sibling_or_token:");
                    let mut prev = n.prev_sibling_or_token();
                    let mut j = 0;
                    while let Some(p) = prev {
                        match &p {
                            NodeOrToken::Token(t) => {
                                eprintln!("      [{}] Token: {:?} = {:?}", j, t.kind(), t.text());
                            }
                            NodeOrToken::Node(pn) => {
                                eprintln!("      [{}] Node: {:?}", j, pn.kind());
                            }
                        }
                        prev = match p {
                            NodeOrToken::Token(t) => t.prev_sibling_or_token(),
                            NodeOrToken::Node(n) => n.prev_sibling_or_token(),
                        };
                        j += 1;
                        if j >= max_depth {
                            break;
                        }
                    }
                }
            }
        }
    }
    eprintln!("=== END {} ===", label);
}

#[cfg(not(debug_assertions))]
pub fn debug_node_siblings(_node: &ra_ap_syntax::SyntaxNode, _label: &str, _max_depth: usize) {}

#[cfg(not(debug_assertions))]
pub fn debug_children_with_tokens(
    _node: &ra_ap_syntax::SyntaxNode,
    _label: &str,
    _max_depth: usize,
) {
}
