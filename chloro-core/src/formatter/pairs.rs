//! Binary-operator-like pairs (rustfmt's `pairs.rs`).

use ra_ap_syntax::ast;

use super::context::{Rewrite, RewriteContext};
use super::lists::SeparatorPlace;
use super::shape::Shape;
use super::utils::{
    first_line_width, is_assignment, is_single_line, last_line_width, trimmed_last_line_width,
    wrap_str,
};

/// Sigils that decorate a binop pair.
#[derive(Clone, Copy)]
pub(crate) struct PairParts<'a> {
    prefix: &'a str,
    infix: &'a str,
    suffix: &'a str,
}

impl<'a> PairParts<'a> {
    pub(crate) const fn new(prefix: &'a str, infix: &'a str, suffix: &'a str) -> Self {
        Self {
            prefix,
            infix,
            suffix,
        }
    }

    pub(crate) fn infix(infix: &'a str) -> PairParts<'a> {
        PairParts {
            prefix: "",
            infix,
            suffix: "",
        }
    }
}

/// Flattens a tree of pairs into a list and tries to rewrite them all at once.
pub(crate) fn rewrite_all_pairs(
    expr: &ast::BinExpr,
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<String> {
    let list = flatten(expr, context, shape)?;
    if list.let_chain_count() > 0 && !list.can_rewrite_let_chain_single_line() {
        rewrite_pairs_multiline(&list, shape, context)
    } else {
        // First we try formatting on one line.
        rewrite_pairs_one_line(&list, shape, context)
            .or_else(|| rewrite_pairs_multiline(&list, shape, context))
    }
}

/// This may return a multi-line result since we allow the last expression to go
/// multiline in a 'single line' formatting.
fn rewrite_pairs_one_line(
    list: &PairList,
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<String> {
    debug_assert!(list.list.len() >= 2, "Not a pair?");

    let mut result = String::new();
    let base_shape = shape.block();

    for ((_, rewrite), s) in list.list.iter().zip(list.separators.iter()) {
        let rewrite = rewrite.as_ref()?;
        if !is_single_line(rewrite) || result.len() > shape.width {
            return None;
        }

        result.push_str(rewrite);
        result.push(' ');
        result.push_str(s);
        result.push(' ');
    }

    let prefix_len = result.len();
    let last = &list.list.last()?.0;
    let cur_shape = base_shape.offset_left(last_line_width(&result))?;
    let last_rewrite = last.rewrite(context, cur_shape)?;
    result.push_str(&last_rewrite);

    if first_line_width(&result) > shape.width {
        return None;
    }

    // Check the last expression in the list. We sometimes let this expression
    // go over multiple lines, but we check for some ugly conditions.
    if !(is_single_line(&result) || last_rewrite.starts_with('{'))
        && (last_rewrite.starts_with('(') || prefix_len > context.config.tab_spaces())
    {
        return None;
    }

    wrap_str(result, context.config.max_width(), shape)
}

fn rewrite_pairs_multiline(
    list: &PairList,
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<String> {
    let rhs_offset = shape.rhs_overhead(context.config);
    let nested_shape = shape
        .block_indent(context.config.tab_spaces())
        .with_max_width(context.config)
        .sub_width(rhs_offset)?;

    let indent_str = nested_shape.indent.to_string_with_newline(context.config);
    let mut result = String::new();

    result.push_str(list.list[0].1.as_ref()?);

    for ((e, default_rw), s) in list.list[1..].iter().zip(list.separators.iter()) {
        // The following test checks if we should keep two subexprs on the same
        // line. We do this if not doing so would create an orphan and there is
        // enough space to do so.
        let offset = if result.contains('\n') {
            0
        } else {
            shape.used_width()
        };
        if last_line_width(&result) + offset <= nested_shape.used_width() {
            // We must snuggle the next line onto the previous line to avoid an orphan.
            if let Some(line_shape) =
                shape.offset_left(s.len() + 2 + trimmed_last_line_width(&result))
                && let Some(rewrite) = e.rewrite(context, line_shape)
            {
                result.push(' ');
                result.push_str(s);
                result.push(' ');
                result.push_str(&rewrite);
                continue;
            }
        }

        match context.config.binop_separator() {
            SeparatorPlace::Back => {
                result.push(' ');
                result.push_str(s);
                result.push_str(&indent_str);
            }
            SeparatorPlace::Front => {
                result.push_str(&indent_str);
                result.push_str(s);
                result.push(' ');
            }
        }

        result.push_str(default_rw.as_ref()?);
    }
    Some(result)
}

/// Rewrites a single pair.
pub(crate) fn rewrite_pair<LHS, RHS>(
    lhs: &LHS,
    rhs: &RHS,
    pp: PairParts<'_>,
    context: &RewriteContext<'_>,
    shape: Shape,
    separator_place: SeparatorPlace,
) -> Option<String>
where
    LHS: Rewrite + ?Sized,
    RHS: Rewrite + ?Sized,
{
    let tab_spaces = context.config.tab_spaces();
    let lhs_overhead = match separator_place {
        SeparatorPlace::Back => shape.used_width() + pp.prefix.len() + pp.infix.trim_end().len(),
        SeparatorPlace::Front => shape.used_width(),
    };
    let lhs_shape = Shape {
        width: context.budget(lhs_overhead),
        ..shape
    };
    let lhs_result = lhs
        .rewrite(context, lhs_shape)
        .map(|lhs_str| format!("{}{}", pp.prefix, lhs_str))?;

    // Try to put both lhs and rhs on the same line.
    let rhs_orig_result = shape
        .offset_left(last_line_width(&lhs_result) + pp.infix.len())
        .and_then(|s| s.sub_width(pp.suffix.len()))
        .and_then(|rhs_shape| rhs.rewrite(context, rhs_shape));

    if let Some(ref rhs_result) = rhs_orig_result {
        // If the length of the lhs is equal to or shorter than the tab width or
        // the rhs looks like block expression, we put the rhs on the same
        // line with the lhs even if the rhs is multi-lined.
        let allow_same_line = lhs_result.len() <= tab_spaces
            || rhs_result
                .lines()
                .next()
                .is_some_and(|first_line| first_line.ends_with('{'));
        if !rhs_result.contains('\n') || allow_same_line {
            let one_line_width = last_line_width(&lhs_result)
                + pp.infix.len()
                + first_line_width(rhs_result)
                + pp.suffix.len();
            if one_line_width <= shape.width {
                return Some(format!(
                    "{}{}{}{}",
                    lhs_result, pp.infix, rhs_result, pp.suffix
                ));
            }
        }
    }

    // We have to use multiple lines.
    // Re-evaluate the rhs because we have more space now:
    let mut rhs_shape = {
        // Try to calculate the initial constraint on the right hand side.
        let rhs_overhead = shape.rhs_overhead(context.config);
        Shape::indented(shape.indent.block_indent(context.config), context.config)
            .sub_width(rhs_overhead)?
    };
    let infix = match separator_place {
        SeparatorPlace::Back => pp.infix.trim_end(),
        SeparatorPlace::Front => pp.infix.trim_start(),
    };
    if separator_place == SeparatorPlace::Front {
        rhs_shape = rhs_shape.offset_left(infix.len())?;
    }
    let rhs_result = rhs.rewrite(context, rhs_shape)?;
    let indent_str = rhs_shape.indent.to_string_with_newline(context.config);
    let infix_with_sep = match separator_place {
        SeparatorPlace::Back => format!("{infix}{indent_str}"),
        SeparatorPlace::Front => format!("{indent_str}{infix}"),
    };
    Some(format!(
        "{}{}{}{}",
        lhs_result, infix_with_sep, rhs_result, pp.suffix
    ))
}

struct PairList {
    list: Vec<(ast::Expr, Option<String>)>,
    separators: Vec<String>,
}

fn is_ident_or_bool_lit(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::PathExpr(p) => p.path().is_some_and(|p| {
            p.qualifier().is_none() && p.segment().is_some_and(|s| s.type_anchor().is_none())
        }),
        ast::Expr::Literal(lit) => matches!(lit.kind(), ast::LiteralKind::Bool(_)),
        ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| is_ident_or_bool_lit(&e)),
        ast::Expr::RefExpr(r) => r.expr().is_some_and(|e| is_ident_or_bool_lit(&e)),
        ast::Expr::ParenExpr(p) => p.expr().is_some_and(|e| is_ident_or_bool_lit(&e)),
        ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| is_ident_or_bool_lit(&e)),
        _ => false,
    }
}

impl PairList {
    fn let_chain_count(&self) -> usize {
        self.list
            .iter()
            .filter(|(expr, _)| matches!(expr, ast::Expr::LetExpr(..)))
            .count()
    }

    fn can_rewrite_let_chain_single_line(&self) -> bool {
        if self.list.len() != 2 {
            return false;
        }

        let first_item_is_ident_or_bool_lit = is_ident_or_bool_lit(&self.list[0].0);
        let second_item_is_let_chain = matches!(self.list[1].0, ast::Expr::LetExpr(..));

        first_item_is_ident_or_bool_lit && second_item_is_let_chain
    }
}

/// The operator of a binary expression as written, e.g. `&&` or `+=`.
pub(crate) fn bin_op_text(expr: &ast::BinExpr) -> Option<String> {
    expr.op_token().map(|t| t.text().to_string())
}

/// Turn a tree of same-operator binary expressions into a list using a depth-first,
/// in-order traversal.
fn flatten(expr: &ast::BinExpr, context: &RewriteContext<'_>, shape: Shape) -> Option<PairList> {
    if is_assignment(expr) {
        return None;
    }
    let top_op = expr.op_kind()?;

    let default_rewrite = |node: &ast::Expr, sep: usize, is_first: bool| -> Option<String> {
        if is_first {
            return node.rewrite(context, shape);
        }
        let nested_overhead = sep + 1;
        let rhs_offset = shape.rhs_overhead(context.config);
        let nested_shape = shape
            .block_indent(context.config.tab_spaces())
            .with_max_width(context.config)
            .sub_width(rhs_offset)?;
        let default_shape = match context.config.binop_separator() {
            SeparatorPlace::Back => nested_shape.sub_width(nested_overhead)?,
            SeparatorPlace::Front => nested_shape.offset_left(nested_overhead)?,
        };
        node.rewrite(context, default_shape)
    };

    let mut stack: Vec<ast::BinExpr> = vec![];
    let mut list: Vec<(ast::Expr, Option<String>)> = vec![];
    let mut separators: Vec<String> = vec![];
    let mut node = ast::Expr::BinExpr(expr.clone());
    loop {
        match &node {
            ast::Expr::BinExpr(b) if b.op_kind() == Some(top_op) && b.lhs().is_some() => {
                stack.push(b.clone());
                node = b.lhs()?;
            }
            _ => {
                let op_len = separators.last().map_or(0, |s| s.len());
                let rw = default_rewrite(&node, op_len, list.is_empty());
                list.push((node.clone(), rw));
                if let Some(pop) = stack.pop() {
                    separators.push(bin_op_text(&pop)?);
                    node = pop.rhs()?;
                } else {
                    break;
                }
            }
        }
    }

    debug_assert_eq!(list.len() - 1, separators.len());
    Some(PairList { list, separators })
}
