//! Items: functions, structs, enums, traits, impls, type aliases, statics (rustfmt's
//! `items.rs`), plus `let` statements, which rustfmt formats here too.
//!
//! rustc keeps a fn's signature, generics and where clause in separate AST nodes with
//! spans; [`FnSig`], [`Generics`] and [`WhereClause`] rebuild those views from
//! rust-analyzer nodes, including rustc's empty spans for absent generics and where
//! clauses, which the comment-recovery logic relies on.

use std::borrow::Cow;
use std::cmp::{max, min};

use ra_ap_syntax::ast::{self, AstNode, HasGenericParams, HasName, HasVisibility};
use ra_ap_syntax::{SyntaxKind, SyntaxNode, T};

use super::comment::{
    FindUncommented, combine_strs_with_missing_comments, contains_comment, is_last_comment_block,
    recover_comment_removed, recover_missing_comment_in_span, rewrite_missing_comment,
};
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{
    RhsAssignKind, RhsTactics, is_empty_block, is_simple_block_stmt, rewrite_assign_rhs,
    rewrite_assign_rhs_with, rewrite_assign_rhs_with_comments, rewrite_else_kw_with_comments,
    rewrite_let_else_block,
};
use super::lists::{
    DefinitiveListTactic, ListFormatting, ListTactic, Separator, SeparatorTactic,
    definitive_tactic, itemize_list, write_list,
};
use super::macros::{MacroPosition, rewrite_macro};
use super::nodes::node_text;
use super::nodes::{
    Attribute, Block, contains_skip, generics_span, inner_attributes, outer_attributes,
    span_without_attrs, where_clause_span,
};
use super::overflow::{self, OverflowableItem};
use super::shape::{Indent, Shape};
use super::span::{BytePos, Span, Spanned, mk_sp, node_range_span, rustc_span, token_span};
use super::stmt::Stmt;
use super::types::{Bounds, format_abi, opaque_ty};
use super::utils::{
    colon_spaces, first_line_contains_single_line_comment, first_line_width, format_visibility,
    is_attributes_extendable, is_single_line, last_line_contains_single_line_comment,
    last_line_used_width, last_line_width, starts_with_newline, trimmed_last_line_width,
};
use super::visitor::FmtVisitor;

fn type_annotation_separator(context: &RewriteContext<'_>) -> &'static str {
    colon_spaces(context.config)
}

/// rustc's item span: the item without its outer attributes and doc comments.
pub(crate) fn item_span(node: &SyntaxNode) -> Span {
    span_without_attrs(node)
}

/// `true` if `node` has a direct child token of `kind`.
fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    node.children_with_tokens().any(|el| el.kind() == kind)
}

fn name_span(name: &ast::Name) -> Span {
    node_range_span(name.syntax())
}

fn name_text(name: Option<ast::Name>) -> Option<String> {
    Some(node_text(name?.syntax()))
}

/// The `(lo, hi)` of rustc's visibility span: the visibility, or an empty span at the start
/// of the item.
fn vis_lo_hi(vis: Option<&ast::Visibility>, item_lo: BytePos) -> (BytePos, BytePos) {
    match vis {
        Some(v) => {
            let span = node_range_span(v.syntax());
            (span.lo(), span.hi())
        }
        None => (item_lo, item_lo),
    }
}

// ---------------------------------------------------------------------------------------
// Generics and where clauses
// ---------------------------------------------------------------------------------------

/// rustc's `ast::WhereClause`: the predicates and the clause span (empty when absent).
#[derive(Clone, Debug)]
pub(crate) struct WhereClause {
    pub(crate) predicates: Vec<ast::WherePred>,
    pub(crate) span: Span,
}

impl WhereClause {
    pub(crate) fn new(wc: Option<ast::WhereClause>, fallback: BytePos) -> WhereClause {
        WhereClause {
            span: where_clause_span(wc.as_ref(), fallback),
            predicates: wc.map(|wc| wc.predicates().collect()).unwrap_or_default(),
        }
    }
}

/// rustc's `ast::Generics`: parameters, their `<...>` span (empty right after the name
/// when absent) and the where clause.
#[derive(Clone, Debug)]
pub(crate) struct Generics {
    pub(crate) params: Vec<ast::GenericParam>,
    pub(crate) span: Span,
    pub(crate) where_clause: WhereClause,
}

impl Generics {
    /// `where_fallback` is where rustc would have parsed a `where` keyword.
    pub(crate) fn new(
        list: Option<ast::GenericParamList>,
        name_hi: BytePos,
        wc: Option<ast::WhereClause>,
        where_fallback: BytePos,
    ) -> Generics {
        Generics {
            span: generics_span(list.as_ref(), name_hi),
            params: list
                .map(|l| l.generic_params().collect())
                .unwrap_or_default(),
            where_clause: WhereClause::new(wc, where_fallback),
        }
    }
}

// ---------------------------------------------------------------------------------------
// `let` statements
// ---------------------------------------------------------------------------------------

/// Statements of the form `let pat: ty = init;` or `let pat: ty = init else { .. };`.
impl Rewrite for ast::LetStmt {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let attrs = outer_attributes(self.syntax());
        if contains_skip(&attrs) {
            return None;
        }
        // rustfmt does not format `super let` yet.
        if self.super_token().is_some() {
            return None;
        }
        let span = item_span(self.syntax());

        let attrs_str = attrs.rewrite(context, shape)?;
        let mut result = if attrs_str.is_empty() {
            "let ".to_owned()
        } else {
            combine_strs_with_missing_comments(
                context,
                &attrs_str,
                "let ",
                mk_sp(attrs.last()?.span().hi(), span.lo()),
                shape,
                false,
            )?
        };
        let let_kw_offset = result.len() - "let ".len();

        // 4 = "let ".len()
        let pat_shape = shape.offset_left(4)?;
        // 1 = ;
        let pat_shape = pat_shape.sub_width(1)?;
        let pat = self.pat()?;
        let pat_str = pat.rewrite(context, pat_shape)?;

        result.push_str(&pat_str);

        // String that is placed within the assignment pattern and expression.
        let infix = {
            let mut infix = String::with_capacity(32);

            if let Some(ty) = self.ty() {
                let separator = type_annotation_separator(context);
                let ty_shape = if pat_str.contains('\n') {
                    shape.with_max_width(context.config)
                } else {
                    shape
                }
                .offset_left(last_line_width(&result) + separator.len())?
                // 2 = ` =`
                .sub_width(2)?;

                let rewrite = ty.rewrite(context, ty_shape)?;

                infix.push_str(separator);
                infix.push_str(&rewrite);
            }

            if self.initializer().is_some() {
                infix.push_str(" =");
            }

            infix
        };

        result.push_str(&infix);

        if let Some(init) = self.initializer() {
            // 1 = trailing semicolon;
            let nested_shape = shape.sub_width(1)?;

            result =
                rewrite_assign_rhs(context, result, &init, &RhsAssignKind::Expr, nested_shape)?;

            if let Some(block_expr) = self.let_else().and_then(|le| le.block_expr()) {
                let block = Block::from_block_expr(&block_expr)?;
                let else_kw_span = init.span().between(block.span);
                // Strip attributes and comments to check if newline is needed before the else
                // keyword from the initializer part.
                let style_edition = context.config.style_edition();
                let init_str = if style_edition >= StyleEdition::Edition2024 {
                    &result[let_kw_offset..]
                } else {
                    result.as_str()
                };
                let force_newline_else = pat_str.contains('\n')
                    || !same_line_else_kw_and_brace(init_str, context, else_kw_span, nested_shape);
                let else_kw = rewrite_else_kw_with_comments(
                    force_newline_else,
                    true,
                    context,
                    else_kw_span,
                    shape,
                );
                result.push_str(&else_kw);

                // At this point `let {pat} = {expr} else` is in the buffer; decide up front
                // whether the divergent block fits on the same line. The available space is
                // the smaller of `shape.width` and `single_line_let_else_max_width`.
                let max_width =
                    std::cmp::min(shape.width, context.config.single_line_let_else_max_width());

                // If available_space hits zero we know for sure this will be a multi-lined block
                let assign_str_with_else_kw = if style_edition >= StyleEdition::Edition2024 {
                    &result[let_kw_offset..]
                } else {
                    result.as_str()
                };
                let available_space = max_width.saturating_sub(assign_str_with_else_kw.len());

                let allow_single_line = !force_newline_else
                    && available_space > 0
                    && allow_single_line_let_else_block(assign_str_with_else_kw, &block);

                let mut rw_else_block =
                    rewrite_let_else_block(&block, allow_single_line, context, shape)?;

                let single_line_else = !rw_else_block.contains('\n');
                // +1 for the trailing `;`
                let else_block_exceeds_width = rw_else_block.len() + 1 > available_space;

                if allow_single_line && single_line_else && else_block_exceeds_width {
                    // writing this on one line would exceed the available width
                    // so rewrite the else block over multiple lines.
                    rw_else_block = rewrite_let_else_block(&block, false, context, shape)?;
                }

                result.push_str(&rw_else_block);
            };
        }

        result.push(';');
        Some(result)
    }
}

/// When the initializer expression is multi-lined, then the else keyword and opening brace of the
/// block ( i.e. "else {") should be put on the same line as the end of the initializer expression
/// if all the following are true:
///
/// 1. The initializer expression ends with one or more closing parentheses, square brackets,
///    or braces
/// 2. There is nothing else on that line
/// 3. That line is not indented beyond the indent on the first line of the let keyword
fn same_line_else_kw_and_brace(
    init_str: &str,
    context: &RewriteContext<'_>,
    else_kw_span: Span,
    init_shape: Shape,
) -> bool {
    if !init_str.contains('\n') {
        // initializer expression is single lined. The "else {" can only be placed on the same line
        // as the initializer expression if there is enough room for it.
        // 7 = ` else {`
        return init_shape.width.saturating_sub(init_str.len()) >= 7;
    }

    // 1. The initializer expression ends with one or more `)`, `]`, `}`.
    if !init_str.ends_with([')', ']', '}']) {
        return false;
    }

    // 2. There is nothing else on that line
    // For example, there are no comments
    let else_kw_snippet = context.snippet(else_kw_span).trim();
    if else_kw_snippet != "else" {
        return false;
    }

    // 3. The last line of the initializer expression is not indented beyond the `let` keyword
    let indent = init_shape.indent.to_string(context.config);
    init_str
        .lines()
        .last()
        .and_then(|l| l.strip_prefix(indent.as_ref()))
        .is_some_and(|l| !l.starts_with(char::is_whitespace))
}

fn allow_single_line_let_else_block(result: &str, block: &Block) -> bool {
    !result.contains('\n') && block.stmts.len() <= 1
}

// ---------------------------------------------------------------------------------------
// Function signatures
// ---------------------------------------------------------------------------------------

/// A function parameter: rustc models `self` as an ordinary parameter.
#[derive(Clone, Debug)]
pub(crate) enum FnParam {
    SelfParam(ast::SelfParam),
    Param(ast::Param),
}

impl FnParam {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            FnParam::SelfParam(p) => p.syntax(),
            FnParam::Param(p) => p.syntax(),
        }
    }

    fn span_lo(&self) -> BytePos {
        match self {
            FnParam::SelfParam(p) => p.span().lo(),
            FnParam::Param(p) => span_lo_for_param(p),
        }
    }

    /// rustc's `param.ty.span.hi()`.
    fn ty_hi(&self) -> BytePos {
        match self {
            FnParam::SelfParam(p) => node_range_span(p.syntax()).hi(),
            FnParam::Param(p) => param_ty_hi(p),
        }
    }
}

impl Spanned for FnParam {
    fn span(&self) -> Span {
        rustc_span(self.syntax())
    }
}

impl Rewrite for FnParam {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            FnParam::SelfParam(p) => p.rewrite(context, shape),
            FnParam::Param(p) => p.rewrite(context, shape),
        }
    }
}

/// A fn's signature (rustc's `FnSig` together with the fn's generics and visibility).
pub(crate) struct FnSig {
    vis: Option<ast::Visibility>,
    defaultness: &'static str,
    constness: &'static str,
    coroutine: &'static str,
    safety: &'static str,
    abi: Option<ast::Abi>,
    has_abi: bool,
    pub(crate) generics: Generics,
    params: Vec<FnParam>,
    variadic: bool,
    ret: Option<ast::RetType>,
    /// rustc's `FnRetTy::Default(span)`: an empty span right after the parameter list.
    default_ret_lo: BytePos,
}

impl FnSig {
    pub(crate) fn from_fn(f: &ast::Fn) -> Option<FnSig> {
        let name_hi = name_span(&f.name()?).hi();
        let param_list = f.param_list()?;
        let params_hi = node_range_span(param_list.syntax()).hi();
        let ret = f.ret_type();
        let where_fallback = ret
            .as_ref()
            .map_or(params_hi, |r| node_range_span(r.syntax()).hi());
        let mut params = Vec::new();
        if let Some(s) = param_list.self_param() {
            params.push(FnParam::SelfParam(s));
        }
        params.extend(param_list.params().map(FnParam::Param));
        // rustc's `FnDecl::c_variadic`: the last parameter is `...`.
        let variadic = super::types::has_c_variadic(&param_list);
        let coroutine = match (f.async_token().is_some(), f.gen_token().is_some()) {
            (true, true) => "async gen ",
            (true, false) => "async ",
            (false, true) => "gen ",
            (false, false) => "",
        };
        let safety = if f.unsafe_token().is_some() {
            "unsafe "
        } else if f.safe_token().is_some() {
            "safe "
        } else {
            ""
        };
        Some(FnSig {
            vis: f.visibility(),
            defaultness: if f.default_token().is_some() {
                "default "
            } else {
                ""
            },
            constness: if f.const_token().is_some() {
                "const "
            } else {
                ""
            },
            coroutine,
            safety,
            has_abi: f.abi().is_some(),
            abi: f.abi(),
            generics: Generics::new(
                f.generic_param_list(),
                name_hi,
                f.where_clause(),
                where_fallback,
            ),
            params,
            variadic,
            ret,
            default_ret_lo: params_hi,
        })
    }

    fn to_str(&self, context: &RewriteContext<'_>) -> String {
        let mut result = String::with_capacity(128);
        // Vis defaultness constness unsafety abi.
        result.push_str(&format_visibility(self.vis.as_ref()));
        result.push_str(self.defaultness);
        result.push_str(self.constness);
        result.push_str(self.coroutine);
        result.push_str(self.safety);
        if self.has_abi {
            result.push_str(&format_abi(self.abi.as_ref(), context));
        }
        result
    }

    fn ret_span(&self) -> Span {
        match self.ret.as_ref().and_then(|r| r.ty()) {
            Some(ty) => ty.span(),
            None => mk_sp(self.default_ret_lo, self.default_ret_lo),
        }
    }
}

impl Rewrite for ast::RetType {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let ty = self.ty()?;
        let arrow_width = "-> ".len();
        if context.config.style_edition() <= StyleEdition::Edition2021 {
            let inner_width = shape.width.checked_sub(arrow_width)?;
            return ty
                .rewrite(
                    context,
                    Shape::legacy(inner_width, shape.indent + arrow_width),
                )
                .map(|r| format!("-> {r}"));
        }

        let shape = shape.offset_left(arrow_width)?;
        ty.rewrite(context, shape).map(|s| format!("-> {s}"))
    }
}

fn rewrite_ret(
    ret: Option<&ast::RetType>,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    match ret {
        Some(r) if r.ty().is_some() => r.rewrite(context, shape),
        _ => Some(String::new()),
    }
}

/// Recover any missing comments between the param and the type.
///
/// Returns the comment before the colon and the comment after the colon.
fn get_missing_param_comments(
    context: &RewriteContext<'_>,
    pat_span: Span,
    ty_span: Span,
    shape: Shape,
) -> (String, String) {
    let missing_comment_span = mk_sp(pat_span.hi(), ty_span.lo());

    let span_before_colon = {
        let missing_comment_span_hi = context
            .snippet_provider
            .span_before(missing_comment_span, ":");
        mk_sp(pat_span.hi(), missing_comment_span_hi)
    };
    let span_after_colon = {
        let missing_comment_span_lo = context
            .snippet_provider
            .span_after(missing_comment_span, ":");
        mk_sp(missing_comment_span_lo, ty_span.lo())
    };

    let comment_before_colon = rewrite_missing_comment(span_before_colon, shape, context)
        .filter(|comment| !comment.is_empty())
        .map_or(String::new(), |comment| format!(" {comment}"));
    let comment_after_colon = rewrite_missing_comment(span_after_colon, shape, context)
        .filter(|comment| !comment.is_empty())
        .map_or(String::new(), |comment| format!("{comment} "));
    (comment_before_colon, comment_after_colon)
}

/// The attributes of a parameter, the span between them and the parameter, and whether
/// they span several lines or include doc comments.
fn param_attrs(
    node: &SyntaxNode,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<(String, Span, bool, bool)> {
    let attrs = outer_attributes(node);
    let attrs_str = attrs.rewrite(context, Shape::legacy(shape.width, shape.indent))?;
    let lo = span_without_attrs(node).lo();
    Some(match attrs.last() {
        Some(last) => (
            attrs_str.clone(),
            mk_sp(last.span().hi(), lo),
            attrs_str.contains('\n'),
            attrs.iter().any(Attribute::is_doc_comment),
        ),
        None => (attrs_str, mk_sp(lo, lo), false, false),
    })
}

impl Rewrite for ast::Param {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let (param_attrs_result, span, has_multiple_attr_lines, has_doc_comments) =
            param_attrs(self.syntax(), context, shape)?;

        let Some(pat) = self.pat() else {
            // An unnamed parameter (fn pointer types, C variadics). rustfmt rewrites only the
            // type, which drops any attributes of the parameter.
            if self.dotdotdot_token().is_some() {
                return Some("...".to_owned());
            }
            return self.ty()?.rewrite(context, shape);
        };
        if self.dotdotdot_token().is_some() {
            // `name: ...`
            let pat_str = pat.rewrite(context, Shape::legacy(shape.width, shape.indent))?;
            let variadic = format!("{pat_str}{}...", colon_spaces(context.config));
            return combine_strs_with_missing_comments(
                context,
                &param_attrs_result,
                &variadic,
                span,
                shape,
                !has_multiple_attr_lines && !has_doc_comments,
            );
        }
        let param_name = &pat.rewrite(context, Shape::legacy(shape.width, shape.indent))?;
        let mut result = combine_strs_with_missing_comments(
            context,
            &param_attrs_result,
            param_name,
            span,
            shape,
            !has_multiple_attr_lines && !has_doc_comments,
        )?;

        // A closure parameter without a type annotation.
        let Some(ty) = self.ty() else {
            return Some(result);
        };
        let pat_span = pat.span();
        let ty_span = ty.span();
        let (before_comment, after_comment) =
            get_missing_param_comments(context, pat_span, ty_span, shape);
        result.push_str(&before_comment);
        result.push_str(colon_spaces(context.config));
        result.push_str(&after_comment);
        let overhead = last_line_width(&result);
        let max_width = shape.width.checked_sub(overhead)?;
        if let Some(ty_str) = ty.rewrite(context, Shape::legacy(max_width, shape.indent)) {
            result.push_str(&ty_str);
        } else {
            let prev_str = if param_attrs_result.is_empty() {
                param_attrs_result
            } else {
                param_attrs_result + &shape.to_string_with_newline(context.config)
            };

            result = combine_strs_with_missing_comments(
                context,
                &prev_str,
                param_name,
                span,
                shape,
                !has_multiple_attr_lines,
            )?;
            result.push_str(&before_comment);
            result.push_str(colon_spaces(context.config));
            result.push_str(&after_comment);
            let overhead = last_line_width(&result);
            let max_width = shape.width.checked_sub(overhead)?;
            let ty_str = ty.rewrite(context, Shape::legacy(max_width, shape.indent))?;
            result.push_str(&ty_str);
        }

        Some(result)
    }
}

impl Rewrite for ast::SelfParam {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let (param_attrs, span, has_multiple_attr_lines, _) =
            param_attrs(self.syntax(), context, shape)?;
        let mut_str = if self.mut_token().is_some() {
            "mut "
        } else {
            ""
        };
        let self_str = if self.amp_token().is_some() {
            let lifetime_str = match self.lifetime() {
                Some(l) => format!("{} ", l.syntax().text()),
                None => String::new(),
            };
            let is_pinned = self
                .syntax()
                .children_with_tokens()
                .any(|el| el.as_token().is_some_and(|t| t.text() == "pin"));
            if is_pinned {
                // `&pin mut self` / `&pin const self`
                let ptr = if self.mut_token().is_some() {
                    "mut"
                } else {
                    "const"
                };
                format!("&{lifetime_str}pin {ptr} self")
            } else {
                format!("&{lifetime_str}{mut_str}self")
            }
        } else if let Some(ty) = self.ty() {
            let type_str = ty.rewrite(
                context,
                Shape::legacy(context.config.max_width(), Indent::empty()),
            )?;
            format!("{mut_str}self: {type_str}")
        } else {
            format!("{mut_str}self")
        };
        combine_strs_with_missing_comments(
            context,
            &param_attrs,
            &self_str,
            span,
            shape,
            !has_multiple_attr_lines,
        )
    }
}

/// rustc's `span_lo_for_param`: the first attribute, else the pattern, else the type.
pub(crate) fn span_lo_for_param(param: &ast::Param) -> BytePos {
    match outer_attributes(param.syntax()).first() {
        Some(attr) => attr.span().lo(),
        None => span_without_attrs(param.syntax()).lo(),
    }
}

fn param_ty_hi(param: &ast::Param) -> BytePos {
    match (param.ty(), param.pat()) {
        (Some(ty), _) => ty.span().hi(),
        (None, Some(pat)) if param.dotdotdot_token().is_none() => pat.span().hi(),
        _ => node_range_span(param.syntax()).hi(),
    }
}

/// rustc's `span_hi_for_param`: the end of the type, or of the pattern for a closure
/// parameter without a type annotation.
pub(crate) fn span_hi_for_param(_context: &RewriteContext<'_>, param: &ast::Param) -> BytePos {
    param_ty_hi(param)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum FnBraceStyle {
    SameLine,
    NextLine,
    None,
}

/// Returns `(result, ends_with_comment, force_new_line_for_brace)`.
fn rewrite_fn_base(
    context: &RewriteContext<'_>,
    indent: Indent,
    name: &str,
    fn_sig: &FnSig,
    span: Span,
    fn_brace_style: FnBraceStyle,
) -> Option<(String, bool, bool)> {
    let mut force_new_line_for_brace = false;

    let where_clause = &fn_sig.generics.where_clause;

    let mut result = String::with_capacity(1024);
    result.push_str(&fn_sig.to_str(context));

    // fn foo
    result.push_str("fn ");

    // Generics.
    let overhead = if let FnBraceStyle::SameLine = fn_brace_style {
        // 4 = `() {`
        4
    } else {
        // 2 = `()`
        2
    };
    let used_width = last_line_used_width(&result, indent.width());
    let one_line_budget = context.budget(used_width + overhead);
    let shape = Shape {
        width: one_line_budget,
        indent,
        offset: used_width,
    };
    let generics_str = rewrite_generics(context, name, &fn_sig.generics, shape)?;
    result.push_str(&generics_str);

    // Note that the width and indent don't really matter, we'll re-layout the
    // return type later anyway.
    let ret_str = rewrite_ret(
        fn_sig.ret.as_ref(),
        context,
        Shape::indented(indent, context.config),
    )?;

    let multi_line_ret_str = ret_str.contains('\n');
    let ret_str_len = if multi_line_ret_str { 0 } else { ret_str.len() };

    // Params.
    let (one_line_budget, multi_line_budget, mut param_indent) = compute_budgets_for_params(
        context,
        &result,
        indent,
        ret_str_len,
        fn_brace_style,
        multi_line_ret_str,
    );

    result.push('(');

    let params_end = match fn_sig.params.last() {
        None => context
            .snippet_provider
            .span_after(mk_sp(fn_sig.generics.span.hi(), span.hi()), ")"),
        Some(last) => {
            let last_span = mk_sp(last.span().hi(), span.hi());
            context.snippet_provider.span_after(last_span, ")")
        }
    };
    let params_span = mk_sp(
        context
            .snippet_provider
            .span_after(mk_sp(fn_sig.generics.span.hi(), span.hi()), "("),
        params_end,
    );
    let param_str = rewrite_params(
        context,
        &fn_sig.params,
        one_line_budget,
        multi_line_budget,
        indent,
        param_indent,
        params_span,
        fn_sig.variadic,
    )?;

    let put_params_in_block = (param_str.contains('\n') || param_str.len() > one_line_budget)
        && !fn_sig.params.is_empty();

    let mut params_last_line_contains_comment = false;
    let mut no_params_and_over_max_width = false;

    if put_params_in_block {
        param_indent = indent.block_indent(context.config);
        result.push_str(&param_indent.to_string_with_newline(context.config));
        result.push_str(&param_str);
        result.push_str(&indent.to_string_with_newline(context.config));
        result.push(')');
    } else {
        result.push_str(&param_str);
        let used_width = last_line_used_width(&result, indent.width()) + first_line_width(&ret_str);
        // Put the closing brace on the next line if it overflows the max width.
        // 1 = `)`
        let closing_paren_overflow_max_width =
            fn_sig.params.is_empty() && used_width + 1 > context.config.max_width();
        // If the last line of params contains comment, we cannot put the closing paren
        // on the same line.
        params_last_line_contains_comment = param_str
            .lines()
            .last()
            .is_some_and(|last_line| last_line.contains("//"));

        if context.config.style_edition() >= StyleEdition::Edition2024 {
            if closing_paren_overflow_max_width {
                result.push(')');
                result.push_str(&indent.to_string_with_newline(context.config));
                no_params_and_over_max_width = true;
            } else if params_last_line_contains_comment {
                result.push_str(&indent.to_string_with_newline(context.config));
                result.push(')');
                no_params_and_over_max_width = true;
            } else {
                result.push(')');
            }
        } else {
            if closing_paren_overflow_max_width || params_last_line_contains_comment {
                result.push_str(&indent.to_string_with_newline(context.config));
            }
            result.push(')');
        }
    }

    // Return type.
    let has_ret = fn_sig.ret.as_ref().is_some_and(|r| r.ty().is_some());
    if has_ret {
        let ret_should_indent = if put_params_in_block || fn_sig.params.is_empty() {
            // If our params are block layout then we surely must have space.
            false
        } else if params_last_line_contains_comment {
            false
        } else if result.contains('\n') || multi_line_ret_str {
            true
        } else {
            // If the return type would push over the max width, then put the return type on
            // a new line. With the +1 for the signature length an additional space between
            // the closing parenthesis of the param and the arrow '->' is considered.
            let mut sig_length = result.len() + indent.width() + ret_str_len + 1;

            // If there is no where-clause, take into account the space after the return type
            // and the brace.
            if where_clause.predicates.is_empty() {
                sig_length += 2;
            }

            sig_length > context.config.max_width()
        };
        let ret_shape = if ret_should_indent {
            if context.config.style_edition() <= StyleEdition::Edition2021 {
                let indent = if param_str.is_empty() {
                    // Aligning with nonexistent params looks silly.
                    force_new_line_for_brace = true;
                    indent + 4
                } else {
                    param_indent
                };

                result.push_str(&indent.to_string_with_newline(context.config));
                Shape::indented(indent, context.config)
            } else {
                let mut ret_shape = Shape::indented(indent, context.config);
                if param_str.is_empty() {
                    // Aligning with nonexistent params looks silly.
                    force_new_line_for_brace = true;
                    ret_shape = ret_shape.offset_left(4).unwrap_or(ret_shape);
                }

                result.push_str(&ret_shape.indent.to_string_with_newline(context.config));
                ret_shape
            }
        } else {
            if context.config.style_edition() >= StyleEdition::Edition2024 {
                if !param_str.is_empty() || !no_params_and_over_max_width {
                    result.push(' ');
                }
            } else {
                result.push(' ');
            }

            let ret_shape = Shape::indented(indent, context.config);
            ret_shape
                .offset_left(last_line_width(&result))
                .unwrap_or(ret_shape)
        };

        if multi_line_ret_str || ret_should_indent {
            // Now that we know the proper indent and width, we need to
            // re-layout the return type.
            let ret_str = rewrite_ret(fn_sig.ret.as_ref(), context, ret_shape)?;
            result.push_str(&ret_str);
        } else {
            result.push_str(&ret_str);
        }

        // Comment between return type and the end of the decl.
        let snippet_lo = fn_sig.ret_span().hi();
        if where_clause.predicates.is_empty() {
            let snippet_hi = span.hi();
            let snippet = context.snippet(mk_sp(snippet_lo, snippet_hi));
            // Try to preserve the layout of the original snippet.
            let original_starts_with_newline = snippet
                .find(|c| c != ' ')
                .is_some_and(|i| starts_with_newline(&snippet[i..]));
            let original_ends_with_newline = snippet
                .rfind(|c| c != ' ')
                .is_some_and(|i| snippet[i..].ends_with('\n'));
            let snippet = snippet.trim();
            if !snippet.is_empty() {
                result.push(if original_starts_with_newline {
                    '\n'
                } else {
                    ' '
                });
                result.push_str(snippet);
                if original_ends_with_newline {
                    force_new_line_for_brace = true;
                }
            }
        }
    }

    let pos_before_where = if has_ret {
        fn_sig.ret_span().hi()
    } else {
        params_span.hi()
    };

    let is_params_multi_lined = param_str.contains('\n');

    let space = if put_params_in_block && ret_str.is_empty() {
        WhereClauseSpace::Space
    } else {
        WhereClauseSpace::Newline
    };
    let mut option = WhereClauseOption::new(fn_brace_style == FnBraceStyle::None, space);
    if is_params_multi_lined {
        option.veto_single_line();
    }
    let where_clause_str = rewrite_where_clause(
        context,
        where_clause,
        Shape::indented(indent, context.config),
        "{",
        Some(span.hi()),
        pos_before_where,
        option,
    )?;
    // If there are neither where-clause nor return type, we may be missing comments between
    // params and `{`.
    if where_clause_str.is_empty() && !has_ret {
        let ret_lo = fn_sig.ret_span().lo();
        // from after the closing paren to right before block or semicolon
        if let Some(missing_comment) = recover_missing_comment_in_span(
            mk_sp(ret_lo, span.hi()),
            shape,
            context,
            last_line_width(&result),
        ) && !missing_comment.is_empty()
        {
            result.push_str(&missing_comment);
            force_new_line_for_brace = true;
        }
    }

    result.push_str(&where_clause_str);

    let ends_with_comment = last_line_contains_single_line_comment(&result);
    force_new_line_for_brace |= ends_with_comment;
    force_new_line_for_brace |=
        is_params_multi_lined && context.config.where_single_line() && !where_clause_str.is_empty();
    Some((result, ends_with_comment, force_new_line_for_brace))
}

/// Kind of spaces to put before `where`.
#[derive(Copy, Clone)]
enum WhereClauseSpace {
    /// A single space.
    Space,
    /// A new line.
    Newline,
    /// Nothing.
    None,
}

#[derive(Copy, Clone)]
struct WhereClauseOption {
    suppress_comma: bool, // Force no trailing comma
    snuggle: WhereClauseSpace,
    allow_single_line: bool, // Try single line where-clause instead of vertical layout
    veto_single_line: bool,  // Disallow a single-line where-clause.
}

impl WhereClauseOption {
    fn new(suppress_comma: bool, snuggle: WhereClauseSpace) -> WhereClauseOption {
        WhereClauseOption {
            suppress_comma,
            snuggle,
            allow_single_line: false,
            veto_single_line: false,
        }
    }

    fn snuggled(current: &str) -> WhereClauseOption {
        WhereClauseOption {
            suppress_comma: false,
            snuggle: if last_line_width(current) == 1 {
                WhereClauseSpace::Space
            } else {
                WhereClauseSpace::Newline
            },
            allow_single_line: false,
            veto_single_line: false,
        }
    }

    fn suppress_comma(&mut self) {
        self.suppress_comma = true
    }

    fn allow_single_line(&mut self) {
        self.allow_single_line = true
    }

    fn snuggle(&mut self) {
        self.snuggle = WhereClauseSpace::Space
    }

    fn veto_single_line(&mut self) {
        self.veto_single_line = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_params(
    context: &RewriteContext<'_>,
    params: &[FnParam],
    one_line_budget: usize,
    multi_line_budget: usize,
    indent: Indent,
    param_indent: Indent,
    span: Span,
    variadic: bool,
) -> Option<String> {
    if params.is_empty() {
        let comment = context
            .snippet(mk_sp(
                span.lo(),
                // to remove ')'
                span.hi().saturating_sub(1),
            ))
            .trim();
        return Some(comment.to_owned());
    }
    let param_items: Vec<_> = itemize_list(
        context.snippet_provider,
        params.iter(),
        ")",
        ",",
        |param| param.span_lo(),
        |param| param.ty_hi(),
        |param| {
            param
                .rewrite(context, Shape::legacy(multi_line_budget, param_indent))
                .or_else(|| Some(context.snippet(param.span()).to_owned()))
        },
        span.lo(),
        span.hi(),
        false,
    )
    .collect();

    let tactic = definitive_tactic(
        &param_items,
        context
            .config
            .fn_params_layout()
            .to_list_tactic(param_items.len()),
        Separator::Comma,
        one_line_budget,
    );
    let budget = match tactic {
        DefinitiveListTactic::Horizontal => one_line_budget,
        _ => multi_line_budget,
    };
    let indent = indent.block_indent(context.config);
    let trailing_separator = if variadic {
        SeparatorTactic::Never
    } else {
        context.config.trailing_comma()
    };
    let fmt = ListFormatting::new(Shape::legacy(budget, indent), context.config)
        .tactic(tactic)
        .trailing_separator(trailing_separator)
        .ends_with_newline(tactic.ends_with_newline())
        .preserve_newline(true);
    write_list(&param_items, &fmt)
}

fn compute_budgets_for_params(
    context: &RewriteContext<'_>,
    result: &str,
    indent: Indent,
    ret_str_len: usize,
    fn_brace_style: FnBraceStyle,
    force_vertical_layout: bool,
) -> (usize, usize, Indent) {
    // Try keeping everything on the same line.
    if !result.contains('\n') && !force_vertical_layout {
        // 2 = `()`, 3 = `() `, space is before ret_string.
        let overhead = if ret_str_len == 0 { 2 } else { 3 };
        let mut used_space = indent.width() + result.len() + ret_str_len + overhead;
        match fn_brace_style {
            FnBraceStyle::None => used_space += 1,     // 1 = `;`
            FnBraceStyle::SameLine => used_space += 2, // 2 = `{}`
            FnBraceStyle::NextLine => (),
        }
        let one_line_budget = context.budget(used_space);

        if one_line_budget > 0 {
            let indent = indent.block_indent(context.config);
            let multi_line_budget = context.budget(indent.width() + 1);
            return (one_line_budget, multi_line_budget, indent);
        }
    }

    // Didn't work. we must force vertical layout and put params on a newline.
    let new_indent = indent.block_indent(context.config);
    // 1 = `,`
    let used_space = new_indent.width() + 1;
    (0, context.budget(used_space), new_indent)
}

fn newline_for_brace(context: &RewriteContext<'_>, where_clause: &WhereClause) -> FnBraceStyle {
    let predicate_count = where_clause.predicates.len();

    if context.config.where_single_line() && predicate_count == 1 {
        return FnBraceStyle::SameLine;
    }
    // `brace_style` is fixed at `SameLineWhere`.
    if predicate_count > 0 {
        FnBraceStyle::NextLine
    } else {
        FnBraceStyle::SameLine
    }
}

pub(crate) fn rewrite_generics(
    context: &RewriteContext<'_>,
    ident: &str,
    generics: &Generics,
    shape: Shape,
) -> Option<String> {
    if generics.params.is_empty() {
        return Some(ident.to_owned());
    }

    let params = generics
        .params
        .iter()
        .cloned()
        .map(OverflowableItem::GenericParam);
    overflow::rewrite_with_angle_brackets(context, ident, params, shape, generics.span)
}

#[allow(clippy::too_many_arguments)]
fn rewrite_where_clause_rfc_style(
    context: &RewriteContext<'_>,
    predicates: &[ast::WherePred],
    where_span: Span,
    shape: Shape,
    terminator: &str,
    span_end: Option<BytePos>,
    span_end_before_where: BytePos,
    where_clause_option: WhereClauseOption,
) -> Option<String> {
    let (where_keyword, allow_single_line) = rewrite_where_keyword(
        context,
        predicates,
        where_span,
        shape,
        span_end_before_where,
        where_clause_option,
    )?;

    // 1 = `,`
    let clause_shape = shape
        .block()
        .with_max_width(context.config)
        .block_left(context.config.tab_spaces())?
        .sub_width(1)?;
    let force_single_line = context.config.where_single_line()
        && predicates.len() == 1
        && !where_clause_option.veto_single_line;

    let preds_str = rewrite_bounds_on_where_clause(
        context,
        predicates,
        clause_shape,
        terminator,
        span_end,
        where_clause_option,
        force_single_line,
    )?;

    // 6 = `where `
    let clause_sep =
        if allow_single_line && !preds_str.contains('\n') && 6 + preds_str.len() <= shape.width
            || force_single_line
        {
            Cow::from(" ")
        } else {
            clause_shape.indent.to_string_with_newline(context.config)
        };

    Some(format!("{where_keyword}{clause_sep}{preds_str}"))
}

/// Rewrite `where` and comment around it.
fn rewrite_where_keyword(
    context: &RewriteContext<'_>,
    predicates: &[ast::WherePred],
    where_span: Span,
    shape: Shape,
    span_end_before_where: BytePos,
    where_clause_option: WhereClauseOption,
) -> Option<(String, bool)> {
    let block_shape = shape.block().with_max_width(context.config);
    // 1 = `,`
    let clause_shape = block_shape
        .block_left(context.config.tab_spaces())?
        .sub_width(1)?;

    let comment_separator = |comment: &str, shape: Shape| {
        if comment.is_empty() {
            Cow::from("")
        } else {
            shape.indent.to_string_with_newline(context.config)
        }
    };

    let (span_before, span_after) =
        missing_span_before_after_where(span_end_before_where, predicates, where_span);
    let (comment_before, comment_after) =
        rewrite_comments_before_after_where(context, span_before, span_after, shape)?;

    let starting_newline = match where_clause_option.snuggle {
        WhereClauseSpace::Space if comment_before.is_empty() => Cow::from(" "),
        WhereClauseSpace::None => Cow::from(""),
        _ => block_shape.indent.to_string_with_newline(context.config),
    };

    let newline_before_where = comment_separator(&comment_before, shape);
    let newline_after_where = comment_separator(&comment_after, clause_shape);
    let result = format!(
        "{starting_newline}{comment_before}{newline_before_where}where\
{newline_after_where}{comment_after}"
    );
    let allow_single_line = where_clause_option.allow_single_line
        && comment_before.is_empty()
        && comment_after.is_empty();

    Some((result, allow_single_line))
}

/// Rewrite bounds on a where clause.
fn rewrite_bounds_on_where_clause(
    context: &RewriteContext<'_>,
    predicates: &[ast::WherePred],
    shape: Shape,
    terminator: &str,
    span_end: Option<BytePos>,
    where_clause_option: WhereClauseOption,
    force_single_line: bool,
) -> Option<String> {
    let span_start = predicates[0].span().lo();
    // If we don't have the start of the next span, then use the end of the
    // predicates, but that means we miss comments.
    let len = predicates.len();
    let end_of_preds = predicates[len - 1].span().hi();
    let span_end = span_end.unwrap_or(end_of_preds);
    let items = itemize_list(
        context.snippet_provider,
        predicates.iter(),
        terminator,
        ",",
        |pred| pred.span().lo(),
        |pred| pred.span().hi(),
        |pred| pred.rewrite(context, shape),
        span_start,
        span_end,
        false,
    );
    let comma_tactic = if where_clause_option.suppress_comma || force_single_line {
        SeparatorTactic::Never
    } else {
        context.config.trailing_comma()
    };

    // shape should be vertical only and only if we have `force_single_line` option enabled
    // and the number of items of the where-clause is equal to 1
    let shape_tactic = if force_single_line {
        DefinitiveListTactic::Horizontal
    } else {
        DefinitiveListTactic::Vertical
    };

    let preserve_newline = context.config.style_edition() <= StyleEdition::Edition2021;

    let fmt = ListFormatting::new(shape, context.config)
        .tactic(shape_tactic)
        .trailing_separator(comma_tactic)
        .preserve_newline(preserve_newline);
    write_list(&items.collect::<Vec<_>>(), &fmt)
}

/// `indent_style` is fixed at `Block`, so this is rustfmt's RFC-style where clause.
fn rewrite_where_clause(
    context: &RewriteContext<'_>,
    where_clause: &WhereClause,
    shape: Shape,
    terminator: &str,
    span_end: Option<BytePos>,
    span_end_before_where: BytePos,
    where_clause_option: WhereClauseOption,
) -> Option<String> {
    if where_clause.predicates.is_empty() {
        return Some(String::new());
    }

    rewrite_where_clause_rfc_style(
        context,
        &where_clause.predicates,
        where_clause.span,
        shape,
        terminator,
        span_end,
        span_end_before_where,
        where_clause_option,
    )
}

fn missing_span_before_after_where(
    before_item_span_end: BytePos,
    predicates: &[ast::WherePred],
    where_span: Span,
) -> (Span, Span) {
    let missing_span_before = mk_sp(before_item_span_end, where_span.lo());
    // 5 = `where`
    let pos_after_where = where_span.lo() + 5;
    let missing_span_after = mk_sp(pos_after_where, predicates[0].span().lo());
    (missing_span_before, missing_span_after)
}

fn rewrite_comments_before_after_where(
    context: &RewriteContext<'_>,
    span_before_where: Span,
    span_after_where: Span,
    shape: Shape,
) -> Option<(String, String)> {
    let before_comment = rewrite_missing_comment(span_before_where, shape, context)?;
    let after_comment = rewrite_missing_comment(
        span_after_where,
        shape.block_indent(context.config.tab_spaces()),
        context,
    )?;
    Some((before_comment, after_comment))
}

/// `vis item_name ident`, keeping any comment between the visibility and the item name.
fn format_header(
    context: &RewriteContext<'_>,
    item_name: &str,
    name: &ast::Name,
    vis: Option<&ast::Visibility>,
    item_lo: BytePos,
    offset: Indent,
) -> String {
    let mut result = String::with_capacity(128);
    let shape = Shape::indented(offset, context.config);

    result.push_str(format_visibility(vis).trim());

    // Check for a missing comment between the visibility and the item name.
    let (vis_lo, after_vis) = vis_lo_hi(vis, item_lo);
    if let Some(before_item_name) = context
        .snippet_provider
        .opt_span_before(mk_sp(vis_lo, name_span(name).hi()), item_name.trim())
    {
        let missing_span = mk_sp(after_vis, before_item_name);
        if let Some(result_with_comment) = combine_strs_with_missing_comments(
            context,
            &result,
            item_name,
            missing_span,
            shape,
            /* allow_extend */ true,
        ) {
            result = result_with_comment;
        }
    }

    result.push_str(&node_text(name.syntax()));

    result
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum BracePos {
    None,
    Auto,
    ForceSameLine,
}

/// `brace_style` is fixed at `SameLineWhere`.
fn format_generics(
    context: &RewriteContext<'_>,
    generics: &Generics,
    brace_pos: BracePos,
    offset: Indent,
    span: Span,
    used_width: usize,
) -> Option<String> {
    let shape = Shape::legacy(context.budget(used_width + offset.width()), offset);
    let mut result = rewrite_generics(context, "", generics, shape)?;

    // If the generics are not parameterized then generics.span.hi() == 0,
    // so we use span.lo(), which is the position after `struct Foo`.
    let span_end_before_where = if !generics.params.is_empty() {
        generics.span.hi()
    } else {
        span.lo()
    };
    let (same_line_brace, missed_comments) = if !generics.where_clause.predicates.is_empty() {
        let budget = context.budget(last_line_used_width(&result, offset.width()));
        let mut option = WhereClauseOption::snuggled(&result);
        if brace_pos == BracePos::None {
            option.suppress_comma = true;
        }
        let where_clause_str = rewrite_where_clause(
            context,
            &generics.where_clause,
            Shape::legacy(budget, offset.block_only()),
            "{",
            Some(span.hi()),
            span_end_before_where,
            option,
        )?;
        result.push_str(&where_clause_str);
        (
            brace_pos == BracePos::ForceSameLine,
            // missed comments are taken care of in #rewrite_where_clause
            None,
        )
    } else {
        (
            // `brace_style != AlwaysNextLine` always holds.
            true,
            rewrite_missing_comment(
                mk_sp(
                    span_end_before_where,
                    if brace_pos == BracePos::None {
                        span.hi()
                    } else {
                        context.snippet_provider.span_before_last(span, "{")
                    },
                ),
                shape,
                context,
            ),
        )
    };
    // add missing comments
    let missed_line_comments = missed_comments
        .filter(|missed_comments| !missed_comments.is_empty())
        .is_some_and(|missed_comments| {
            let is_block = is_last_comment_block(&missed_comments);
            let sep = if is_block { " " } else { "\n" };
            result.push_str(sep);
            result.push_str(&missed_comments);
            !is_block
        });
    if brace_pos == BracePos::None {
        return Some(result);
    }
    let total_used_width = last_line_used_width(&result, used_width);
    let remaining_budget = context.budget(total_used_width);
    // If the same line brace if forced, it indicates that we are rewriting an item with empty body,
    // and hence we take the closer into account as well for one line budget.
    // We assume that the closer has the same length as the opener.
    let overhead = if brace_pos == BracePos::ForceSameLine {
        // 3 = ` {}`
        3
    } else {
        // 2 = ` {`
        2
    };
    let forbid_same_line_brace = missed_line_comments || overhead > remaining_budget;
    if !forbid_same_line_brace && same_line_brace {
        result.push(' ');
    } else {
        result.push('\n');
        result.push_str(&offset.block_only().to_string(context.config));
    }
    result.push('{');

    Some(result)
}

// ---------------------------------------------------------------------------------------
// Visitor entry points
// ---------------------------------------------------------------------------------------

impl FmtVisitor<'_> {
    /// `extern "abi" { .. }` (rustc's foreign module).
    pub(crate) fn format_foreign_mod(&mut self, fm: &ast::ExternBlock, span: Span) {
        if fm.unsafe_token().is_some() {
            self.buffer.push_str("unsafe ");
        }
        let abi = format_abi(fm.abi().as_ref(), &self.get_context());
        let abi = if abi.is_empty() {
            // rustc's `Extern::from_abi(None)` is an implicit ABI.
            "extern \"C\" ".to_owned()
        } else {
            abi
        };
        self.buffer.push_str(&abi);

        let snippet = self.snippet(span);
        let Some(brace_pos) = snippet.find_uncommented("{") else {
            self.push_str(snippet);
            self.last_pos = span.hi();
            return;
        };

        let items: Vec<ast::ExternItem> = fm
            .extern_item_list()
            .map(|l| l.extern_items().collect())
            .unwrap_or_default();

        self.push_str("{");
        if !items.is_empty() || contains_comment(&snippet[brace_pos..]) {
            // FIXME: this skips comments between the extern keyword and the opening
            // brace.
            self.last_pos = span.lo() + brace_pos as BytePos + 1;
            self.block_indent = self.block_indent.block_indent(self.config);

            // rustfmt does not format the inner attributes of an `extern` block: they are
            // copied with the text before the first item.
            for item in &items {
                self.format_foreign_item(item);
            }

            self.format_missing_no_indent(span.hi() - 1);
            self.block_indent = self.block_indent.block_unindent(self.config);
            let indent_str = self.block_indent.to_string(self.config);
            self.push_str(&indent_str);
        }

        self.push_str("}");
        self.last_pos = span.hi();
    }

    fn format_foreign_item(&mut self, item: &ast::ExternItem) {
        let rewrite = item.rewrite(&self.get_context(), self.shape());
        let span = rustc_span(item.syntax());
        self.push_rewrite(span, rewrite);
        self.last_pos = span.hi();
    }

    pub(crate) fn rewrite_fn_before_block(
        &mut self,
        indent: Indent,
        name: &str,
        fn_sig: &FnSig,
        span: Span,
    ) -> Option<(String, FnBraceStyle)> {
        let context = self.get_context();

        let mut fn_brace_style = newline_for_brace(&context, &fn_sig.generics.where_clause);
        let (result, _, force_newline_brace) =
            rewrite_fn_base(&context, indent, name, fn_sig, span, fn_brace_style)?;

        // 2 = ` {`
        if force_newline_brace || last_line_width(&result) + 2 > self.shape().width {
            fn_brace_style = FnBraceStyle::NextLine
        }

        Some((result, fn_brace_style))
    }

    pub(crate) fn rewrite_required_fn(
        &mut self,
        indent: Indent,
        f: &ast::Fn,
        span: Span,
    ) -> Option<String> {
        // Drop semicolon or it will be interpreted as comment.
        let span = mk_sp(span.lo(), span.hi().saturating_sub(1));
        let context = self.get_context();

        let (mut result, ends_with_comment, _) = rewrite_fn_base(
            &context,
            indent,
            &name_text(f.name())?,
            &FnSig::from_fn(f)?,
            span,
            FnBraceStyle::None,
        )?;

        // If `result` ends with a comment, then remember to add a newline
        if ends_with_comment {
            result.push_str(&indent.to_string_with_newline(context.config));
        }

        // Re-attach semicolon
        result.push(';');

        Some(result)
    }

    pub(crate) fn single_line_fn(
        &self,
        fn_str: &str,
        block: &Block,
        inner_attrs: &[Attribute],
    ) -> Option<String> {
        if fn_str.contains('\n') || !inner_attrs.is_empty() {
            return None;
        }

        let context = self.get_context();

        if self.config.empty_item_single_line()
            && is_empty_block(&context, block, None)
            && self.block_indent.width() + fn_str.len() + 3 <= self.config.max_width()
            && !last_line_contains_single_line_comment(fn_str)
        {
            return Some(format!("{fn_str} {{}}"));
        }

        if !self.config.fn_single_line() || !is_simple_block_stmt(&context, block, None) {
            return None;
        }

        let res = Stmt::from_ast_node(block.stmts.first()?, true)
            .rewrite(&self.get_context(), self.shape())?;

        let width = self.block_indent.width() + fn_str.len() + res.len() + 5;
        if !res.contains('\n') && width <= self.config.max_width() {
            Some(format!("{fn_str} {{ {res} }}"))
        } else {
            None
        }
    }

    /// A fn with a body (rustfmt's `visit_fn`).
    pub(crate) fn visit_fn(&mut self, f: &ast::Fn, s: Span) {
        let (Some(body), Some(sig), Some(name)) =
            (f.body(), FnSig::from_fn(f), name_text(f.name()))
        else {
            self.push_rewrite(s, None);
            return;
        };
        let Some(block) = Block::from_block_expr(&body) else {
            self.push_rewrite(s, None);
            return;
        };
        let inner_attrs = block.inner_attrs();
        let indent = self.block_indent;
        let rewrite =
            self.rewrite_fn_before_block(indent, &name, &sig, mk_sp(s.lo(), block.span.lo()));

        if let Some((fn_str, fn_brace_style)) = rewrite {
            self.format_missing_with_indent(s.lo());

            if let Some(rw) = self.single_line_fn(&fn_str, &block, &inner_attrs) {
                self.push_str(&rw);
                self.last_pos = s.hi();
                return;
            }

            self.push_str(&fn_str);
            match fn_brace_style {
                FnBraceStyle::NextLine => {
                    self.push_str(&self.block_indent.to_string_with_newline(self.config))
                }
                _ => self.push_str(" "),
            }
            self.last_pos = block.span.lo();
        } else {
            self.format_missing(block.span.lo());
        }

        self.visit_block(&block, Some(&inner_attrs), true)
    }

    pub(crate) fn visit_static(&mut self, static_parts: &StaticParts) {
        let rewrite = rewrite_static(&self.get_context(), static_parts, self.block_indent);
        self.push_rewrite(static_parts.span, rewrite);
    }

    pub(crate) fn visit_struct(&mut self, struct_parts: &StructParts) {
        let is_tuple = matches!(struct_parts.def, VariantData::Tuple(..));
        let rewrite = format_struct(&self.get_context(), struct_parts, self.block_indent, None)
            .map(|s| if is_tuple { s + ";" } else { s });
        self.push_rewrite(struct_parts.span, rewrite);
    }

    pub(crate) fn visit_enum(&mut self, e: &ast::Enum, span: Span) {
        let Some(name) = e.name() else {
            self.push_rewrite(span, None);
            return;
        };
        let variants: Vec<ast::Variant> = e
            .variant_list()
            .map(|l| l.variants().collect())
            .unwrap_or_default();
        let enum_snippet = self.snippet(span);
        let Some(brace_pos) = enum_snippet.find_uncommented("{") else {
            self.push_rewrite(span, None);
            return;
        };
        let body_start = span.lo() + brace_pos as BytePos + 1;

        let context = self.get_context();
        let enum_header = format_header(
            &context,
            "enum ",
            &name,
            e.visibility().as_ref(),
            span.lo(),
            self.block_indent,
        );
        let name_hi = name_span(&name).hi();
        let params_hi = e
            .generic_param_list()
            .map_or(name_hi, |l| node_range_span(l.syntax()).hi());
        let generics = Generics::new(e.generic_param_list(), name_hi, e.where_clause(), params_hi);
        let generics_str = format_generics(
            &context,
            &generics,
            if variants.is_empty() {
                BracePos::ForceSameLine
            } else {
                BracePos::Auto
            },
            self.block_indent,
            // make a span that starts right after `enum Foo`
            mk_sp(name_hi, body_start),
            last_line_width(&enum_header),
        );
        let Some(generics_str) = generics_str else {
            self.push_rewrite(span, None);
            return;
        };
        self.push_str(&enum_header);
        self.push_str(&generics_str);

        self.last_pos = body_start;

        match self.format_variant_list(&variants, body_start, span.hi()) {
            Some(ref s) if variants.is_empty() => self.push_str(s),
            rw => {
                self.push_rewrite(mk_sp(body_start, span.hi()), rw);
                self.block_indent = self.block_indent.block_unindent(self.config);
            }
        }
    }

    // Format the body of an enum definition
    fn format_variant_list(
        &mut self,
        variants: &[ast::Variant],
        body_lo: BytePos,
        body_hi: BytePos,
    ) -> Option<String> {
        if variants.is_empty() {
            let mut buffer = String::with_capacity(128);
            // 1 = "}"
            let span = mk_sp(body_lo, body_hi.saturating_sub(1));
            format_empty_struct_or_tuple(
                &self.get_context(),
                span,
                self.block_indent,
                &mut buffer,
                "",
                "}",
            );
            return Some(buffer);
        }
        let mut result = String::with_capacity(1024);
        let original_offset = self.block_indent;
        self.block_indent = self.block_indent.block_indent(self.config);

        // Discriminants are not aligned with `enum_discrim_align_threshold = 0`.
        debug_assert_eq!(self.config.enum_discrim_align_threshold(), 0);
        let itemize_list_with = |one_line_width: usize| {
            itemize_list(
                self.snippet_provider,
                variants.iter(),
                "}",
                ",",
                |f| f.span().lo(),
                |f| f.span().hi(),
                |f| self.format_variant(f, one_line_width),
                body_lo,
                body_hi,
                false,
            )
            .collect()
        };
        let mut items: Vec<_> = itemize_list_with(self.config.struct_variant_width());

        // If one of the variants use multiple lines, use multi-lined formatting for all variants.
        let has_multiline_variant = items.iter().any(|item| item.inner_as_ref().contains('\n'));
        let has_single_line_variant = items.iter().any(|item| !item.inner_as_ref().contains('\n'));
        if has_multiline_variant && has_single_line_variant {
            items = itemize_list_with(0);
        }

        let shape = self.shape().sub_width(2)?;
        let fmt = ListFormatting::new(shape, self.config)
            .trailing_separator(self.config.trailing_comma())
            .preserve_newline(true);

        let list = write_list(&items, &fmt)?;
        result.push_str(&list);
        result.push_str(&original_offset.to_string_with_newline(self.config));
        result.push('}');
        Some(result)
    }

    // Variant of an enum.
    fn format_variant(&self, field: &ast::Variant, one_line_width: usize) -> Option<String> {
        let attrs = outer_attributes(field.syntax());
        let field_span = item_span(field.syntax());
        if contains_skip(&attrs) {
            let lo = attrs[0].span().lo();
            let span = mk_sp(lo, field_span.hi());
            return Some(self.snippet(span).to_owned());
        }

        let context = self.get_context();
        let shape = self.shape();
        let attrs_str = if context.config.style_edition() >= StyleEdition::Edition2024 {
            attrs.rewrite(&context, shape)?
        } else {
            // StyleEdition::Edition20{15|18|21} formatting that was off by 1. See issue #5801
            attrs.rewrite(&context, shape.sub_width(1)?)?
        };
        // sub_width(1) to take the trailing comma into account
        let shape = shape.sub_width(1)?;

        let lo = attrs
            .last()
            .map_or(field_span.lo(), |attr| attr.span().hi());
        let span = mk_sp(lo, field_span.lo());

        let variant_body = match field.field_list() {
            Some(_) => format_struct(
                &context,
                &StructParts::from_variant(field, &context)?,
                self.block_indent,
                Some(one_line_width),
            )?,
            None => node_text(field.name()?.syntax()),
        };

        let variant_body = if let Some(expr) = field.expr() {
            let lhs = format!("{variant_body} =");
            rewrite_assign_rhs_with(
                &context,
                lhs,
                &expr,
                shape,
                &RhsAssignKind::Expr,
                RhsTactics::AllowOverflow,
            )?
        } else {
            variant_body
        };

        combine_strs_with_missing_comments(&context, &attrs_str, &variant_body, span, shape, false)
    }

    /// `reorder_impl_items` is fixed at `false`: items keep their order.
    fn visit_impl_items(&mut self, items: &[ast::AssocItem]) {
        debug_assert!(!self.config.reorder_impl_items());
        for item in items {
            self.visit_assoc_item(item);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Impls and traits
// ---------------------------------------------------------------------------------------

fn assoc_items(list: Option<ast::AssocItemList>) -> Vec<ast::AssocItem> {
    list.map(|l| l.assoc_items().collect()).unwrap_or_default()
}

pub(crate) fn format_impl(
    context: &RewriteContext<'_>,
    iimpl: &ast::Impl,
    offset: Indent,
) -> Option<String> {
    let item_span = item_span(iimpl.syntax());
    let self_ty = iimpl.self_ty()?;
    let self_ty_hi = self_ty.span().hi();
    let generics = Generics::new(
        iimpl.generic_param_list(),
        iimpl
            .impl_token()
            .map_or(item_span.lo(), |t| token_span(&t).hi()),
        iimpl.where_clause(),
        self_ty_hi,
    );
    let items = assoc_items(iimpl.assoc_item_list());

    let mut result = String::with_capacity(128);
    let ref_and_type = format_impl_ref_and_type(context, iimpl, &generics, &self_ty, offset)?;
    let sep = offset.to_string_with_newline(context.config);
    result.push_str(&ref_and_type);

    let where_budget = if result.contains('\n') {
        context.config.max_width()
    } else {
        context.budget(last_line_width(&result))
    };

    let mut option = WhereClauseOption::snuggled(&ref_and_type);
    let snippet = context.snippet(item_span);
    let open_pos = snippet.find_uncommented("{")? + 1;
    if !contains_comment(&snippet[open_pos..])
        && items.is_empty()
        && generics.where_clause.predicates.len() == 1
        && !result.contains('\n')
    {
        option.suppress_comma();
        option.snuggle();
        option.allow_single_line();
    }

    let missing_span = mk_sp(self_ty_hi, item_span.hi());
    let where_span_end = context.snippet_provider.opt_span_before(missing_span, "{");
    let where_clause_str = rewrite_where_clause(
        context,
        &generics.where_clause,
        Shape::legacy(where_budget, offset.block_only()),
        "{",
        where_span_end,
        self_ty_hi,
        option,
    )?;

    // If there is no where-clause, we may have missing comments between the trait name and
    // the opening brace.
    if generics.where_clause.predicates.is_empty()
        && let Some(hi) = where_span_end
        && let Some(missing_comment) = recover_missing_comment_in_span(
            mk_sp(self_ty_hi, hi),
            Shape::indented(offset, context.config),
            context,
            last_line_width(&result),
        )
        && !missing_comment.is_empty()
    {
        result.push_str(&missing_comment);
    }

    if is_impl_single_line(context, &items, &result, &where_clause_str, item_span)? {
        result.push_str(&where_clause_str);
        if where_clause_str.contains('\n') {
            // If there is only one where-clause predicate
            // and the where-clause spans multiple lines,
            // then recover the suppressed comma in single line where-clause formatting
            if generics.where_clause.predicates.len() == 1 {
                result.push(',');
            }
        }
        if where_clause_str.contains('\n') || last_line_contains_single_line_comment(&result) {
            result.push_str(&format!("{sep}{{{sep}}}"));
        } else {
            result.push_str(" {}");
        }
        return Some(result);
    }

    result.push_str(&where_clause_str);

    let need_newline = last_line_contains_single_line_comment(&result) || result.contains('\n');
    // `brace_style` is fixed at `SameLineWhere`.
    if need_newline || !where_clause_str.is_empty() {
        result.push_str(&sep);
    } else {
        result.push(' ');
    }

    result.push('{');
    // this is an impl body snippet(impl SampleImpl { /* here */ })
    let lo = max(self_ty_hi, generics.where_clause.span.hi());
    let snippet = context.snippet(mk_sp(lo, item_span.hi()));
    let open_pos = snippet.find_uncommented("{")? + 1;

    if !items.is_empty() || contains_comment(&snippet[open_pos..]) {
        let mut visitor = FmtVisitor::from_context(context);
        let item_indent = offset.block_only().block_indent(context.config);
        visitor.block_indent = item_indent;
        visitor.last_pos = lo + open_pos as BytePos;

        if let Some(list) = iimpl.assoc_item_list() {
            visitor.visit_attrs(
                &inner_attributes(list.syntax()),
                super::nodes::AttrStyle::Inner,
            );
        }
        visitor.visit_impl_items(&items);

        visitor.format_missing(item_span.hi() - 1);

        let inner_indent_str = visitor.block_indent.to_string_with_newline(context.config);
        let outer_indent_str = offset.block_only().to_string_with_newline(context.config);

        result.push_str(&inner_indent_str);
        result.push_str(visitor.buffer.trim());
        result.push_str(&outer_indent_str);
    } else if need_newline || !context.config.empty_item_single_line() {
        result.push_str(&sep);
    }

    result.push('}');

    Some(result)
}

fn is_impl_single_line(
    context: &RewriteContext<'_>,
    items: &[ast::AssocItem],
    result: &str,
    where_clause_str: &str,
    item_span: Span,
) -> Option<bool> {
    let snippet = context.snippet(item_span);
    let open_pos = snippet.find_uncommented("{")? + 1;

    Some(
        context.config.empty_item_single_line()
            && items.is_empty()
            && !result.contains('\n')
            && result.len() + where_clause_str.len() <= context.config.max_width()
            && !contains_comment(&snippet[open_pos..]),
    )
}

fn format_impl_ref_and_type(
    context: &RewriteContext<'_>,
    iimpl: &ast::Impl,
    generics: &Generics,
    self_ty: &ast::Type,
    offset: Indent,
) -> Option<String> {
    let mut result = String::with_capacity(128);

    result.push_str(&format_visibility(iimpl.visibility().as_ref()));

    let of_trait = iimpl.trait_();
    let constness = iimpl.const_token().is_some();
    if of_trait.is_some() {
        if iimpl.default_token().is_some() {
            result.push_str("default ");
        }
        if iimpl.unsafe_token().is_some() {
            result.push_str("unsafe ");
        }
    } else {
        if iimpl.unsafe_token().is_some() {
            // rustc rejects `unsafe` inherent impls; keep it rather than drop code.
            result.push_str("unsafe ");
        }
        if constness {
            result.push_str("const ");
        }
    }

    let shape = if context.config.style_edition() >= StyleEdition::Edition2024 {
        Shape::indented(offset + last_line_width(&result), context.config)
    } else {
        generics_shape_from_config(
            context,
            Shape::indented(offset + last_line_width(&result), context.config),
        )?
    };
    let generics_str = rewrite_generics(context, "impl", generics, shape)?;
    result.push_str(&generics_str);

    let trait_ref_overhead;
    if let Some(of_trait) = &of_trait {
        if constness {
            result.push_str(" const");
        }
        let polarity_str = if iimpl.excl_token().is_some() {
            "!"
        } else {
            ""
        };
        let result_len = last_line_width(&result);
        result.push_str(&rewrite_trait_ref(
            context,
            of_trait,
            offset,
            polarity_str,
            result_len,
        )?);
        trait_ref_overhead = " for".len();
    } else {
        trait_ref_overhead = 0;
    }

    // Try to put the self type in a single line.
    let curly_brace_overhead = if generics.where_clause.predicates.is_empty() {
        // If there is no where-clause adapt budget for type formatting to take space and curly
        // brace into account.
        2
    } else {
        0
    };
    let used_space = last_line_width(&result) + trait_ref_overhead + curly_brace_overhead;
    // 1 = space before the type.
    let budget = context.budget(used_space + 1);
    if let Some(self_ty_str) = self_ty.rewrite(context, Shape::legacy(budget, offset))
        && !self_ty_str.contains('\n')
    {
        if of_trait.is_some() {
            result.push_str(" for ");
        } else {
            result.push(' ');
        }
        result.push_str(&self_ty_str);
        return Some(result);
    }

    // Couldn't fit the self type on a single line, put it on a new line.
    result.push('\n');
    // Add indentation of one additional tab.
    let new_line_offset = offset.block_indent(context.config);
    result.push_str(&new_line_offset.to_string(context.config));
    if of_trait.is_some() {
        result.push_str("for ");
    }
    let budget = context.budget(last_line_width(&result));
    let type_offset = new_line_offset;
    result.push_str(&self_ty.rewrite(context, Shape::legacy(budget, type_offset))?);
    Some(result)
}

fn rewrite_trait_ref(
    context: &RewriteContext<'_>,
    trait_ref: &ast::Type,
    offset: Indent,
    polarity_str: &str,
    result_len: usize,
) -> Option<String> {
    // 1 = space between generics and trait_ref
    let used_space = 1 + polarity_str.len() + result_len;
    let shape = Shape::indented(offset + used_space, context.config);
    if let Some(trait_ref_str) = trait_ref.rewrite(context, shape)
        && !trait_ref_str.contains('\n')
    {
        return Some(format!(" {polarity_str}{trait_ref_str}"));
    }
    // We could not make enough space for trait_ref, so put it on new line.
    let offset = offset.block_indent(context.config);
    let shape = Shape::indented(offset, context.config);
    let trait_ref_str = trait_ref.rewrite(context, shape)?;
    Some(format!(
        "{}{}{}",
        offset.to_string_with_newline(context.config),
        polarity_str,
        trait_ref_str
    ))
}

fn generics_shape_from_config(context: &RewriteContext<'_>, shape: Shape) -> Option<Shape> {
    // 1 = ","
    shape
        .block()
        .block_indent(context.config.tab_spaces())
        .with_max_width(context.config)
        .sub_width(1)
}

/// The type bounds of a trait or type alias (`: A + B`).
fn type_bounds(node: &SyntaxNode) -> Vec<ast::TypeBound> {
    node.children()
        .find_map(ast::TypeBoundList::cast)
        .map(|l| l.bounds().collect())
        .unwrap_or_default()
}

pub(crate) fn format_trait(
    context: &RewriteContext<'_>,
    trait_: &ast::Trait,
    offset: Indent,
) -> Option<String> {
    let item_span = item_span(trait_.syntax());
    let name = trait_.name()?;
    let name_hi = name_span(&name).hi();
    let bounds = type_bounds(trait_.syntax());
    let params_hi = trait_
        .generic_param_list()
        .map_or(name_hi, |l| node_range_span(l.syntax()).hi());
    let where_fallback = bounds.last().map_or(params_hi, |b| b.span().hi());
    let generics = Generics::new(
        trait_.generic_param_list(),
        name_hi,
        trait_.where_clause(),
        where_fallback,
    );
    let items = assoc_items(trait_.assoc_item_list());

    let mut result = String::with_capacity(128);
    let header = format!(
        "{}{}{}{}trait ",
        format_visibility(trait_.visibility().as_ref()),
        if has_token(trait_.syntax(), T![const]) {
            "const "
        } else {
            ""
        },
        if trait_.unsafe_token().is_some() {
            "unsafe "
        } else {
            ""
        },
        if trait_.auto_token().is_some() {
            "auto "
        } else {
            ""
        },
    );
    result.push_str(&header);

    let body_lo = context.snippet_provider.span_after(item_span, "{");

    let shape = Shape::indented(offset, context.config).offset_left(result.len())?;
    let generics_str = rewrite_generics(context, &node_text(name.syntax()), &generics, shape)?;
    result.push_str(&generics_str);

    // FIXME(#2055): rustfmt fails to format when there are comments between trait bounds.
    if !bounds.is_empty() {
        let bound_hi = bounds.last()?.span().hi();
        let snippet = context.snippet(mk_sp(name_hi, bound_hi));
        if contains_comment(snippet) {
            return None;
        }

        result = rewrite_assign_rhs_with(
            context,
            result + ":",
            &Bounds(bounds.clone()),
            shape,
            &RhsAssignKind::Bounds,
            RhsTactics::ForceNextLineWithoutIndent,
        )?;
    }

    // Rewrite where-clause.
    if !generics.where_clause.predicates.is_empty() {
        let where_budget = context.budget(last_line_width(&result));
        let pos_before_where = if bounds.is_empty() {
            generics.where_clause.span.lo()
        } else {
            bounds.last()?.span().hi()
        };
        let option = WhereClauseOption::snuggled(&generics_str);
        let where_clause_str = rewrite_where_clause(
            context,
            &generics.where_clause,
            Shape::legacy(where_budget, offset.block_only()),
            "{",
            None,
            pos_before_where,
            option,
        )?;

        // If the where-clause cannot fit on the same line,
        // put the where-clause on a new line
        if !where_clause_str.contains('\n')
            && last_line_width(&result) + where_clause_str.len() + offset.width()
                > context.config.comment_width()
        {
            let width = offset.block_indent + context.config.tab_spaces() - 1;
            let where_indent = Indent::new(0, width);
            result.push_str(&where_indent.to_string_with_newline(context.config));
        }
        result.push_str(&where_clause_str);
    } else {
        let item_snippet = context.snippet(item_span);
        if let Some(lo) = item_snippet.find('/') {
            // 1 = `{`
            let comment_hi = if !generics.params.is_empty() {
                generics.span.lo().saturating_sub(1)
            } else {
                body_lo.saturating_sub(1)
            };
            let comment_lo = item_span.lo() + lo as BytePos;
            if comment_lo < comment_hi
                && let Some(missing_comment) = recover_missing_comment_in_span(
                    mk_sp(comment_lo, comment_hi),
                    Shape::indented(offset, context.config),
                    context,
                    last_line_width(&result),
                )
                && !missing_comment.is_empty()
            {
                result.push_str(&missing_comment);
            }
        }
    }

    let block_span = mk_sp(generics.where_clause.span.hi(), item_span.hi());
    let snippet = context.snippet(block_span);
    let open_pos = snippet.find_uncommented("{")? + 1;

    // `brace_style` is fixed at `SameLineWhere`.
    if last_line_contains_single_line_comment(&result)
        || last_line_width(&result) + 2 > context.budget(offset.width())
    {
        result.push_str(&offset.to_string_with_newline(context.config));
    } else if context.config.empty_item_single_line()
        && items.is_empty()
        && !result.contains('\n')
        && !contains_comment(&snippet[open_pos..])
    {
        result.push_str(" {}");
        return Some(result);
    } else if result.contains('\n')
        || (!generics.where_clause.predicates.is_empty() && !items.is_empty())
    {
        result.push_str(&offset.to_string_with_newline(context.config));
    } else {
        result.push(' ');
    }
    result.push('{');

    let outer_indent_str = offset.block_only().to_string_with_newline(context.config);

    if !items.is_empty() || contains_comment(&snippet[open_pos..]) {
        let mut visitor = FmtVisitor::from_context(context);
        visitor.block_indent = offset.block_only().block_indent(context.config);
        visitor.last_pos = block_span.lo() + open_pos as BytePos;

        if let Some(list) = trait_.assoc_item_list() {
            visitor.visit_attrs(
                &inner_attributes(list.syntax()),
                super::nodes::AttrStyle::Inner,
            );
        }
        for item in &items {
            visitor.visit_assoc_item(item);
        }

        visitor.format_missing(item_span.hi() - 1);

        let inner_indent_str = visitor.block_indent.to_string_with_newline(context.config);

        result.push_str(&inner_indent_str);
        result.push_str(visitor.buffer.trim());
        result.push_str(&outer_indent_str);
    } else if result.contains('\n') {
        result.push_str(&outer_indent_str);
    }

    result.push('}');
    Some(result)
}

struct TraitAliasBounds<'a> {
    generic_bounds: &'a [ast::TypeBound],
    generics: &'a Generics,
}

impl Rewrite for TraitAliasBounds<'_> {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let generic_bounds_str = Bounds(self.generic_bounds.to_vec()).rewrite(context, shape)?;

        let mut option = WhereClauseOption::new(true, WhereClauseSpace::None);
        option.allow_single_line();

        let where_str = rewrite_where_clause(
            context,
            &self.generics.where_clause,
            shape,
            ";",
            None,
            self.generics.where_clause.span.lo(),
            option,
        )?;

        let fits_single_line = !generic_bounds_str.contains('\n')
            && !where_str.contains('\n')
            && generic_bounds_str.len() + where_str.len() < shape.width;
        let space = if generic_bounds_str.is_empty() || where_str.is_empty() {
            Cow::from("")
        } else if fits_single_line {
            Cow::from(" ")
        } else {
            shape.indent.to_string_with_newline(context.config)
        };

        Some(format!("{generic_bounds_str}{space}{where_str}"))
    }
}

/// `trait Alias = Bounds;`
pub(crate) fn format_trait_alias(
    context: &RewriteContext<'_>,
    ta: &ast::Trait,
    shape: Shape,
) -> Option<String> {
    let name = ta.name()?;
    let name_hi = name_span(&name).hi();
    let bounds = type_bounds(ta.syntax());
    let where_fallback = bounds.last().map_or(name_hi, |b| b.span().hi());
    let generics = Generics::new(
        ta.generic_param_list(),
        name_hi,
        ta.where_clause(),
        where_fallback,
    );
    let alias = node_text(name.syntax());
    // 6 = "trait ", 2 = " ="
    let g_shape = shape.offset_left(6)?.sub_width(2)?;
    let generics_str = rewrite_generics(context, &alias, &generics, g_shape)?;
    let vis_str = format_visibility(ta.visibility().as_ref());
    let constness = if has_token(ta.syntax(), T![const]) {
        "const "
    } else {
        ""
    };
    let lhs = format!("{vis_str}{constness}trait {generics_str} =");
    let trait_alias_bounds = TraitAliasBounds {
        generic_bounds: &bounds,
        generics: &generics,
    };
    // 1 = ";"
    let result = rewrite_assign_rhs(
        context,
        lhs,
        &trait_alias_bounds,
        &RhsAssignKind::Bounds,
        shape.sub_width(1)?,
    )?;
    Some(result + ";")
}

// ---------------------------------------------------------------------------------------
// Structs, unions and enum variants
// ---------------------------------------------------------------------------------------

/// rustc's `VariantData`.
#[derive(Clone)]
pub(crate) enum VariantData {
    Unit,
    Tuple(Vec<ast::TupleField>),
    Struct(Vec<ast::RecordField>),
}

impl VariantData {
    fn of(field_list: Option<ast::FieldList>) -> VariantData {
        match field_list {
            None => VariantData::Unit,
            Some(ast::FieldList::TupleFieldList(l)) => VariantData::Tuple(l.fields().collect()),
            Some(ast::FieldList::RecordFieldList(l)) => VariantData::Struct(l.fields().collect()),
        }
    }
}

pub(crate) struct StructParts {
    prefix: &'static str,
    name: ast::Name,
    vis: Option<ast::Visibility>,
    def: VariantData,
    generics: Option<Generics>,
    pub(crate) span: Span,
}

impl StructParts {
    fn format_header(&self, context: &RewriteContext<'_>, offset: Indent) -> String {
        format_header(
            context,
            self.prefix,
            &self.name,
            self.vis.as_ref(),
            self.span.lo(),
            offset,
        )
    }

    fn name_hi(&self) -> BytePos {
        name_span(&self.name).hi()
    }

    fn from_variant(variant: &ast::Variant, context: &RewriteContext<'_>) -> Option<Self> {
        Some(StructParts {
            prefix: "",
            name: variant.name()?,
            vis: None,
            def: VariantData::of(variant.field_list()),
            generics: None,
            span: enum_variant_span(variant, context),
        })
    }

    pub(crate) fn from_struct(s: &ast::Struct) -> Option<Self> {
        let name = s.name()?;
        let name_hi = name_span(&name).hi();
        let params_hi = s
            .generic_param_list()
            .map_or(name_hi, |l| node_range_span(l.syntax()).hi());
        // A tuple struct's where clause follows its fields.
        let where_fallback = match s.field_list() {
            Some(ast::FieldList::TupleFieldList(l)) => node_range_span(l.syntax()).hi(),
            _ => params_hi,
        };
        Some(StructParts {
            prefix: "struct ",
            generics: Some(Generics::new(
                s.generic_param_list(),
                name_hi,
                s.where_clause(),
                where_fallback,
            )),
            name,
            vis: s.visibility(),
            def: VariantData::of(s.field_list()),
            span: item_span(s.syntax()),
        })
    }

    pub(crate) fn from_union(u: &ast::Union) -> Option<Self> {
        let name = u.name()?;
        let name_hi = name_span(&name).hi();
        let params_hi = u
            .generic_param_list()
            .map_or(name_hi, |l| node_range_span(l.syntax()).hi());
        Some(StructParts {
            prefix: "union ",
            generics: Some(Generics::new(
                u.generic_param_list(),
                name_hi,
                u.where_clause(),
                params_hi,
            )),
            name,
            vis: u.visibility(),
            def: VariantData::Struct(
                u.record_field_list()
                    .map(|l| l.fields().collect())
                    .unwrap_or_default(),
            ),
            span: item_span(u.syntax()),
        })
    }
}

fn enum_variant_span(variant: &ast::Variant, context: &RewriteContext<'_>) -> Span {
    let span = item_span(variant.syntax());
    let Some(expr) = variant.expr() else {
        return span;
    };
    let span_before_consts = mk_sp(span.lo(), expr.span().lo());
    let hi = match variant.field_list() {
        Some(ast::FieldList::RecordFieldList(_)) => context
            .snippet_provider
            .span_after_last(span_before_consts, "}"),
        Some(ast::FieldList::TupleFieldList(_)) => context
            .snippet_provider
            .span_after_last(span_before_consts, ")"),
        None => variant
            .name()
            .map_or(span_before_consts.hi(), |n| name_span(&n).hi()),
    };
    mk_sp(span_before_consts.lo(), hi)
}

fn format_struct(
    context: &RewriteContext<'_>,
    struct_parts: &StructParts,
    offset: Indent,
    one_line_width: Option<usize>,
) -> Option<String> {
    match &struct_parts.def {
        VariantData::Unit => format_unit_struct(context, struct_parts, offset),
        VariantData::Tuple(fields) => format_tuple_struct(context, struct_parts, fields, offset),
        VariantData::Struct(fields) => {
            format_struct_struct(context, struct_parts, fields, offset, one_line_width)
        }
    }
}

fn format_unit_struct(
    context: &RewriteContext<'_>,
    p: &StructParts,
    offset: Indent,
) -> Option<String> {
    let header_str = p.format_header(context, offset);
    let generics_str = if let Some(generics) = &p.generics {
        let hi = context.snippet_provider.span_before_last(p.span, ";");
        format_generics(
            context,
            generics,
            BracePos::None,
            offset,
            // make a span that starts right after `struct Foo`
            mk_sp(p.name_hi(), hi),
            last_line_width(&header_str),
        )?
    } else {
        String::new()
    };
    Some(format!("{header_str}{generics_str};"))
}

pub(crate) fn format_struct_struct(
    context: &RewriteContext<'_>,
    struct_parts: &StructParts,
    fields: &[ast::RecordField],
    offset: Indent,
    one_line_width: Option<usize>,
) -> Option<String> {
    let mut result = String::with_capacity(1024);
    let span = struct_parts.span;

    let header_str = struct_parts.format_header(context, offset);
    result.push_str(&header_str);

    let header_hi = struct_parts.name_hi();
    let body_lo = if let Some(generics) = &struct_parts.generics {
        // Adjust the span to start at the end of the generic arguments before searching for the '{'
        let span = span.with_lo(generics.where_clause.span.hi());
        context.snippet_provider.span_after(span, "{")
    } else {
        context.snippet_provider.span_after(span, "{")
    };

    let generics_str = match &struct_parts.generics {
        Some(g) => format_generics(
            context,
            g,
            if fields.is_empty() {
                BracePos::ForceSameLine
            } else {
                BracePos::Auto
            },
            offset,
            // make a span that starts right after `struct Foo`
            mk_sp(header_hi, body_lo),
            last_line_width(&result),
        )?,
        None => {
            // 3 = ` {}`, 2 = ` {`.
            let overhead = if fields.is_empty() { 3 } else { 2 };
            if context.config.max_width() < overhead + result.len() {
                format!("\n{}{{", offset.block_only().to_string(context.config))
            } else {
                " {".to_owned()
            }
        }
    };
    // 1 = `}`
    let overhead = if fields.is_empty() { 1 } else { 0 };
    let total_width = result.len() + generics_str.len() + overhead;
    if !generics_str.is_empty()
        && !generics_str.contains('\n')
        && total_width > context.config.max_width()
    {
        result.push('\n');
        result.push_str(&offset.to_string(context.config));
        result.push_str(generics_str.trim_start());
    } else {
        result.push_str(&generics_str);
    }

    if fields.is_empty() {
        let inner_span = mk_sp(body_lo, span.hi().saturating_sub(1));
        format_empty_struct_or_tuple(context, inner_span, offset, &mut result, "", "}");
        return Some(result);
    }

    // 3 = ` ` and ` }`
    let one_line_budget = context.budget(result.len() + 3 + offset.width());
    let one_line_budget =
        one_line_width.map_or(0, |one_line_width| min(one_line_width, one_line_budget));

    let items_str = rewrite_struct_fields(
        fields,
        context,
        Shape::indented(offset.block_indent(context.config), context.config).sub_width(1)?,
        mk_sp(body_lo, span.hi()),
        one_line_budget,
    )?;

    if !items_str.contains('\n')
        && !result.contains('\n')
        && items_str.len() <= one_line_budget
        && !last_line_contains_single_line_comment(&items_str)
    {
        Some(format!("{result} {items_str} }}"))
    } else {
        Some(format!(
            "{}\n{}{}\n{}}}",
            result,
            offset
                .block_indent(context.config)
                .to_string(context.config),
            items_str,
            offset.to_string(context.config)
        ))
    }
}

/// The named fields of a struct (rustfmt's `rewrite_with_alignment` with
/// `struct_field_align_threshold = 0`, its default: no column alignment).
fn rewrite_struct_fields(
    fields: &[ast::RecordField],
    context: &RewriteContext<'_>,
    shape: Shape,
    span: Span,
    one_line_width: usize,
) -> Option<String> {
    debug_assert_eq!(context.config.struct_field_align_threshold(), 0);
    // 1 = ","
    let item_shape = Shape::indented(shape.indent, context.config).sub_width(1)?;

    let items = itemize_list(
        context.snippet_provider,
        fields.iter(),
        "}",
        ",",
        |field| field.span().lo(),
        |field| field.span().hi(),
        |field| rewrite_struct_field(context, &FieldDef::Named((*field).clone()), item_shape),
        span.lo(),
        span.hi(),
        false,
    )
    .collect::<Vec<_>>();

    let tactic = definitive_tactic(
        &items,
        ListTactic::HorizontalVertical,
        Separator::Comma,
        one_line_width,
    );

    let fmt = ListFormatting::new(item_shape, context.config)
        .tactic(tactic)
        .trailing_separator(context.config.trailing_comma())
        .preserve_newline(true);
    write_list(&items, &fmt)
}

fn get_bytepos_after_visibility(vis: Option<&ast::Visibility>, default_span: Span) -> BytePos {
    match vis {
        // `pub(crate)`, `pub(in path)`, ...
        Some(v)
            if v.syntax()
                .children_with_tokens()
                .any(|el| el.kind() == T!['(']) =>
        {
            node_range_span(v.syntax()).hi()
        }
        _ => default_span.lo(),
    }
}

/// Format tuple or struct without any fields. We need to make sure that the comments
/// inside the delimiters are preserved.
fn format_empty_struct_or_tuple(
    context: &RewriteContext<'_>,
    span: Span,
    offset: Indent,
    result: &mut String,
    opener: &str,
    closer: &str,
) {
    // 3 = " {}" or "();"
    let used_width = last_line_used_width(result, offset.width()) + 3;
    if used_width > context.config.max_width() {
        result.push_str(&offset.to_string_with_newline(context.config))
    }
    result.push_str(opener);

    // indented shape for proper indenting of multi-line comments
    let shape = Shape::indented(offset.block_indent(context.config), context.config);
    match rewrite_missing_comment(span, shape, context) {
        Some(ref s) if s.is_empty() => (),
        Some(ref s) => {
            let is_multi_line = !is_single_line(s);
            if is_multi_line || first_line_contains_single_line_comment(s) {
                let nested_indent_str = offset
                    .block_indent(context.config)
                    .to_string_with_newline(context.config);
                result.push_str(&nested_indent_str);
            }
            result.push_str(s);
            if is_multi_line || last_line_contains_single_line_comment(s) {
                result.push_str(&offset.to_string_with_newline(context.config));
            }
        }
        None => result.push_str(context.snippet(span)),
    }
    result.push_str(closer);
}

fn format_tuple_struct(
    context: &RewriteContext<'_>,
    struct_parts: &StructParts,
    fields: &[ast::TupleField],
    offset: Indent,
) -> Option<String> {
    let mut result = String::with_capacity(1024);
    let span = struct_parts.span;

    let header_str = struct_parts.format_header(context, offset);
    result.push_str(&header_str);

    let body_lo = match fields.first() {
        None => {
            let lo = get_bytepos_after_visibility(struct_parts.vis.as_ref(), span);
            context
                .snippet_provider
                .span_after(mk_sp(lo, span.hi()), "(")
        }
        Some(first) => item_span(first.syntax()).lo(),
    };
    let body_hi = match fields.last() {
        None => context
            .snippet_provider
            .span_after(mk_sp(body_lo, span.hi()), ")"),
        Some(last) => {
            // This is a dirty hack to work around a missing `)` from the span of the last field.
            let last_arg_span = item_span(last.syntax());
            context
                .snippet_provider
                .opt_span_after(mk_sp(last_arg_span.hi(), span.hi()), ")")
                .unwrap_or_else(|| last_arg_span.hi())
        }
    };

    let where_clause_str = match &struct_parts.generics {
        Some(generics) => {
            let budget = context.budget(last_line_width(&header_str));
            let shape = Shape::legacy(budget, offset);
            let generics_str = rewrite_generics(context, "", generics, shape)?;
            result.push_str(&generics_str);

            let where_budget = context.budget(last_line_width(&result));
            let option = WhereClauseOption::new(true, WhereClauseSpace::Newline);
            rewrite_where_clause(
                context,
                &generics.where_clause,
                Shape::legacy(where_budget, offset.block_only()),
                ";",
                None,
                body_hi,
                option,
            )?
        }
        None => String::new(),
    };

    if fields.is_empty() {
        let body_hi = context
            .snippet_provider
            .span_before(mk_sp(body_lo, span.hi()), ")");
        let inner_span = mk_sp(body_lo, body_hi);
        format_empty_struct_or_tuple(context, inner_span, offset, &mut result, "(", ")");
    } else {
        let lo = match &struct_parts.generics {
            Some(generics) => generics.span.hi(),
            None => struct_parts.name_hi(),
        };
        let shape = Shape::indented(offset, context.config).sub_width(1)?;
        result = overflow::rewrite_with_parens(
            context,
            &result,
            fields.iter().cloned().map(OverflowableItem::FieldDef),
            shape,
            mk_sp(lo, span.hi()),
            context.config.fn_call_width(),
            None,
        )?;
    }

    if !where_clause_str.is_empty()
        && !where_clause_str.contains('\n')
        && (result.contains('\n')
            || offset.block_indent + result.len() + where_clause_str.len() + 1
                > context.config.max_width())
    {
        // We need to put the where-clause on a new line, but we didn't
        // know that earlier, so the where-clause will not be indented properly.
        result.push('\n');
        result.push_str(
            &(offset.block_only() + (context.config.tab_spaces() - 1)).to_string(context.config),
        );
    }
    result.push_str(&where_clause_str);

    Some(result)
}

/// A struct field: named (`a: T`) or positional (`T`).
pub(crate) enum FieldDef {
    Named(ast::RecordField),
    Positional(ast::TupleField),
}

impl FieldDef {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            FieldDef::Named(f) => f.syntax(),
            FieldDef::Positional(f) => f.syntax(),
        }
    }

    fn vis(&self) -> Option<ast::Visibility> {
        match self {
            FieldDef::Named(f) => f.visibility(),
            FieldDef::Positional(f) => f.visibility(),
        }
    }

    fn ty(&self) -> Option<ast::Type> {
        match self {
            FieldDef::Named(f) => f.ty(),
            FieldDef::Positional(f) => f.ty(),
        }
    }
}

fn rewrite_struct_field_prefix(field: &FieldDef) -> Option<String> {
    let vis = format_visibility(field.vis().as_ref());
    match field {
        FieldDef::Named(f) => {
            let safety = if f.unsafe_token().is_some() {
                "unsafe "
            } else {
                ""
            };
            // `space_before_colon` is fixed at `false`.
            Some(format!("{vis}{safety}{}:", f.name()?.syntax().text()))
        }
        FieldDef::Positional(_) => Some(vis.into_owned()),
    }
}

impl Rewrite for ast::TupleField {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        rewrite_struct_field(context, &FieldDef::Positional(self.clone()), shape)
    }
}

/// rustfmt's `rewrite_struct_field` with `lhs_max_width = 0` (no alignment).
fn rewrite_struct_field(
    context: &RewriteContext<'_>,
    field: &FieldDef,
    shape: Shape,
) -> Option<String> {
    // rustfmt does not format default field values yet.
    if let FieldDef::Named(f) = field
        && f.expr().is_some()
    {
        return None;
    }

    let attrs = outer_attributes(field.syntax());
    if contains_skip(&attrs) {
        return Some(context.snippet(rustc_span(field.syntax())).to_owned());
    }

    let is_named = matches!(field, FieldDef::Named(_));
    let prefix = rewrite_struct_field_prefix(field)?;

    let field_lo = item_span(field.syntax()).lo();
    let attrs_str = attrs.rewrite(context, shape)?;
    let attrs_extendable = !is_named && is_attributes_extendable(&attrs_str);
    let missing_span = match attrs.last() {
        None => mk_sp(field_lo, field_lo),
        Some(last) => mk_sp(last.span().hi(), field_lo),
    };
    // `space_after_colon` is fixed at `true`.
    let mut spacing = String::from(if is_named { " " } else { "" });
    // Try to put everything on a single line.
    let attr_prefix = combine_strs_with_missing_comments(
        context,
        &attrs_str,
        &prefix,
        missing_span,
        shape,
        attrs_extendable,
    )?;
    let overhead = trimmed_last_line_width(&attr_prefix);
    // In this extreme case we will be missing a space between an attribute and a field.
    if prefix.is_empty() && !attrs_str.is_empty() && attrs_extendable && spacing.is_empty() {
        spacing.push(' ');
    }

    let ty = field.ty()?;
    let orig_ty = shape
        .offset_left(overhead + spacing.len())
        .and_then(|ty_shape| ty.rewrite(context, ty_shape));

    if let Some(ref ty) = orig_ty
        && !ty.contains('\n')
        && !contains_comment(context.snippet(missing_span))
    {
        return Some(attr_prefix + &spacing + ty);
    }

    let is_prefix_empty = prefix.is_empty();
    // We must use multiline. We are going to put attributes and a field on different lines.
    let field_str = rewrite_assign_rhs(context, prefix, &ty, &RhsAssignKind::Ty, shape)?;
    // Remove a leading white-space from `rewrite_assign_rhs()` when rewriting a tuple struct.
    let field_str = if is_prefix_empty {
        field_str.trim_start()
    } else {
        &field_str
    };
    combine_strs_with_missing_comments(context, &attrs_str, field_str, missing_span, shape, false)
}

// ---------------------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemVisitorKind {
    Item,
    AssocTraitItem,
    AssocImplItem,
    ForeignItem,
}

pub(crate) fn rewrite_type_alias(
    ta: &ast::TypeAlias,
    context: &RewriteContext<'_>,
    indent: Indent,
    visitor_kind: ItemVisitorKind,
    span: Span,
) -> Option<String> {
    use ItemVisitorKind::*;

    let name = ta.name()?;
    let name_hi = name_span(&name).hi();
    let bounds = type_bounds(ta.syntax());
    let ty = ta.ty();
    let params_hi = ta
        .generic_param_list()
        .map_or(name_hi, |l| node_range_span(l.syntax()).hi());
    let before_ty = bounds.last().map_or(params_hi, |b| b.span().hi());

    // rust-analyzer keeps a single where clause; rustc distinguishes the one before `=`
    // from the one after the type.
    let wc = ta.where_clause();
    let wc_after = match (&wc, &ty) {
        (Some(wc), Some(ty)) => wc.syntax().text_range().start() >= ty.syntax().text_range().end(),
        _ => false,
    };
    let (generics, after_where_clause) = if wc_after {
        (
            Generics::new(ta.generic_param_list(), name_hi, None, before_ty),
            WhereClause::new(wc, before_ty),
        )
    } else {
        let rhs_hi = ty.as_ref().map_or(before_ty, |t| t.span().hi());
        (
            Generics::new(ta.generic_param_list(), name_hi, wc, before_ty),
            WhereClause::new(None, rhs_hi),
        )
    };
    let rhs_hi = ty
        .as_ref()
        .map_or(generics.where_clause.span.hi(), |ty| ty.span().hi());
    let rw_info = TyAliasRewriteInfo {
        context,
        indent,
        generics: &generics,
        after_where_clause: &after_where_clause,
        name: node_text(name.syntax()),
        bounds: &bounds,
        span,
    };
    let op_ty = opaque_ty(ty.as_ref());
    // Type Aliases are formatted slightly differently depending on the context
    // in which they appear, whether they are opaque, and whether they are associated.
    match (visitor_kind, &op_ty) {
        (Item | AssocTraitItem | ForeignItem, Some(op_bounds)) => {
            let op = OpaqueType { bounds: op_bounds };
            rewrite_ty(&rw_info, Some(&op), rhs_hi, ta.visibility().as_ref())
        }
        (Item | AssocTraitItem | ForeignItem, None) => {
            rewrite_ty(&rw_info, ty.as_ref(), rhs_hi, ta.visibility().as_ref())
        }
        (AssocImplItem, _) => {
            let result = if let Some(op_bounds) = &op_ty {
                let op = OpaqueType { bounds: op_bounds };
                rewrite_ty(&rw_info, Some(&op), rhs_hi, None)
            } else {
                rewrite_ty(&rw_info, ty.as_ref(), rhs_hi, ta.visibility().as_ref())
            }?;
            if ta.default_token().is_some() {
                Some(format!("default {result}"))
            } else {
                Some(result)
            }
        }
    }
}

struct TyAliasRewriteInfo<'c, 'g> {
    context: &'c RewriteContext<'c>,
    indent: Indent,
    generics: &'g Generics,
    after_where_clause: &'g WhereClause,
    name: String,
    bounds: &'g [ast::TypeBound],
    span: Span,
}

fn rewrite_ty<R: Rewrite>(
    rw_info: &TyAliasRewriteInfo<'_, '_>,
    rhs: Option<&R>,
    // the span of the end of the RHS (or the end of the generics, if there is no RHS)
    rhs_hi: BytePos,
    vis: Option<&ast::Visibility>,
) -> Option<String> {
    let mut result = String::with_capacity(128);
    let TyAliasRewriteInfo {
        context,
        indent,
        generics,
        after_where_clause,
        ref name,
        bounds,
        span,
    } = *rw_info;
    result.push_str(&format!("{}type ", format_visibility(vis)));

    if generics.params.is_empty() {
        result.push_str(name)
    } else {
        // 2 = `= `
        let g_shape = Shape::indented(indent, context.config);
        let g_shape = g_shape.offset_left(result.len())?.sub_width(2)?;
        let generics_str = rewrite_generics(context, name, generics, g_shape)?;
        result.push_str(&generics_str);
    }

    if !bounds.is_empty() {
        // 2 = `: `
        let shape = Shape::indented(indent, context.config);
        let shape = shape.offset_left(result.len() + 2)?;
        let type_bounds = Bounds(bounds.to_vec())
            .rewrite(context, shape)
            .map(|s| format!(": {s}"))?;
        result.push_str(&type_bounds);
    }

    let where_budget = context.budget(last_line_width(&result));
    let mut option = WhereClauseOption::snuggled(&result);
    if rhs.is_none() {
        option.suppress_comma();
    }
    let before_where_clause_str = rewrite_where_clause(
        context,
        &generics.where_clause,
        Shape::legacy(where_budget, indent),
        "=",
        None,
        generics.span.hi(),
        option,
    )?;
    result.push_str(&before_where_clause_str);

    let mut result = if let Some(ty) = rhs {
        // If there are any where clauses, add a newline before the assignment.
        // If there is a before where clause, do not indent, but if there is
        // only an after where clause, additionally indent the type.
        if !generics.where_clause.predicates.is_empty() {
            result.push_str(&indent.to_string_with_newline(context.config));
        } else if !after_where_clause.predicates.is_empty() {
            result.push_str(
                &indent
                    .block_indent(context.config)
                    .to_string_with_newline(context.config),
            );
        } else {
            result.push(' ');
        }

        let comment_span = context
            .snippet_provider
            .opt_span_before(span, "=")
            .map(|op_lo| mk_sp(generics.where_clause.span.hi(), op_lo));

        let lhs = match comment_span {
            Some(comment_span)
                if contains_comment(context.snippet_provider.span_to_snippet(comment_span)?) =>
            {
                let comment_shape = if !generics.where_clause.predicates.is_empty() {
                    Shape::indented(indent, context.config)
                } else {
                    let shape = Shape::indented(indent, context.config);
                    shape.block_left(context.config.tab_spaces())?
                };

                combine_strs_with_missing_comments(
                    context,
                    result.trim_end(),
                    "=",
                    comment_span,
                    comment_shape,
                    true,
                )?
            }
            _ => format!("{result}="),
        };

        // 1 = `;` unless there's a trailing where clause
        let shape = Shape::indented(indent, context.config);
        let shape = if after_where_clause.predicates.is_empty() {
            Shape::indented(indent, context.config).sub_width(1)?
        } else {
            shape
        };
        rewrite_assign_rhs(context, lhs, ty, &RhsAssignKind::Ty, shape)?
    } else {
        result
    };

    if !after_where_clause.predicates.is_empty() {
        let option = WhereClauseOption::new(true, WhereClauseSpace::Newline);
        let after_where_clause_str = rewrite_where_clause(
            context,
            after_where_clause,
            Shape::indented(indent, context.config),
            ";",
            None,
            rhs_hi,
            option,
        )?;
        result.push_str(&after_where_clause_str);
    }

    result += ";";
    Some(result)
}

// FIXME(calebcartwright) - This is a hack around a bug in the handling of TyKind::ImplTrait.
// This should be removed once that bug is resolved, with the type alias formatting using the
// defined Ty for the RHS directly.
// https://github.com/rust-lang/rustfmt/issues/4373
// https://github.com/rust-lang/rustfmt/issues/5027
struct OpaqueType<'a> {
    bounds: &'a [ast::TypeBound],
}

impl Rewrite for OpaqueType<'_> {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let shape = shape.offset_left(5)?; // `impl `
        Bounds(self.bounds.to_vec())
            .rewrite(context, shape)
            .map(|s| format!("impl {s}"))
    }
}

// ---------------------------------------------------------------------------------------
// Statics and constants
// ---------------------------------------------------------------------------------------

pub(crate) struct StaticParts {
    prefix: &'static str,
    safety: &'static str,
    vis: Option<ast::Visibility>,
    name: String,
    has_generics: bool,
    ty: Option<ast::Type>,
    mutability: &'static str,
    expr: Option<ast::Expr>,
    defaultness: &'static str,
    pub(crate) span: Span,
}

impl StaticParts {
    pub(crate) fn from_const(c: &ast::Const) -> Option<Self> {
        let name = match c.name() {
            Some(n) => node_text(n.syntax()),
            None => c.underscore_token()?.text().to_string(),
        };
        Some(StaticParts {
            prefix: "const",
            safety: "",
            vis: c.visibility(),
            name,
            has_generics: c.generic_param_list().is_some() || c.where_clause().is_some(),
            ty: c.ty(),
            mutability: "",
            expr: c.body(),
            defaultness: if c.default_token().is_some() {
                "default "
            } else {
                ""
            },
            span: item_span(c.syntax()),
        })
    }

    pub(crate) fn from_static(s: &ast::Static) -> Option<Self> {
        Some(StaticParts {
            prefix: "static",
            safety: if s.unsafe_token().is_some() {
                "unsafe "
            } else if s.safe_token().is_some() {
                "safe "
            } else {
                ""
            },
            vis: s.visibility(),
            name: node_text(s.name()?.syntax()),
            has_generics: false,
            ty: s.ty(),
            mutability: if s.mut_token().is_some() { "mut " } else { "" },
            expr: s.body(),
            defaultness: "",
            span: item_span(s.syntax()),
        })
    }
}

fn rewrite_static(
    context: &RewriteContext<'_>,
    static_parts: &StaticParts,
    offset: Indent,
) -> Option<String> {
    // For now, if this static (or const) has generics, then bail.
    if static_parts.has_generics {
        return None;
    }
    // `const X = 1;` (no type) does not parse in rustc.
    let ty = static_parts.ty.as_ref()?;

    let colon = colon_spaces(context.config);
    let mut prefix = format!(
        "{}{}{}{} {}{}{}",
        format_visibility(static_parts.vis.as_ref()),
        static_parts.defaultness,
        static_parts.safety,
        static_parts.prefix,
        static_parts.mutability,
        static_parts.name,
        colon,
    );
    // 2 = " =".len()
    let ty_shape =
        Shape::indented(offset.block_only(), context.config).offset_left(prefix.len() + 2)?;
    let ty_str = match ty.rewrite(context, ty_shape) {
        Some(ty_str) => ty_str,
        None => {
            if prefix.ends_with(' ') {
                prefix.pop();
            }
            let nested_indent = offset.block_indent(context.config);
            let nested_shape = Shape::indented(nested_indent, context.config);
            let ty_str = ty.rewrite(context, nested_shape)?;
            format!(
                "{}{}",
                nested_indent.to_string_with_newline(context.config),
                ty_str
            )
        }
    };

    if let Some(expr) = &static_parts.expr {
        let comments_lo = context.snippet_provider.span_after(static_parts.span, "=");
        let expr_lo = expr.span().lo();
        let comments_span = mk_sp(comments_lo, expr_lo);

        let lhs = format!("{prefix}{ty_str} =");

        // 1 = ;
        let remaining_width = context.budget(offset.block_indent + 1);
        rewrite_assign_rhs_with_comments(
            context,
            &lhs,
            expr,
            Shape::legacy(remaining_width, offset.block_only()),
            &RhsAssignKind::Expr,
            RhsTactics::Default,
            comments_span,
            true,
        )
        .map(|res| recover_comment_removed(res, static_parts.span, context))
        .map(|s| if s.ends_with(';') { s } else { s + ";" })
    } else {
        Some(format!("{prefix}{ty_str};"))
    }
}

// ---------------------------------------------------------------------------------------
// Items of `extern` blocks
// ---------------------------------------------------------------------------------------

impl Rewrite for ast::ExternItem {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let attrs = outer_attributes(self.syntax());
        let attrs_str = attrs.rewrite(context, shape)?;
        let item_span = item_span(self.syntax());
        // Drop semicolon or it will be interpreted as comment.
        let span = mk_sp(item_span.lo(), item_span.hi().saturating_sub(1));

        let item_str = match self {
            ast::ExternItem::Fn(f) => {
                if f.body().is_some() {
                    let mut visitor = FmtVisitor::from_context(context);
                    visitor.block_indent = shape.indent;
                    visitor.last_pos = item_span.lo();
                    visitor.visit_fn(f, item_span);
                    std::mem::take(&mut visitor.buffer)
                } else {
                    rewrite_fn_base(
                        context,
                        shape.indent,
                        &name_text(f.name())?,
                        &FnSig::from_fn(f)?,
                        span,
                        FnBraceStyle::None,
                    )
                    .map(|(s, _, _)| format!("{s};"))?
                }
            }
            ast::ExternItem::Static(s) => {
                // FIXME(#21): we're dropping potential comments in between the
                // function kw here.
                let vis = format_visibility(s.visibility().as_ref());
                let safety = if s.unsafe_token().is_some() {
                    "unsafe "
                } else if s.safe_token().is_some() {
                    "safe "
                } else {
                    ""
                };
                let mut_str = if s.mut_token().is_some() { "mut " } else { "" };
                let prefix = format!(
                    "{}{}static {}{}:",
                    vis,
                    safety,
                    mut_str,
                    s.name()?.syntax().text()
                );
                // 1 = ;
                rewrite_assign_rhs(
                    context,
                    prefix,
                    &s.ty()?,
                    &RhsAssignKind::Ty,
                    shape.sub_width(1)?,
                )
                .map(|s| s + ";")?
            }
            ast::ExternItem::TypeAlias(ta) => {
                let kind = ItemVisitorKind::ForeignItem;
                rewrite_type_alias(ta, context, shape.indent, kind, item_span)?
            }
            ast::ExternItem::MacroCall(mac) => {
                rewrite_macro(mac, context, shape, MacroPosition::Item)?
            }
        };

        let missing_span = match attrs.last() {
            None => mk_sp(item_span.lo(), item_span.lo()),
            Some(last) => mk_sp(last.span().hi(), item_span.lo()),
        };
        combine_strs_with_missing_comments(
            context,
            &attrs_str,
            &item_str,
            missing_span,
            shape,
            false,
        )
    }
}

// ---------------------------------------------------------------------------------------
// `mod foo;`, `extern crate foo;`
// ---------------------------------------------------------------------------------------

/// Rewrite the attributes of an item.
fn rewrite_attrs(
    context: &RewriteContext<'_>,
    node: &SyntaxNode,
    item_str: &str,
    shape: Shape,
) -> Option<String> {
    let attrs = outer_attributes(node);
    let attrs_str = attrs.rewrite(context, shape)?;
    let item_lo = item_span(node).lo();

    let missed_span = match attrs.last() {
        None => mk_sp(item_lo, item_lo),
        Some(last) => mk_sp(last.span().hi(), item_lo),
    };

    let allow_extend = if attrs.len() == 1 {
        let line_len = attrs_str.len() + 1 + item_str.len();
        !attrs[0].is_doc_comment() && context.config.inline_attribute_width() >= line_len
    } else {
        false
    };

    combine_strs_with_missing_comments(
        context,
        &attrs_str,
        item_str,
        missed_span,
        shape,
        allow_extend,
    )
}

/// Rewrite `mod foo;`. The given shape is used to format the mod's attributes.
pub(crate) fn rewrite_mod(
    context: &RewriteContext<'_>,
    module: &ast::Module,
    attrs_shape: Shape,
) -> Option<String> {
    let mut result = String::with_capacity(32);
    result.push_str(&format_visibility(module.visibility().as_ref()));
    if module
        .syntax()
        .children_with_tokens()
        .any(|el| el.kind() == T![unsafe])
    {
        result.push_str("unsafe ");
    }
    result.push_str("mod ");
    result.push_str(&node_text(module.name()?.syntax()));
    result.push(';');
    rewrite_attrs(context, module.syntax(), &result, attrs_shape)
}

/// Rewrite `extern crate foo;`. The given shape is used to format the extern crate's
/// attributes.
pub(crate) fn rewrite_extern_crate(
    context: &RewriteContext<'_>,
    item: &ast::ExternCrate,
    attrs_shape: Shape,
) -> Option<String> {
    let new_str = context.snippet(item_span(item.syntax()));
    let item_str = if contains_comment(new_str) {
        new_str.to_owned()
    } else {
        let no_whitespace = new_str.split_whitespace().collect::<Vec<&str>>().join(" ");
        // rustfmt removes whitespace before the `;` with the regex `\s;`.
        no_whitespace.replace(" ;", ";")
    };
    rewrite_attrs(context, item.syntax(), &item_str, attrs_shape)
}

/// Returns `true` for `mod foo;`, false for `mod foo { .. }`.
pub(crate) fn is_mod_decl(item: &ast::Item) -> bool {
    match item {
        ast::Item::Module(m) => m.item_list().is_none(),
        _ => false,
    }
}

pub(crate) fn is_use_item(item: &ast::Item) -> bool {
    matches!(item, ast::Item::Use(_))
}
