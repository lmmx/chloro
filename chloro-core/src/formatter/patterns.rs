//! Patterns (rustfmt's `patterns.rs`).

use ra_ap_syntax::ast::{self, AstNode, HasName};

use super::attr::rewrite_attrs;
use super::comment::combine_strs_with_missing_comments;
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{
    LitKind, can_be_overflowed_expr, rewrite_literal, rewrite_unary_prefix, wrap_struct_field,
};
use super::lists::{
    DefinitiveListTactic, ListFormatting, ListItem, ListTactic, Separator, SeparatorPlace,
    SeparatorTactic, definitive_tactic, itemize_list, shape_for_tactic, struct_lit_formatting,
    struct_lit_shape, struct_lit_tactic, write_list,
};
use super::macros::{MacroPosition, rewrite_macro};
use super::nodes::node_text;
use super::nodes::outer_attributes;
use super::overflow::{self, OverflowableItem};
use super::pairs::{PairParts, rewrite_pair};
use super::shape::Shape;
use super::span::{Span, Spanned, mk_sp};
use super::types::{PathContext, path_segments, rewrite_path};

/// Returns `true` if the given pattern is "short".
/// A short pattern is defined by the following grammar:
///
/// `[small, ntp]`:
///     - single token
///     - `&[single-line, ntp]`
///
/// `[small]`:
///     - `[small, ntp]`
///     - unary tuple constructor `([small, ntp])`
///     - `&[small]`
pub(crate) fn is_short_pattern(
    context: &RewriteContext<'_>,
    pat: &ast::Pat,
    pat_str: &str,
) -> bool {
    // We also require that the pattern is reasonably 'small' with its literal width.
    pat_str.len() <= 20 && !pat_str.contains('\n') && is_short_pattern_inner(context, pat)
}

fn is_short_pattern_inner(context: &RewriteContext<'_>, pat: &ast::Pat) -> bool {
    match pat {
        ast::Pat::RestPat(_) | ast::Pat::WildcardPat(_) | ast::Pat::LiteralPat(_) => true,
        ast::Pat::ConstBlockPat(_) => context.config.style_edition() <= StyleEdition::Edition2024,
        ast::Pat::IdentPat(p) => p.pat().is_none(),
        ast::Pat::RecordPat(..)
        | ast::Pat::PathPat(..)
        | ast::Pat::MacroPat(..)
        | ast::Pat::SlicePat(..)
        | ast::Pat::RangePat(..) => false,
        ast::Pat::TuplePat(t) => t.fields().count() <= 1,
        ast::Pat::TupleStructPat(t) => {
            t.path().is_some_and(|p| path_segments(&p).len() <= 1) && t.fields().count() <= 1
        }
        ast::Pat::BoxPat(p) => p.pat().is_some_and(|p| is_short_pattern_inner(context, &p)),
        ast::Pat::RefPat(p) => p.pat().is_some_and(|p| is_short_pattern_inner(context, &p)),
        ast::Pat::ParenPat(p) => p.pat().is_some_and(|p| is_short_pattern_inner(context, &p)),
        ast::Pat::OrPat(p) => p.pats().all(|p| is_short_pattern_inner(context, &p)),
    }
}

/// One side of a range pattern; empty when the side is omitted.
pub(crate) struct RangeOperand {
    pub(crate) operand: Option<ast::Pat>,
}

impl Rewrite for RangeOperand {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match &self.operand {
            None => Some(String::new()),
            Some(exp) => exp.rewrite(context, shape),
        }
    }
}

impl Rewrite for ast::Pat {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            ast::Pat::OrPat(or) => {
                let pats: Vec<ast::Pat> = or.pats().collect();
                let pat_strs = pats
                    .iter()
                    .map(|p| p.rewrite(context, shape))
                    .collect::<Option<Vec<_>>>()?;

                let use_mixed_layout = pats
                    .iter()
                    .zip(pat_strs.iter())
                    .all(|(pat, pat_str)| is_short_pattern(context, pat, pat_str));
                let items: Vec<_> = pat_strs.into_iter().map(ListItem::from_str).collect();
                let tactic = if use_mixed_layout {
                    DefinitiveListTactic::Mixed
                } else {
                    definitive_tactic(
                        &items,
                        ListTactic::HorizontalVertical,
                        Separator::VerticalBar,
                        shape.width,
                    )
                };
                let fmt = ListFormatting::new(shape, context.config)
                    .tactic(tactic)
                    .separator(" |")
                    .separator_place(context.config.binop_separator())
                    .ends_with_newline(false);
                write_list(&items, &fmt)
            }
            ast::Pat::BoxPat(p) => rewrite_unary_prefix(context, "box ", &p.pat()?, shape),
            ast::Pat::IdentPat(p) => rewrite_ident_pat(p, context, shape),
            ast::Pat::WildcardPat(_) => (1 <= shape.width).then(|| "_".to_owned()),
            ast::Pat::RestPat(_) => (1 <= shape.width).then(|| "..".to_owned()),
            ast::Pat::RangePat(r) => rewrite_range_pat(context, shape, r),
            ast::Pat::RefPat(r) => {
                let prefix = if r.mut_token().is_some() {
                    "&mut "
                } else {
                    "&"
                };
                rewrite_unary_prefix(context, prefix, &r.pat()?, shape)
            }
            ast::Pat::TuplePat(t) => {
                let fields: Vec<ast::Pat> = t.fields().collect();
                rewrite_tuple_pat(&fields, None, self.span(), context, shape)
            }
            ast::Pat::PathPat(p) => rewrite_path(context, PathContext::Expr, &p.path()?, shape),
            ast::Pat::TupleStructPat(t) => {
                let path_str = rewrite_path(context, PathContext::Expr, &t.path()?, shape)?;
                let fields: Vec<ast::Pat> = t.fields().collect();
                rewrite_tuple_pat(&fields, Some(path_str), self.span(), context, shape)
            }
            ast::Pat::LiteralPat(l) => {
                let lit = l.literal()?;
                let kind = LitKind::of(&lit);
                let lit_str = rewrite_literal(context, kind, lit.span(), shape)?;
                if l.minus_token().is_some() {
                    Some(format!("-{lit_str}"))
                } else {
                    Some(lit_str)
                }
            }
            ast::Pat::ConstBlockPat(c) => {
                let block = c.block_expr()?;
                block
                    .rewrite(context, shape.offset_left(6)?)
                    .map(|b| format!("const {b}"))
            }
            ast::Pat::SlicePat(s)
                if context.config.style_edition() <= StyleEdition::Edition2021 =>
            {
                let rw: Vec<String> = s
                    .pats()
                    .map(|p| {
                        p.rewrite(context, shape)
                            .unwrap_or_else(|| context.snippet(p.span()).to_owned())
                    })
                    .collect();
                Some(format!("[{}]", rw.join(", ")))
            }
            ast::Pat::SlicePat(s) => {
                let pats: Vec<_> = s.pats().map(OverflowableItem::Pat).collect();
                overflow::rewrite_with_square_brackets(
                    context,
                    "",
                    pats,
                    shape,
                    self.span(),
                    None,
                    None,
                )
            }
            ast::Pat::RecordPat(r) => rewrite_struct_pat(r, self.span(), context, shape),
            ast::Pat::MacroPat(m) => {
                rewrite_macro(&m.macro_call()?, context, shape, MacroPosition::Pat)
            }
            ast::Pat::ParenPat(p) => p
                .pat()?
                .rewrite(context, shape.offset_left(1)?.sub_width(1)?)
                .map(|inner_pat| format!("({inner_pat})")),
        }
    }
}

fn rewrite_ident_pat(
    p: &ast::IdentPat,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let span = p.span();
    let mut_prefix = if p.mut_token().is_some() && p.ref_token().is_none() {
        "mut"
    } else {
        ""
    };
    // `ref mut x`: rustc's `ByRef::Yes(Mut)`.
    let (ref_kw, mut_infix) = match (p.ref_token(), p.mut_token()) {
        (Some(_), Some(_)) => ("ref", "mut"),
        (Some(_), None) => ("ref", ""),
        _ => ("", ""),
    };
    let name = p.name()?;
    let id_str = node_text(name.syntax());
    let name_span = name.span();
    let sub_pat = match p.pat() {
        Some(sub) => {
            // 2 - `@ `.
            let width = shape.width.checked_sub(
                mut_prefix.len() + ref_kw.len() + mut_infix.len() + id_str.len() + 2,
            )?;
            let lo = context.snippet_provider.span_after(span, "@");
            combine_strs_with_missing_comments(
                context,
                "@",
                &sub.rewrite(context, Shape::legacy(width, shape.indent))?,
                mk_sp(lo, sub.span().lo()),
                shape,
                true,
            )?
        }
        None => String::new(),
    };

    // combine prefix and ref
    let (first_lo, first) = match (mut_prefix.is_empty(), ref_kw.is_empty()) {
        (false, true) => (
            context.snippet_provider.span_after(span, "mut"),
            mut_prefix.to_owned(),
        ),
        (true, false) => (
            context.snippet_provider.span_after(span, "ref"),
            ref_kw.to_owned(),
        ),
        _ => (span.lo(), String::new()),
    };

    // combine result of above and const|mut
    let (third_lo, third) = match (first.is_empty(), mut_infix.is_empty()) {
        (false, false) => {
            let lo = context.snippet_provider.span_after(span, "ref");
            let end_span = mk_sp(first_lo, span.hi());
            let hi = context.snippet_provider.span_before(end_span, mut_infix);
            (
                context.snippet_provider.span_after(end_span, mut_infix),
                combine_strs_with_missing_comments(
                    context,
                    &first,
                    mut_infix,
                    mk_sp(lo, hi),
                    shape,
                    true,
                )?,
            )
        }
        (false, true) => (first_lo, first),
        _ => (span.lo(), String::new()),
    };

    let next = if !sub_pat.is_empty() {
        let hi = context.snippet_provider.span_before(span, "@");
        combine_strs_with_missing_comments(
            context,
            &id_str,
            &sub_pat,
            mk_sp(name_span.hi(), hi),
            shape,
            true,
        )?
    } else {
        id_str
    };

    combine_strs_with_missing_comments(
        context,
        &third,
        &next,
        mk_sp(third_lo, name_span.lo()),
        shape,
        true,
    )
}

pub(crate) fn rewrite_range_pat(
    context: &RewriteContext<'_>,
    shape: Shape,
    pat: &ast::RangePat,
) -> Option<String> {
    let op = pat
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| matches!(t.text(), ".." | "..=" | "..."))?;
    let infix = op.text().to_owned();
    // `RangeItem::start/end` do not recognise the legacy `...` operator.
    let op_pos = op.text_range().start();
    let mut lhs = None;
    let mut rhs = None;
    for p in pat.syntax().children().filter_map(ast::Pat::cast) {
        if p.syntax().text_range().end() <= op_pos {
            lhs = Some(p);
        } else if rhs.is_none() {
            rhs = Some(p);
        }
    }
    let infix = if context.config.spaces_around_ranges() {
        let lhs_spacing = if lhs.is_some() { " " } else { "" };
        let rhs_spacing = if rhs.is_some() { " " } else { "" };
        format!("{lhs_spacing}{infix}{rhs_spacing}")
    } else {
        infix
    };
    rewrite_pair(
        &RangeOperand { operand: lhs },
        &RangeOperand { operand: rhs },
        PairParts::infix(&infix),
        context,
        shape,
        SeparatorPlace::Front,
    )
}

fn rewrite_struct_pat(
    pat: &ast::RecordPat,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    // 2 =  ` {`
    let path_shape = shape.sub_width(2)?;
    let path_str = rewrite_path(context, PathContext::Expr, &pat.path()?, path_shape)?;
    let field_list = pat.record_pat_field_list()?;
    let fields: Vec<ast::RecordPatField> = field_list.fields().collect();
    let ellipsis = field_list.rest_pat().is_some();

    if fields.is_empty() && !ellipsis {
        return Some(format!("{path_str} {{}}"));
    }

    let (ellipsis_str, terminator) = if ellipsis { (", ..", "..") } else { ("", "}") };

    // 3 = ` { `, 2 = ` }`.
    let (h_shape, v_shape) =
        struct_lit_shape(shape, context, path_str.len() + 3, ellipsis_str.len() + 2)?;

    let items = itemize_list(
        context.snippet_provider,
        fields.iter(),
        terminator,
        ",",
        |f| f.span().lo(),
        |f| f.span().hi(),
        |f| f.rewrite(context, v_shape),
        context.snippet_provider.span_after(span, "{"),
        span.hi(),
        false,
    );
    let item_vec = items.collect::<Vec<_>>();

    let tactic = struct_lit_tactic(h_shape, context, &item_vec);
    let nested_shape = shape_for_tactic(tactic, h_shape, v_shape);
    let fmt = struct_lit_formatting(nested_shape, tactic, context, false);

    let mut fields_str = write_list(&item_vec, &fmt)?;
    let one_line_width = h_shape.map_or(0, |shape| shape.width);

    let has_trailing_comma = fmt.needs_trailing_separator();

    if ellipsis {
        if fields_str.contains('\n') || fields_str.len() > one_line_width {
            // Add a missing trailing comma.
            if !has_trailing_comma {
                fields_str.push(',');
            }
            fields_str.push('\n');
            fields_str.push_str(&nested_shape.indent.to_string(context.config));
        } else if !fields_str.is_empty() {
            // there are preceding struct fields being matched on
            if has_trailing_comma {
                fields_str.push(' ');
            } else {
                fields_str.push_str(", ");
            }
        }
        fields_str.push_str("..");
    }

    // ast::Pat doesn't have attrs so use &[]
    let fields_str = wrap_struct_field(context, &[], &fields_str, shape, v_shape, one_line_width)?;
    Some(format!("{path_str} {{{fields_str}}}"))
}

impl Rewrite for ast::RecordPatField {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let attrs = outer_attributes(self.syntax());
        let pat = self.pat()?;
        let hi_pos = if let Some(last) = attrs.last() {
            last.span().hi()
        } else {
            pat.span().lo()
        };

        let attrs_str = if attrs.is_empty() {
            String::new()
        } else {
            rewrite_attrs(&attrs, context, shape)?
        };

        let pat_str = pat.rewrite(context, shape)?;
        let start = super::nodes::span_without_attrs(self.syntax()).lo();
        match self.name_ref() {
            None => combine_strs_with_missing_comments(
                context,
                &attrs_str,
                &pat_str,
                mk_sp(hi_pos, start),
                shape,
                false,
            ),
            Some(name_ref) => {
                let nested_shape = shape.block_indent(context.config.tab_spaces());
                let id_str = node_text(name_ref.syntax());
                let one_line_width = id_str.len() + 2 + pat_str.len();
                let pat_and_id_str = if one_line_width <= shape.width {
                    format!("{id_str}: {pat_str}")
                } else {
                    format!(
                        "{}:\n{}{}",
                        id_str,
                        nested_shape.indent.to_string(context.config),
                        pat.rewrite(context, nested_shape)?
                    )
                };
                combine_strs_with_missing_comments(
                    context,
                    &attrs_str,
                    &pat_and_id_str,
                    mk_sp(hi_pos, start),
                    nested_shape,
                    false,
                )
            }
        }
    }
}

/// An element of a tuple or tuple struct pattern.
#[derive(Clone, Debug)]
pub(crate) struct TuplePatField(pub(crate) ast::Pat);

impl Rewrite for TuplePatField {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        self.0.rewrite(context, shape)
    }
}

impl Spanned for TuplePatField {
    fn span(&self) -> Span {
        self.0.span()
    }
}

impl TuplePatField {
    fn is_dotdot(&self) -> bool {
        matches!(self.0, ast::Pat::RestPat(_))
    }
}

pub(crate) fn can_be_overflowed_pat(
    context: &RewriteContext<'_>,
    pat: &ast::Pat,
    len: usize,
) -> bool {
    match pat {
        ast::Pat::PathPat(..)
        | ast::Pat::TuplePat(..)
        | ast::Pat::RecordPat(..)
        | ast::Pat::TupleStructPat(..) => context.use_block_indent() && len == 1,
        ast::Pat::RefPat(r) => r
            .pat()
            .is_some_and(|p| can_be_overflowed_pat(context, &p, len)),
        ast::Pat::BoxPat(b) => b
            .pat()
            .is_some_and(|p| can_be_overflowed_pat(context, &p, len)),
        ast::Pat::LiteralPat(l) => l
            .literal()
            .is_some_and(|lit| can_be_overflowed_expr(context, &ast::Expr::Literal(lit), len)),
        _ => false,
    }
}

fn rewrite_tuple_pat(
    pats: &[ast::Pat],
    path_str: Option<String>,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    if pats.is_empty() {
        return Some(format!("{}()", path_str.unwrap_or_default()));
    }
    debug_assert!(!context.config.condense_wildcard_suffixes());
    let pat_vec: Vec<TuplePatField> = pats.iter().cloned().map(TuplePatField).collect();

    let is_last_pat_dotdot = pat_vec.last().is_some_and(TuplePatField::is_dotdot);
    let add_comma = path_str.is_none() && pat_vec.len() == 1 && !is_last_pat_dotdot;
    let path_str = path_str.unwrap_or_default();

    overflow::rewrite_with_parens(
        context,
        &path_str,
        pat_vec.into_iter().map(OverflowableItem::TuplePatField),
        shape,
        span,
        context.config.max_width(),
        if add_comma {
            Some(SeparatorTactic::Always)
        } else {
            None
        },
    )
}
