//!
//! Expression formatting dispatcher.
//!
//! Internal helpers return `Option<String>` where `None` means "unsupported, use verbatim".
//! The public interface uses `FormatResult` for clarity at call sites.

pub mod collections;
pub mod controlflow;
pub mod jumps;
pub mod operators;
pub mod simple;

use ra_ap_syntax::{SyntaxKind, SyntaxNode};

/// Result of attempting to format an expression.
pub enum FormatResult {
    /// Successfully formatted the expression.
    Formatted(String),
    /// Expression type not yet supported; caller should preserve verbatim.
    Unsupported,
}

impl From<Option<String>> for FormatResult {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(s) => FormatResult::Formatted(s),
            None => FormatResult::Unsupported,
        }
    }
}

/// Attempt to format an expression node.
///
/// Returns `Unsupported` for expression types we don't yet handle,
/// allowing the caller to fall back to verbatim preservation.
pub fn try_format_expr(node: &SyntaxNode, indent: usize) -> FormatResult {
    try_format_expr_inner(node, indent).into()
}

/// Inner implementation returning Option for easier chaining.
pub fn try_format_expr_inner(node: &SyntaxNode, indent: usize) -> Option<String> {
    match node.kind() {
        // === Simple / Pass-through ===
        SyntaxKind::PATH_EXPR | SyntaxKind::LITERAL | SyntaxKind::UNDERSCORE_EXPR => {
            Some(node.text().to_string())
        }

        // === Wrapping expressions ===
        SyntaxKind::PAREN_EXPR => simple::format_paren_expr(node, indent),
        SyntaxKind::TRY_EXPR => simple::format_try_expr(node, indent),
        SyntaxKind::AWAIT_EXPR => simple::format_await_expr(node, indent),
        SyntaxKind::REF_EXPR => simple::format_ref_expr(node, indent),
        SyntaxKind::PREFIX_EXPR => simple::format_prefix_expr(node, indent),

        // === Collections / Call-like ===
        SyntaxKind::ARRAY_EXPR => collections::format_array_expr(node, indent),
        SyntaxKind::TUPLE_EXPR => collections::format_tuple_expr(node, indent),
        SyntaxKind::CALL_EXPR => collections::format_call_expr(node, indent),
        SyntaxKind::METHOD_CALL_EXPR => collections::format_method_call_expr(node, indent),
        SyntaxKind::INDEX_EXPR => collections::format_index_expr(node, indent),
        SyntaxKind::RECORD_EXPR => collections::format_record_expr(node, indent),

        // === Operators ===
        SyntaxKind::BIN_EXPR => operators::format_bin_expr(node, indent),
        SyntaxKind::RANGE_EXPR => operators::format_range_expr(node, indent),
        SyntaxKind::CAST_EXPR => operators::format_cast_expr(node, indent),
        SyntaxKind::FIELD_EXPR => operators::format_field_expr(node, indent),

        // === Control flow ===
        SyntaxKind::IF_EXPR => controlflow::format_if_expr(node, indent),
        SyntaxKind::MATCH_EXPR => controlflow::format_match_expr(node, indent),
        SyntaxKind::LOOP_EXPR => controlflow::format_loop_expr(node, indent),
        SyntaxKind::WHILE_EXPR => controlflow::format_while_expr(node, indent),
        SyntaxKind::FOR_EXPR => controlflow::format_for_expr(node, indent),
        SyntaxKind::BLOCK_EXPR => controlflow::format_block_expr(node, indent),
        SyntaxKind::CLOSURE_EXPR => controlflow::format_closure_expr(node, indent),

        // === Jumps ===
        SyntaxKind::RETURN_EXPR => jumps::format_return_expr(node, indent),
        SyntaxKind::BREAK_EXPR => jumps::format_break_expr(node, indent),
        SyntaxKind::CONTINUE_EXPR => jumps::format_continue_expr(node, indent),
        SyntaxKind::YIELD_EXPR => jumps::format_yield_expr(node, indent),
        SyntaxKind::YEET_EXPR => jumps::format_yeet_expr(node, indent),
        SyntaxKind::BECOME_EXPR => jumps::format_become_expr(node, indent),
        SyntaxKind::LET_EXPR => jumps::format_let_expr(node, indent),

        // === Preserve verbatim (macros, asm, builtins) ===
        SyntaxKind::MACRO_EXPR
        | SyntaxKind::FORMAT_ARGS_EXPR
        | SyntaxKind::ASM_EXPR
        | SyntaxKind::ASM_OPERAND_EXPR
        | SyntaxKind::OFFSET_OF_EXPR => Some(node.text().to_string()),

        _ => None,
    }
}
