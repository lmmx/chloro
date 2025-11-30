// chloro-core/src/formatter/node/structliteral.rs
use ra_ap_syntax::ast::HasArgList;
use ra_ap_syntax::{AstNode, ast};

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

/// Format a well-formed record expression with 2+ fields as multi-line
pub fn try_format_record_expr(
    record_expr: &ast::RecordExpr,
    buf: &mut String,
    indent: usize,
) -> bool {
    let Some(path) = record_expr.path() else {
        return false;
    };

    let Some(field_list) = record_expr.record_expr_field_list() else {
        return false;
    };

    let fields: Vec<_> = field_list.fields().collect();

    // Only format if 2+ fields and all are well-formed (have both name and expr)
    if fields.len() < 2 {
        return false;
    }

    for field in &fields {
        if field.name_ref().is_none() || field.expr().is_none() {
            return false;
        }
    }

    // Multi-line format
    buf.push_str(&path.syntax().text().to_string());
    buf.push_str(" {\n");

    for field in fields {
        let name = field.name_ref().unwrap().text().to_string();
        let expr = field.expr().unwrap();
        let expr_text = expr.syntax().text().to_string();

        // Calculate if this field fits on one line
        // Format: "    name: expr,"
        let field_line_len = indent + 4 + name.len() + 2 + expr_text.len() + 1;

        if field_line_len <= MAX_WIDTH {
            // Single line
            write_indent(buf, indent + 4);
            buf.push_str(&name);
            buf.push_str(": ");
            buf.push_str(&expr_text);
            buf.push_str(",\n");
        } else {
            // Try to format the expression across multiple lines
            write_indent(buf, indent + 4);
            buf.push_str(&name);
            buf.push_str(": ");

            if let Some(formatted) = try_format_field_value(&expr, indent + 4) {
                buf.push_str(&formatted);
            } else {
                buf.push_str(&expr_text);
            }
            buf.push_str(",\n");
        }
    }

    write_indent(buf, indent);
    buf.push('}');
    true
}

/// Try to format a field value expression across multiple lines
fn try_format_field_value(expr: &ast::Expr, indent: usize) -> Option<String> {
    match expr {
        ast::Expr::MethodCallExpr(method_call) => format_method_call_multiline(method_call, indent),
        ast::Expr::CallExpr(call_expr) => format_call_expr_multiline(call_expr, indent),
        ast::Expr::TryExpr(try_expr) => {
            // Handle `expr?` - format the inner expression
            let inner = try_expr.expr()?;
            let inner_formatted = try_format_field_value(&inner, indent)?;
            Some(format!("{}?", inner_formatted))
        }
        _ => None,
    }
}

/// Format a method call expression, breaking arguments across lines
fn format_method_call_multiline(
    method_call: &ast::MethodCallExpr,
    indent: usize,
) -> Option<String> {
    let receiver = method_call.receiver()?;
    let method_name = method_call.name_ref()?.text().to_string();
    let arg_list = method_call.arg_list()?;

    let receiver_text = receiver.syntax().text().to_string();
    let args: Vec<_> = arg_list.args().collect();

    // Check if we need to break
    let single_line = format!(
        "{}.{}({})",
        receiver_text,
        method_name,
        args.iter()
            .map(|a| a.syntax().text().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Multi-line: break arguments
    let mut result = String::new();
    result.push_str(&receiver_text);
    result.push('.');
    result.push_str(&method_name);
    result.push_str("(\n");

    for arg in &args {
        write_indent_to_string(&mut result, indent + 4);
        result.push_str(&arg.syntax().text().to_string());
        result.push_str(",\n");
    }

    write_indent_to_string(&mut result, indent);
    result.push(')');

    Some(result)
}

/// Format a call expression, breaking arguments across lines
fn format_call_expr_multiline(call_expr: &ast::CallExpr, indent: usize) -> Option<String> {
    let callee = call_expr.expr()?;
    let arg_list = call_expr.arg_list()?;

    let callee_text = callee.syntax().text().to_string();
    let args: Vec<_> = arg_list.args().collect();

    // Check if we need to break
    let single_line = format!(
        "{}({})",
        callee_text,
        args.iter()
            .map(|a| a.syntax().text().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Multi-line: break arguments
    let mut result = String::new();
    result.push_str(&callee_text);
    result.push_str("(\n");

    for arg in &args {
        write_indent_to_string(&mut result, indent + 4);
        result.push_str(&arg.syntax().text().to_string());
        result.push_str(",\n");
    }

    write_indent_to_string(&mut result, indent);
    result.push(')');

    Some(result)
}

fn write_indent_to_string(s: &mut String, indent: usize) {
    for _ in 0..indent {
        s.push(' ');
    }
}
