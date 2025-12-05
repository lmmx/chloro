use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;
use ra_ap_syntax::ast::{self, AstNode, HasArgList, HasGenericArgs};
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode};

use super::try_format_expr_inner;

/// Check if there's a newline after the opening paren in an argument list
fn has_newline_after_open_paren(arg_list: &ast::ArgList) -> bool {
    let mut after_lparen = false;
    for child in arg_list.syntax().children_with_tokens() {
        match &child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::L_PAREN => {
                after_lparen = true;
            }
            NodeOrToken::Token(t) if after_lparen && t.kind() == SyntaxKind::WHITESPACE => {
                return t.text().contains('\n');
            }
            NodeOrToken::Token(_) if after_lparen => {
                return false;
            }
            NodeOrToken::Node(_) if after_lparen => {
                return false;
            }
            _ => {}
        }
    }
    false
}

/// Check if there's a newline between the receiver and the dot in a method call
fn has_newline_before_dot(node: &SyntaxNode) -> bool {
    let mut found_receiver = false;
    for child in node.children_with_tokens() {
        match &child {
            NodeOrToken::Node(_) if !found_receiver => {
                found_receiver = true;
            }
            NodeOrToken::Token(t) if found_receiver && t.kind() == SyntaxKind::WHITESPACE => {
                if t.text().contains('\n') {
                    return true;
                }
            }
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::DOT => {
                return false;
            }
            _ => {}
        }
    }
    false
}

pub fn format_array_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let array = ast::ArrayExpr::cast(node.clone())?;

    // Check if it's a repeat expression [expr; len]
    if let Some(semicolon) = array.semicolon_token() {
        let mut exprs = array.exprs();
        let expr = exprs.next()?;
        let len = exprs.next()?;
        let _ = semicolon;

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

    // No args - always single line
    if args.is_empty() {
        return Some(format!("{}()", callee_text));
    }

    // Try to format all args
    let args_formatted: Option<Vec<_>> = args
        .iter()
        .map(|a| try_format_expr_inner(a.syntax(), indent))
        .collect();

    let args_vec = args_formatted?;

    // Try single-line
    let single_line = format!("{}({})", callee_text, args_vec.join(", "));
    if indent + single_line.len() <= MAX_WIDTH {
        return Some(single_line);
    }

    // Single argument that's already multi-line: snug wrap
    if args.len() == 1 && args_vec[0].contains('\n') {
        return Some(format!("{}({})", callee_text, args_vec[0]));
    }

    // Multi-line with trailing commas
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
    let is_chain = matches!(
        receiver,
        ast::Expr::MethodCallExpr(_) | ast::Expr::FieldExpr(_) | ast::Expr::AwaitExpr(_)
    );

    if is_chain {
        return None;
    }

    let name = method.name_ref()?;
    let arg_list = method.arg_list()?;
    let args: Vec<_> = arg_list.args().collect();

    let receiver_str = try_format_expr_inner(receiver.syntax(), indent)?;

    let generic_args = method
        .generic_arg_list()
        .map(|g| g.syntax().text().to_string())
        .unwrap_or_default();

    // Check if there's a newline before the dot
    let newline_before_dot = has_newline_before_dot(node);

    // Check if args were originally on separate lines
    let multiline_args = has_newline_after_open_paren(&arg_list);

    // Build the dot and method part
    let dot_method = if newline_before_dot {
        format!("\n{}.{}{}", " ".repeat(indent), name.text(), generic_args)
    } else {
        format!(".{}{}", name.text(), generic_args)
    };

    // No args
    if args.is_empty() {
        return Some(format!("{}{}()", receiver_str, dot_method));
    }

    // Format args
    let args_formatted: Option<Vec<_>> = args
        .iter()
        .map(|a| try_format_expr_inner(a.syntax(), indent + 4))
        .collect();

    let args_vec = args_formatted?;

    // If original had multiline args, preserve that
    if multiline_args {
        let mut buf = String::new();
        buf.push_str(&receiver_str);
        buf.push_str(&dot_method);
        buf.push_str("(\n");

        for arg_str in &args_vec {
            write_indent(&mut buf, indent + 4);
            buf.push_str(arg_str);
            buf.push_str(",\n");
        }

        write_indent(&mut buf, indent);
        buf.push(')');
        return Some(buf);
    }

    // Try single-line
    let single_line = format!("{}{}({})", receiver_str, dot_method, args_vec.join(", "));
    if indent + single_line.len() <= MAX_WIDTH && !newline_before_dot {
        return Some(single_line);
    }

    // Single argument that's already multi-line: snug wrap
    if args.len() == 1 && args_vec[0].contains('\n') {
        return Some(format!("{}{}({})", receiver_str, dot_method, args_vec[0]));
    }

    // Multi-line args
    let mut buf = String::new();
    buf.push_str(&receiver_str);
    buf.push_str(&dot_method);
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

    // Single field or fewer: keep inline (return None to use verbatim)
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
