use ra_ap_syntax::{
    AstNode, AstToken, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility, Type},
};

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

enum TypeAliasRhsKind {
    Simple,
    GenericWrapped,
    FunctionType,
}

fn classify_type_alias_rhs(ty: &Type) -> TypeAliasRhsKind {
    let text = ty.syntax().text().to_string();
    if text.starts_with('&') && text.contains("Fn(") {
        return TypeAliasRhsKind::FunctionType;
    }
    if text.starts_with("fn(") || text.contains("for<") && text.contains("Fn") {
        return TypeAliasRhsKind::FunctionType;
    }
    if text.len() > MAX_WIDTH {
        return TypeAliasRhsKind::GenericWrapped;
    }
    TypeAliasRhsKind::Simple
}

pub fn format_type_alias(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let type_alias = match ast::TypeAlias::cast(node.clone()) {
        Some(t) => t,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in type_alias.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in type_alias.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = type_alias.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("type ");

    // Name
    if let Some(name) = type_alias.name() {
        buf.push_str(&name.text());
    }

    // Generic params inline for now
    if let Some(generics) = type_alias.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    // Where clause (inline only — expansion will come later)
    if let Some(where_clause) = type_alias.where_clause() {
        buf.push(' ');
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    if let Some(ty) = type_alias.ty() {
        let kind = classify_type_alias_rhs(&ty);
        match kind {
            TypeAliasRhsKind::Simple => {
                buf.push_str(" = ");
                buf.push_str(&ty.syntax().text().to_string());
                buf.push_str(";\n");
            }

            TypeAliasRhsKind::GenericWrapped => {
                buf.push_str(" =\n");
                write_indent(buf, indent + 4);
                buf.push_str(&ty.syntax().text().to_string());
                buf.push_str(";\n");
            }

            TypeAliasRhsKind::FunctionType => {
                if let Some(fn_type) = ast::FnPtrType::cast(ty.syntax().clone()) {
                    buf.push_str(" =\n");
                    write_indent(buf, indent + 4);

                    // Print keywords like `unsafe`, `extern "C"`, `fn`
                    if let Some(unsafe_tok) = fn_type.unsafe_token() {
                        buf.push_str(unsafe_tok.text());
                        buf.push(' ');
                    }
                    if let Some(abi) = fn_type.abi() {
                        buf.push_str(&abi.syntax().text().to_string());
                        buf.push(' ');
                    }
                    if let Some(fn_tok) = fn_type.fn_token() {
                        buf.push_str(fn_tok.text());
                    }

                    buf.push_str("(\n");

                    // Params
                    if let Some(param_list) = fn_type.param_list() {
                        for param in param_list.params() {
                            write_indent(buf, indent + 8);
                            buf.push_str(&param.syntax().text().to_string());
                            buf.push_str(",\n");
                        }
                    }

                    write_indent(buf, indent + 4);
                    buf.push(')');

                    // Return type (if any)
                    if let Some(ret) = fn_type.ret_type() {
                        buf.push(' ');
                        buf.push_str(&ret.syntax().text().to_string());
                    }

                    buf.push_str(";\n");
                } else {
                    // Fallback if somehow not an Fn pointer
                    buf.push_str(" = ");
                    buf.push_str(&ty.syntax().text().to_string());
                    buf.push_str(";\n");
                }
            }
        }
    } else {
        buf.push_str(";\n");
    }
}
