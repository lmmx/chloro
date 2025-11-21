use ra_ap_syntax::{
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
};

use super::format_block_expr_contents;
use crate::formatter::write_indent;

pub fn format_function(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let func = match ast::Fn::cast(node.clone()) {
        Some(f) => f,
        None => return,
    };

    // Format doc comments using the HasDocComments trait
    for doc_comment in func.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using the HasAttrs trait
    for attr in func.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = func.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    // Modifiers
    if func.const_token().is_some() {
        buf.push_str("const ");
    }
    if func.async_token().is_some() {
        buf.push_str("async ");
    }
    if func.unsafe_token().is_some() {
        buf.push_str("unsafe ");
    }

    buf.push_str("fn ");

    // Name
    if let Some(name) = func.name() {
        buf.push_str(&name.text());
    }

    // Generic params
    if let Some(generics) = func.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    // Parameters
    if let Some(params) = func.param_list() {
        let params_vec: Vec<_> = params
            .params()
            .map(|p| p.syntax().text().to_string())
            .collect();

        let has_self = params.self_param().is_some();
        let is_single_line = params_vec.len() + if has_self { 1 } else { 0 } <= 1
            && params.syntax().text().len() <= 60.into();

        buf.push('(');

        if is_single_line {
            if let Some(self_param) = params.self_param() {
                buf.push_str(&self_param.syntax().text().to_string());
                if !params_vec.is_empty() {
                    buf.push_str(", ");
                }
            }
            for (i, p) in params_vec.iter().enumerate() {
                if i > 0 {
                    buf.push_str(", ");
                }
                buf.push_str(p);
            }
            buf.push(')');
        } else {
            buf.push('\n');
            let inner_indent = indent + 4;

            if let Some(self_param) = params.self_param() {
                write_indent(buf, inner_indent);
                buf.push_str(&self_param.syntax().text().to_string());
                buf.push_str(",\n");
            }

            for p in params_vec {
                write_indent(buf, inner_indent);
                buf.push_str(&p);
                buf.push_str(",\n");
            }

            write_indent(buf, indent);
            buf.push(')');
        }
    } else {
        buf.push_str("()");
    }

    // Return type
    if let Some(ret) = func.ret_type() {
        buf.push_str(" -> ");
        if let Some(ty) = ret.ty() {
            buf.push_str(&ty.syntax().text().to_string());
        }
    }

    // Where clause
    if let Some(where_clause) = func.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    // Body
    if let Some(body) = func.body() {
        buf.push_str(" {\n");

        // Process body contents, preserving inline comments
        for child in body.syntax().children_with_tokens() {
            match child {
                NodeOrToken::Node(n) => {
                    format_block_expr_contents(&n, buf, indent + 4);
                }
                NodeOrToken::Token(t) => {
                    if t.kind() == SyntaxKind::COMMENT {
                        write_indent(buf, indent + 4);
                        buf.push_str(t.text());
                        buf.push('\n');
                    }
                }
            }
        }

        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push(';');
    }
    buf.push('\n');
}
