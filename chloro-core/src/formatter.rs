use ra_ap_syntax::{
    ast::{self, HasGenericParams, HasModuleItem, HasName, HasVisibility},
    AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode,
};

/// Format Rust source code with canonical style.
pub fn format_source(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let root = parse.tree();

    let mut output = String::with_capacity(source.len());
    format_node(root.syntax(), &mut output, 0);
    output
}

fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
    match node.kind() {
        SyntaxKind::SOURCE_FILE => {
            for child in node.children() {
                format_node(&child, buf, indent);
                // Add spacing between top-level items
                if matches!(
                    child.kind(),
                    SyntaxKind::FN | SyntaxKind::STRUCT | SyntaxKind::ENUM | SyntaxKind::IMPL
                ) {
                    buf.push('\n');
                }
            }
        }

        SyntaxKind::FN => format_function(node, buf, indent),
        SyntaxKind::STRUCT => format_struct(node, buf, indent),
        SyntaxKind::ENUM => format_enum(node, buf, indent),
        SyntaxKind::IMPL => format_impl(node, buf, indent),
        SyntaxKind::USE => format_use(node, buf, indent),
        SyntaxKind::MODULE => format_module(node, buf, indent),

        SyntaxKind::BLOCK_EXPR => format_block(node, buf, indent),
        SyntaxKind::STMT_LIST => format_stmt_list(node, buf, indent),

        // Skip whitespace and comments in first pass (simplified)
        SyntaxKind::WHITESPACE => {}
        SyntaxKind::COMMENT => {
            write_indent(buf, indent);
            buf.push_str(&node.text().to_string());
            buf.push('\n');
        }

        _ => {
            // Default: recurse on children
            for child in node.children() {
                format_node(&child, buf, indent);
            }
        }
    }
}

fn format_function(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let func = match ast::Fn::cast(node.clone()) {
        Some(f) => f,
        None => return,
    };

    write_indent(buf, indent);

    // Visibility
    if let Some(vis) = func.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    // Const/async/unsafe modifiers
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
        buf.push('(');
        for (idx, param) in params.params().enumerate() {
            if idx > 0 {
                buf.push_str(", ");
            }
            buf.push_str(&param.syntax().text().to_string());
        }
        buf.push(')');
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
        format_block_expr_contents(body.syntax(), buf, indent + 4);
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push(';');
    }
    buf.push('\n');
}

fn format_struct(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let strukt = match ast::Struct::cast(node.clone()) {
        Some(s) => s,
        None => return,
    };

    write_indent(buf, indent);

    if let Some(vis) = strukt.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("struct ");

    if let Some(name) = strukt.name() {
        buf.push_str(&name.text());
    }

    if let Some(generics) = strukt.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    if let Some(field_list) = strukt.field_list() {
        match field_list {
            ast::FieldList::RecordFieldList(fields) => {
                buf.push_str(" {\n");
                for field in fields.fields() {
                    write_indent(buf, indent + 4);
                    if let Some(vis) = field.visibility() {
                        buf.push_str(&vis.syntax().text().to_string());
                        buf.push(' ');
                    }
                    if let Some(name) = field.name() {
                        buf.push_str(&name.text());
                    }
                    buf.push_str(": ");
                    if let Some(ty) = field.ty() {
                        buf.push_str(&ty.syntax().text().to_string());
                    }
                    buf.push_str(",\n");
                }
                write_indent(buf, indent);
                buf.push('}');
            }
            ast::FieldList::TupleFieldList(fields) => {
                buf.push('(');
                for (idx, field) in fields.fields().enumerate() {
                    if idx > 0 {
                        buf.push_str(", ");
                    }
                    if let Some(vis) = field.visibility() {
                        buf.push_str(&vis.syntax().text().to_string());
                        buf.push(' ');
                    }
                    if let Some(ty) = field.ty() {
                        buf.push_str(&ty.syntax().text().to_string());
                    }
                }
                buf.push_str(");");
            }
        }
    } else {
        buf.push(';');
    }
    buf.push('\n');
}

fn format_enum(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let enum_ = match ast::Enum::cast(node.clone()) {
        Some(e) => e,
        None => return,
    };

    write_indent(buf, indent);

    if let Some(vis) = enum_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("enum ");

    if let Some(name) = enum_.name() {
        buf.push_str(&name.text());
    }

    if let Some(generics) = enum_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    if let Some(variants) = enum_.variant_list() {
        buf.push_str(" {\n");
        for variant in variants.variants() {
            write_indent(buf, indent + 4);
            if let Some(name) = variant.name() {
                buf.push_str(&name.text());
            }
            if let Some(field_list) = variant.field_list() {
                match field_list {
                    ast::FieldList::RecordFieldList(_) => {
                        buf.push_str(" { ... }");
                    }
                    ast::FieldList::TupleFieldList(fields) => {
                        buf.push('(');
                        for (idx, field) in fields.fields().enumerate() {
                            if idx > 0 {
                                buf.push_str(", ");
                            }
                            if let Some(ty) = field.ty() {
                                buf.push_str(&ty.syntax().text().to_string());
                            }
                        }
                        buf.push(')');
                    }
                }
            }
            buf.push_str(",\n");
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}

fn format_impl(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let impl_ = match ast::Impl::cast(node.clone()) {
        Some(i) => i,
        None => return,
    };

    write_indent(buf, indent);

    if impl_.unsafe_token().is_some() {
        buf.push_str("unsafe ");
    }

    buf.push_str("impl");

    if let Some(generics) = impl_.generic_param_list() {
        buf.push_str(&generics.syntax().text().to_string());
    }

    buf.push(' ');

    if let Some(ty) = impl_.self_ty() {
        buf.push_str(&ty.syntax().text().to_string());
    }

    if let Some(where_clause) = impl_.where_clause() {
        buf.push('\n');
        write_indent(buf, indent);
        buf.push_str(&where_clause.syntax().text().to_string());
    }

    if let Some(assoc_items) = impl_.assoc_item_list() {
        buf.push_str(" {\n");
        for item in assoc_items.assoc_items() {
            format_node(item.syntax(), buf, indent + 4);
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push_str(" {}");
    }
    buf.push('\n');
}

fn format_use(node: &SyntaxNode, buf: &mut String, indent: usize) {
    write_indent(buf, indent);

    let use_ = match ast::Use::cast(node.clone()) {
        Some(u) => u,
        None => return,
    };

    if let Some(vis) = use_.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("use ");

    if let Some(use_tree) = use_.use_tree() {
        buf.push_str(&use_tree.syntax().text().to_string());
    }

    buf.push_str(";\n");
}

fn format_module(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let module = match ast::Module::cast(node.clone()) {
        Some(m) => m,
        None => return,
    };

    write_indent(buf, indent);

    if let Some(vis) = module.visibility() {
        buf.push_str(&vis.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("mod ");

    if let Some(name) = module.name() {
        buf.push_str(&name.text());
    }

    if let Some(item_list) = module.item_list() {
        buf.push_str(" {\n");
        for item in item_list.items() {
            format_node(item.syntax(), buf, indent + 4);
        }
        write_indent(buf, indent);
        buf.push('}');
    } else {
        buf.push(';');
    }
    buf.push('\n');
}

fn format_block(node: &SyntaxNode, buf: &mut String, indent: usize) {
    buf.push_str("{\n");
    format_block_expr_contents(node, buf, indent + 4);
    write_indent(buf, indent);
    buf.push('}');
}

fn format_stmt_list(node: &SyntaxNode, buf: &mut String, indent: usize) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::WHITESPACE | SyntaxKind::COMMENT => continue,
            _ => {
                write_indent(buf, indent);
                let text = child.text().to_string();
                buf.push_str(&text);
                if !text.ends_with(';') && !text.ends_with('}') {
                    buf.push(';');
                }
                buf.push('\n');
            }
        }
    }
}

fn format_block_expr_contents(node: &SyntaxNode, buf: &mut String, indent: usize) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::STMT_LIST => format_stmt_list(&child, buf, indent),
            SyntaxKind::WHITESPACE => continue,
            _ => {
                write_indent(buf, indent);
                buf.push_str(&child.text().to_string());
                buf.push('\n');
            }
        }
    }
}

fn write_indent(buf: &mut String, indent: usize) {
    for _ in 0..indent {
        buf.push(' ');
    }
}
