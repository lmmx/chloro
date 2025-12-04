use crate::formatter::write_indent;
use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments, HasGenericParams, HasName, HasVisibility},
};

use super::{format_block_expr_contents, format_stmt_list};

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

        // Calculate total length for single-line formatting
        let self_text = if let Some(self_param) = params.self_param() {
            self_param.syntax().text().to_string()
        } else {
            String::new()
        };

        // Build what the single-line version would look like
        let mut single_line_content = String::new();
        if has_self {
            single_line_content.push_str(&self_text);
            if !params_vec.is_empty() {
                single_line_content.push_str(", ");
            }
        }
        for (i, p) in params_vec.iter().enumerate() {
            if i > 0 {
                single_line_content.push_str(", ");
            }
            single_line_content.push_str(p);
        }

        // Calculate the full line length if we format on single line
        // Format: "pub fn name(params) -> ReturnType"
        let mut hypothetical_line_len = indent;

        // Add visibility
        if let Some(vis) = func.visibility() {
            hypothetical_line_len += u32::from(vis.syntax().text().len()) as usize + 1;
        }

        // Add modifiers
        if func.const_token().is_some() {
            hypothetical_line_len += 6; // "const "
        }
        if func.async_token().is_some() {
            hypothetical_line_len += 6; // "async "
        }
        if func.unsafe_token().is_some() {
            hypothetical_line_len += 7; // "unsafe "
        }

        hypothetical_line_len += 3; // "fn "

        // Add name
        if let Some(name) = func.name() {
            hypothetical_line_len += name.text().len();
        }

        // Add generics
        if let Some(generics) = func.generic_param_list() {
            hypothetical_line_len += u32::from(generics.syntax().text().len()) as usize;
        }

        // Add parameters with parens
        hypothetical_line_len += 2 + single_line_content.len(); // "(content)"

        // Add return type
        if let Some(ret) = func.ret_type() {
            hypothetical_line_len += 4; // " -> "
            if let Some(ty) = ret.ty() {
                hypothetical_line_len += u32::from(ty.syntax().text().len()) as usize;
            }
        }

        // Add space before opening brace or semicolon
        hypothetical_line_len += 2; // " {" or ";"

        let is_single_line = hypothetical_line_len < crate::formatter::config::MAX_WIDTH;

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

    // Body or semicolon
    if let Some(body) = func.body() {
        // Has a body - add opening brace
        if func.where_clause().is_some() {
            buf.push('\n');
            write_indent(buf, indent);
            buf.push_str("{\n");
        } else {
            buf.push_str(" {\n");
        }

        // Check if the body is a single record expression (tail expression)
        let stmt_list = body.stmt_list();
        if let Some(stmt_list) = stmt_list {
            // Use our block formatting which handles record expressions
            format_stmt_list(stmt_list.syntax(), buf, indent + 4);
        } else {
            // Fallback: Process body contents directly
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
        }

        write_indent(buf, indent);
        buf.push('}');
    } else {
        // No body - just semicolon
        buf.push(';');
    }
    buf.push('\n');
}
