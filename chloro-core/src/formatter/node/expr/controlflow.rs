//! Control flow expressions: if, match, loop, while, for, block, closure.

use ra_ap_syntax::SyntaxNode;

pub fn format_if_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: condition + then block + optional else
    None
}

pub fn format_match_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: scrutinee + match arms
    None
}

pub fn format_loop_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: optional label + block
    None
}

pub fn format_while_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: condition + block
    None
}

pub fn format_for_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: pattern + iterable + block
    None
}

pub fn format_block_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // Block expressions are typically handled via block.rs at the statement level.
    // Returning None here causes fallback to verbatim, which may be fine,
    // or we could delegate to the block formatter.
    None
}

pub fn format_closure_expr(_node: &SyntaxNode, _indent: usize) -> Option<String> {
    // TODO: move/async keywords + params + body
    None
}
