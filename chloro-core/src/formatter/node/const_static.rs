use ra_ap_syntax::{
    AstNode, SyntaxNode,
    ast::{self, HasName},
};

use crate::formatter::printer::Printer;

pub fn format_const_or_static(node: &SyntaxNode, buf: &mut String, indent: usize) {
    if let Some(c) = ast::Const::cast(node.clone()) {
        buf.item_preamble(&c, indent);
        buf.push_str("const ");
        format_name_type_body(buf, c.name(), c.ty(), c.body());
    } else if let Some(s) = ast::Static::cast(node.clone()) {
        buf.item_preamble(&s, indent);
        if s.mut_token().is_some() {
            buf.push_str("static mut ");
        } else {
            buf.push_str("static ");
        }
        format_name_type_body(buf, s.name(), s.ty(), s.body());
    }
}

fn format_name_type_body(
    buf: &mut String,
    name: Option<ast::Name>,
    ty: Option<ast::Type>,
    body: Option<ast::Expr>,
) {
    if let Some(name) = name {
        buf.push_str(name.text().as_ref());
        buf.push_str(": ");
    }
    if let Some(ty) = ty {
        buf.push_str(&ty.syntax().text().to_string());
    }
    if let Some(expr) = body {
        buf.push_str(" = ");
        buf.push_str(&expr.syntax().text().to_string());
    }
    buf.push_str(";\n");
}
