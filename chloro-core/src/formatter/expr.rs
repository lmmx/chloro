//! Expressions (rustfmt's `expr.rs`).

use std::borrow::Cow;
use std::cmp::min;

use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::ast::{self, AstNode, AstToken, HasArgList, HasLoopBody, IsString};

use super::attr::rewrite_attrs;
use super::chains::rewrite_chain;
use super::closures;
use super::comment::{
    CharClasses, FindUncommented, combine_strs_with_missing_comments, contains_comment,
    recover_comment_removed, rewrite_comment, rewrite_missing_comment,
};
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::lists::{
    SeparatorPlace, SeparatorTactic, itemize_list, shape_for_tactic, struct_lit_formatting,
    struct_lit_shape, struct_lit_tactic, write_list,
};
use super::macros::{MacroPosition, rewrite_macro};
use super::matches::rewrite_match;
use super::nodes::{
    Attribute, Block, BlockExprKind, Stmt, StmtKind, block_expr_kind, contains_skip,
    inner_attributes, outer_attributes, span_without_attrs,
};
use super::overflow::{self, Delimiter, OverflowableItem};
use super::pairs::{PairParts, bin_op_text, rewrite_all_pairs, rewrite_pair};
use super::shape::{Indent, Shape};
use super::span::{Span, Spanned, mk_sp, token_span};
use super::stmt::Stmt as FmtStmt;
use super::types::{PathContext, rewrite_path};
use super::utils::{
    colon_spaces, count_newlines, filtered_str_fits, first_line_ends_with, is_assignment,
    last_line_extendable, last_line_width, semicolon_for_expr, unicode_str_width, wrap_str,
};
use super::visitor::FmtVisitor;

impl Rewrite for ast::Expr {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        format_expr(self, ExprType::SubExpression, context, shape)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ExprType {
    Statement,
    SubExpression,
}

/// The kinds of literal that rustfmt treats differently.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum LitKind {
    Str,
    StrRaw,
    Integer,
    Float,
    Other,
}

impl LitKind {
    pub(crate) fn of(lit: &ast::Literal) -> LitKind {
        match lit.kind() {
            ast::LiteralKind::String(s) => {
                if s.is_raw() {
                    LitKind::StrRaw
                } else {
                    LitKind::Str
                }
            }
            ast::LiteralKind::IntNumber(_) => LitKind::Integer,
            ast::LiteralKind::FloatNumber(_) => LitKind::Float,
            _ => LitKind::Other,
        }
    }
}

/// rustc's `expr.span`: the expression without its outer attributes.
pub(crate) fn expr_span(expr: &ast::Expr) -> Span {
    span_without_attrs(expr.syntax())
}

/// The attributes rustc attaches to an expression: its outer attributes and, for block-like
/// expressions, the inner attributes of the block.
pub(crate) fn expr_attrs(expr: &ast::Expr) -> Vec<Attribute> {
    let mut attrs = outer_attributes(expr.syntax());
    if let ast::Expr::BlockExpr(b) = expr
        && let Some(list) = b.stmt_list()
    {
        attrs.extend(inner_attributes(list.syntax()));
    }
    attrs
}

pub(crate) fn lit_ends_in_dot(lit: &ast::Literal) -> bool {
    match lit.kind() {
        ast::LiteralKind::FloatNumber(f) => {
            let text = f.text();
            // `Preserve` is the only (unstable, default) `float_literal_trailing_zero`.
            text.ends_with('.')
        }
        _ => false,
    }
}

pub(crate) fn format_expr(
    expr: &ast::Expr,
    expr_type: ExprType,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    context.memoize(expr.syntax(), expr_type as u8, shape, || {
        format_expr_uncached(expr, expr_type, context, shape)
    })
}

fn format_expr_uncached(
    expr: &ast::Expr,
    expr_type: ExprType,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let outer = outer_attributes(expr.syntax());
    if contains_skip(&outer) {
        return Some(context.snippet(expr.span()).to_owned());
    }
    let shape = if expr_type == ExprType::Statement && semicolon_for_expr(context, expr) {
        shape.sub_width(1)?
    } else {
        shape
    };
    let span = expr_span(expr);

    let expr_rw = match expr {
        ast::Expr::ArrayExpr(array) => {
            if array.semicolon_token().is_some() {
                let mut exprs = array.exprs();
                let value = exprs.next()?;
                let repeats = exprs.next()?;
                rewrite_pair(
                    &value,
                    &repeats,
                    PairParts::new("[", "; ", "]"),
                    context,
                    shape,
                    SeparatorPlace::Back,
                )
            } else {
                rewrite_array(
                    "",
                    array.exprs().map(OverflowableItem::Expr),
                    span,
                    context,
                    shape,
                    choose_separator_tactic(context, span),
                    None,
                )
            }
        }
        ast::Expr::Literal(lit) => {
            let kind = LitKind::of(lit);
            rewrite_literal(context, kind, span, shape).or_else(|| {
                (kind == LitKind::StrRaw).then(|| context.snippet(span).trim().to_owned())
            })
        }
        ast::Expr::CallExpr(call) => {
            let callee = call.expr()?;
            let inner_span = mk_sp(callee.span().hi(), span.hi());
            let callee_str = callee.rewrite(context, shape)?;
            let args: Vec<_> = call
                .arg_list()?
                .args()
                .map(OverflowableItem::Expr)
                .collect();
            rewrite_call(context, &callee_str, args, inner_span, shape)
        }
        ast::Expr::ParenExpr(p) => rewrite_paren(context, p, shape, span),
        ast::Expr::BinExpr(bin) if is_assignment(bin) => rewrite_assignment(context, bin, shape),
        ast::Expr::BinExpr(bin) => {
            // FIXME: format comments between operands and operator
            rewrite_all_pairs(bin, shape, context).or_else(|| {
                rewrite_pair(
                    &bin.lhs()?,
                    &bin.rhs()?,
                    PairParts::infix(&format!(" {} ", bin_op_text(bin)?)),
                    context,
                    shape,
                    context.config.binop_separator(),
                )
            })
        }
        ast::Expr::PrefixExpr(p) => {
            let op = p.op_token()?;
            rewrite_unary_prefix(context, op.text(), &p.expr()?, shape)
        }
        ast::Expr::RecordExpr(r) => rewrite_struct_lit(context, r, &expr_attrs(expr), span, shape),
        ast::Expr::TupleExpr(t) => {
            let fields: Vec<_> = t.fields().map(OverflowableItem::Expr).collect();
            let len = fields.len();
            rewrite_tuple(context, fields, span, shape, len == 1)
        }
        ast::Expr::LetExpr(l) => rewrite_let(context, shape, &l.pat()?, &l.expr()?),
        ast::Expr::IfExpr(_)
        | ast::Expr::ForExpr(_)
        | ast::Expr::LoopExpr(_)
        | ast::Expr::WhileExpr(_) => {
            to_control_flow(expr, expr_type).and_then(|cf| cf.rewrite(context, shape))
        }
        ast::Expr::BlockExpr(b) => rewrite_block_expr(expr, b, expr_type, context, shape),
        ast::Expr::MatchExpr(m) => rewrite_match(context, m, shape, span, &expr_attrs(expr)),
        ast::Expr::PathExpr(p) => rewrite_path(context, PathContext::Expr, &p.path()?, shape),
        ast::Expr::ContinueExpr(c) => {
            let id_str = match c.lifetime() {
                Some(label) => format!(" {}", context.snippet(label.span())),
                None => String::new(),
            };
            Some(format!("continue{id_str}"))
        }
        ast::Expr::BreakExpr(b) => {
            let id_str = match b.lifetime() {
                Some(label) => format!(" {}", context.snippet(label.span())),
                None => String::new(),
            };
            if let Some(e) = b.expr() {
                rewrite_unary_prefix(context, &format!("break{id_str} "), &e, shape)
            } else {
                Some(format!("break{id_str}"))
            }
        }
        ast::Expr::YieldExpr(y) => match y.expr() {
            Some(e) => rewrite_unary_prefix(context, "yield ", &e, shape),
            None => Some("yield".to_string()),
        },
        ast::Expr::ClosureExpr(c) => closures::rewrite_closure(c, span, context, shape),
        ast::Expr::TryExpr(_)
        | ast::Expr::FieldExpr(_)
        | ast::Expr::MethodCallExpr(_)
        | ast::Expr::AwaitExpr(_) => rewrite_chain(expr, context, shape),
        ast::Expr::MacroExpr(m) => {
            rewrite_macro(&m.macro_call()?, context, shape, MacroPosition::Expression).or_else(
                || {
                    wrap_str(
                        context.snippet(span).to_owned(),
                        context.config.max_width(),
                        shape,
                    )
                },
            )
        }
        ast::Expr::ReturnExpr(r) => match r.expr() {
            None => Some("return".to_owned()),
            Some(e) => rewrite_unary_prefix(context, "return ", &e, shape),
        },
        ast::Expr::BecomeExpr(b) => rewrite_unary_prefix(context, "become ", &b.expr()?, shape),
        ast::Expr::YeetExpr(y) => match y.expr() {
            None => Some("do yeet".to_owned()),
            Some(e) => rewrite_unary_prefix(context, "do yeet ", &e, shape),
        },
        ast::Expr::RefExpr(r) => {
            let operator_str = match (r.raw_token(), r.const_token(), r.mut_token()) {
                (Some(_), _, Some(_)) => "&raw mut ",
                (Some(_), _, None) => "&raw const ",
                (None, _, Some(_)) => "&mut ",
                (None, _, None) => "&",
            };
            rewrite_unary_prefix(context, operator_str, &r.expr()?, shape)
        }
        ast::Expr::CastExpr(c) => rewrite_pair(
            &c.expr()?,
            &c.ty()?,
            PairParts::infix(" as "),
            context,
            shape,
            SeparatorPlace::Front,
        ),
        ast::Expr::IndexExpr(i) => rewrite_index(&i.base()?, &i.index()?, context, shape),
        ast::Expr::RangeExpr(r) => rewrite_range(context, r, shape),
        // We do not format these expressions yet, but they should still
        // satisfy our width restrictions.
        ast::Expr::AsmExpr(_) => Some(context.snippet(span).to_owned()),
        ast::Expr::UnderscoreExpr(_) => Some("_".to_owned()),
        ast::Expr::FormatArgsExpr(_) | ast::Expr::OffsetOfExpr(_) => None,
    };

    let expr_str = expr_rw.map(|expr_str| recover_comment_removed(expr_str, span, context))?;
    let attrs_str = rewrite_attrs(&outer, context, shape)?;
    let attrs_span = mk_sp(
        outer.last().map_or(span.lo(), |attr| attr.span().hi()),
        span.lo(),
    );
    combine_strs_with_missing_comments(context, &attrs_str, &expr_str, attrs_span, shape, false)
}

fn rewrite_range(context: &RewriteContext<'_>, r: &ast::RangeExpr, shape: Shape) -> Option<String> {
    use ast::RangeItem;
    let delim = match r.op_kind()? {
        ast::RangeOp::Exclusive => "..",
        ast::RangeOp::Inclusive => "..=",
    };
    // `...` is accepted by the parser in old code; preserve the token as written.
    let delim = match r.op_token() {
        Some(t) if t.text() == "..." => "...",
        _ => delim,
    };

    fn needs_space_before_range(lhs: &ast::Expr) -> bool {
        match lhs {
            ast::Expr::Literal(lit) => lit_ends_in_dot(lit),
            ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| needs_space_before_range(&e)),
            ast::Expr::BinExpr(b) if !is_assignment(b) => {
                b.rhs().is_some_and(|e| needs_space_before_range(&e))
            }
            _ => false,
        }
    }

    fn needs_space_after_range(rhs: &ast::Expr) -> bool {
        // Don't format `.. ..` into `....`, which is invalid.
        //
        // This check is unnecessary for `lhs`, because a range
        // starting from another range needs parentheses as `(x ..) ..`
        // (`x .. ..` is a range from `x` to `..`).
        matches!(rhs, ast::Expr::RangeExpr(r) if r.start().is_none())
    }

    let default_sp_delim = |lhs: Option<&ast::Expr>, rhs: Option<&ast::Expr>| {
        let space_if = |b: bool| if b { " " } else { "" };

        format!(
            "{}{}{}",
            lhs.map_or("", |lhs| space_if(needs_space_before_range(lhs))),
            delim,
            rhs.map_or("", |rhs| space_if(needs_space_after_range(rhs))),
        )
    };

    match (r.start(), r.end()) {
        (Some(lhs), Some(rhs)) => {
            let sp_delim = if context.config.spaces_around_ranges() {
                format!(" {delim} ")
            } else {
                default_sp_delim(Some(&lhs), Some(&rhs))
            };
            rewrite_pair(
                &lhs,
                &rhs,
                PairParts::infix(&sp_delim),
                context,
                shape,
                context.config.binop_separator(),
            )
        }
        (None, Some(rhs)) => {
            let sp_delim = if context.config.spaces_around_ranges() {
                format!("{delim} ")
            } else {
                default_sp_delim(None, Some(&rhs))
            };
            rewrite_unary_prefix(context, &sp_delim, &rhs, shape)
        }
        (Some(lhs), None) => {
            let sp_delim = if context.config.spaces_around_ranges() {
                format!(" {delim}")
            } else {
                default_sp_delim(Some(&lhs), None)
            };
            rewrite_unary_suffix(context, &sp_delim, &lhs, shape)
        }
        (None, None) => Some(delim.to_owned()),
    }
}

/// The keyword prefix of a block expression (`async move `, `try `, `gen `, ...), for the
/// block kinds rustc represents as distinct expressions.
fn block_modifiers(b: &ast::BlockExpr) -> Option<String> {
    let mut parts = Vec::new();
    for t in b
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
    {
        match t.kind() {
            SyntaxKind::ASYNC_KW => parts.push("async"),
            SyntaxKind::GEN_KW => parts.push("gen"),
            SyntaxKind::MOVE_KW => parts.push("move"),
            SyntaxKind::TRY_KW => parts.push("try"),
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" ") + " ")
    }
}

fn rewrite_block_expr(
    expr: &ast::Expr,
    b: &ast::BlockExpr,
    expr_type: ExprType,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let block = Block::from_block_expr(b)?;
    let attrs = expr_attrs(expr);
    let label = b.label();
    if b.const_token().is_some() {
        // Inner attributes are associated with the const block expression, not the inner block.
        let rewrite = rewrite_block(&block, Some(&attrs), label.as_ref(), context, shape)?;
        return Some(format!("const {rewrite}"));
    }
    if let Some(prefix) = block_modifiers(b) {
        // `async {}`, `async move {}`, `gen {}`, `try {}`
        if let Some(rw) =
            rewrite_single_line_block(context, &prefix, &block, Some(&attrs), None, shape)
        {
            return Some(rw);
        }
        // 6 = `async `, 9 = `try `
        let budget = shape
            .width
            .saturating_sub(if prefix == "try " { 9 } else { 6 });
        return Some(format!(
            "{prefix}{}",
            rewrite_block(
                &block,
                Some(&attrs),
                None,
                context,
                Shape::legacy(budget, shape.indent)
            )?
        ));
    }
    match expr_type {
        ExprType::Statement => {
            if block.is_unsafe() {
                rewrite_block(&block, Some(&attrs), label.as_ref(), context, shape)
            } else if let Some(rw) =
                rewrite_empty_block(context, &block, Some(&attrs), label.as_ref(), "", shape)
            {
                // Rewrite block without trying to put it in a single line.
                Some(rw)
            } else {
                let prefix = block_prefix(context, &block, shape)?;

                rewrite_block_with_visitor(
                    context,
                    &prefix,
                    &block,
                    Some(&attrs),
                    label.as_ref(),
                    shape,
                    true,
                )
            }
        }
        ExprType::SubExpression => {
            rewrite_block(&block, Some(&attrs), label.as_ref(), context, shape)
        }
    }
}

pub(crate) fn rewrite_array(
    name: &str,
    exprs: impl IntoIterator<Item = OverflowableItem>,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
    force_separator_tactic: Option<SeparatorTactic>,
    delim_token: Option<Delimiter>,
) -> Option<String> {
    overflow::rewrite_with_square_brackets(
        context,
        name,
        exprs,
        shape,
        span,
        force_separator_tactic,
        delim_token,
    )
}

fn has_inner_attrs(attrs: Option<&[Attribute]>) -> bool {
    attrs.is_some_and(|a| {
        a.iter()
            .any(|a| a.style() == super::nodes::AttrStyle::Inner)
    })
}

fn rewrite_empty_block(
    context: &RewriteContext<'_>,
    block: &Block,
    attrs: Option<&[Attribute]>,
    label: Option<&ast::Label>,
    prefix: &str,
    shape: Shape,
) -> Option<String> {
    if block_has_statements(block) {
        return None;
    }

    let label_str = rewrite_label(context, label);
    if has_inner_attrs(attrs) {
        return None;
    }

    if !block_contains_comment(context, block) && shape.width >= 2 {
        return Some(format!("{prefix}{label_str}{{}}"));
    }

    // If a block contains only a single-line comment, then leave it on one line.
    let user_str = context.snippet(block.span);
    let user_str = user_str.trim();
    if user_str.starts_with('{') && user_str.ends_with('}') {
        let comment_str = user_str[1..user_str.len() - 1].trim();
        if block.stmts.is_empty()
            && !comment_str.contains('\n')
            && !comment_str.starts_with("//")
            && comment_str.len() + 4 <= shape.width
        {
            return Some(format!("{prefix}{label_str}{{ {comment_str} }}"));
        }
    }

    None
}

fn block_prefix(context: &RewriteContext<'_>, block: &Block, shape: Shape) -> Option<String> {
    Some(match block.rules {
        super::nodes::BlockRules::Unsafe => {
            let snippet = context.snippet(block.span);
            let open_pos = snippet.find_uncommented("{")?;
            // Extract comment between unsafe and block start.
            let trimmed = snippet.get(6..open_pos)?.trim();

            if !trimmed.is_empty() {
                // 9 = "unsafe  {".len(), 7 = "unsafe ".len()
                let budget = shape.width.checked_sub(9)?;
                format!(
                    "unsafe {} ",
                    rewrite_comment(
                        trimmed,
                        true,
                        Shape::legacy(budget, shape.indent + 7),
                        context.config,
                    )?
                )
            } else {
                "unsafe ".to_owned()
            }
        }
        super::nodes::BlockRules::Default => String::new(),
    })
}

fn rewrite_single_line_block(
    context: &RewriteContext<'_>,
    prefix: &str,
    block: &Block,
    attrs: Option<&[Attribute]>,
    label: Option<&ast::Label>,
    shape: Shape,
) -> Option<String> {
    if let Some(block_expr) = FmtStmt::from_simple_block(context, block, attrs) {
        let expr_shape = shape.offset_left(last_line_width(prefix))?;
        let expr_str = block_expr.rewrite(context, expr_shape)?;
        let label_str = rewrite_label(context, label);
        let result = format!("{prefix}{label_str}{{ {expr_str} }}");
        if result.len() <= shape.width && !result.contains('\n') {
            return Some(result);
        }
    }
    None
}

pub(crate) fn rewrite_block_with_visitor(
    context: &RewriteContext<'_>,
    prefix: &str,
    block: &Block,
    attrs: Option<&[Attribute]>,
    label: Option<&ast::Label>,
    shape: Shape,
    has_braces: bool,
) -> Option<String> {
    if let Some(rw_str) = rewrite_empty_block(context, block, attrs, label, prefix, shape) {
        return Some(rw_str);
    }

    let mut visitor = FmtVisitor::from_context(context);
    visitor.block_indent = shape.indent;
    visitor.is_if_else_block = context.is_if_else_block();
    visitor.is_loop_block = context.is_loop_block();
    if block.is_unsafe() || label.is_some() {
        let snippet = context.snippet(block.span);
        let open_pos = snippet.find_uncommented("{")?;
        visitor.last_pos = block.span.lo() + open_pos as u32;
    } else {
        visitor.last_pos = block.span.lo();
    }

    let inner_attrs: Option<Vec<Attribute>> = attrs.map(|a| {
        a.iter()
            .filter(|a| a.style() == super::nodes::AttrStyle::Inner)
            .cloned()
            .collect()
    });
    let label_str = rewrite_label(context, label);
    visitor.visit_block(block, inner_attrs.as_deref(), has_braces);
    if visitor.macro_rewrite_failure {
        context.macro_rewrite_failure.replace(true);
    }
    Some(format!("{}{}{}", prefix, label_str, visitor.buffer))
}

impl Rewrite for Block {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        rewrite_block(self, None, None, context, shape)
    }
}

impl Rewrite for ast::BlockExpr {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        rewrite_block(&Block::from_block_expr(self)?, None, None, context, shape)
    }
}

pub(crate) fn rewrite_block(
    block: &Block,
    attrs: Option<&[Attribute]>,
    label: Option<&ast::Label>,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    rewrite_block_inner(block, attrs, label, true, context, shape)
}

fn rewrite_block_inner(
    block: &Block,
    attrs: Option<&[Attribute]>,
    label: Option<&ast::Label>,
    allow_single_line: bool,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let prefix = block_prefix(context, block, shape)?;

    // shape.width is used only for the single line case: either the empty block `{}`,
    // or an unsafe expression `unsafe { e }`.
    if let Some(rw_str) = rewrite_empty_block(context, block, attrs, label, &prefix, shape) {
        return Some(rw_str);
    }

    let result_str =
        rewrite_block_with_visitor(context, &prefix, block, attrs, label, shape, true)?;
    if allow_single_line
        && result_str.lines().count() <= 3
        && let Some(rw) = rewrite_single_line_block(context, &prefix, block, attrs, label, shape)
    {
        return Some(rw);
    }
    Some(result_str)
}

/// Rewrite the divergent block of a `let-else` statement.
pub(crate) fn rewrite_let_else_block(
    block: &Block,
    allow_single_line: bool,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    rewrite_block_inner(block, None, None, allow_single_line, context, shape)
}

/// Rewrite condition if the given expression has one.
pub(crate) fn rewrite_cond(
    context: &RewriteContext<'_>,
    expr: &ast::Expr,
    shape: Shape,
) -> Option<String> {
    match expr {
        ast::Expr::MatchExpr(m) => {
            // `match `cond` {`
            let cond_shape = shape.offset_left(8)?;
            m.expr()?.rewrite(context, cond_shape)
        }
        _ => to_control_flow(expr, ExprType::SubExpression).and_then(|control_flow| {
            let alt_block_sep =
                String::from("\n") + &shape.indent.block_only().to_string(context.config);
            control_flow
                .rewrite_cond(context, shape, &alt_block_sep)
                .map(|rw| rw.0)
        }),
    }
}

/// Abstraction over control flow expressions
#[derive(Debug)]
struct ControlFlow<'a> {
    cond: Option<ast::Expr>,
    block: Block,
    else_block: Option<ast::Expr>,
    label: Option<ast::Label>,
    pat: Option<ast::Pat>,
    keyword: &'a str,
    matcher: &'a str,
    connector: &'a str,
    allow_single_line: bool,
    /// HACK: `true` if this is an `if` expression in an `else if`.
    nested_if: bool,
    is_loop: bool,
    span: Span,
}

fn extract_pats_and_cond(expr: ast::Expr) -> (Option<ast::Pat>, Option<ast::Expr>) {
    match expr {
        ast::Expr::LetExpr(l) => (l.pat(), l.expr()),
        _ => (None, Some(expr)),
    }
}

fn else_branch_expr(branch: ast::ElseBranch) -> ast::Expr {
    match branch {
        ast::ElseBranch::Block(b) => ast::Expr::BlockExpr(b),
        ast::ElseBranch::IfExpr(i) => ast::Expr::IfExpr(i),
    }
}

fn to_control_flow(expr: &ast::Expr, expr_type: ExprType) -> Option<ControlFlow<'static>> {
    let span = expr_span(expr);
    match expr {
        ast::Expr::IfExpr(i) => {
            let (pat, cond) = extract_pats_and_cond(i.condition()?);
            Some(ControlFlow::new_if(
                cond?,
                pat,
                Block::from_block_expr(&i.then_branch()?)?,
                i.else_branch().map(else_branch_expr),
                expr_type == ExprType::SubExpression,
                false,
                span,
            ))
        }
        ast::Expr::ForExpr(f) => Some(ControlFlow {
            cond: Some(f.iterable()?),
            block: Block::from_block_expr(&f.loop_body()?)?,
            else_block: None,
            label: f.label(),
            pat: Some(f.pat()?),
            keyword: "for",
            matcher: "",
            connector: " in",
            allow_single_line: false,
            nested_if: false,
            is_loop: true,
            span,
        }),
        ast::Expr::LoopExpr(l) => Some(ControlFlow {
            cond: None,
            block: Block::from_block_expr(&l.loop_body()?)?,
            else_block: None,
            label: l.label(),
            pat: None,
            keyword: "loop",
            matcher: "",
            connector: "",
            allow_single_line: false,
            nested_if: false,
            is_loop: true,
            span,
        }),
        ast::Expr::WhileExpr(w) => {
            let (pat, cond) = extract_pats_and_cond(w.condition()?);
            Some(ControlFlow {
                cond: Some(cond?),
                block: Block::from_block_expr(&w.loop_body()?)?,
                else_block: None,
                label: w.label(),
                matcher: if pat.is_some() { "let" } else { "" },
                pat,
                keyword: "while",
                connector: " =",
                allow_single_line: false,
                nested_if: false,
                is_loop: true,
                span,
            })
        }
        _ => None,
    }
}

impl ControlFlow<'static> {
    fn new_if(
        cond: ast::Expr,
        pat: Option<ast::Pat>,
        block: Block,
        else_block: Option<ast::Expr>,
        allow_single_line: bool,
        nested_if: bool,
        span: Span,
    ) -> ControlFlow<'static> {
        let matcher = if pat.is_some() { "let" } else { "" };
        ControlFlow {
            cond: Some(cond),
            block,
            else_block,
            label: None,
            pat,
            keyword: "if",
            matcher,
            connector: " =",
            allow_single_line,
            nested_if,
            is_loop: false,
            span,
        }
    }
}

impl ControlFlow<'_> {
    fn rewrite_single_line(
        &self,
        pat_expr_str: &str,
        context: &RewriteContext<'_>,
        width: usize,
    ) -> Option<String> {
        debug_assert!(self.allow_single_line);
        let else_block = self.else_block.as_ref()?;
        let fixed_cost = self.keyword.len() + "  {  } else {  }".len();

        if let ast::Expr::BlockExpr(else_node) = else_block {
            let else_node = Block::from_block_expr(else_node)?;
            let (if_expr, else_expr) = match (
                FmtStmt::from_simple_block(context, &self.block, None),
                FmtStmt::from_simple_block(context, &else_node, None),
                pat_expr_str.contains('\n'),
            ) {
                (Some(if_expr), Some(else_expr), false) => (if_expr, else_expr),
                _ => return None,
            };

            let new_width = width.checked_sub(pat_expr_str.len() + fixed_cost)?;
            let if_str = if_expr.rewrite(context, Shape::legacy(new_width, Indent::empty()))?;

            let new_width = new_width.checked_sub(if_str.len())?;
            let else_str = else_expr.rewrite(context, Shape::legacy(new_width, Indent::empty()))?;

            if if_str.contains('\n') || else_str.contains('\n') {
                return None;
            }

            let result = format!(
                "{} {} {{ {} }} else {{ {} }}",
                self.keyword, pat_expr_str, if_str, else_str
            );

            if result.len() <= width {
                return Some(result);
            }
        }

        None
    }
}

/// Returns `true` if the last line of pat_str has leading whitespace and it is wider than the
/// shape's indent.
fn last_line_offsetted(start_column: usize, pat_str: &str) -> bool {
    let mut leading_whitespaces = 0;
    for c in pat_str.chars().rev() {
        match c {
            '\n' => break,
            _ if c.is_whitespace() => leading_whitespaces += 1,
            _ => leading_whitespaces = 0,
        }
    }
    leading_whitespaces > start_column
}

impl ControlFlow<'_> {
    fn rewrite_pat_expr(
        &self,
        context: &RewriteContext<'_>,
        expr: &ast::Expr,
        shape: Shape,
        offset: usize,
    ) -> Option<String> {
        let cond_shape = shape.offset_left(offset)?;
        if let Some(pat) = &self.pat {
            let matcher = if self.matcher.is_empty() {
                self.matcher.to_owned()
            } else {
                format!("{} ", self.matcher)
            };
            let pat_shape = cond_shape
                .offset_left(matcher.len())?
                .sub_width(self.connector.len())?;
            let pat_string = pat.rewrite(context, pat_shape)?;
            let comments_lo = context
                .snippet_provider
                .span_after(self.span.with_lo(pat.span().hi()), self.connector.trim());
            let comments_span = mk_sp(comments_lo, expr_span(expr).lo());
            return rewrite_assign_rhs_with_comments(
                context,
                format!("{}{}{}", matcher, pat_string, self.connector),
                expr,
                cond_shape,
                &RhsAssignKind::Expr,
                RhsTactics::Default,
                comments_span,
                true,
            );
        }

        let expr_rw = expr.rewrite(context, cond_shape);
        // The expression may (partially) fit on the current line.
        // We do not allow splitting between `if` and condition.
        if self.keyword == "if" || expr_rw.is_some() {
            return expr_rw;
        }

        // The expression won't fit on the current line, jump to next.
        let nested_shape = shape
            .block_indent(context.config.tab_spaces())
            .with_max_width(context.config);
        let nested_indent_str = nested_shape.indent.to_string_with_newline(context.config);
        expr.rewrite(context, nested_shape)
            .map(|expr_rw| format!("{nested_indent_str}{expr_rw}"))
    }

    fn rewrite_cond(
        &self,
        context: &RewriteContext<'_>,
        shape: Shape,
        alt_block_sep: &str,
    ) -> Option<(String, usize)> {
        // Do not take the rhs overhead from the upper expressions into account
        // when rewriting pattern.
        let new_width = context.budget(shape.used_width());
        let fresh_shape = Shape {
            width: new_width,
            ..shape
        };
        let constr_shape = if self.nested_if {
            // We are part of an if-elseif-else chain. Our constraints are tightened.
            // 7 = "} else " .len()
            fresh_shape.offset_left(7)?
        } else {
            fresh_shape
        };

        let label_string = rewrite_label(context, self.label.as_ref());
        // 1 = space after keyword.
        let offset = self.keyword.len() + label_string.len() + 1;

        let pat_expr_string = match &self.cond {
            Some(cond) => self.rewrite_pat_expr(context, cond, constr_shape, offset)?,
            None => String::new(),
        };

        // `control_brace_style` is an unstable option fixed at `AlwaysSameLine`.
        // 2 = ` {`
        let brace_overhead = 2;
        let one_line_budget = context
            .config
            .max_width()
            .saturating_sub(constr_shape.used_width() + offset + brace_overhead);
        let force_newline_brace = (pat_expr_string.contains('\n')
            || pat_expr_string.len() > one_line_budget)
            && (!last_line_extendable(&pat_expr_string)
                || last_line_offsetted(shape.used_width(), &pat_expr_string));

        // Try to format if-else on single line.
        if self.allow_single_line
            && context.config.single_line_if_else_max_width() > 0
            && let Some(cond_str) = self.rewrite_single_line(&pat_expr_string, context, shape.width)
            && cond_str.len() <= context.config.single_line_if_else_max_width()
        {
            return Some((cond_str, 0));
        }

        let cond_span = if let Some(cond) = &self.cond {
            expr_span(cond)
        } else {
            mk_sp(self.block.span.lo(), self.block.span.lo())
        };

        // `for event in event`
        // Do not include label in the span.
        let lo = self
            .label
            .as_ref()
            .map_or(self.span.lo(), |label| label.span().hi());
        let between_kwd_cond = mk_sp(
            context
                .snippet_provider
                .span_after(mk_sp(lo, self.span.hi()), self.keyword.trim()),
            if self.pat.is_none() {
                cond_span.lo()
            } else if self.matcher.is_empty() {
                self.pat.as_ref()?.span().lo()
            } else {
                context
                    .snippet_provider
                    .span_before(self.span, self.matcher.trim())
            },
        );

        let between_kwd_cond_comment = extract_comment(between_kwd_cond, context, shape);

        let after_cond_comment =
            extract_comment(mk_sp(cond_span.hi(), self.block.span.lo()), context, shape);

        let block_sep = if self.cond.is_none() && between_kwd_cond_comment.is_some() {
            ""
        } else if force_newline_brace {
            alt_block_sep
        } else {
            " "
        };

        let used_width = if pat_expr_string.contains('\n') {
            last_line_width(&pat_expr_string)
        } else {
            // 2 = spaces after keyword and condition.
            label_string.len() + self.keyword.len() + pat_expr_string.len() + 2
        };

        Some((
            format!(
                "{}{}{}{}{}",
                label_string,
                self.keyword,
                between_kwd_cond_comment.as_deref().unwrap_or(
                    if pat_expr_string.is_empty() || pat_expr_string.starts_with('\n') {
                        ""
                    } else {
                        " "
                    }
                ),
                pat_expr_string,
                after_cond_comment.as_deref().unwrap_or(block_sep)
            ),
            used_width,
        ))
    }
}

/// Rewrite the `else` keyword with surrounding comments.
///
/// force_newline_else: whether or not to rewrite the `else` keyword on a newline.
/// is_last: true if this is an `else` and `false` if this is an `else if` block.
/// span: Span between the end of the last expression and the start of the else block,
///       which contains the `else` keyword
pub(crate) fn rewrite_else_kw_with_comments(
    force_newline_else: bool,
    _is_last: bool,
    context: &RewriteContext<'_>,
    span: Span,
    shape: Shape,
) -> String {
    let else_kw_lo = context.snippet_provider.span_before(span, "else");
    let before_else_kw = mk_sp(span.lo(), else_kw_lo);
    let before_else_kw_comment = extract_comment(before_else_kw, context, shape);

    let else_kw_hi = context.snippet_provider.span_after(span, "else");
    let after_else_kw = mk_sp(else_kw_hi, span.hi());
    let after_else_kw_comment = extract_comment(after_else_kw, context, shape);

    let newline_sep = &shape.indent.to_string_with_newline(context.config);
    // `control_brace_style` is fixed at `AlwaysSameLine`.
    let before_sep = if force_newline_else {
        newline_sep.as_ref()
    } else {
        " "
    };
    let after_sep = " ";

    format!(
        "{}else{}",
        before_else_kw_comment.as_deref().unwrap_or(before_sep),
        after_else_kw_comment.as_deref().unwrap_or(after_sep),
    )
}

impl Rewrite for ControlFlow<'_> {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let alt_block_sep = &shape.indent.to_string_with_newline(context.config);
        let (cond_str, used_width) = self.rewrite_cond(context, shape, alt_block_sep)?;
        // If `used_width` is 0, it indicates that whole control flow is written in a single line.
        if used_width == 0 {
            return Some(cond_str);
        }

        let block_width = shape.width.saturating_sub(used_width);
        // This is used only for the empty block case: `{}`. So, we use 1 if we know
        // we should avoid the single line case.
        let block_width = if self.else_block.is_some() || self.nested_if {
            min(1, block_width)
        } else {
            block_width
        };
        let block_shape = Shape {
            width: block_width,
            ..shape
        };
        let block_str = {
            let old_val = context.is_if_else_block.replace(self.else_block.is_some());
            let old_is_loop = context.is_loop_block.replace(self.is_loop);
            let result =
                rewrite_block_with_visitor(context, "", &self.block, None, None, block_shape, true);
            context.is_loop_block.replace(old_is_loop);
            context.is_if_else_block.replace(old_val);
            result?
        };

        let mut result = format!("{cond_str}{block_str}");

        if let Some(else_block) = &self.else_block {
            let shape = Shape::indented(shape.indent, context.config);
            let mut last_in_chain = false;
            let rewrite = match else_block {
                // If the else expression is another if-else expression, prevent it
                // from being formatted on a single line.
                // Note how we're passing the original shape, as the
                // cost of "else" should not cascade.
                ast::Expr::IfExpr(i) => {
                    let (pats, cond) = extract_pats_and_cond(i.condition()?);
                    ControlFlow::new_if(
                        cond?,
                        pats,
                        Block::from_block_expr(&i.then_branch()?)?,
                        i.else_branch().map(else_branch_expr),
                        false,
                        true,
                        mk_sp(expr_span(else_block).lo(), self.span.hi()),
                    )
                    .rewrite(context, shape)
                }
                _ => {
                    last_in_chain = true;
                    // When rewriting a block, the width is only used for single line
                    // blocks, passing 1 lets us avoid that.
                    let else_shape = Shape {
                        width: min(1, shape.width),
                        ..shape
                    };
                    format_expr(else_block, ExprType::Statement, context, else_shape)
                }
            };

            let else_kw = rewrite_else_kw_with_comments(
                false,
                last_in_chain,
                context,
                mk_sp(self.block.span.hi(), else_block.span().lo()),
                shape,
            );
            result.push_str(&else_kw);
            result.push_str(&rewrite?);
        }

        Some(result)
    }
}

pub(crate) fn rewrite_label(
    context: &RewriteContext<'_>,
    opt_label: Option<&ast::Label>,
) -> Cow<'static, str> {
    match opt_label.and_then(|l| l.lifetime()) {
        Some(lt) => Cow::from(format!("{}: ", context.snippet(lt.span()))),
        None => Cow::from(""),
    }
}

fn extract_comment(span: Span, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
    match rewrite_missing_comment(span, shape, context) {
        Some(ref comment) if !comment.is_empty() => Some(format!(
            "{indent}{comment}{indent}",
            indent = shape.indent.to_string_with_newline(context.config)
        )),
        _ => None,
    }
}

pub(crate) fn block_contains_comment(context: &RewriteContext<'_>, block: &Block) -> bool {
    contains_comment(context.snippet(block.span))
}

/// Checks that a block contains no statements, an expression and no comments or
/// attributes.
pub(crate) fn is_simple_block(
    context: &RewriteContext<'_>,
    block: &Block,
    attrs: Option<&[Attribute]>,
) -> bool {
    block.stmts.len() == 1
        && stmt_is_expr(&block.stmts[0])
        && !block_contains_comment(context, block)
        && attrs.is_none_or(|a| a.is_empty())
}

/// Checks whether a block contains at most one statement or expression, and no
/// comments or attributes.
pub(crate) fn is_simple_block_stmt(
    context: &RewriteContext<'_>,
    block: &Block,
    attrs: Option<&[Attribute]>,
) -> bool {
    block.stmts.len() <= 1
        && !block_contains_comment(context, block)
        && attrs.is_none_or(|a| a.is_empty())
}

fn block_has_statements(block: &Block) -> bool {
    block
        .stmts
        .iter()
        .any(|stmt| !matches!(stmt.kind, StmtKind::Empty))
}

/// Checks whether a block contains no statements, expressions, comments, or
/// inner attributes.
pub(crate) fn is_empty_block(
    context: &RewriteContext<'_>,
    block: &Block,
    attrs: Option<&[Attribute]>,
) -> bool {
    !block_has_statements(block)
        && !block_contains_comment(context, block)
        && !has_inner_attrs(attrs)
}

pub(crate) fn stmt_is_expr(stmt: &Stmt) -> bool {
    matches!(stmt.kind, StmtKind::Expr(..))
}

pub(crate) fn rewrite_literal(
    context: &RewriteContext<'_>,
    kind: LitKind,
    span: Span,
    shape: Shape,
) -> Option<String> {
    debug_assert!(!context.config.format_strings());
    match kind {
        LitKind::Str => rewrite_string_lit(context, span, shape),
        // `hex_literal_case` and `float_literal_trailing_zero` are unstable options fixed at
        // `Preserve`, so numbers are kept as written.
        _ => wrap_str(
            context.snippet(span).to_owned(),
            context.config.max_width(),
            shape,
        ),
    }
}

fn rewrite_string_lit(context: &RewriteContext<'_>, span: Span, shape: Shape) -> Option<String> {
    let string_lit = context.snippet(span);
    let line_count = string_lit.lines().count();
    if string_lit
        .lines()
        .take(line_count.saturating_sub(1))
        .all(|line| line.ends_with('\\'))
        && context.config.style_edition() >= StyleEdition::Edition2024
    {
        Some(string_lit.to_owned())
    } else {
        wrap_str(string_lit.to_owned(), context.config.max_width(), shape)
    }
}

pub(crate) fn choose_separator_tactic(
    context: &RewriteContext<'_>,
    span: Span,
) -> Option<SeparatorTactic> {
    if context.inside_macro() {
        if span_ends_with_comma(context, span) {
            Some(SeparatorTactic::Always)
        } else {
            Some(SeparatorTactic::Never)
        }
    } else {
        None
    }
}

pub(crate) fn rewrite_call(
    context: &RewriteContext<'_>,
    callee: &str,
    args: Vec<OverflowableItem>,
    span: Span,
    shape: Shape,
) -> Option<String> {
    overflow::rewrite_with_parens(
        context,
        callee,
        args,
        shape,
        span,
        context.config.fn_call_width(),
        choose_separator_tactic(context, span),
    )
}

pub(crate) fn is_simple_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Literal(..) => true,
        ast::Expr::PathExpr(p) => p.path().is_some_and(|p| {
            p.qualifier().is_none() && p.segment().is_some_and(|s| s.type_anchor().is_none())
        }),
        ast::Expr::RefExpr(r) => r.expr().is_some_and(|e| is_simple_expr(&e)),
        ast::Expr::CastExpr(c) => c.expr().is_some_and(|e| is_simple_expr(&e)),
        ast::Expr::FieldExpr(f) => f.expr().is_some_and(|e| is_simple_expr(&e)),
        ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| is_simple_expr(&e)),
        ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| is_simple_expr(&e)),
        ast::Expr::IndexExpr(i) => {
            i.base().is_some_and(|e| is_simple_expr(&e))
                && i.index().is_some_and(|e| is_simple_expr(&e))
        }
        ast::Expr::ArrayExpr(a) if a.semicolon_token().is_some() => {
            let mut exprs = a.exprs();
            exprs.next().is_some_and(|e| is_simple_expr(&e))
                && exprs.next().is_some_and(|e| is_simple_expr(&e))
        }
        _ => false,
    }
}

pub(crate) fn can_be_overflowed_expr(
    context: &RewriteContext<'_>,
    expr: &ast::Expr,
    args_len: usize,
) -> bool {
    if !outer_attributes(expr.syntax()).is_empty() {
        return false;
    }
    match expr {
        ast::Expr::MatchExpr(..) => {
            (context.use_block_indent() && args_len == 1)
                || context.config.overflow_delimited_expr()
        }
        ast::Expr::IfExpr(..)
        | ast::Expr::ForExpr(..)
        | ast::Expr::LoopExpr(..)
        | ast::Expr::WhileExpr(..) => {
            context.config.combine_control_expr() && context.use_block_indent() && args_len == 1
        }

        // Handle always block-like expressions
        ast::Expr::BlockExpr(b) => matches!(
            block_expr_kind(b),
            BlockExprKind::Block | BlockExprKind::Gen
        ),
        ast::Expr::ClosureExpr(..) => true,

        // Handle `[]` and `{}`-like expressions
        ast::Expr::ArrayExpr(..) | ast::Expr::RecordExpr(..) => {
            context.config.overflow_delimited_expr()
                || (context.use_block_indent() && args_len == 1)
        }
        ast::Expr::MacroExpr(m) => {
            let delim = m.macro_call().and_then(|c| c.token_tree()).map(|tt| {
                if tt.l_brack_token().is_some() {
                    Delimiter::Bracket
                } else if tt.l_curly_token().is_some() {
                    Delimiter::Brace
                } else {
                    Delimiter::Parenthesis
                }
            });
            match (delim, context.config.overflow_delimited_expr()) {
                (Some(Delimiter::Bracket), true) | (Some(Delimiter::Brace), true) => true,
                _ => context.use_block_indent() && args_len == 1,
            }
        }

        // Handle parenthetical expressions
        ast::Expr::CallExpr(..) | ast::Expr::MethodCallExpr(..) | ast::Expr::TupleExpr(..) => {
            context.use_block_indent() && args_len == 1
        }

        // Handle unary-like expressions
        ast::Expr::RefExpr(r) => r
            .expr()
            .is_some_and(|e| can_be_overflowed_expr(context, &e, args_len)),
        ast::Expr::TryExpr(t) => t
            .expr()
            .is_some_and(|e| can_be_overflowed_expr(context, &e, args_len)),
        ast::Expr::PrefixExpr(p) => p
            .expr()
            .is_some_and(|e| can_be_overflowed_expr(context, &e, args_len)),
        ast::Expr::CastExpr(c) => c
            .expr()
            .is_some_and(|e| can_be_overflowed_expr(context, &e, args_len)),
        _ => false,
    }
}

pub(crate) fn is_nested_call(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::CallExpr(..) | ast::Expr::MacroExpr(..) => true,
        ast::Expr::RefExpr(r) => r.expr().is_some_and(|e| is_nested_call(&e)),
        ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| is_nested_call(&e)),
        ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| is_nested_call(&e)),
        ast::Expr::CastExpr(c) => c.expr().is_some_and(|e| is_nested_call(&e)),
        _ => false,
    }
}

/// Returns `true` if a function call or a method call represented by the given span ends with a
/// trailing comma. This function is used when rewriting macro, as adding or removing a trailing
/// comma from macro can potentially break the code.
pub(crate) fn span_ends_with_comma(context: &RewriteContext<'_>, span: Span) -> bool {
    let mut result = false;
    let mut prev_char: char = Default::default();
    let closing_delimiters = &[')', '}', ']'];

    for (kind, _, c) in CharClasses::new(context.snippet(span)) {
        match c {
            _ if kind.is_comment() || c.is_whitespace() => continue,
            c if closing_delimiters.contains(&c) => {
                result &= !closing_delimiters.contains(&prev_char);
            }
            ',' => result = true,
            _ => result = false,
        }
        prev_char = c;
    }

    result
}

pub(crate) fn rewrite_paren(
    context: &RewriteContext<'_>,
    paren: &ast::ParenExpr,
    shape: Shape,
    span: Span,
) -> Option<String> {
    let mut subexpr = paren.expr()?;
    let mut span = span;
    // Extract comments within parens.
    let mut pre_span;
    let mut post_span;
    let mut pre_comment;
    let mut post_comment;
    let remove_nested_parens = context.config.remove_nested_parens();
    loop {
        // 1 = "(" or ")"
        pre_span = mk_sp(span.lo() + 1, subexpr.span().lo());
        post_span = mk_sp(expr_span(&subexpr).hi(), span.hi() - 1);
        pre_comment = rewrite_missing_comment(pre_span, shape, context)?;
        post_comment = rewrite_missing_comment(post_span, shape, context)?;

        // Remove nested parens if there are no comments.
        if let ast::Expr::ParenExpr(ref subsub) = subexpr
            && remove_nested_parens
            && pre_comment.is_empty()
            && post_comment.is_empty()
            && let Some(inner) = subsub.expr()
        {
            span = expr_span(&subexpr);
            subexpr = inner;
            continue;
        }

        break;
    }

    // 1 = `(` and `)`
    let sub_shape = shape.offset_left(1)?.sub_width(1)?;
    let subexpr_str = subexpr.rewrite(context, sub_shape)?;
    let fits_single_line = !pre_comment.contains("//") && !post_comment.contains("//");
    if fits_single_line {
        Some(format!("({pre_comment}{subexpr_str}{post_comment})"))
    } else {
        rewrite_paren_in_multi_line(context, &subexpr, shape, pre_span, post_span)
    }
}

fn rewrite_paren_in_multi_line(
    context: &RewriteContext<'_>,
    subexpr: &ast::Expr,
    shape: Shape,
    pre_span: Span,
    post_span: Span,
) -> Option<String> {
    let nested_indent = shape.indent.block_indent(context.config);
    let nested_shape = Shape::indented(nested_indent, context.config);
    let pre_comment = rewrite_missing_comment(pre_span, nested_shape, context)?;
    let post_comment = rewrite_missing_comment(post_span, nested_shape, context)?;
    let subexpr_str = subexpr.rewrite(context, nested_shape)?;

    let mut result = String::with_capacity(subexpr_str.len() * 2);
    result.push('(');
    if !pre_comment.is_empty() {
        result.push_str(&nested_indent.to_string_with_newline(context.config));
        result.push_str(&pre_comment);
    }
    result.push_str(&nested_indent.to_string_with_newline(context.config));
    result.push_str(&subexpr_str);
    if !post_comment.is_empty() {
        result.push_str(&nested_indent.to_string_with_newline(context.config));
        result.push_str(&post_comment);
    }
    result.push_str(&shape.indent.to_string_with_newline(context.config));
    result.push(')');

    Some(result)
}

fn rewrite_index(
    expr: &ast::Expr,
    index: &ast::Expr,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let expr_str = expr.rewrite(context, shape)?;

    let offset = last_line_width(&expr_str) + 1;
    let rhs_overhead = shape.rhs_overhead(context.config);
    let index_shape = if expr_str.contains('\n') {
        Shape::legacy(context.config.max_width(), shape.indent)
            .offset_left(offset)
            .and_then(|shape| shape.sub_width(1 + rhs_overhead))
    } else {
        shape
            .offset_left(offset)
            .and_then(|shape| shape.sub_width(1))
    };
    let orig_index_rw = index_shape.and_then(|s| index.rewrite(context, s));

    // Return if index fits in a single line.
    match orig_index_rw {
        Some(ref index_str) if !index_str.contains('\n') => {
            return Some(format!("{expr_str}[{index_str}]"));
        }
        _ => (),
    }

    // Try putting index on the next line and see if it fits in a single line.
    let indent = shape.indent.block_indent(context.config);
    let index_shape = Shape::indented(indent, context.config)
        .offset_left(1)?
        .sub_width(1 + rhs_overhead)?;
    let new_index_rw = index.rewrite(context, index_shape);
    match (orig_index_rw, new_index_rw) {
        (_, Some(ref new_index_str)) if !new_index_str.contains('\n') => Some(format!(
            "{}{}[{}]",
            expr_str,
            indent.to_string_with_newline(context.config),
            new_index_str,
        )),
        (None, Some(ref new_index_str)) => Some(format!(
            "{}{}[{}]",
            expr_str,
            indent.to_string_with_newline(context.config),
            new_index_str,
        )),
        (Some(ref index_str), _) => Some(format!("{expr_str}[{index_str}]")),
        (None, None) => None,
    }
}

enum StructLitField {
    Regular(ast::RecordExprField),
    Base(ast::Expr, Span),
    Rest(Span),
}

fn rewrite_struct_lit(
    context: &RewriteContext<'_>,
    record: &ast::RecordExpr,
    attrs: &[Attribute],
    span: Span,
    shape: Shape,
) -> Option<String> {
    // 2 = " {".len()
    let path_shape = shape.sub_width(2)?;
    let path_str = rewrite_path(context, PathContext::Expr, &record.path()?, path_shape)?;

    let field_list = record.record_expr_field_list()?;
    let fields: Vec<ast::RecordExprField> = field_list.fields().collect();
    let dotdot = field_list.dotdot_token();
    let base = field_list.spread();

    let has_base_or_rest = match (&dotdot, &base) {
        (None, _) if fields.is_empty() => return Some(format!("{path_str} {{}}")),
        (Some(_), None) if fields.is_empty() => return Some(format!("{path_str} {{ .. }}")),
        (Some(_), _) => true,
        _ => false,
    };

    // Foo { a: Foo } - indent is +3, width is -5.
    let (h_shape, v_shape) = struct_lit_shape(shape, context, path_str.len() + 3, 2)?;

    let one_line_width = h_shape.map_or(0, |shape| shape.width);
    let body_lo = context.snippet_provider.span_after(span, "{");
    debug_assert_eq!(context.config.struct_field_align_threshold(), 0);
    let fields_str = {
        let mut field_iter: Vec<StructLitField> = fields
            .iter()
            .cloned()
            .map(StructLitField::Regular)
            .collect();
        match (&dotdot, base) {
            (Some(dd), Some(expr)) => field_iter.push(StructLitField::Base(expr, token_span(dd))),
            (Some(dd), None) => field_iter.push(StructLitField::Rest(token_span(dd))),
            _ => {}
        }

        let span_lo = |item: &StructLitField| match item {
            StructLitField::Regular(field) => field.span().lo(),
            StructLitField::Base(_, dotdot) => dotdot.lo(),
            StructLitField::Rest(span) => span.lo(),
        };
        let span_hi = |item: &StructLitField| match item {
            StructLitField::Regular(field) => field.span().hi(),
            StructLitField::Base(expr, _) => expr_span(expr).hi(),
            StructLitField::Rest(span) => span.hi(),
        };
        let rewrite = |item: &StructLitField| match item {
            StructLitField::Regular(field) => {
                // The 1 taken from the v_budget is for the comma.
                rewrite_field(context, field, v_shape.sub_width(1)?, 0)
            }
            StructLitField::Base(expr, _) => {
                // 2 = ..
                expr.rewrite(context, v_shape.offset_left(2)?)
                    .map(|s| format!("..{s}"))
            }
            StructLitField::Rest(_) => Some("..".to_owned()),
        };

        let items = itemize_list(
            context.snippet_provider,
            field_iter.iter(),
            "}",
            ",",
            |item| span_lo(item),
            |item| span_hi(item),
            |item| rewrite(item),
            body_lo,
            span.hi(),
            false,
        );
        let item_vec = items.collect::<Vec<_>>();

        let tactic = struct_lit_tactic(h_shape, context, &item_vec);
        let nested_shape = shape_for_tactic(tactic, h_shape, v_shape);

        let ends_with_comma = span_ends_with_comma(context, span);
        let force_no_trailing_comma = context.inside_macro() && !ends_with_comma;

        let fmt = struct_lit_formatting(
            nested_shape,
            tactic,
            context,
            force_no_trailing_comma || has_base_or_rest || !context.use_block_indent(),
        );

        write_list(&item_vec, &fmt)?
    };

    let fields_str =
        wrap_struct_field(context, attrs, &fields_str, shape, v_shape, one_line_width)?;
    Some(format!("{path_str} {{{fields_str}}}"))
}

pub(crate) fn wrap_struct_field(
    context: &RewriteContext<'_>,
    attrs: &[Attribute],
    fields_str: &str,
    shape: Shape,
    nested_shape: Shape,
    one_line_width: usize,
) -> Option<String> {
    let should_vertical = fields_str.contains('\n')
        || !context.config.struct_lit_single_line()
        || fields_str.len() > one_line_width;

    let inner_attrs: Vec<Attribute> = attrs
        .iter()
        .filter(|a| a.style() == super::nodes::AttrStyle::Inner)
        .cloned()
        .collect();
    if inner_attrs.is_empty() {
        if should_vertical {
            Some(format!(
                "{}{}{}",
                nested_shape.indent.to_string_with_newline(context.config),
                fields_str,
                shape.indent.to_string_with_newline(context.config)
            ))
        } else {
            // One liner or visual indent.
            Some(format!(" {fields_str} "))
        }
    } else {
        Some(format!(
            "{}{}{}{}{}",
            nested_shape.indent.to_string_with_newline(context.config),
            rewrite_attrs(&inner_attrs, context, shape)?,
            nested_shape.indent.to_string_with_newline(context.config),
            fields_str,
            shape.indent.to_string_with_newline(context.config)
        ))
    }
}

pub(crate) fn rewrite_field(
    context: &RewriteContext<'_>,
    field: &ast::RecordExprField,
    shape: Shape,
    prefix_max_width: usize,
) -> Option<String> {
    let attrs = outer_attributes(field.syntax());
    if contains_skip(&attrs) {
        return Some(context.snippet(field.span()).to_owned());
    }
    let mut attrs_str = rewrite_attrs(&attrs, context, shape)?;
    if !attrs_str.is_empty() {
        attrs_str.push_str(&shape.indent.to_string_with_newline(context.config));
    };
    let expr = field.expr()?;
    match field.name_ref() {
        None => {
            // Shorthand: `Foo { a }`.
            Some(attrs_str + context.snippet(expr_span(&expr)))
        }
        Some(name_ref) => {
            let name = context.snippet(name_ref.span());
            let mut separator = String::from(colon_spaces(context.config));
            for _ in 0..prefix_max_width.saturating_sub(name.len()) {
                separator.push(' ');
            }
            let overhead = name.len() + separator.len();
            let expr_shape = shape.offset_left(overhead)?;
            let expr_rw = expr.rewrite(context, expr_shape);
            let is_lit = matches!(expr, ast::Expr::Literal(_));
            match expr_rw {
                Some(ref e)
                    if !is_lit
                        && e.as_str() == name
                        && context.config.use_field_init_shorthand() =>
                {
                    Some(attrs_str + name)
                }
                Some(e) => Some(format!("{attrs_str}{name}{separator}{e}")),
                None => {
                    let expr_offset = shape.indent.block_indent(context.config);
                    let expr = expr.rewrite(context, Shape::indented(expr_offset, context.config));
                    expr.map(|s| {
                        format!(
                            "{}{}:\n{}{}",
                            attrs_str,
                            name,
                            expr_offset.to_string(context.config),
                            s
                        )
                    })
                }
            }
        }
    }
}

fn rewrite_let(
    context: &RewriteContext<'_>,
    shape: Shape,
    pat: &ast::Pat,
    expr: &ast::Expr,
) -> Option<String> {
    let mut result = "let ".to_owned();

    // TODO(ytmimi) comments could appear between `let` and the `pat`

    // 4 = "let ".len()
    let pat_shape = shape.offset_left(4)?;
    let pat_str = pat.rewrite(context, pat_shape)?;
    result.push_str(&pat_str);

    // TODO(ytmimi) comments could appear between `pat` and `=`
    result.push_str(" =");

    let comments_lo = context
        .snippet_provider
        .span_after(expr_span(expr).with_lo(pat.span().hi()), "=");
    let comments_span = mk_sp(comments_lo, expr_span(expr).lo());
    rewrite_assign_rhs_with_comments(
        context,
        result,
        expr,
        shape,
        &RhsAssignKind::Expr,
        RhsTactics::Default,
        comments_span,
        true,
    )
}

pub(crate) fn rewrite_tuple(
    context: &RewriteContext<'_>,
    items: Vec<OverflowableItem>,
    span: Span,
    shape: Shape,
    is_singleton_tuple: bool,
) -> Option<String> {
    // We use the same rule as function calls for rewriting tuples.
    let force_tactic = if context.inside_macro() {
        if span_ends_with_comma(context, span) {
            Some(SeparatorTactic::Always)
        } else {
            Some(SeparatorTactic::Never)
        }
    } else if is_singleton_tuple {
        Some(SeparatorTactic::Always)
    } else {
        None
    };
    overflow::rewrite_with_parens(
        context,
        "",
        items,
        shape,
        span,
        context.config.fn_call_width(),
        force_tactic,
    )
}

pub(crate) fn rewrite_unary_prefix<R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    prefix: &str,
    rewrite: &R,
    shape: Shape,
) -> Option<String> {
    let shape = shape.offset_left(prefix.len())?;
    rewrite
        .rewrite(context, shape)
        .map(|r| format!("{prefix}{r}"))
}

/// FIXME: this is probably not correct for multi-line Rewrites. we should
/// subtract suffix.len() from the last line budget, not the first!
pub(crate) fn rewrite_unary_suffix<R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    suffix: &str,
    rewrite: &R,
    shape: Shape,
) -> Option<String> {
    let shape = shape.sub_width(suffix.len())?;
    rewrite.rewrite(context, shape).map(|mut r| {
        r.push_str(suffix);
        r
    })
}

/// What is on the right-hand side of an assignment-like construct.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum RhsAssignKind {
    Expr,
    Bounds,
    Ty,
}

fn rewrite_assignment(
    context: &RewriteContext<'_>,
    bin: &ast::BinExpr,
    shape: Shape,
) -> Option<String> {
    let lhs = bin.lhs()?;
    let rhs = bin.rhs()?;
    let operator_str = bin_op_text(bin)?;

    // 1 = space between lhs and operator.
    let lhs_shape = shape.sub_width(operator_str.len() + 1)?;
    let lhs_str = format!("{} {}", lhs.rewrite(context, lhs_shape)?, operator_str);

    rewrite_assign_rhs(context, lhs_str, &rhs, &RhsAssignKind::Expr, shape)
}

/// Controls where to put the rhs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum RhsTactics {
    /// Use heuristics.
    Default,
    /// Put the rhs on the next line if it uses multiple line, without extra indentation.
    ForceNextLineWithoutIndent,
    /// Allow overflowing max width if neither `Default` nor `ForceNextLineWithoutIndent`
    /// did not work.
    AllowOverflow,
}

/// The left hand side must contain everything up to, and including, the
/// assignment operator.
pub(crate) fn rewrite_assign_rhs<S: Into<String>, R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    lhs: S,
    ex: &R,
    rhs_kind: &RhsAssignKind,
    shape: Shape,
) -> Option<String> {
    rewrite_assign_rhs_with(context, lhs, ex, shape, rhs_kind, RhsTactics::Default)
}

pub(crate) fn rewrite_assign_rhs_expr<R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    lhs: &str,
    ex: &R,
    shape: Shape,
    rhs_kind: &RhsAssignKind,
    rhs_tactics: RhsTactics,
) -> Option<String> {
    let last_line_width = last_line_width(lhs).saturating_sub(if lhs.contains('\n') {
        shape.indent.width()
    } else {
        0
    });
    // 1 = space between operator and rhs.
    let orig_shape = shape.offset_left(last_line_width + 1).unwrap_or(Shape {
        width: 0,
        offset: shape.offset + last_line_width + 1,
        ..shape
    });
    let has_rhs_comment = if let Some(offset) = lhs.find_last_uncommented("=") {
        lhs.trim_end().len() > offset + 1
    } else {
        false
    };

    choose_rhs(
        context,
        ex,
        orig_shape,
        ex.rewrite(context, orig_shape),
        rhs_kind,
        rhs_tactics,
        has_rhs_comment,
    )
}

pub(crate) fn rewrite_assign_rhs_with<S: Into<String>, R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    lhs: S,
    ex: &R,
    shape: Shape,
    rhs_kind: &RhsAssignKind,
    rhs_tactics: RhsTactics,
) -> Option<String> {
    let lhs = lhs.into();
    let rhs = rewrite_assign_rhs_expr(context, &lhs, ex, shape, rhs_kind, rhs_tactics)?;
    Some(lhs + &rhs)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn rewrite_assign_rhs_with_comments<S: Into<String>, R: Rewrite + ?Sized + Spanned>(
    context: &RewriteContext<'_>,
    lhs: S,
    ex: &R,
    shape: Shape,
    rhs_kind: &RhsAssignKind,
    rhs_tactics: RhsTactics,
    between_span: Span,
    allow_extend: bool,
) -> Option<String> {
    let lhs = lhs.into();
    let contains_comment = contains_comment(context.snippet(between_span));
    let shape = if contains_comment {
        shape.block_left(context.config.tab_spaces())?
    } else {
        shape
    };
    let rhs = rewrite_assign_rhs_expr(context, &lhs, ex, shape, rhs_kind, rhs_tactics)?;
    if contains_comment {
        let rhs = rhs.trim_start();
        combine_strs_with_missing_comments(context, &lhs, rhs, between_span, shape, allow_extend)
    } else {
        Some(lhs + &rhs)
    }
}

fn choose_rhs<R: Rewrite + ?Sized>(
    context: &RewriteContext<'_>,
    expr: &R,
    shape: Shape,
    orig_rhs: Option<String>,
    _rhs_kind: &RhsAssignKind,
    rhs_tactics: RhsTactics,
    has_rhs_comment: bool,
) -> Option<String> {
    match orig_rhs {
        Some(ref new_str) if new_str.is_empty() => Some(String::new()),
        Some(ref new_str)
            if !new_str.contains('\n') && unicode_str_width(new_str) <= shape.width =>
        {
            Some(format!(" {new_str}"))
        }
        _ => {
            // Expression did not fit on the same line as the identifier.
            // Try splitting the line and see if that works better.
            let new_shape = shape_from_rhs_tactic(context, shape, rhs_tactics)?;
            let new_rhs = expr.rewrite(context, new_shape);
            let new_indent_str = &shape
                .indent
                .block_indent(context.config)
                .to_string_with_newline(context.config);
            let before_space_str = if has_rhs_comment { "" } else { " " };

            match (orig_rhs, new_rhs) {
                (Some(ref orig_rhs), Some(ref new_rhs))
                    if !filtered_str_fits(new_rhs, context.config.max_width(), new_shape) =>
                {
                    Some(format!("{before_space_str}{orig_rhs}"))
                }
                (Some(ref orig_rhs), Some(ref new_rhs))
                    if prefer_next_line(orig_rhs, new_rhs, rhs_tactics) =>
                {
                    Some(format!("{new_indent_str}{new_rhs}"))
                }
                (None, Some(ref new_rhs)) => Some(format!("{new_indent_str}{new_rhs}")),
                (None, None) if rhs_tactics == RhsTactics::AllowOverflow => {
                    let shape = shape.infinite_width();
                    expr.rewrite(context, shape)
                        .map(|s| format!("{before_space_str}{s}"))
                }
                (None, None) => None,
                (Some(orig_rhs), _) => Some(format!("{before_space_str}{orig_rhs}")),
            }
        }
    }
}

fn shape_from_rhs_tactic(
    context: &RewriteContext<'_>,
    shape: Shape,
    rhs_tactic: RhsTactics,
) -> Option<Shape> {
    match rhs_tactic {
        RhsTactics::ForceNextLineWithoutIndent => shape
            .with_max_width(context.config)
            .sub_width(shape.indent.width()),
        RhsTactics::Default | RhsTactics::AllowOverflow => {
            Shape::indented(shape.indent.block_indent(context.config), context.config)
                .sub_width(shape.rhs_overhead(context.config))
        }
    }
}

/// Returns true if formatting next_line_rhs is better on a new line when compared to the
/// original's line formatting.
///
/// It is considered better if:
/// 1. the tactic is ForceNextLineWithoutIndent
/// 2. next_line_rhs doesn't have newlines
/// 3. the original line has more newlines than next_line_rhs
/// 4. the original formatting of the first line ends with `(`, `{`, or `[` and next_line_rhs
///    doesn't
pub(crate) fn prefer_next_line(
    orig_rhs: &str,
    next_line_rhs: &str,
    rhs_tactics: RhsTactics,
) -> bool {
    rhs_tactics == RhsTactics::ForceNextLineWithoutIndent
        || !next_line_rhs.contains('\n')
        || count_newlines(orig_rhs) > count_newlines(next_line_rhs) + 1
        || first_line_ends_with(orig_rhs, '(') && !first_line_ends_with(next_line_rhs, '(')
        || first_line_ends_with(orig_rhs, '{') && !first_line_ends_with(next_line_rhs, '{')
        || first_line_ends_with(orig_rhs, '[') && !first_line_ends_with(next_line_rhs, '[')
}

pub(crate) fn is_method_call(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::MethodCallExpr(..) => true,
        ast::Expr::RefExpr(r) => r.expr().is_some_and(|e| is_method_call(&e)),
        ast::Expr::CastExpr(c) => c.expr().is_some_and(|e| is_method_call(&e)),
        ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| is_method_call(&e)),
        ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| is_method_call(&e)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_line_offsetted() {
        let lines = "one\n    two";
        assert!(last_line_offsetted(2, lines));
        assert!(!last_line_offsetted(4, lines));
        assert!(!last_line_offsetted(6, lines));

        let lines = "one    two";
        assert!(!last_line_offsetted(2, lines));
        assert!(!last_line_offsetted(0, lines));

        let lines = "\ntwo";
        assert!(!last_line_offsetted(2, lines));
        assert!(!last_line_offsetted(0, lines));

        let lines = "one\n    two      three";
        assert!(last_line_offsetted(2, lines));
        let lines = "one\n two      three";
        assert!(!last_line_offsetted(2, lines));
    }
}
