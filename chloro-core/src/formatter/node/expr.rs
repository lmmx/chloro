// chloro-core/src/formatter/node/expr.rs
use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;
use ra_ap_syntax::ast::HasArgList;
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, ast};

/// Result of attempting to format an expression.
pub enum FormatResult {
    /// Successfully formatted the expression
    Formatted(String),
    /// Expression type not yet supported; caller should preserve verbatim
    Unsupported,
}

/// Attempt to format an expression node.
/// Returns `Unsupported` for expression types we don't yet handle,
/// allowing the caller to fall back to verbatim preservation.
pub fn try_format_expr(node: &SyntaxNode, indent: usize) -> FormatResult {
    match node.kind() {
        SyntaxKind::RECORD_EXPR => {
            if let Some(record_expr) = ast::RecordExpr::cast(node.clone()) {
                if let Some(formatted) = format_record_expr(&record_expr, indent) {
                    return FormatResult::Formatted(formatted);
                }
            }
            FormatResult::Unsupported
        }

        SyntaxKind::CALL_EXPR => {
            if let Some(call_expr) = ast::CallExpr::cast(node.clone()) {
                if let Some(formatted) = format_call_expr(&call_expr, indent) {
                    return FormatResult::Formatted(formatted);
                }
            }
            FormatResult::Unsupported
        }

        SyntaxKind::METHOD_CALL_EXPR => {
            if let Some(method_call) = ast::MethodCallExpr::cast(node.clone()) {
                if let Some(formatted) = format_method_call_expr(&method_call, indent) {
                    return FormatResult::Formatted(formatted);
                }
            }
            FormatResult::Unsupported
        }

        // Add more expression types as you implement them
        _ => FormatResult::Unsupported,
    }
}

/// Format a record expression, returning None if it can't be formatted nicely.
fn format_record_expr(expr: &ast::RecordExpr, indent: usize) -> Option<String> {
    let path = expr.path()?;
    let field_list = expr.record_expr_field_list()?;
    let fields: Vec<_> = field_list.fields().collect();

    // Only format if 2+ fields and all well-formed
    if fields.len() < 2 {
        return None;
    }

    for field in &fields {
        if field.name_ref().is_none() || field.expr().is_none() {
            return None;
        }
    }

    let mut buf = String::new();
    buf.push_str(&path.syntax().text().to_string());
    buf.push_str(" {\n");

    for field in fields {
        crate::formatter::write_indent(&mut buf, indent + 4);
        buf.push_str(&field.name_ref().unwrap().text());
        buf.push_str(": ");

        // Recursively try to format the field's expression
        let field_expr = field.expr().unwrap();
        match try_format_expr(field_expr.syntax(), indent + 4) {
            FormatResult::Formatted(s) => buf.push_str(&s),
            FormatResult::Unsupported => buf.push_str(&field_expr.syntax().text().to_string()),
        }

        buf.push_str(",\n");
    }

    crate::formatter::write_indent(&mut buf, indent);
    buf.push('}');
    Some(buf)
}

fn format_call_expr(expr: &ast::CallExpr, indent: usize) -> Option<String> {
    let callee = expr.expr()?;
    let arg_list = expr.arg_list()?;
    let args: Vec<_> = arg_list.args().collect();

    let callee_text = callee.syntax().text().to_string();

    // Try single-line first
    let args_single: Vec<_> = args.iter().map(|a| a.syntax().text().to_string()).collect();
    let single_line = format!("{}({})", callee_text, args_single.join(", "));

    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Multi-line: each arg on its own line
    let mut buf = String::new();
    buf.push_str(&callee_text);
    buf.push_str("(\n");

    for arg in args {
        write_indent(&mut buf, indent + 4);
        // Recursively format the argument expression
        match try_format_expr(arg.syntax(), indent + 4) {
            FormatResult::Formatted(s) => buf.push_str(&s),
            FormatResult::Unsupported => buf.push_str(&arg.syntax().text().to_string()),
        }
        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push(')');
    Some(buf)
}

fn format_method_call_expr(_expr: &ast::MethodCallExpr, _indent: usize) -> Option<String> {
    // TODO: Implement when ready
    None
}
