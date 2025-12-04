use crate::formatter::printer::Printer;
use ra_ap_syntax::{
    AstNode, NodeOrToken, SyntaxKind, SyntaxNode,
    ast::{self, HasGenericParams, HasName, HasVisibility},
};

use super::{format_block_expr_contents, format_stmt_list};

pub fn format_function(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let func = match ast::Fn::cast(node.clone()) {
        Some(f) => f,
        None => return,
    };

    // Output leading non-doc comments (// style) that appear before visibility/keywords
    // These are children of the FN node that come before VISIBILITY or FN_KW
    for child in node.children_with_tokens() {
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::COMMENT {
                    let text = t.text();
                    // Skip doc comments - they're handled by doc_comments() below
                    if !text.starts_with("///") && !text.starts_with("//!") {
                        buf.line(indent, text);
                    }
                } else if t.kind() != SyntaxKind::WHITESPACE {
                    // Hit a non-comment, non-whitespace token - stop
                    break;
                }
            }
            NodeOrToken::Node(_) => {
                // Hit a node (like VISIBILITY) - stop
                break;
            }
        }
    }

    // Format doc comments using the HasDocComments trait
    buf.doc_comments(&func, indent);

    // Format attributes using the HasAttrs trait
    buf.attrs(&func, indent);

    buf.indent(indent);

    // Visibility
    buf.visibility(&func);

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

    // Track if we used multi-line params
    let mut used_multiline_params = false;

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
        let mut hypothetical_line_len = indent;

        if let Some(vis) = func.visibility() {
            hypothetical_line_len += u32::from(vis.syntax().text().len()) as usize + 1;
        }

        if func.const_token().is_some() {
            hypothetical_line_len += 6;
        }
        if func.async_token().is_some() {
            hypothetical_line_len += 6;
        }
        if func.unsafe_token().is_some() {
            hypothetical_line_len += 7;
        }

        hypothetical_line_len += 3; // "fn "

        if let Some(name) = func.name() {
            hypothetical_line_len += name.text().len();
        }

        if let Some(generics) = func.generic_param_list() {
            hypothetical_line_len += u32::from(generics.syntax().text().len()) as usize;
        }

        hypothetical_line_len += 2 + single_line_content.len();

        if let Some(ret) = func.ret_type() {
            hypothetical_line_len += 4;
            if let Some(ty) = ret.ty() {
                hypothetical_line_len += u32::from(ty.syntax().text().len()) as usize;
            }
        }

        hypothetical_line_len += 2;

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
            used_multiline_params = true;
            buf.push('\n');
            let inner_indent = indent + 4;

            if let Some(self_param) = params.self_param() {
                buf.line(inner_indent, &format!("{},", self_param.syntax().text()));
            }

            for p in params_vec {
                buf.line(inner_indent, &format!("{},", p));
            }

            buf.indent(indent);
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
        buf.indent(indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    // Body or semicolon
    if let Some(body) = func.body() {
        // Check if body is empty
        let is_empty = if let Some(stmt_list) = body.stmt_list() {
            stmt_list.statements().next().is_none()
                && stmt_list.tail_expr().is_none()
                && !stmt_list
                    .syntax()
                    .children_with_tokens()
                    .any(|c| matches!(&c, NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT))
        } else {
            true
        };

        // Keep empty body expanded if:
        // - params were multi-line, OR
        // - there's a where clause, OR
        // - the function has certain attributes that suggest it's a stub
        let keep_expanded = used_multiline_params || func.where_clause().is_some();

        if is_empty && !keep_expanded {
            // Empty body - keep on one line
            buf.newline(" {}");
        } else if is_empty && keep_expanded {
            // Empty body but keep expanded
            if func.where_clause().is_some() {
                buf.open_brace_newline(indent);
            } else {
                buf.open_brace();
            }
            buf.close_brace_ln(indent);
        } else {
            // Has a body - add opening brace
            if func.where_clause().is_some() {
                buf.open_brace_newline(indent);
            } else {
                buf.open_brace();
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
                                buf.line(indent + 4, t.text());
                            }
                        }
                    }
                }
            }

            buf.close_brace_ln(indent);
        }
    } else {
        // No body - just semicolon
        buf.newline(";");
    }
}
