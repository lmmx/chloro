// In structliteral.rs - keep it simple
use crate::formatter::write_indent;
use ra_ap_syntax::{AstNode, ast};

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
        write_indent(buf, indent + 4);
        buf.push_str(&field.name_ref().unwrap().text());
        buf.push_str(": ");
        buf.push_str(&field.expr().unwrap().syntax().text().to_string());
        buf.push_str(",\n");
    }

    write_indent(buf, indent);
    buf.push('}');
    true
}
