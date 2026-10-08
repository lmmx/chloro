//! Closures (rustfmt's `closures.rs`).
//!
//! This module is messy because of the rules around closures and blocks:
//!   * if there is a return type, then there must be braces,
//!   * given a closure with braces, whether that is parsed to give an inner block
//!     or not depends on if there is a return type and if there are statements
//!     in that block,
//!   * if the first expression in the body ends with a block (i.e., is a
//!     statement without needing a semi-colon), then adding or removing braces
//!     can change whether it is treated as an expression or statement.

use ra_ap_syntax::ast::{self, AstNode};

use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{
    block_contains_comment, expr_attrs, expr_span, is_simple_block, rewrite_block_with_visitor,
    rewrite_cond,
};
use super::items::{span_hi_for_param, span_lo_for_param};
use super::lists::{
    DefinitiveListTactic, ListFormatting, ListTactic, Separator, definitive_tactic, itemize_list,
    write_list,
};
use super::nodes::{Block, StmtKind};
use super::overflow::OverflowableItem;
use super::shape::Shape;
use super::span::{Span, Spanned};
use super::types::{binder_params, rewrite_bound_params};
use super::utils::{last_line_width, left_most_sub_expr};

/// The body of a closure as a plain block, if it is one (rustc's `ExprKind::Block`; `async`,
/// `const`, `gen` and `try` blocks are distinct expression kinds in rustc).
fn plain_block(expr: &ast::Expr) -> Option<(ast::BlockExpr, Block)> {
    match expr {
        ast::Expr::BlockExpr(b)
            if b.async_token().is_none()
                && b.const_token().is_none()
                && b.gen_token().is_none()
                && b.try_token().is_none() =>
        {
            Some((b.clone(), Block::from_block_expr(b)?))
        }
        _ => None,
    }
}

pub(crate) fn rewrite_closure(
    closure: &ast::ClosureExpr,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let body = closure.body()?;
    let (prefix, extra_offset) = rewrite_closure_fn_decl(closure, &body, span, context, shape)?;
    // 1 = space between `|...|` and body.
    let body_shape = shape.offset_left(extra_offset)?;

    if let Some((_, block)) = plain_block(&body) {
        // The body of the closure is an empty block.
        if block.stmts.is_empty() && !block_contains_comment(context, &block) {
            return body
                .rewrite(context, shape)
                .map(|s| format!("{prefix} {s}"));
        }

        let result = if closure.ret_type().is_none() && !context.inside_macro() {
            try_rewrite_without_block(&body, &prefix, context, shape, body_shape)
        } else {
            None
        };

        result.or_else(|| {
            // Either we require a block, or tried without and failed.
            rewrite_closure_block(&body, &prefix, context, body_shape)
        })
    } else {
        rewrite_closure_expr(&body, &prefix, context, body_shape).or_else(|| {
            // The closure originally had a non-block expression, but we can't fit on
            // one line, so we'll insert a block.
            rewrite_closure_with_block(&body, &prefix, context, body_shape)
        })
    }
}

fn try_rewrite_without_block(
    expr: &ast::Expr,
    prefix: &str,
    context: &RewriteContext<'_>,
    shape: Shape,
    body_shape: Shape,
) -> Option<String> {
    let expr = get_inner_expr(expr, prefix, context);

    if is_block_closure_forced(context, &expr) {
        rewrite_closure_with_block(&expr, prefix, context, shape)
    } else {
        rewrite_closure_expr(&expr, prefix, context, body_shape)
    }
}

fn get_inner_expr(expr: &ast::Expr, prefix: &str, context: &RewriteContext<'_>) -> ast::Expr {
    if let Some((block_expr, block)) = plain_block(expr)
        && !needs_block(&block, block_expr.label().is_some(), prefix, context)
    {
        // block.stmts.len() == 1 except with `|| {{}}`;
        // https://github.com/rust-lang/rustfmt/issues/3844
        if let Some(expr) = block.stmts.first().and_then(|s| s.expr()) {
            return get_inner_expr(expr, prefix, context);
        }
    }

    expr.clone()
}

/// Figure out if a block is necessary.
fn needs_block(block: &Block, has_label: bool, prefix: &str, context: &RewriteContext<'_>) -> bool {
    let has_attributes = block
        .stmts
        .first()
        .is_some_and(|first_stmt| !first_stmt.attrs().is_empty());

    block.is_unsafe()
        || block.stmts.len() > 1
        || has_attributes
        || block_contains_comment(context, block)
        || prefix.contains('\n')
        || has_label
}

fn veto_block(e: &ast::Expr) -> bool {
    matches!(
        e,
        ast::Expr::CallExpr(..)
            | ast::Expr::BinExpr(..)
            | ast::Expr::CastExpr(..)
            | ast::Expr::FieldExpr(..)
            | ast::Expr::IndexExpr(..)
            | ast::Expr::RangeExpr(..)
            | ast::Expr::TryExpr(..)
    )
}

/// Rewrite closure with a single expression wrapping its body with block.
/// `|| { #[attr] foo() }` -> `Block { #[attr] foo() }`
fn rewrite_closure_with_block(
    body: &ast::Expr,
    prefix: &str,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let left_most = left_most_sub_expr(body);
    let veto_block = veto_block(body) && !expr_requires_semi_to_be_stmt(&left_most);
    if veto_block {
        return None;
    }

    let block = Block::synthetic(body);
    let attrs = expr_attrs(body);
    let block = rewrite_block_with_visitor(context, "", &block, Some(&attrs), None, shape, false)?;
    Some(format!("{prefix} {block}"))
}

/// Rewrite closure with a single expression without wrapping its body with block.
fn rewrite_closure_expr(
    expr: &ast::Expr,
    prefix: &str,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    fn allow_multi_line(expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::MatchExpr(..)
            | ast::Expr::BlockExpr(..)
            | ast::Expr::LoopExpr(..)
            | ast::Expr::RecordExpr(..) => true,

            ast::Expr::RefExpr(r) => r.expr().is_some_and(|e| allow_multi_line(&e)),
            ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| allow_multi_line(&e)),
            ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| allow_multi_line(&e)),
            ast::Expr::CastExpr(c) => c.expr().is_some_and(|e| allow_multi_line(&e)),

            _ => false,
        }
    }

    // When rewriting closure's body without block, we require it to fit in a single line
    // unless it is a block-like expression or we are inside macro call.
    let veto_multiline = (!allow_multi_line(expr) && !context.inside_macro())
        || context.config.force_multiline_blocks();
    expr.rewrite(context, shape)
        .filter(|rw| !(veto_multiline && rw.contains('\n')))
        .map(|rw| format!("{prefix} {rw}"))
}

/// Rewrite closure whose body is block.
fn rewrite_closure_block(
    block: &ast::Expr,
    prefix: &str,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    Some(format!("{} {}", prefix, block.rewrite(context, shape)?))
}

/// Return type is (prefix, extra_offset)
fn rewrite_closure_fn_decl(
    closure: &ast::ClosureExpr,
    body: &ast::Expr,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<(String, usize)> {
    let binder = match closure.for_binder() {
        Some(binder) => {
            let generic_params = binder_params(Some(binder));
            match rewrite_bound_params(context, shape, &generic_params) {
                Some(lifetime_str) => format!("for<{lifetime_str}> "),
                None if generic_params.is_empty() => "for<> ".to_owned(),
                None => return None,
            }
        }
        None => String::new(),
    };

    let const_ = if closure.const_token().is_some() {
        "const "
    } else {
        ""
    };

    let immovable = if closure.static_token().is_some() {
        "static "
    } else {
        ""
    };
    let coro = match (closure.async_token(), closure.gen_token()) {
        (Some(_), Some(_)) => "async gen ",
        (Some(_), None) => "async ",
        (None, Some(_)) => "gen ",
        (None, None) => "",
    };
    let capture_str = if closure.move_token().is_some() {
        "move "
    } else if closure
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t| t.kind() == ra_ap_syntax::SyntaxKind::USE_KW)
    {
        "use "
    } else {
        ""
    };
    // 4 = "|| {".len(), which is overconservative when the closure consists of
    // a single expression.
    let offset = binder.len() + const_.len() + immovable.len() + coro.len() + capture_str.len();
    let nested_shape = shape.shrink_left(offset)?.sub_width(4)?;

    // 1 = |
    let param_offset = nested_shape.indent + 1;
    let param_shape = nested_shape.offset_left(1)?.visual_indent(0);
    let ret_str = match closure.ret_type() {
        Some(ret) => {
            let ty = ret.ty()?;
            let arrow_width = "-> ".len();
            let shape = param_shape.offset_left(arrow_width)?;
            format!("-> {}", ty.rewrite(context, shape)?)
        }
        None => String::new(),
    };

    let params: Vec<ast::Param> = closure
        .param_list()
        .map(|l| l.params().collect())
        .unwrap_or_default();
    let param_items = itemize_list(
        context.snippet_provider,
        params.iter(),
        "|",
        ",",
        |param| span_lo_for_param(param),
        |param| span_hi_for_param(context, param),
        |param| param.rewrite(context, param_shape),
        context.snippet_provider.span_after(span, "|"),
        body.span().lo(),
        false,
    );
    let item_vec = param_items.collect::<Vec<_>>();
    // 1 = space between parameters and return type.
    let horizontal_budget = nested_shape.width.saturating_sub(ret_str.len() + 1);
    let tactic = definitive_tactic(
        &item_vec,
        ListTactic::HorizontalVertical,
        Separator::Comma,
        horizontal_budget,
    );
    let param_shape = match tactic {
        DefinitiveListTactic::Horizontal => param_shape.sub_width(ret_str.len() + 1)?,
        _ => param_shape,
    };

    let fmt = ListFormatting::new(param_shape, context.config)
        .tactic(tactic)
        .preserve_newline(true);
    let list_str = write_list(&item_vec, &fmt)?;
    let mut prefix = format!("{binder}{const_}{immovable}{coro}{capture_str}|{list_str}|");

    if !ret_str.is_empty() {
        if prefix.contains('\n') {
            prefix.push('\n');
            prefix.push_str(&param_offset.to_string(context.config));
        } else {
            prefix.push(' ');
        }
        prefix.push_str(&ret_str);
    }
    // 1 = space between `|...|` and body.
    let extra_offset = last_line_width(&prefix) + 1;

    Some((prefix, extra_offset))
}

/// Rewriting closure which is placed at the end of the function call's arg.
/// Returns `None` if the reformatted closure 'looks bad'.
pub(crate) fn rewrite_last_closure(
    context: &RewriteContext<'_>,
    expr: &ast::Expr,
    shape: Shape,
) -> Option<String> {
    let ast::Expr::ClosureExpr(closure) = expr else {
        return None;
    };
    let body = closure.body()?;
    let body = match plain_block(&body) {
        Some((block_expr, block))
            if !block.is_unsafe()
                && !context.inside_macro()
                && is_simple_block(context, &block, Some(&expr_attrs(&body)))
                && block_expr.label().is_none() =>
        {
            match &block.stmts[0].kind {
                StmtKind::Expr(e) => e.clone(),
                _ => body,
            }
        }
        _ => body,
    };
    let (prefix, extra_offset) =
        rewrite_closure_fn_decl(closure, &body, expr_span(expr), context, shape)?;
    // If the closure goes multi line before its body, do not overflow the closure.
    if prefix.contains('\n') {
        return None;
    }

    let body_shape = shape.offset_left(extra_offset)?;

    // We force to use block for the body of the closure for certain kinds of expressions.
    if is_block_closure_forced(context, &body) {
        return rewrite_closure_with_block(&body, &prefix, context, body_shape).map(|body_str| {
            match closure.ret_type() {
                None if body_str.lines().count() <= 7 => {
                    // If the expression can fit in a single line, we need not force block
                    // closure.  However, if the closure has a return type, then we must
                    // keep the blocks.
                    match rewrite_closure_expr(&body, &prefix, context, shape) {
                        Some(single_line_body_str) if !single_line_body_str.contains('\n') => {
                            single_line_body_str
                        }
                        _ => body_str,
                    }
                }
                _ => body_str,
            }
        });
    }

    // When overflowing the closure which consists of a single control flow expression,
    // force to use block if its condition uses multi line.
    let is_multi_lined_cond = rewrite_cond(context, &body, body_shape)
        .is_some_and(|cond| cond.contains('\n') || cond.len() > body_shape.width);
    if is_multi_lined_cond {
        return rewrite_closure_with_block(&body, &prefix, context, body_shape);
    }

    // Seems fine, just format the closure in usual manner.
    expr.rewrite(context, shape)
}

/// Returns `true` if the given vector of arguments has more than one closure.
pub(crate) fn args_have_many_closure(args: &[OverflowableItem]) -> bool {
    args.iter()
        .filter_map(OverflowableItem::to_expr)
        .filter(|expr| matches!(expr, ast::Expr::ClosureExpr(..)))
        .count()
        > 1
}

fn is_block_closure_forced(context: &RewriteContext<'_>, expr: &ast::Expr) -> bool {
    // If we are inside macro, we do not want to add or remove block from closure body.
    if context.inside_macro() {
        false
    } else {
        is_block_closure_forced_inner(expr, context.config.style_edition())
    }
}

fn is_block_closure_forced_inner(expr: &ast::Expr, style_edition: StyleEdition) -> bool {
    match expr {
        ast::Expr::IfExpr(..) | ast::Expr::WhileExpr(..) | ast::Expr::ForExpr(..) => true,
        ast::Expr::LoopExpr(..) if style_edition >= StyleEdition::Edition2024 => true,
        ast::Expr::RefExpr(r) => r
            .expr()
            .is_some_and(|e| is_block_closure_forced_inner(&e, style_edition)),
        ast::Expr::TryExpr(t) => t
            .expr()
            .is_some_and(|e| is_block_closure_forced_inner(&e, style_edition)),
        ast::Expr::PrefixExpr(p) => p
            .expr()
            .is_some_and(|e| is_block_closure_forced_inner(&e, style_edition)),
        ast::Expr::CastExpr(c) => c
            .expr()
            .is_some_and(|e| is_block_closure_forced_inner(&e, style_edition)),
        _ => false,
    }
}

/// Does this expression require a semicolon to be treated
/// as a statement? The negation of this: 'can this expression
/// be used as a statement without a semicolon' -- is used
/// as an early-bail-out in the parser so that, for instance,
///     if true {...} else {...}
///      |x| 5
/// isn't parsed as (if true {...} else {...} | x) | 5
fn expr_requires_semi_to_be_stmt(e: &ast::Expr) -> bool {
    !matches!(
        e,
        ast::Expr::IfExpr(..)
            | ast::Expr::MatchExpr(..)
            | ast::Expr::BlockExpr(..)
            | ast::Expr::WhileExpr(..)
            | ast::Expr::LoopExpr(..)
            | ast::Expr::ForExpr(..)
    )
}
