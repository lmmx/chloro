//! Types, paths, generics and bounds (rustfmt's `types.rs`).

use ra_ap_syntax::ast::{self, AstNode, AstToken, HasGenericArgs, HasName, HasTypeBounds};
use ra_ap_syntax::{SyntaxKind, SyntaxToken};

use super::attr::rewrite_attrs;
use super::comment::{combine_strs_with_missing_comments, contains_comment};
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{RhsAssignKind, rewrite_assign_rhs, rewrite_tuple, rewrite_unary_prefix};
use super::lists::{
    DefinitiveListTactic, ListFormatting, ListItem, ListTactic, Separator, SeparatorPlace,
    SeparatorTactic, definitive_tactic, itemize_list, write_list,
};
use super::macros::{MacroPosition, rewrite_macro};
use super::nodes::{Attribute, outer_attributes};
use super::overflow::{self, OverflowableItem};
use super::pairs::{PairParts, rewrite_pair};
use super::shape::Shape;
use super::span::{BytePos, Span, Spanned, mk_sp, rustc_span, token_span};
use super::utils::{
    colon_spaces, extra_offset, first_line_width, last_line_extendable, last_line_width,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathContext {
    Expr,
    Type,
    Import,
}

/// The segments of a path, outermost qualifier first.
pub(crate) fn path_segments(path: &ast::Path) -> Vec<ast::PathSegment> {
    let mut segments = Vec::new();
    let mut cur = Some(path.clone());
    while let Some(p) = cur {
        if let Some(seg) = p.segment() {
            segments.push(seg);
        }
        cur = p.qualifier();
    }
    segments.reverse();
    segments
}

/// The text of a path segment's name as written (`r#type` keeps its prefix).
pub(crate) fn segment_ident(segment: &ast::PathSegment) -> Option<String> {
    segment.name_ref().map(|n| n.syntax().text().to_string())
}

/// Does not wrap on simple segments.
pub(crate) fn rewrite_path(
    context: &RewriteContext<'_>,
    path_context: PathContext,
    path: &ast::Path,
    shape: Shape,
) -> Option<String> {
    let segments = path_segments(path);
    let first = segments.first()?;
    let is_global = first.coloncolon_token().is_some() && first.type_anchor().is_none();
    let path_span = path.span();

    // 32 covers almost all path lengths measured when compiling core, and there isn't a big
    // downside from allocating slightly more than necessary.
    let mut result = String::with_capacity(32);

    if is_global && path_context != PathContext::Import {
        result.push_str("::");
    }

    let mut span_lo = path_span.lo();
    let mut rest = &segments[..];

    if let Some(anchor) = first.type_anchor() {
        // `<Ty>::rest` or `<Ty as Trait>::rest`
        result.push('<');
        // `<Ty as Trait>`: the self type, then the trait after `as`.
        let mut anchor_types = anchor.syntax().children().filter_map(ast::Type::cast);
        let ty = anchor_types.next()?;
        let fmt_ty = ty.rewrite(context, shape)?;
        result.push_str(&fmt_ty);

        if let Some(ast::Type::PathType(trait_ty)) = anchor_types.next() {
            result.push_str(" as ");
            let trait_path = trait_ty.path()?;
            let trait_segments = path_segments(&trait_path);
            if trait_segments
                .first()
                .is_some_and(|s| s.coloncolon_token().is_some())
                && path_context != PathContext::Import
            {
                result.push_str("::");
            }

            // 3 = ">::".len()
            let shape = shape.sub_width(3)?;

            result = rewrite_path_segments(
                PathContext::Type,
                result,
                &trait_segments,
                span_lo,
                path_span.hi(),
                context,
                shape,
            )?;
        }

        result.push_str(">::");
        span_lo = ty.span().hi() + 1;
        rest = &segments[1..];
    }

    rewrite_path_segments(
        path_context,
        result,
        rest,
        span_lo,
        path_span.hi(),
        context,
        shape,
    )
}

fn rewrite_path_segments(
    path_context: PathContext,
    mut buffer: String,
    segments: &[ast::PathSegment],
    mut span_lo: BytePos,
    span_hi: BytePos,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let mut first = true;
    let shape = shape.visual_indent(0);

    for segment in segments {
        if first {
            first = false;
        } else {
            buffer.push_str("::");
        }

        let extra_offset = extra_offset(&buffer, shape);
        let new_shape = shape.shrink_left(extra_offset)?;
        let segment_string = rewrite_segment(
            path_context,
            segment,
            &mut span_lo,
            span_hi,
            context,
            new_shape,
        )?;

        buffer.push_str(&segment_string);
    }

    Some(buffer)
}

/// A generic argument of a path segment.
#[derive(Clone, Debug)]
pub(crate) enum SegmentParam {
    Const(ast::ConstArg),
    LifeTime(ast::LifetimeArg),
    Type(ast::Type),
    Binding(ast::AssocTypeArg),
}

impl SegmentParam {
    pub(crate) fn from_generic_arg(arg: &ast::GenericArg) -> Option<SegmentParam> {
        Some(match arg {
            ast::GenericArg::LifetimeArg(lt) => SegmentParam::LifeTime(lt.clone()),
            ast::GenericArg::TypeArg(ty) => SegmentParam::Type(ty.ty()?),
            ast::GenericArg::ConstArg(c) => SegmentParam::Const(c.clone()),
            ast::GenericArg::AssocTypeArg(a) => SegmentParam::Binding(a.clone()),
        })
    }
}

impl Spanned for SegmentParam {
    fn span(&self) -> Span {
        match self {
            SegmentParam::Const(c) => c.span(),
            SegmentParam::LifeTime(lt) => lt.span(),
            SegmentParam::Type(ty) => ty.span(),
            SegmentParam::Binding(binding) => binding.span(),
        }
    }
}

impl Rewrite for SegmentParam {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            SegmentParam::Const(c) => c.expr()?.rewrite(context, shape),
            SegmentParam::LifeTime(lt) => Some(context.snippet(lt.span()).to_owned()),
            SegmentParam::Type(ty) => ty.rewrite(context, shape),
            SegmentParam::Binding(atc) => rewrite_assoc_type_arg(atc, context, shape),
        }
    }
}

impl Rewrite for ast::UseBoundGenericArg {
    fn rewrite(&self, context: &RewriteContext<'_>, _shape: Shape) -> Option<String> {
        Some(context.snippet(self.span()).to_owned())
    }
}

fn rewrite_assoc_type_arg(
    atc: &ast::AssocTypeArg,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let mut result = String::with_capacity(128);
    result.push_str(&atc.name_ref()?.syntax().text().to_string());

    if let Some(gen_args) = atc.generic_arg_list() {
        let budget = shape.width.checked_sub(result.len())?;
        let shape = Shape::legacy(budget, shape.indent + result.len());
        let gen_str = rewrite_generic_args(&gen_args, context, shape, gen_args.span())?;
        result.push_str(&gen_str);
    } else if let Some(args) = atc
        .syntax()
        .children()
        .find_map(ast::ParenthesizedArgList::cast)
    {
        // `Item(T): Bound`
        let budget = shape.width.checked_sub(result.len())?;
        let shape = Shape::legacy(budget, shape.indent + result.len());
        result.push_str(&rewrite_parenthesized_args(&args, None, context, shape)?);
    } else if atc.return_type_syntax().is_some() {
        // Return type notation: `method(..): Bound`.
        result.push_str("(..)");
    }

    let is_bound = atc.type_bound_list().is_some();
    let infix = if is_bound { ": " } else { " = " };
    result.push_str(infix);

    let budget = shape.width.checked_sub(result.len())?;
    let shape = Shape::legacy(budget, shape.indent + result.len());
    let rewrite = if let Some(bounds) = atc.type_bound_list() {
        let bounds: Vec<_> = bounds.bounds().collect();
        rewrite_bounds(&bounds, context, shape)?
    } else if let Some(ty) = atc.ty() {
        ty.rewrite(context, shape)?
    } else {
        atc.const_arg()?.expr()?.rewrite(context, shape)?
    };
    result.push_str(&rewrite);

    Some(result)
}

/// Formats a path segment. There are some hacks involved to correctly determine
/// the segment's associated span since it's not part of the AST.
///
/// The span_lo is assumed to be greater than the end of any previous segment's
/// parameters and lesser or equal than the start of current segment.
///
/// span_hi is assumed equal to the end of the entire path.
///
/// When the segment contains a positive number of parameters, we update span_lo
/// so that invariants described above will hold for the next segment.
fn rewrite_segment(
    path_context: PathContext,
    segment: &ast::PathSegment,
    span_lo: &mut BytePos,
    span_hi: BytePos,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let mut result = String::with_capacity(128);
    result.push_str(&segment_ident(segment)?);

    let ident_len = result.len();
    let shape = shape.offset_left(ident_len)?;

    if let Some(args) = segment.generic_arg_list() {
        let generics_str = rewrite_generic_args(&args, context, shape, mk_sp(*span_lo, span_hi))?;
        if args.generic_args().next().is_some() {
            // HACK: squeeze out the span between the identifier and the parameters.
            // The hack is required so that we don't remove the separator inside macro calls.
            let force_separator = context.inside_macro() && args.coloncolon_token().is_some();
            let separator = if path_context == PathContext::Expr || force_separator {
                "::"
            } else {
                ""
            };
            result.push_str(separator);

            // Update position of last bracket.
            *span_lo = context
                .snippet_provider
                .span_after(mk_sp(*span_lo, span_hi), "<");
        }
        result.push_str(&generics_str)
    } else if let Some(args) = segment.parenthesized_arg_list() {
        let s = rewrite_parenthesized_args(&args, segment.ret_type(), context, shape)?;
        result.push_str(&s);
    } else if segment.return_type_syntax().is_some() {
        result.push_str("(..)");
    }

    Some(result)
}

/// rustc's parenthesized generic args: `Fn(A, B) -> C`, or `Item(T)` in an associated
/// item constraint.
fn rewrite_parenthesized_args(
    args: &ast::ParenthesizedArgList,
    ret: Option<ast::RetType>,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let inputs: Vec<ast::Type> = args.type_args().filter_map(|a| a.ty()).collect();
    let span = mk_sp(
        args.span().lo(),
        ret.as_ref().map_or(args.span().hi(), |r| r.span().hi()),
    );
    let output = ret.and_then(|r| r.ty());
    format_function_type(&inputs, output.as_ref(), false, span, context, shape)
}

pub(crate) fn format_function_type<T: Rewrite + Spanned>(
    inputs: &[T],
    output: Option<&ast::Type>,
    variadic: bool,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    // 4 = " -> "
    let ty_shape = shape.offset_left(4)?;
    let output = match output {
        Some(ty) => {
            let type_str = ty.rewrite(context, ty_shape)?;
            format!(" -> {type_str}")
        }
        None => String::new(),
    };

    let list_shape = Shape::indented(
        shape.block().indent.block_indent(context.config),
        context.config,
    );

    let is_inputs_empty = inputs.is_empty();
    let list_lo = context.snippet_provider.span_after(span, "(");
    let (list_str, tactic) = if is_inputs_empty {
        let tactic = get_tactics(&[], &output, shape);
        let list_hi = context.snippet_provider.span_before(span, ")");
        let comment = context
            .snippet_provider
            .span_to_snippet(mk_sp(list_lo, list_hi))?
            .trim();
        let comment = if comment.starts_with("//") {
            format!(
                "{}{}{}",
                list_shape.indent.to_string_with_newline(context.config),
                comment,
                shape.block().indent.to_string_with_newline(context.config)
            )
        } else {
            comment.to_string()
        };
        (comment, tactic)
    } else {
        let items = itemize_list(
            context.snippet_provider,
            inputs.iter(),
            ")",
            ",",
            |arg| arg.span().lo(),
            |arg| arg.span().hi(),
            |arg| arg.rewrite(context, list_shape),
            list_lo,
            span.hi(),
            false,
        );

        let item_vec: Vec<_> = items.collect();
        let tactic = get_tactics(&item_vec, &output, shape);
        let trailing_separator = if variadic {
            SeparatorTactic::Never
        } else {
            context.config.trailing_comma()
        };

        let fmt = ListFormatting::new(list_shape, context.config)
            .tactic(tactic)
            .trailing_separator(trailing_separator)
            .ends_with_newline(tactic.ends_with_newline())
            .preserve_newline(true);
        (write_list(&item_vec, &fmt)?, tactic)
    };

    let args = if tactic == DefinitiveListTactic::Horizontal || is_inputs_empty {
        format!("({list_str})")
    } else {
        format!(
            "({}{}{})",
            list_shape.indent.to_string_with_newline(context.config),
            list_str,
            shape.block().indent.to_string_with_newline(context.config),
        )
    };
    if output.is_empty() || last_line_width(&args) + first_line_width(&output) <= shape.width {
        Some(format!("{args}{output}"))
    } else {
        Some(format!(
            "{}\n{}{}",
            args,
            list_shape.indent.to_string(context.config),
            output.trim_start()
        ))
    }
}

fn type_bound_colon(context: &RewriteContext<'_>) -> &'static str {
    colon_spaces(context.config)
}

/// If the return type is multi-lined, then force to use multiple lines for
/// arguments as well.
fn get_tactics(item_vec: &[ListItem], output: &str, shape: Shape) -> DefinitiveListTactic {
    if output.contains('\n') {
        DefinitiveListTactic::Vertical
    } else {
        definitive_tactic(
            item_vec,
            ListTactic::HorizontalVertical,
            Separator::Comma,
            // 2 is for the case of ',\n'
            shape.width.saturating_sub(2 + output.len()),
        )
    }
}

impl Rewrite for ast::WherePred {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let attrs = outer_attributes(self.syntax());
        let attrs_str = rewrite_attrs(&attrs, context, shape)?;
        let bounds: Vec<ast::TypeBound> = self
            .type_bound_list()
            .map(|l| l.bounds().collect())
            .unwrap_or_default();
        let pred_str = if let Some(lifetime) = self.lifetime() {
            rewrite_bounded_lifetime(&lifetime, &bounds, context, shape)?
        } else {
            let bounded_ty = self.ty()?;
            let type_str = bounded_ty.rewrite(context, shape)?;
            let colon = type_bound_colon(context).trim_end();
            let generic_params = binder_params(self.for_binder());
            let lhs =
                if let Some(binder_str) = rewrite_bound_params(context, shape, &generic_params) {
                    format!("for<{binder_str}> {type_str}{colon}")
                } else {
                    format!("{type_str}{colon}")
                };

            rewrite_assign_rhs(context, lhs, &Bounds(bounds), &RhsAssignKind::Bounds, shape)?
        };

        let mut result = String::with_capacity(attrs_str.len() + pred_str.len() + 1);
        result.push_str(&attrs_str);
        let pred_start = super::nodes::span_without_attrs(self.syntax()).lo();
        let line_len = last_line_width(&attrs_str) + 1 + first_line_width(&pred_str);
        if let Some(last_attr) = attrs.last().filter(|last_attr| {
            contains_comment(context.snippet(mk_sp(last_attr.span().hi(), pred_start)))
        }) {
            result = combine_strs_with_missing_comments(
                context,
                &result,
                &pred_str,
                mk_sp(last_attr.span().hi(), pred_start),
                Shape {
                    width: shape.width.min(context.config.inline_attribute_width()),
                    ..shape
                },
                !last_attr.is_doc_comment(),
            )?;
        } else {
            if !attrs.is_empty() {
                if context.config.inline_attribute_width() < line_len
                    || attrs.len() > 1
                    || attrs.last().is_some_and(Attribute::is_doc_comment)
                {
                    result.push_str(&shape.indent.to_string_with_newline(context.config));
                } else {
                    result.push(' ');
                }
            }
            result.push_str(&pred_str);
        }

        Some(result)
    }
}

/// The generic parameters of an optional `for<...>` binder.
pub(crate) fn binder_params(binder: Option<ast::ForBinder>) -> Vec<ast::GenericParam> {
    binder
        .and_then(|b| b.generic_param_list())
        .map(|l| l.generic_params().collect())
        .unwrap_or_default()
}

pub(crate) fn rewrite_generic_args(
    gen_args: &ast::GenericArgList,
    context: &RewriteContext<'_>,
    shape: Shape,
    span: Span,
) -> Option<String> {
    let args: Vec<_> = gen_args
        .generic_args()
        .filter_map(|a| SegmentParam::from_generic_arg(&a))
        .map(OverflowableItem::SegmentParam)
        .collect();
    if args.is_empty() {
        Some(String::new())
    } else {
        overflow::rewrite_with_angle_brackets(context, "", args, shape, span)
    }
}

fn rewrite_bounded_lifetime(
    lt: &ast::Lifetime,
    bounds: &[ast::TypeBound],
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let result = context.snippet(lt.span()).to_owned();

    if bounds.is_empty() {
        Some(result)
    } else {
        let colon = type_bound_colon(context);
        let overhead = last_line_width(&result) + colon.len();
        let shape = shape.sub_width(overhead)?;
        let result = format!(
            "{}{}{}",
            result,
            colon,
            join_bounds(context, shape, bounds, true)?
        );
        Some(result)
    }
}

/// A list of bounds, rewritten with `join_bounds`.
pub(crate) struct Bounds(pub(crate) Vec<ast::TypeBound>);

impl Rewrite for Bounds {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        rewrite_bounds(&self.0, context, shape)
    }
}

pub(crate) fn rewrite_bounds(
    bounds: &[ast::TypeBound],
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    if bounds.is_empty() {
        return Some(String::new());
    }
    join_bounds(context, shape, bounds, true)
}

/// Kinds of bounds, as rustc distinguishes them.
enum BoundKind {
    Trait,
    Outlives,
    Use,
}

fn bound_kind(bound: &ast::TypeBound) -> BoundKind {
    if bound.lifetime().is_some() {
        BoundKind::Outlives
    } else if bound.use_bound_generic_args().is_some() {
        BoundKind::Use
    } else {
        BoundKind::Trait
    }
}

impl Rewrite for ast::TypeBound {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match bound_kind(self) {
            BoundKind::Outlives => Some(context.snippet(self.lifetime()?.span()).to_owned()),
            BoundKind::Use => {
                let args: Vec<_> = self
                    .use_bound_generic_args()?
                    .use_bound_generic_args()
                    .map(OverflowableItem::PreciseCapturingArg)
                    .collect();
                overflow::rewrite_with_angle_brackets(context, "use", args, shape, self.span())
            }
            BoundKind::Trait => {
                let has_paren = self
                    .syntax()
                    .children_with_tokens()
                    .any(|el| el.kind() == SyntaxKind::L_PAREN);
                let shape = if has_paren {
                    shape.offset_left(1)?.sub_width(1)?
                } else {
                    shape
                };
                rewrite_poly_trait_ref(self, context, shape)
                    .map(|s| if has_paren { format!("({s})") } else { s })
            }
        }
    }
}

fn token_text_of(bound: &ast::TypeBound, kind: SyntaxKind) -> Option<SyntaxToken> {
    super::nodes::child_token(bound.syntax(), kind)
}

/// `for<'a> const async ?Trait`
fn rewrite_poly_trait_ref(
    bound: &ast::TypeBound,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let generic_params = binder_params(bound.for_binder());
    let (binder, shape) =
        if let Some(lifetime_str) = rewrite_bound_params(context, shape, &generic_params) {
            // 6 is "for<> ".len()
            let extra_offset = lifetime_str.len() + 6;
            let shape = shape.offset_left(extra_offset)?;
            (format!("for<{lifetime_str}> "), shape)
        } else if bound.for_binder().is_some() {
            ("for<> ".to_owned(), shape.offset_left(6)?)
        } else {
            (String::new(), shape)
        };

    // rustc's `BoundConstness::as_str`: `~const` and `[const]` are both "maybe const" and
    // print as `[const]`.
    let mut constness = String::new();
    if token_text_of(bound, SyntaxKind::TILDE).is_some()
        || token_text_of(bound, SyntaxKind::L_BRACK).is_some()
    {
        constness.push_str("[const] ");
    } else if bound
        .syntax()
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t| t.kind() == SyntaxKind::CONST_KW)
    {
        constness.push_str("const ");
    }
    let asyncness = if token_text_of(bound, SyntaxKind::ASYNC_KW).is_some() {
        "async "
    } else {
        ""
    };
    let polarity = if token_text_of(bound, SyntaxKind::QUESTION).is_some() {
        "?"
    } else if token_text_of(bound, SyntaxKind::BANG).is_some() {
        "!"
    } else {
        ""
    };
    let shape = shape.offset_left(constness.len() + polarity.len())?;

    let path_str = match bound.ty()? {
        ast::Type::PathType(p) => rewrite_path(context, PathContext::Type, &p.path()?, shape)?,
        other => other.rewrite(context, shape)?,
    };
    Some(format!(
        "{binder}{constness}{asyncness}{polarity}{path_str}"
    ))
}

impl Rewrite for ast::GenericParam {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        // FIXME: If there are more than one attributes, this will force multiline.
        let attrs = outer_attributes(self.syntax());
        let mut result = rewrite_attrs(&attrs, context, shape).unwrap_or_default();
        let has_attrs = !result.is_empty();

        let mut param = String::with_capacity(128);

        let bounds: Vec<ast::TypeBound> = match self {
            ast::GenericParam::LifetimeParam(lp) => lp
                .type_bound_list()
                .map(|l| l.bounds().collect())
                .unwrap_or_default(),
            ast::GenericParam::TypeParam(tp) => tp
                .type_bound_list()
                .map(|l| l.bounds().collect())
                .unwrap_or_default(),
            ast::GenericParam::ConstParam(_) => vec![],
        };

        let param_start = match self {
            ast::GenericParam::ConstParam(cp) => {
                param.push_str("const ");
                param.push_str(&cp.name()?.syntax().text().to_string());
                param.push_str(": ");
                param.push_str(&cp.ty()?.rewrite(context, shape)?);
                if let Some(default) = cp.default_val() {
                    param.push_str(" = ");
                    let budget = shape.width.checked_sub(param.len())?;
                    let rewrite = default
                        .expr()?
                        .rewrite(context, Shape::legacy(budget, shape.indent))?;
                    param.push_str(&rewrite);
                }
                token_span(&cp.const_token()?).lo()
            }
            ast::GenericParam::LifetimeParam(lp) => {
                let lt = lp.lifetime()?;
                param.push_str(context.snippet(lt.span()));
                lt.span().lo()
            }
            ast::GenericParam::TypeParam(tp) => {
                let name = tp.name()?;
                param.push_str(&name.syntax().text().to_string());
                name.span().lo()
            }
        };

        if !bounds.is_empty() {
            param.push_str(type_bound_colon(context));
            param.push_str(&rewrite_bounds(&bounds, context, shape)?)
        }
        if let ast::GenericParam::TypeParam(tp) = self
            && let Some(def) = tp.default_type()
        {
            param.push_str(" = ");
            let budget = shape.width.checked_sub(param.len())?;
            let rewrite =
                def.rewrite(context, Shape::legacy(budget, shape.indent + param.len()))?;
            param.push_str(&rewrite);
        }

        if let Some(last_attr) = attrs.last().filter(|last_attr| {
            contains_comment(context.snippet(mk_sp(last_attr.span().hi(), param_start)))
        }) {
            result = combine_strs_with_missing_comments(
                context,
                &result,
                &param,
                mk_sp(last_attr.span().hi(), param_start),
                shape,
                !last_attr.is_doc_comment(),
            )?;
        } else {
            // When rewriting generic params, an extra newline should be put
            // if the attributes end with a doc comment
            if let Some(true) = attrs.last().map(Attribute::is_doc_comment) {
                result.push_str(&shape.indent.to_string_with_newline(context.config));
            } else if has_attrs {
                result.push(' ');
            }
            result.push_str(&param);
        }

        Some(result)
    }
}

/// rustc's span of a generic parameter (attributes included).
pub(crate) fn generic_param_span(param: &ast::GenericParam) -> Span {
    rustc_span(param.syntax())
}

impl Rewrite for ast::Type {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            ast::Type::DynTraitType(dt) => {
                // A bare trait object (`'a + Trait`, edition 2015 syntax) stays bare.
                let (shape, prefix) = if dt.dyn_token().is_some() {
                    // 4 = "dyn "
                    (shape.offset_left(4)?, "dyn ")
                } else {
                    (shape, "")
                };
                let bounds: Vec<_> = dt
                    .type_bound_list()
                    .map(|l| l.bounds().collect())
                    .unwrap_or_default();
                let mut res = rewrite_bounds(&bounds, context, shape)?;
                // We may have falsely removed a trailing `+` inside macro call.
                if context.inside_macro()
                    && bounds.len() == 1
                    && context.snippet(self.span()).ends_with('+')
                    && !res.ends_with('+')
                {
                    res.push('+');
                }
                Some(format!("{prefix}{res}"))
            }
            ast::Type::PtrType(pt) => {
                let prefix = if pt.mut_token().is_some() {
                    "*mut "
                } else {
                    "*const "
                };
                rewrite_unary_prefix(context, prefix, &pt.ty()?, shape)
            }
            ast::Type::RefType(rt) => rewrite_ref_type(rt, context, shape),
            ast::Type::ParenType(pt) => {
                let ty = pt.ty()?;
                if context.config.style_edition() <= StyleEdition::Edition2021 {
                    let budget = shape.width.checked_sub(2)?;
                    return ty
                        .rewrite(context, Shape::legacy(budget, shape.indent + 1))
                        .map(|ty_str| format!("({ty_str})"));
                }
                // 2 = ()
                if let Some(sh) = shape.sub_width(2)
                    && let Some(ref s) = ty.rewrite(context, sh)
                    && !s.contains('\n')
                {
                    return Some(format!("({s})"));
                }

                let indent_str = shape.indent.to_string_with_newline(context.config);
                let shape = shape
                    .block_indent(context.config.tab_spaces())
                    .with_max_width(context.config);
                let rw = ty.rewrite(context, shape)?;
                Some(format!(
                    "({}{}{})",
                    shape.to_string_with_newline(context.config),
                    rw,
                    indent_str
                ))
            }
            ast::Type::SliceType(st) => {
                let budget = shape.width.checked_sub(4)?;
                st.ty()?
                    .rewrite(context, Shape::legacy(budget, shape.indent + 1))
                    .map(|ty_str| format!("[{ty_str}]"))
            }
            ast::Type::TupleType(tt) => {
                let fields: Vec<_> = tt.fields().map(OverflowableItem::Ty).collect();
                let len = fields.len();
                rewrite_tuple(context, fields, self.span(), shape, len == 1)
            }
            ast::Type::PathType(pt) => rewrite_path(context, PathContext::Type, &pt.path()?, shape),
            ast::Type::ArrayType(at) => rewrite_pair(
                &at.ty()?,
                &at.const_arg()?.expr()?,
                PairParts::new("[", "; ", "]"),
                context,
                shape,
                SeparatorPlace::Back,
            ),
            ast::Type::InferType(_) => (shape.width >= 1).then(|| "_".to_owned()),
            ast::Type::FnPtrType(fp) => rewrite_fn_ptr(fp, &[], false, self.span(), context, shape),
            ast::Type::ForType(ft) => {
                let params = binder_params(ft.for_binder());
                match ft.ty()? {
                    ast::Type::FnPtrType(fp) => {
                        rewrite_fn_ptr(&fp, &params, true, self.span(), context, shape)
                    }
                    inner => {
                        // A bare trait object with a binder, e.g. `for<'a> Fn(&'a u8)`.
                        let binder =
                            rewrite_bound_params(context, shape, &params).unwrap_or_default();
                        let prefix = format!("for<{binder}> ");
                        let shape = shape.offset_left(prefix.len())?;
                        inner
                            .rewrite(context, shape)
                            .map(|s| format!("{prefix}{s}"))
                    }
                }
            }
            ast::Type::NeverType(_) => Some(String::from("!")),
            ast::Type::MacroType(mt) => {
                rewrite_macro(&mt.macro_call()?, context, shape, MacroPosition::Expression)
            }
            ast::Type::ImplTraitType(it) => {
                let bounds: Vec<_> = it
                    .type_bound_list()
                    .map(|l| l.bounds().collect())
                    .unwrap_or_default();
                // Empty trait is not a parser error.
                if bounds.is_empty() {
                    return Some("impl".to_owned());
                }
                // Style edition 2021 rewrites the bounds as `GenericBounds`, which indents
                // continuation lines.
                let need_indent = context.config.style_edition() <= StyleEdition::Edition2021;
                join_bounds(context, shape, &bounds, need_indent).map(|it_str| {
                    let space = if it_str.is_empty() { "" } else { " " };
                    format!("impl{space}{it_str}")
                })
            }
        }
    }
}

fn rewrite_ref_type(
    rt: &ast::RefType,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let span = rt.span();
    let is_mut = rt.mut_token().is_some();
    let mut_str = if is_mut { "mut " } else { "" };
    let mut_len = mut_str.len();
    let mut result = String::with_capacity(128);
    result.push('&');
    let ref_hi = context.snippet_provider.span_after(span, "&");
    let mut cmnt_lo = ref_hi;
    let ty = rt.ty()?;

    if let Some(lifetime) = rt.lifetime() {
        shape.width.checked_sub(2 + mut_len)?;
        let lt_str = context.snippet(lifetime.span()).to_owned();
        let before_lt_span = mk_sp(cmnt_lo, lifetime.span().lo());
        if contains_comment(context.snippet(before_lt_span)) {
            result = combine_strs_with_missing_comments(
                context,
                &result,
                &lt_str,
                before_lt_span,
                shape,
                true,
            )?;
        } else {
            result.push_str(&lt_str);
        }
        result.push(' ');
        cmnt_lo = lifetime.span().hi();
    }

    if is_mut {
        let mut_hi = token_span(&rt.mut_token()?).hi();
        let before_mut_span = mk_sp(cmnt_lo, mut_hi - 3);
        if contains_comment(context.snippet(before_mut_span)) {
            result = combine_strs_with_missing_comments(
                context,
                result.trim_end(),
                mut_str,
                before_mut_span,
                shape,
                true,
            )?;
        } else {
            result.push_str(mut_str);
        }
        cmnt_lo = mut_hi;
    }

    let before_ty_span = mk_sp(cmnt_lo, ty.span().lo());
    if contains_comment(context.snippet(before_ty_span)) {
        result = combine_strs_with_missing_comments(
            context,
            result.trim_end(),
            &ty.rewrite(context, shape)?,
            before_ty_span,
            shape,
            true,
        )?;
    } else {
        let used_width = last_line_width(&result);
        let budget = shape.width.checked_sub(used_width)?;
        let ty_str = ty.rewrite(context, Shape::legacy(budget, shape.indent + used_width))?;
        result.push_str(&ty_str);
    }

    Some(result)
}

/// The text of an `extern` ABI, honouring `force_explicit_abi`.
pub(crate) fn format_abi(abi: Option<&ast::Abi>, context: &RewriteContext<'_>) -> String {
    let Some(abi) = abi else {
        return String::new();
    };
    match abi.abi_string() {
        Some(s) => {
            let text = s.syntax().text();
            if !context.config.force_explicit_abi() && (text == "\"C\"") {
                "extern ".to_owned()
            } else {
                format!("extern {text} ")
            }
        }
        None if context.config.force_explicit_abi() => "extern \"C\" ".to_owned(),
        None => "extern ".to_owned(),
    }
}

fn rewrite_fn_ptr(
    fn_ptr: &ast::FnPtrType,
    generic_params: &[ast::GenericParam],
    has_binder: bool,
    span: Span,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let mut result = String::with_capacity(128);

    if let Some(ref lifetime_str) = rewrite_bound_params(context, shape, generic_params) {
        result.push_str("for<");
        // 6 = "for<> ".len(), 4 = "for<".
        // This doesn't work out so nicely for multiline situation with lots of
        // rightward drift. If that is a problem, we could use the list stuff.
        result.push_str(lifetime_str);
        result.push_str("> ");
    } else if has_binder {
        result.push_str("for<> ");
    }

    if fn_ptr.unsafe_token().is_some() {
        result.push_str("unsafe ");
    } else if fn_ptr
        .syntax()
        .children_with_tokens()
        .any(|el| el.kind() == SyntaxKind::SAFE_KW)
    {
        result.push_str("safe ");
    }

    result.push_str(&format_abi(fn_ptr.abi().as_ref(), context));

    result.push_str("fn");

    let func_ty_shape = shape.offset_left(result.len())?;

    let param_list = fn_ptr.param_list()?;
    let params: Vec<ast::Param> = param_list.params().collect();
    let output = fn_ptr.ret_type().and_then(|r| r.ty());
    let decl_span = mk_sp(
        param_list.span().lo(),
        fn_ptr
            .ret_type()
            .map_or(param_list.span().hi(), |r| r.span().hi()),
    );
    debug_assert!(span.hi() >= decl_span.hi());

    let rewrite = format_function_type(
        &params,
        output.as_ref(),
        has_c_variadic(&param_list),
        decl_span,
        context,
        func_ty_shape,
    )?;

    result.push_str(&rewrite);

    Some(result)
}

/// Whether a parameter list ends with C variadics (`...`).
pub(crate) fn has_c_variadic(params: &ast::ParamList) -> bool {
    params
        .params()
        .last()
        .is_some_and(|p| p.dotdotdot_token().is_some())
}

fn is_generic_bounds_in_order(generic_bounds: &[ast::TypeBound]) -> bool {
    let is_trait = |b: &ast::TypeBound| !matches!(bound_kind(b), BoundKind::Outlives);
    let is_lifetime = |b: &ast::TypeBound| !is_trait(b);
    let last_trait_index = generic_bounds.iter().rposition(is_trait);
    let first_lifetime_index = generic_bounds.iter().position(is_lifetime);
    match (last_trait_index, first_lifetime_index) {
        (Some(last_trait_index), Some(first_lifetime_index)) => {
            last_trait_index < first_lifetime_index
        }
        _ => true,
    }
}

pub(crate) fn join_bounds(
    context: &RewriteContext<'_>,
    shape: Shape,
    items: &[ast::TypeBound],
    need_indent: bool,
) -> Option<String> {
    join_bounds_inner(context, shape, items, need_indent, false)
}

fn join_bounds_inner(
    context: &RewriteContext<'_>,
    shape: Shape,
    items: &[ast::TypeBound],
    need_indent: bool,
    force_newline: bool,
) -> Option<String> {
    debug_assert!(!items.is_empty());

    let generic_bounds_in_order = is_generic_bounds_in_order(items);
    let is_bound_extendable = |s: &str, b: &ast::TypeBound| match bound_kind(b) {
        BoundKind::Outlives => true,
        // We treat `use<>` like a trait bound here.
        BoundKind::Trait | BoundKind::Use => last_line_extendable(s),
    };

    // Whether a GenericBound item is a PathSegment segment that includes internal array
    // that contains more than one item
    let is_item_with_multi_items_array = |item: &ast::TypeBound| match bound_kind(item) {
        BoundKind::Trait => {
            let Some(ast::Type::PathType(pt)) = item.ty() else {
                return false;
            };
            let Some(path) = pt.path() else {
                return false;
            };
            let segments = path_segments(&path);
            if segments.len() > 1 {
                true
            } else {
                segments[0]
                    .generic_arg_list()
                    .is_some_and(|args| args.generic_args().count() > 1)
            }
        }
        BoundKind::Use => item
            .use_bound_generic_args()
            .is_some_and(|a| a.use_bound_generic_args().count() > 1),
        BoundKind::Outlives => false,
    };

    let mut strs = String::new();
    let mut prev_trailing_span: Option<Span> = None;
    let mut prev_extendable = false;
    for (i, item) in items.iter().enumerate() {
        let trailing_span = if i < items.len() - 1 {
            let hi = context
                .snippet_provider
                .span_before(mk_sp(item.span().hi(), items[i + 1].span().lo()), "+");

            Some(mk_sp(item.span().hi(), hi))
        } else {
            None
        };
        let (leading_span, has_leading_comment) = if i > 0 {
            let lo = context
                .snippet_provider
                .span_after(mk_sp(items[i - 1].span().hi(), item.span().lo()), "+");

            let span = mk_sp(lo, item.span().lo());

            let has_comments = contains_comment(context.snippet(span));

            (Some(mk_sp(lo, item.span().lo())), has_comments)
        } else {
            (None, false)
        };
        let prev_has_trailing_comment = match prev_trailing_span {
            Some(ts) => contains_comment(context.snippet(ts)),
            _ => false,
        };

        let shape = if need_indent && force_newline {
            shape
                .block_indent(context.config.tab_spaces())
                .with_max_width(context.config)
        } else {
            shape
        };
        let whitespace = if force_newline && (!prev_extendable || !generic_bounds_in_order) {
            shape
                .indent
                .to_string_with_newline(context.config)
                .to_string()
        } else {
            String::from(" ")
        };

        let joiner = whitespace + "+ ";
        let joiner = if has_leading_comment {
            joiner.trim_end()
        } else {
            &joiner
        };
        let joiner = if prev_has_trailing_comment {
            joiner.trim_start()
        } else {
            joiner
        };

        let (extendable, trailing_str) = if i == 0 {
            let bound_str = item.rewrite(context, shape)?;
            (is_bound_extendable(&bound_str, item), bound_str)
        } else {
            let bound_str = &item.rewrite(context, shape)?;
            match leading_span {
                Some(ls) if has_leading_comment => (
                    is_bound_extendable(bound_str, item),
                    combine_strs_with_missing_comments(
                        context, joiner, bound_str, ls, shape, true,
                    )?,
                ),
                _ => (
                    is_bound_extendable(bound_str, item),
                    String::from(joiner) + bound_str,
                ),
            }
        };
        strs = match prev_trailing_span {
            Some(ts) if prev_has_trailing_comment => {
                combine_strs_with_missing_comments(context, &strs, &trailing_str, ts, shape, true)?
            }
            _ => strs + &trailing_str,
        };
        prev_trailing_span = trailing_span;
        prev_extendable = extendable;
    }

    // Whether to retry with a forced newline:
    //   Only if result is not already multiline and did not exceed line width,
    //   and either there is more than one item;
    //       or the single item is of type `Trait`,
    //          and any of the internal arrays contains more than one item;
    let retry_with_force_newline = if context.config.style_edition() <= StyleEdition::Edition2021 {
        !force_newline && items.len() > 1 && (strs.contains('\n') || strs.len() > shape.width)
    } else if force_newline || (!strs.contains('\n') && strs.len() <= shape.width) {
        false
    } else if items.len() > 1 {
        true
    } else {
        is_item_with_multi_items_array(&items[0])
    };

    if retry_with_force_newline {
        join_bounds_inner(context, shape, items, need_indent, true)
    } else {
        Some(strs)
    }
}

/// The bounds of an `impl Trait` type, if `ty` is one.
pub(crate) fn opaque_ty(ty: Option<&ast::Type>) -> Option<Vec<ast::TypeBound>> {
    match ty? {
        ast::Type::ImplTraitType(it) => Some(it.type_bound_list()?.bounds().collect()),
        _ => None,
    }
}

pub(crate) fn can_be_overflowed_type(
    context: &RewriteContext<'_>,
    ty: &ast::Type,
    len: usize,
) -> bool {
    match ty {
        ast::Type::TupleType(..) => context.use_block_indent() && len == 1,
        ast::Type::RefType(rt) => rt
            .ty()
            .is_some_and(|ty| can_be_overflowed_type(context, &ty, len)),
        ast::Type::PtrType(pt) => pt
            .ty()
            .is_some_and(|ty| can_be_overflowed_type(context, &ty, len)),
        _ => false,
    }
}

/// Returns `None` if there is no `GenericParam` in the list
pub(crate) fn rewrite_bound_params(
    context: &RewriteContext<'_>,
    shape: Shape,
    generic_params: &[ast::GenericParam],
) -> Option<String> {
    let result = generic_params
        .iter()
        .map(|param| param.rewrite(context, shape))
        .collect::<Option<Vec<_>>>()?
        .join(", ");
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
