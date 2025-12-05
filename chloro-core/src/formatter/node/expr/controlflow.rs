use crate::formatter::printer::Printer;
use crate::formatter::write_indent;
use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasLoopBody};
use ra_ap_syntax::{NodeOrToken, SyntaxKind};

use super::try_format_expr_inner;

fn format_expr_attrs(node: &impl HasAttrs) -> String {
    let mut result = String::new();
    for attr in node.attrs() {
        result.push_str(&attr.syntax().text().to_string());
        result.push(' ');
    }
    result
}

/// Format a block expression body, returning the contents between braces.
fn format_block_contents(block: &ast::BlockExpr, indent: usize) -> String {
    let mut buf = String::new();

    if let Some(stmt_list) = block.stmt_list() {
        let children: Vec<_> = stmt_list.syntax().children_with_tokens().collect();

        // Find the last node index for tail expression handling
        let last_node_idx = children
            .iter()
            .rposition(|child| matches!(child, NodeOrToken::Node(_)));

        let mut prev_was_item = false;

        for (idx, child) in children.iter().enumerate() {
            match child {
                NodeOrToken::Token(t) => {
                    match t.kind() {
                        SyntaxKind::COMMENT => {
                            // Check if there's a newline before this comment
                            let has_newline_before = idx > 0
                                && matches!(
                                    &children[idx - 1],
                                    NodeOrToken::Token(prev) if prev.kind() == SyntaxKind::WHITESPACE && prev.text().contains('\n')
                                );

                            // Only output if it's a leading comment (has newline before)
                            // Trailing comments are handled when outputting the node
                            if has_newline_before {
                                // Check for blank line before this comment
                                if prev_was_item && has_blank_line_before(&children, idx) {
                                    buf.push('\n');
                                }
                                write_indent(&mut buf, indent);
                                buf.push_str(t.text());
                                buf.push('\n');
                            }
                            prev_was_item = false;
                        }
                        SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE => {
                            // Skip braces and whitespace
                        }
                        _ => {}
                    }
                }
                NodeOrToken::Node(n) => {
                    // Check for blank line before this node
                    if prev_was_item && has_blank_line_before(&children, idx) {
                        buf.push('\n');
                    }

                    let is_last = Some(idx) == last_node_idx;

                    write_indent(&mut buf, indent);
                    match try_format_expr_inner(n, indent) {
                        Some(s) => {
                            buf.push_str(&s);
                            // Add semicolon for statements (not for tail expression)
                            if !is_last && n.kind() != SyntaxKind::EXPR_STMT {
                                // EXPR_STMT already includes semicolon in its text
                            }
                        }
                        None => {
                            buf.push_str(&n.text().to_string());
                        }
                    }

                    // Check for trailing comment on same line
                    if let Some((whitespace, comment)) = get_trailing_comment(&children, idx) {
                        buf.push_str(&whitespace);
                        buf.push_str(&comment);
                    }

                    buf.push('\n');
                    prev_was_item = true;
                }
            }
        }
    }

    buf
}

/// Get trailing comment for node at given index in children list
fn get_trailing_comment(
    children: &[NodeOrToken<SyntaxNode, ra_ap_syntax::SyntaxToken>],
    node_idx: usize,
) -> Option<(String, String)> {
    // Look for: WHITESPACE (no newline) followed by COMMENT
    let mut i = node_idx + 1;
    let mut whitespace = String::new();

    while i < children.len() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                match t.kind() {
                    SyntaxKind::WHITESPACE => {
                        if t.text().contains('\n') {
                            return None; // Newline means no trailing comment
                        }
                        whitespace = t.text().to_string();
                    }
                    SyntaxKind::COMMENT => {
                        return Some((whitespace, t.text().to_string()));
                    }
                    _ => return None,
                }
            }
            NodeOrToken::Node(_) => return None,
        }
        i += 1;
    }
    None
}

/// Format a block expression including braces.
fn format_block_with_braces(block: &ast::BlockExpr, indent: usize) -> String {
    let mut buf = String::from("{\n");
    buf.push_str(&format_block_contents(block, indent + 4));
    write_indent(&mut buf, indent);
    buf.push('}');
    buf
}

pub fn format_if_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let if_expr = ast::IfExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&if_expr);

    let condition = if_expr.condition()?;
    let then_branch = if_expr.then_branch()?;

    let mut buf = String::new();
    buf.push_str(&attrs);
    buf.push_str("if ");

    // Format condition
    match try_format_expr_inner(condition.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&condition.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&then_branch, indent));

    // Handle else branch
    if let Some(else_branch) = if_expr.else_branch() {
        buf.push_str(" else ");
        match else_branch {
            ast::ElseBranch::IfExpr(else_if) => {
                // Recursive: else if ...
                match try_format_expr_inner(else_if.syntax(), indent) {
                    Some(s) => buf.push_str(&s),
                    None => buf.push_str(&else_if.syntax().text().to_string()),
                }
            }
            ast::ElseBranch::Block(else_block) => {
                buf.push_str(&format_block_with_braces(&else_block, indent));
            }
        }
    }

    Some(buf)
}

/// Check if a block expression is empty (no statements, no tail expr, no comments)
fn is_block_empty(block: &ast::BlockExpr) -> bool {
    if let Some(stmt_list) = block.stmt_list() {
        // Check for comments
        if stmt_list.statements().next().is_some() {
            return false;
        }
        if stmt_list.tail_expr().is_some() {
            return false;
        }
        for child in stmt_list.syntax().children_with_tokens() {
            if let NodeOrToken::Token(t) = child
                && t.kind() == SyntaxKind::COMMENT
            {
                return false;
            }
        }
        true
    } else {
        true
    }
}

pub fn format_match_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let match_expr = ast::MatchExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&match_expr);

    let scrutinee = match_expr.expr()?;
    let arm_list = match_expr.match_arm_list()?;

    let mut buf = String::new();
    buf.push_str(&attrs);
    buf.push_str("match ");

    match try_format_expr_inner(scrutinee.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&scrutinee.syntax().text().to_string()),
    }

    buf.push_str(" {\n");

    // Collect all children (arms and comments) to preserve comment positioning
    let children: Vec<_> = arm_list.syntax().children_with_tokens().collect();
    let mut prev_was_arm = false;

    for (idx, child) in children.iter().enumerate() {
        match child {
            NodeOrToken::Token(t) if t.kind() == SyntaxKind::COMMENT => {
                // Check for blank line before this comment
                if prev_was_arm && has_blank_line_before(&children, idx) {
                    buf.push('\n');
                }
                write_indent(&mut buf, indent + 4);
                buf.push_str(t.text());
                buf.push('\n');
                prev_was_arm = false;
            }
            NodeOrToken::Node(n) if n.kind() == SyntaxKind::MATCH_ARM => {
                if let Some(arm) = ast::MatchArm::cast(n.clone()) {
                    // Check for blank line before this arm
                    if prev_was_arm && has_blank_line_before(&children, idx) {
                        buf.push('\n');
                    }

                    write_indent(&mut buf, indent + 4);

                    // Arm attributes
                    for attr in arm.attrs() {
                        buf.push_str(&attr.syntax().text().to_string());
                        buf.push(' ');
                    }

                    // Pattern
                    if let Some(pat) = arm.pat() {
                        buf.push_str(&pat.syntax().text().to_string());
                    }

                    // Guard
                    if let Some(guard) = arm.guard() {
                        buf.push_str(" if ");
                        if let Some(cond) = guard.condition() {
                            match try_format_expr_inner(cond.syntax(), indent + 4) {
                                Some(s) => buf.push_str(&s),
                                None => buf.push_str(&cond.syntax().text().to_string()),
                            }
                        }
                    }

                    buf.push_str(" => ");

                    // Arm expression
                    if let Some(expr) = arm.expr() {
                        // Check if it's an empty block - keep it inline as {}
                        if let ast::Expr::BlockExpr(block) = &expr
                            && is_block_empty(block)
                        {
                            buf.newline("{}");
                            prev_was_arm = true;
                            continue;
                        }

                        let is_block = matches!(expr, ast::Expr::BlockExpr(_));

                        match try_format_expr_inner(expr.syntax(), indent + 4) {
                            Some(s) => buf.push_str(&s),
                            None => buf.push_str(&expr.syntax().text().to_string()),
                        }

                        // No comma after block expressions in match arms
                        if is_block {
                            buf.push('\n');
                        } else {
                            buf.push_str(",\n");
                        }
                    } else {
                        buf.push_str(",\n");
                    }

                    prev_was_arm = true;
                }
            }
            _ => {}
        }
    }

    write_indent(&mut buf, indent);
    buf.push('}');

    Some(buf)
}

/// Check if there's a blank line (2+ newlines) before the item at the given index
fn has_blank_line_before(
    children: &[NodeOrToken<SyntaxNode, ra_ap_syntax::SyntaxToken>],
    idx: usize,
) -> bool {
    for i in (0..idx).rev() {
        match &children[i] {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::WHITESPACE {
                    if t.text().matches('\n').count() >= 2 {
                        return true;
                    }
                } else if t.kind() != SyntaxKind::COMMENT {
                    return false;
                }
            }
            NodeOrToken::Node(_) => return false,
        }
    }
    false
}

pub fn format_loop_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let loop_expr = ast::LoopExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&loop_expr);

    let body = loop_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = loop_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("loop ");
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_while_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let while_expr = ast::WhileExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&while_expr);

    let condition = while_expr.condition()?;
    let body = while_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = while_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("while ");

    match try_format_expr_inner(condition.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&condition.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_for_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let for_expr = ast::ForExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&for_expr);

    let pat = for_expr.pat()?;
    let iterable = for_expr.iterable()?;
    let body = for_expr.loop_body()?;

    let mut buf = String::new();
    buf.push_str(&attrs);

    // Optional label
    if let Some(label) = for_expr.label() {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    buf.push_str("for ");
    buf.push_str(&pat.syntax().text().to_string());
    buf.push_str(" in ");

    match try_format_expr_inner(iterable.syntax(), indent) {
        Some(s) => buf.push_str(&s),
        None => buf.push_str(&iterable.syntax().text().to_string()),
    }

    buf.push(' ');
    buf.push_str(&format_block_with_braces(&body, indent));

    Some(buf)
}

pub fn format_block_expr(node: &SyntaxNode, indent: usize) -> Option<String> {
    let block = ast::BlockExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&block);

    // Check for label
    let label = block.label();

    // Check for async/unsafe/try/const
    let is_async = block.async_token().is_some();
    let is_unsafe = block.unsafe_token().is_some();
    let is_try = block.try_token().is_some();
    let is_const = block.const_token().is_some();

    let mut buf = String::new();
    buf.push_str(&attrs);

    if let Some(label) = label {
        buf.push_str(&label.syntax().text().to_string());
        buf.push(' ');
    }

    if is_try {
        buf.push_str("try ");
    }
    if is_const {
        buf.push_str("const ");
    }
    if is_async {
        buf.push_str("async ");
    }
    if is_unsafe {
        buf.push_str("unsafe ");
    }

    buf.push_str(&format_block_with_braces(&block, indent));

    Some(buf)
}

pub fn format_closure_expr(node: &SyntaxNode, _indent: usize) -> Option<String> {
    let closure = ast::ClosureExpr::cast(node.clone())?;
    let attrs = format_expr_attrs(&closure);

    let mut buf = String::new();
    buf.push_str(&attrs);

    // for<'a> binder
    if let Some(for_binder) = closure.for_binder() {
        buf.push_str(&for_binder.syntax().text().to_string());
        buf.push(' ');
    }

    if closure.const_token().is_some() {
        buf.push_str("const ");
    }
    if closure.static_token().is_some() {
        buf.push_str("static ");
    }
    if closure.async_token().is_some() {
        buf.push_str("async ");
    }
    if closure.move_token().is_some() {
        buf.push_str("move ");
    }

    // Parameter list
    if let Some(param_list) = closure.param_list() {
        buf.push('|');
        let params: Vec<_> = param_list.params().collect();
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(&param.syntax().text().to_string());
        }
        buf.push('|');
    }

    // Return type
    if let Some(ret_type) = closure.ret_type() {
        buf.push(' ');
        buf.push_str(&ret_type.syntax().text().to_string());
    }

    // Body - closures are tricky because they can be:
    // 1. Simple expression: |x| x + 1
    // 2. Block expression: |x| { ... }
    //
    // For now, preserve the body verbatim to avoid breaking method chains.
    // The issue is that when we format a closure's block body, it interacts
    // badly with the parent call expression's formatting.
    if let Some(body) = closure.body() {
        buf.push(' ');
        buf.push_str(&body.syntax().text().to_string());
    }

    Some(buf)
}
