use ra_ap_syntax::ast::{self, AstNode, HasArgList, HasGenericArgs};
use ra_ap_syntax::SyntaxNode;

use super::try_format_expr_inner;
use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

pub fn format_array_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let array = ast::ArrayExpr::cast(node.clone())?;

    // Check if it's a repeat expression [expr; len]
    if let Some(semicolon) = array.semicolon_token() {
        let mut exprs = array.exprs();
        let expr = exprs.next()?;
        let len = exprs.next()?;
        let _ = semicolon; // just to confirm it exists

        let expr_str = try_format_expr_inner(expr.syntax(), indent)?;
        let len_str = try_format_expr_inner(len.syntax(), indent)?;
        return Some(format!("[{}; {}]", expr_str, len_str));
    }

    // Element list
    let elements: Vec<_> = array.exprs().collect();
    format_delimited_list(&elements, indent, "[", "]")
}

pub fn format_tuple_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let tuple = ast::TupleExpr::cast(node.clone())?;
    let elements: Vec<_> = tuple.fields().collect();

    // Single-element tuple needs trailing comma
    if elements.len() == 1 {
        let formatted = try_format_expr_inner(elements[0].syntax(), indent)?;
        return Some(format!("({},)", formatted));
    }

    format_delimited_list(&elements, indent, "(", ")")
}

pub fn format_call_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let call = ast::CallExpr::cast(node.clone())?;
    let callee = call.expr()?;
    let arg_list = call.arg_list()?;
    let args: Vec<_> = arg_list.args().collect();

    let callee_text = try_format_expr_inner(callee.syntax(), indent)?;

    // Try single-line first
    let args_formatted: Option<Vec<_>> = args
        .iter()
        .map(|a| try_format_expr_inner(a.syntax(), indent))
        .collect();

    let args_vec = args_formatted?;

    let single_line = format!("{}({})", callee_text, args_vec.join(", "));
    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Single argument that formats to multi-line: snug wrap it
    if args.len() == 1 {
        let arg_str = &args_vec[0];
        if arg_str.contains('\n') {
            // Snug: no trailing comma, closing paren right after
            return Some(format!("{}({})", callee_text, arg_str));
        }
    }

    // Multi-line with multiple args
    let mut buf = String::new();
    buf.push_str(&callee_text);
    buf.push_str("(\n");

    for arg_str in &args_vec {
        write_indent(&mut buf, indent + 4);
        buf.push_str(arg_str);
        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push(')');
    Some(buf)
}

pub fn format_method_call_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let method = ast::MethodCallExpr::cast(node.clone())?;
    let receiver = method.receiver()?;
    let name = method.name_ref()?;
    let arg_list = method.arg_list()?;
    let args: Vec<_> = arg_list.args().collect();

    let receiver_str = try_format_expr_inner(receiver.syntax(), indent)?;

    // Preserve generic args (turbofish)
    let generic_args = method
        .generic_arg_list()
        .map(|g| g.syntax().text().to_string())
        .unwrap_or_default();

    // Format args
    let args_formatted: Option<Vec<_>> = args
        .iter()
        .map(|a| try_format_expr_inner(a.syntax(), indent))
        .collect();

    let args_vec = args_formatted?;

    // Try single-line
    let single_line = format!(
        "{}.{}{}({})",
        receiver_str,
        name.text(),
        generic_args,
        args_vec.join(", ")
    );
    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Single argument that formats to multi-line: snug wrap it
    if args.len() == 1 {
        let arg_str = &args_vec[0];
        if arg_str.contains('\n') {
            return Some(format!(
                "{}.{}{}({})",
                receiver_str,
                name.text(),
                generic_args,
                arg_str
            ));
        }
    }

    // Multi-line args
    let mut buf = String::new();
    buf.push_str(&receiver_str);
    buf.push('.');
    buf.push_str(&name.text());
    buf.push_str(&generic_args);
    buf.push_str("(\n");

    for arg_str in &args_vec {
        write_indent(&mut buf, indent + 4);
        buf.push_str(arg_str);
        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push(')');
    Some(buf)
}

pub fn format_index_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let index = ast::IndexExpr::cast(node.clone())?;
    let base = index.base()?;
    let idx = index.index()?;

    let base_str = try_format_expr_inner(base.syntax(), indent)?;
    let idx_str = try_format_expr_inner(idx.syntax(), indent)?;

    Some(format!("{}[{}]", base_str, idx_str))
}

pub fn format_record_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let record = ast::RecordExpr::cast(node.clone())?;
    let path = record.path()?;
    let field_list = record.record_expr_field_list()?;
    let fields: Vec<_> = field_list.fields().collect();

    // Single field or fewer: keep inline
    if fields.len() < 2 {
        return None;
    }

    // Check all fields are well-formed
    for field in &fields {
        if field.name_ref().is_none() || field.expr().is_none() {
            return None;
        }
    }

    let mut buf = String::new();
    buf.push_str(&path.syntax().text().to_string());
    buf.push_str(" {\n");

    for field in fields {
        write_indent(&mut buf, indent + 4);
        buf.push_str(&field.name_ref().unwrap().text());
        buf.push_str(": ");

        let field_expr = field.expr().unwrap();
        match try_format_expr_inner(field_expr.syntax(), indent + 4) {
            Some(s) => buf.push_str(&s),
            None => buf.push_str(&field_expr.syntax().text().to_string()),
        }

        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push('}');
    Some(buf)
}

/// Helper for formatting comma-separated lists with delimiters.
fn format_delimited_list(
    elements: &[ast::Expr],
    indent: usize,
    open: &str,
    close: &str,
) -> Option<String> {
    if elements.is_empty() {
        return Some(format!("{}{}", open, close));
    }

    let formatted: Option<Vec<_>> = elements
        .iter()
        .map(|e| try_format_expr_inner(e.syntax(), indent))
        .collect();

    let items = formatted?;

    // Try single-line
    let single_line = format!("{}{}{}", open, items.join(", "), close);
    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Multi-line
    let mut buf = String::new();
    buf.push_str(open);
    buf.push('\n');

    for item in items {
        write_indent(&mut buf, indent + 4);
        buf.push_str(&item);
        buf.push_str(",\n");
    }

    write_indent(&mut buf, indent);
    buf.push_str(close);
    Some(buf)
}
