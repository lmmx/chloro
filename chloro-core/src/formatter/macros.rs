//! Macro calls and macro definitions (rustfmt's `macros.rs`).
//!
//! List-like macro calls, whose token trees parse as comma-separated expressions, types,
//! patterns or items, are formatted as function calls (`foo!(..)`) or array literals
//! (`foo![..]`). Macros that do not parse that way, such as `bar!(key => val)`, keep their
//! original text. Brace-delimited calls keep their body and are only re-indented.
//!
//! `macro_rules!` branches are formatted by substituting the `$name` metavariables with
//! plain identifiers, formatting the body as Rust code, and substituting back. Matchers
//! are kept as written (rustfmt's `format_macro_matchers` is off by default).

use ra_ap_syntax::ast::{self, AstNode, AstToken, HasName, HasVisibility};
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, T};

use super::comment::{CharClasses, FullCodeCharKind, LineClasses, contains_comment};
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{RhsAssignKind, rewrite_array, rewrite_assign_rhs};
use super::formatting::{format_code_block, format_snippet};
use super::lists::{ListFormatting, SeparatorTactic, itemize_list, write_list};
use super::macro_args::{ParsedMacroArgs, parse_expr, parse_lazy_static, parse_macro_args};
use super::nodes::node_text;
use super::nodes::{Block, BlockRules};
use super::overflow::{self, Delimiter, OverflowableItem};
use super::shape::{Indent, Shape};
use super::span::{Span, Spanned, mk_sp, node_range_span, token_span};
use super::utils::{
    filtered_str_fits, format_visibility, indent_next_line, is_empty_line,
    remove_trailing_white_spaces, trim_left_preserve_layout,
};
use super::visitor::FmtVisitor;

const FORCED_BRACKET_MACROS: &[&str] = &["vec!"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroPosition {
    Item,
    Statement,
    Expression,
    Pat,
}

/// One argument of a list-like macro call.
#[derive(Debug, Clone)]
pub(crate) enum MacroArg {
    Expr(ast::Expr),
    Ty(ast::Type),
    Pat(ast::Pat),
    Item(ast::Item),
    /// A lone keyword such as `self` or `_`, which rustc does not parse as an expression.
    Keyword(String, Span),
}

impl MacroArg {
    pub(crate) fn is_item(&self) -> bool {
        matches!(self, MacroArg::Item(..))
    }
}

impl Spanned for MacroArg {
    fn span(&self) -> Span {
        match self {
            MacroArg::Expr(e) => e.span(),
            MacroArg::Ty(t) => t.span(),
            MacroArg::Pat(p) => p.span(),
            MacroArg::Item(i) => i.span(),
            MacroArg::Keyword(_, span) => *span,
        }
    }
}

impl Rewrite for ast::Item {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let mut visitor = FmtVisitor::from_context(context);
        visitor.block_indent = shape.indent;
        visitor.last_pos = self.span().lo();
        visitor.visit_item(self);
        Some(std::mem::take(&mut visitor.buffer))
    }
}

impl Rewrite for MacroArg {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            MacroArg::Expr(expr) => expr.rewrite(context, shape),
            MacroArg::Ty(ty) => ty.rewrite(context, shape),
            MacroArg::Pat(pat) => pat.rewrite(context, shape),
            MacroArg::Item(item) => item.rewrite(context, shape),
            MacroArg::Keyword(text, _) => Some(text.clone()),
        }
    }
}

/// rustc's span of a macro call: from the path to the closing delimiter, without
/// attributes or a trailing `;`.
pub(crate) fn mac_span(mac: &ast::MacroCall) -> Span {
    let lo = mac
        .path()
        .map(|p| node_range_span(p.syntax()).lo())
        .unwrap_or_else(|| mac.span().lo());
    let hi = mac
        .token_tree()
        .map(|tt| node_range_span(tt.syntax()).hi())
        .unwrap_or_else(|| mac.span().hi());
    mk_sp(lo, hi)
}

/// The macro path followed by `!`, without whitespace (rustc's `path_to_string`).
fn rewrite_macro_name(mac: &ast::MacroCall) -> String {
    let mut name = String::new();
    if let Some(path) = mac.path() {
        for token in path
            .syntax()
            .descendants_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .filter(|t| !t.kind().is_trivia())
        {
            name.push_str(token.text());
        }
    }
    name.push('!');
    name
}

/// The delimiter of a token tree.
fn tt_delimiter(tt: &ast::TokenTree) -> Delimiter {
    match tt.syntax().first_token().map(|t| t.kind()) {
        Some(T!['(']) => Delimiter::Parenthesis,
        Some(T!['[']) => Delimiter::Bracket,
        _ => Delimiter::Brace,
    }
}

/// `true` if the token tree holds nothing but its delimiters and whitespace or comments.
fn tt_is_empty(tt: &ast::TokenTree) -> bool {
    tt.syntax()
        .children_with_tokens()
        .filter(|el| !el.kind().is_trivia())
        .count()
        <= 2
}

/// Used when a macro call cannot be formatted.
fn return_macro_parse_failure_fallback(
    context: &RewriteContext<'_>,
    indent: Indent,
    position: MacroPosition,
    span: Span,
) -> Option<String> {
    // Mark this as a failure however we format it
    context.macro_rewrite_failure.set(true);

    // Heuristically determine whether the last line of the macro uses "Block" style
    // rather than using "Visual" style, or another indentation style.
    let snippet = context.snippet(span);
    let is_like_block_indent_style = snippet.lines().last().is_some_and(|closing_line| {
        closing_line
            .trim()
            .chars()
            .all(|ch| matches!(ch, '}' | ')' | ']'))
    });
    if is_like_block_indent_style {
        return trim_left_preserve_layout(snippet, indent, context.config);
    }

    context.add_skipped_span(span);

    // Return the snippet unmodified if the macro is not block-like
    let mut snippet = snippet.to_owned();
    if position == MacroPosition::Item {
        snippet.push(';');
    }
    Some(snippet)
}

pub(crate) fn rewrite_macro(
    mac: &ast::MacroCall,
    context: &RewriteContext<'_>,
    shape: Shape,
    position: MacroPosition,
) -> Option<String> {
    let guard = context.enter_macro();
    let result = rewrite_macro_inner(mac, context, shape, position, guard.is_nested());
    if result.is_none() {
        context.macro_rewrite_failure.set(true);
    }
    result
}

fn cached_macro_args(
    context: &RewriteContext<'_>,
    tt: &ast::TokenTree,
    forced_bracket: bool,
) -> Option<ParsedMacroArgs> {
    let span = node_range_span(tt.syntax());
    let key = (span.lo(), span.hi(), forced_bracket);
    if let Some(cached) = context.run.macro_args.borrow().get(&key) {
        return cached.clone();
    }
    let parsed = parse_macro_args(context, tt, forced_bracket);
    context
        .run
        .macro_args
        .borrow_mut()
        .insert(key, parsed.clone());
    parsed
}

fn rewrite_macro_inner(
    mac: &ast::MacroCall,
    context: &RewriteContext<'_>,
    shape: Shape,
    position: MacroPosition,
    is_nested_macro: bool,
) -> Option<String> {
    if context.config.use_try_shorthand()
        && let Some(inner) = convert_try_mac(mac, context)
    {
        context.leave_macro();
        return super::chains::rewrite_try_shorthand(&inner, context, shape);
    }

    let tt = mac.token_tree()?;
    let span = mac_span(mac);
    let original_style = tt_delimiter(&tt);

    let macro_name = rewrite_macro_name(mac);
    let is_forced_bracket = FORCED_BRACKET_MACROS.contains(&macro_name.as_str());

    let style = if is_forced_bracket && !is_nested_macro {
        Delimiter::Bracket
    } else {
        original_style
    };

    let has_comment = contains_comment(context.snippet(span));
    if tt_is_empty(&tt) && !has_comment {
        return Some(match style {
            Delimiter::Parenthesis if position == MacroPosition::Item => {
                format!("{macro_name}();")
            }
            Delimiter::Bracket if position == MacroPosition::Item => format!("{macro_name}[];"),
            Delimiter::Parenthesis => format!("{macro_name}()"),
            Delimiter::Bracket => format!("{macro_name}[]"),
            Delimiter::Brace => format!("{macro_name} {{}}"),
        });
    }
    // Format well-known macros which cannot be parsed as a valid AST.
    if macro_name == "lazy_static!" && !has_comment {
        match format_lazy_static(context, shape, &tt, &macro_name) {
            LazyStatic::Formatted(rw) => return Some(rw),
            LazyStatic::Failed => return None,
            // Move on to parsing macro args just like other macros.
            LazyStatic::NotParsed => {}
        }
    }

    let ParsedMacroArgs {
        args: arg_vec,
        vec_with_semi,
        trailing_comma,
    } = if style == Delimiter::Brace {
        ParsedMacroArgs {
            args: Vec::new(),
            vec_with_semi: false,
            trailing_comma: false,
        }
    } else {
        match cached_macro_args(context, &tt, is_forced_bracket) {
            Some(args) => args,
            None => {
                return return_macro_parse_failure_fallback(context, shape.indent, position, span);
            }
        }
    };

    if !arg_vec.is_empty() && arg_vec.iter().all(MacroArg::is_item) {
        return rewrite_macro_with_items(
            context,
            &arg_vec,
            &macro_name,
            shape,
            style,
            original_style,
            position,
            span,
        );
    }

    let separator_tactic = if trailing_comma {
        Some(SeparatorTactic::Always)
    } else {
        Some(SeparatorTactic::Never)
    };
    match style {
        Delimiter::Parenthesis => {
            // Handle special case: `vec!(expr; expr)`
            if vec_with_semi {
                handle_vec_semi(context, shape, arg_vec, macro_name, style)
            } else {
                // Format macro invocation as function call, preserve the trailing
                // comma because not all macros support them.
                overflow::rewrite_with_parens(
                    context,
                    &macro_name,
                    arg_vec.into_iter().map(OverflowableItem::MacroArg),
                    shape,
                    span,
                    context.config.fn_call_width(),
                    separator_tactic,
                )
                .map(|rw| match position {
                    MacroPosition::Item => format!("{rw};"),
                    _ => rw,
                })
            }
        }
        Delimiter::Bracket => {
            // Handle special case: `vec![expr; expr]`
            if vec_with_semi {
                handle_vec_semi(context, shape, arg_vec, macro_name, style)
            } else {
                // If we are rewriting `vec!` macro or other special macros,
                // then we can rewrite this as a usual array literal.
                // Otherwise, we must preserve the original existence of trailing comma.
                let mut force_trailing_comma = separator_tactic;
                if is_forced_bracket && !is_nested_macro {
                    context.leave_macro();
                    if context.use_block_indent() {
                        force_trailing_comma = Some(SeparatorTactic::Vertical);
                    }
                }
                let rewrite = rewrite_array(
                    &macro_name,
                    arg_vec.into_iter().map(OverflowableItem::MacroArg),
                    span,
                    context,
                    shape,
                    force_trailing_comma,
                    Some(original_style),
                )?;
                let comma = match position {
                    MacroPosition::Item => ";",
                    _ => "",
                };

                Some(format!("{rewrite}{comma}"))
            }
        }
        Delimiter::Brace => {
            // For macro invocations with braces, always put a space between
            // the `macro_name!` and `{ /* macro_body */ }` but skip modifying
            // anything in between the braces (for now).
            let snippet = context.snippet(span).trim_start_matches(|c| c != '{');
            match trim_left_preserve_layout(snippet, shape.indent, context.config) {
                Some(macro_body) => Some(format!("{macro_name} {macro_body}")),
                None => Some(format!("{macro_name} {snippet}")),
            }
        }
    }
}

fn handle_vec_semi(
    context: &RewriteContext<'_>,
    shape: Shape,
    arg_vec: Vec<MacroArg>,
    macro_name: String,
    delim_token: Delimiter,
) -> Option<String> {
    let (left, right) = match delim_token {
        Delimiter::Parenthesis => ("(", ")"),
        _ => ("[", "]"),
    };

    let mac_shape = shape.offset_left(macro_name.len())?;
    // 8 = `vec![]` + `; ` or `vec!()` + `; `
    let total_overhead = 8;
    let nested_shape = mac_shape.block_indent(context.config.tab_spaces());
    let lhs = arg_vec[0].rewrite(context, nested_shape)?;
    let rhs = arg_vec[1].rewrite(context, nested_shape)?;
    if !lhs.contains('\n')
        && !rhs.contains('\n')
        && lhs.len() + rhs.len() + total_overhead <= shape.width
    {
        // macro_name(lhs; rhs) or macro_name[lhs; rhs]
        Some(format!("{macro_name}{left}{lhs}; {rhs}{right}"))
    } else {
        // macro_name(\nlhs;\nrhs\n) or macro_name[\nlhs;\nrhs\n]
        Some(format!(
            "{}{}{}{};{}{}{}{}",
            macro_name,
            left,
            nested_shape.indent.to_string_with_newline(context.config),
            lhs,
            nested_shape.indent.to_string_with_newline(context.config),
            rhs,
            shape.indent.to_string_with_newline(context.config),
            right
        ))
    }
}

/// Tries to convert a `try!(expr)` call into the shorthand `expr?`. Returns the argument
/// of the call, or `None` when the macro is not `try!` or its argument does not parse.
pub(crate) fn convert_try_mac(
    mac: &ast::MacroCall,
    context: &RewriteContext<'_>,
) -> Option<ast::Expr> {
    let name = rewrite_macro_name(mac);
    if name == "try!" || name == "r#try!" {
        parse_expr(context, &mac.token_tree()?)
    } else {
        None
    }
}

/// The delimiter style of a macro call.
pub(crate) fn macro_style(mac: &ast::MacroCall) -> Delimiter {
    mac.token_tree()
        .map(|tt| tt_delimiter(&tt))
        .unwrap_or(Delimiter::Brace)
}

enum LazyStatic {
    Formatted(String),
    /// The body parsed but could not be formatted within the shape.
    Failed,
    NotParsed,
}

/// Formats `lazy_static!` from <https://crates.io/crates/lazy_static>.
///
/// # Expected syntax
///
/// ```text
/// lazy_static! {
///     [pub] static ref NAME_1: TYPE_1 = EXPR_1;
///     ...
///     [pub] static ref NAME_N: TYPE_N = EXPR_N;
/// }
/// ```
fn format_lazy_static(
    context: &RewriteContext<'_>,
    shape: Shape,
    tt: &ast::TokenTree,
    macro_name: &str,
) -> LazyStatic {
    let Some(parsed_elems) = parse_lazy_static(context, tt) else {
        return LazyStatic::NotParsed;
    };
    if parsed_elems.is_empty() {
        return LazyStatic::NotParsed;
    }
    let rewrite = || -> Option<String> {
        let mut result = String::with_capacity(1024);
        let nested_shape = shape
            .block_indent(context.config.tab_spaces())
            .with_max_width(context.config);

        result.push_str(macro_name);
        result.push_str(" {");
        result.push_str(&nested_shape.indent.to_string_with_newline(context.config));

        let last = parsed_elems.len() - 1;
        for (i, elem) in parsed_elems.iter().enumerate() {
            // Rewrite as a static item.
            let stmt = format!(
                "{}static ref {}: {} =",
                elem.vis,
                elem.name,
                elem.ty.rewrite(context, nested_shape)?
            );
            result.push_str(&rewrite_assign_rhs(
                context,
                stmt,
                &elem.expr,
                &RhsAssignKind::Expr,
                nested_shape.sub_width(1)?,
            )?);
            result.push(';');
            if i != last {
                result.push_str(&nested_shape.indent.to_string_with_newline(context.config));
            }
        }

        result.push_str(&shape.indent.to_string_with_newline(context.config));
        result.push('}');
        Some(result)
    };
    match rewrite() {
        Some(rw) => LazyStatic::Formatted(rw),
        None => LazyStatic::Failed,
    }
}

#[allow(clippy::too_many_arguments)]
fn rewrite_macro_with_items(
    context: &RewriteContext<'_>,
    items: &[MacroArg],
    macro_name: &str,
    shape: Shape,
    style: Delimiter,
    original_style: Delimiter,
    position: MacroPosition,
    span: Span,
) -> Option<String> {
    let style_to_delims = |style| match style {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::Brace => (" {", "}"),
    };

    let (opener, closer) = style_to_delims(style);
    let (original_opener, _) = style_to_delims(original_style);
    let trailing_semicolon = match style {
        Delimiter::Parenthesis | Delimiter::Bracket if position == MacroPosition::Item => ";",
        _ => "",
    };

    let mut visitor = FmtVisitor::from_context(context);
    visitor.block_indent = shape.indent.block_indent(context.config);

    // The current opener may be different from the original opener. This can happen
    // if our macro is a forced bracket macro originally written with non-bracket
    // delimiters. We need to use the original opener to locate the span after it.
    visitor.last_pos = context
        .snippet_provider
        .span_after(span, original_opener.trim());
    for item in items {
        let MacroArg::Item(item) = item else {
            return None;
        };
        visitor.visit_item(item);
    }

    let mut result = String::with_capacity(256);
    result.push_str(macro_name);
    result.push_str(opener);
    result.push_str(&visitor.block_indent.to_string_with_newline(context.config));
    result.push_str(visitor.buffer.trim());
    result.push_str(&shape.indent.to_string_with_newline(context.config));
    result.push_str(closer);
    result.push_str(trailing_semicolon);
    Some(result)
}

// ---------------------------------------------------------------------------------------
// Macro definitions
// ---------------------------------------------------------------------------------------

/// A `macro_rules!` or macros 2.0 definition.
pub(crate) enum MacroDef {
    Rules(ast::MacroRules),
    Def(ast::MacroDef),
}

impl MacroDef {
    fn is_macro_rules(&self) -> bool {
        matches!(self, MacroDef::Rules(..))
    }

    fn name(&self) -> Option<ast::Name> {
        match self {
            MacroDef::Rules(m) => m.name(),
            MacroDef::Def(m) => m.name(),
        }
    }

    fn visibility(&self) -> Option<ast::Visibility> {
        match self {
            MacroDef::Rules(m) => m.visibility(),
            MacroDef::Def(m) => m.visibility(),
        }
    }
}

fn rewrite_empty_macro_def_body(
    context: &RewriteContext<'_>,
    span: Span,
    shape: Shape,
) -> Option<String> {
    // An empty block representing an empty macro body.
    let block = Block {
        stmts: Vec::new(),
        span,
        rules: BlockRules::Default,
        stmt_list: None,
    };
    block.rewrite(context, shape)
}

pub(crate) fn rewrite_macro_def(
    context: &RewriteContext<'_>,
    shape: Shape,
    indent: Indent,
    def: &MacroDef,
    span: Span,
) -> Option<String> {
    let snippet = remove_trailing_white_spaces(context.snippet(span));
    if snippet.ends_with(';') {
        return Some(snippet);
    }

    let Some(parsed_def) = parse_macro_def(def) else {
        return Some(snippet);
    };

    let mut result = if def.is_macro_rules() {
        String::from("macro_rules!")
    } else {
        format!("{}macro", format_visibility(def.visibility().as_ref()))
    };

    result += " ";
    result += &node_text(def.name()?.syntax());

    let multi_branch_style = def.is_macro_rules() || parsed_def.len() != 1;

    let arm_shape = if multi_branch_style {
        shape
            .block_indent(context.config.tab_spaces())
            .with_max_width(context.config)
    } else {
        shape
    };

    if parsed_def.is_empty() {
        let lo = context.snippet_provider.span_before(span, "{");
        result += " ";
        result += &rewrite_empty_macro_def_body(context, span.with_lo(lo), shape)?;
        return Some(result);
    }

    let branch_items = itemize_list(
        context.snippet_provider,
        parsed_def.iter(),
        "}",
        ";",
        |branch| branch.span.lo(),
        |branch| branch.span.hi(),
        |branch| branch.rewrite(context, arm_shape, multi_branch_style),
        context.snippet_provider.span_after(span, "{"),
        span.hi(),
        false,
    )
    .collect::<Vec<_>>();

    let fmt = ListFormatting::new(arm_shape, context.config)
        .separator(if def.is_macro_rules() { ";" } else { "" })
        .trailing_separator(SeparatorTactic::Always)
        .preserve_newline(true);

    if multi_branch_style {
        result += " {";
        result += &arm_shape.indent.to_string_with_newline(context.config);
    }

    match write_list(&branch_items, &fmt) {
        Some(ref s) => result += s,
        None => return Some(snippet),
    }

    if multi_branch_style {
        result += &indent.to_string_with_newline(context.config);
        result += "}";
    }

    Some(result)
}

/// `true` for comments that rustc keeps as doc comment tokens inside token trees.
fn is_doc_comment(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMENT
        && ast::Comment::cast(token.clone()).is_some_and(|c| c.kind().doc.is_some())
}

/// Splits a macro definition into its branches (rustfmt's `MacroParser`):
/// `(` ... `)` `=>` `{` ... `}` `;`?, repeated. `None` if the body has another shape.
fn parse_macro_def(def: &MacroDef) -> Option<Vec<MacroBranch>> {
    let body = match def {
        MacroDef::Rules(m) => m.token_tree()?,
        MacroDef::Def(m) => {
            let body = m.body()?;
            if let Some(args) = m.args() {
                // `macro name(args) { body }` is a single branch.
                let args_span = node_range_span(args.syntax());
                let body_span = node_range_span(body.syntax());
                return Some(vec![MacroBranch::new(
                    args.syntax(),
                    body.syntax(),
                    mk_sp(args_span.lo(), body_span.hi()),
                )]);
            }
            body
        }
    };

    // The token trees and tokens between the outer delimiters, as rustc sees them.
    let elements: Vec<_> = body
        .syntax()
        .children_with_tokens()
        .filter(|el| match el {
            NodeOrToken::Token(t) => !t.kind().is_trivia() || is_doc_comment(t),
            NodeOrToken::Node(_) => true,
        })
        .collect();
    let inner = elements.get(1..elements.len().checked_sub(1)?)?;

    let mut branches = Vec::new();
    let mut iter = inner.iter().peekable();
    while iter.peek().is_some() {
        let args = iter.next()?.as_node()?.clone();
        let eq = iter.next()?.as_token()?.clone();
        let gt = iter.next()?.as_token()?.clone();
        if eq.kind() != T![=]
            || gt.kind() != T![>]
            || eq.text_range().end() != gt.text_range().start()
        {
            return None;
        }
        let body = iter.next()?.as_node()?.clone();
        let mut hi = node_range_span(&body).hi();
        if let Some(NodeOrToken::Token(semi)) = iter.peek()
            && semi.kind() == T![;]
        {
            hi = token_span(semi).hi();
            iter.next();
        }
        let lo = node_range_span(&args).lo();
        branches.push(MacroBranch::new(&args, &body, mk_sp(lo, hi)));
    }
    Some(branches)
}

struct MacroBranch {
    span: Span,
    args_paren_kind: Option<SyntaxKind>,
    /// The matcher, delimiters included.
    args: Span,
    /// The transcriber, delimiters excluded.
    body: Span,
    /// The transcriber, delimiters included.
    whole_body: Span,
}

impl MacroBranch {
    fn new(args: &SyntaxNode, body: &SyntaxNode, span: Span) -> MacroBranch {
        let whole_body = node_range_span(body);
        MacroBranch {
            span,
            args_paren_kind: args.first_token().map(|t| t.kind()),
            args: node_range_span(args),
            body: mk_sp(whole_body.lo() + 1, whole_body.hi().saturating_sub(1)),
            whole_body,
        }
    }

    fn rewrite(
        &self,
        context: &RewriteContext<'_>,
        shape: Shape,
        multi_branch_style: bool,
    ) -> Option<String> {
        // Only attempt to format function-like macros.
        if self.args_paren_kind != Some(T!['(']) {
            return None;
        }

        let old_body = context.snippet(self.body).trim();
        let has_block_body = old_body.starts_with('{');
        let mut prefix_width = 5; // 5 = " => {"
        if context.config.style_edition() >= StyleEdition::Edition2024 && has_block_body {
            prefix_width = 6; // 6 = " => {{"
        }
        // With `format_macro_matchers` off the matcher is kept as written. rustfmt still
        // computes the shape, which fails when the prefix does not fit.
        debug_assert!(!context.config.format_macro_matchers());
        shape.sub_width(prefix_width)?;
        let mut result = context.snippet(self.args).to_owned();

        if multi_branch_style {
            result += " =>";
        }

        if !context.config.format_macro_bodies() {
            result += " ";
            result += context.snippet(self.whole_body);
            return Some(result);
        }

        // The macro body is the most interesting part. It might end up as various
        // AST nodes, but also has special variables (e.g, `$foo`) which can't be
        // parsed as regular Rust code (and note that these can be escaped using
        // `$$`). We'll try and format like an AST node, but we'll substitute
        // variables for new names with the same length first.
        let (body_str, substs) = replace_names(old_body)?;

        result += " {";

        let body_indent = if has_block_body {
            shape.indent
        } else {
            shape.indent.block_indent(context.config)
        };
        let new_width = context
            .config
            .max_width()
            .saturating_sub(body_indent.width());
        let config = context.config.with_max_width(new_width);

        // First try to format as items, then as statements.
        let (new_body_snippet, config) = match format_snippet(&body_str, &config, true) {
            Some(new_body) => (new_body, config),
            None => {
                let config = config.with_max_width(new_width + config.tab_spaces());
                (format_code_block(&body_str, &config, true)?, config)
            }
        };

        if !filtered_str_fits(&new_body_snippet.snippet, config.max_width(), shape) {
            return None;
        }

        // Indent the body since it is in a block.
        let indent_str = body_indent.to_string(&config);
        let mut new_body = String::new();
        let mut need_indent = true;
        for (i, (kind, l)) in LineClasses::new(new_body_snippet.snippet.trim_end()).enumerate() {
            if !is_empty_line(l) && need_indent && !new_body_snippet.is_line_non_formatted(i + 1) {
                new_body += &indent_str;
            }
            new_body += l;
            new_body.push('\n');
            need_indent = indent_next_line(kind, l, &config);
        }

        // Undo our replacement of macro variables.
        for (old, new) in &substs {
            if old_body.contains(new.as_str()) {
                return None;
            }
            new_body = new_body.replace(new.as_str(), old);
        }

        if has_block_body {
            result += new_body.trim();
        } else if !new_body.is_empty() {
            result += "\n";
            result += &new_body;
            result += &shape.indent.to_string(&config);
        }

        result += "}";

        Some(result)
    }
}

fn register_metavariable(
    substs: &mut Vec<(String, String)>,
    result: &mut String,
    name: &str,
    dollar_count: usize,
) {
    let mut new_name = "$".repeat(dollar_count - 1);
    let mut old_name = "$".repeat(dollar_count);

    new_name.push('z');
    new_name.push_str(name);
    old_name.push_str(name);

    result.push_str(&new_name);
    // rustfmt keeps the substitutions in a `HashMap`, so a repeated metavariable is
    // replaced back once; a `Vec` with unique keys behaves the same and is ordered.
    if !substs.iter().any(|(old, _)| *old == old_name) {
        substs.push((old_name, new_name));
    }
}

/// Replaces `$foo` with `zfoo`. Escaped `$` variables keep their earlier `$`s.
fn replace_names(input: &str) -> Option<(String, Vec<(String, String)>)> {
    // Each substitution will require five or six extra bytes.
    let mut result = String::with_capacity(input.len() + 64);
    let mut substs = Vec::new();
    let mut dollar_count = 0;
    let mut cur_name = String::new();

    for (kind, _, c) in CharClasses::new(input) {
        if kind != FullCodeCharKind::Normal {
            result.push(c);
        } else if c == '$' {
            dollar_count += 1;
        } else if dollar_count == 0 {
            result.push(c);
        } else if !c.is_alphanumeric() && !cur_name.is_empty() {
            // Terminates a name following one or more dollars.
            register_metavariable(&mut substs, &mut result, &cur_name, dollar_count);

            result.push(c);
            dollar_count = 0;
            cur_name.clear();
        } else if c == '(' && cur_name.is_empty() {
            // Macro definitions with repetitions are not supported.
            return None;
        } else if c.is_alphanumeric() || c == '_' {
            cur_name.push(c);
        }
    }

    if !cur_name.is_empty() {
        register_metavariable(&mut substs, &mut result, &cur_name, dollar_count);
    }

    Some((result, substs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_names_substitutes_metavariables() {
        // As in rustfmt, `_` ends a metavariable name: `$a_c` is `$a` followed by `_c`.
        let (body, substs) = replace_names("$a + $$b * $a_c").unwrap();
        assert_eq!(body, "za + $zb * za_c");
        assert_eq!(
            substs,
            [
                ("$a".to_owned(), "za".to_owned()),
                ("$$b".to_owned(), "$zb".to_owned()),
            ]
        );
    }

    #[test]
    fn replace_names_rejects_repetitions() {
        assert!(replace_names("$($x),*").is_none());
    }

    #[test]
    fn replace_names_ignores_strings_and_comments() {
        let (body, substs) = replace_names("\"$x\" // $y\n").unwrap();
        assert_eq!(body, "\"$x\" // $y\n");
        assert!(substs.is_empty());
    }
}
