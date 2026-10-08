//! Formatting of chained expressions, i.e., expressions that are chained by
//! dots: struct and enum field access, method calls, and try shorthand (`?`)
//! (rustfmt's `chains.rs`).
//!
//! Instead of walking these subexpressions one-by-one, as is our usual strategy
//! for expression formatting, we collect maximal sequences of these expressions
//! and handle them simultaneously.
//!
//! Whenever possible, the entire chain is put on a single line. If that fails,
//! we put each subexpression on a separate, much like the (default) function
//! argument function argument strategy.
//!
//! ```text
//! let a = foo.bar
//!     .baz()
//!     .qux
//! ```

use std::borrow::Cow;
use std::cmp::min;

use ra_ap_syntax::ast::{self, AstNode, HasArgList, HasGenericArgs, RangeItem};

use super::comment::{CharClasses, FullCodeCharKind, rewrite_comment};
use super::config::StyleEdition;
use super::context::{Rewrite, RewriteContext};
use super::expr::{expr_span, lit_ends_in_dot, rewrite_call};
use super::lists::extract_pre_comment;
use super::macros::convert_try_mac;
use super::overflow::OverflowableItem;
use super::shape::Shape;
use super::span::{BytePos, Span, Spanned, mk_sp};
use super::types::SegmentParam;
use super::utils::{
    first_line_width, last_line_extendable, last_line_width, unicode_str_width, wrap_str,
};

/// Provides the original input contents from the span
/// of a chain element with trailing spaces trimmed.
fn format_overflow_style(span: Span, context: &RewriteContext<'_>) -> Option<String> {
    context.snippet_provider.span_to_snippet(span).map(|s| {
        s.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn format_chain_item(
    item: &ChainItem,
    context: &RewriteContext<'_>,
    rewrite_shape: Shape,
    allow_overflow: bool,
) -> Option<String> {
    if allow_overflow {
        item.rewrite(context, rewrite_shape)
            .or_else(|| format_overflow_style(item.span, context))
    } else {
        item.rewrite(context, rewrite_shape)
    }
}

fn get_block_child_shape(
    prev_ends_with_block: bool,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Shape {
    if prev_ends_with_block {
        shape.block_indent(0)
    } else {
        shape.block_indent(context.config.tab_spaces())
    }
    .with_max_width(context.config)
}

pub(crate) fn rewrite_chain(
    expr: &ast::Expr,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    rewrite_chain_from(SubExpr::new(expr.clone(), false), context, shape)
}

/// Rewrites `inner?`, where the `?` stands for a `try!(inner)` invocation that
/// `use_try_shorthand` converts. rustc builds a synthetic `Try` node for this; the
/// syntax tree here is immutable, so the conversion is recorded on the [`SubExpr`].
pub(crate) fn rewrite_try_shorthand(
    inner: &ast::Expr,
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    let top = SubExpr {
        expr: inner.clone(),
        is_method_call_receiver: false,
        synthetic_try: true,
    };
    rewrite_chain_from(top, context, shape)
}

fn rewrite_chain_from(top: SubExpr, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
    let chain = Chain::from_ast(top, context)?;

    // If this is just an expression with some `?`s, then format it trivially and
    // return early.
    if chain.children.is_empty() {
        return chain.parent.rewrite(context, shape);
    }

    chain.rewrite(context, shape)
}

#[derive(Debug)]
enum CommentPosition {
    Back,
    Top,
}

/// Information about an expression in a chain.
struct SubExpr {
    expr: ast::Expr,
    is_method_call_receiver: bool,
    /// `true` when this entry is the `?` of a converted `try!(expr)`; `expr` then holds
    /// the macro argument, which is also the next entry of the list.
    synthetic_try: bool,
}

impl SubExpr {
    fn new(expr: ast::Expr, is_method_call_receiver: bool) -> SubExpr {
        SubExpr {
            expr,
            is_method_call_receiver,
            synthetic_try: false,
        }
    }

    fn is_try(&self) -> bool {
        self.synthetic_try || matches!(self.expr, ast::Expr::TryExpr(_))
    }
}

/// An expression plus trailing `?`s to be formatted together.
#[derive(Debug)]
struct ChainItem {
    kind: ChainItemKind,
    tries: usize,
    span: Span,
}

#[derive(Debug)]
enum ChainItemKind {
    Parent { expr: ast::Expr, parens: bool },
    MethodCall(ast::NameRef, Vec<SegmentParam>, Vec<ast::Expr>),
    StructField(ast::NameRef),
    TupleField(ast::NameRef, bool),
    Await,
    Comment(String, CommentPosition),
}

impl ChainItemKind {
    fn is_block_like(&self, context: &RewriteContext<'_>, reps: &str) -> bool {
        match self {
            ChainItemKind::Parent { expr, .. } => super::utils::is_block_expr(context, expr, reps),
            _ => false,
        }
    }

    fn is_tup_field_access(expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::FieldExpr(f) => f
                .name_ref()
                .is_some_and(|n| n.text().chars().all(|c| c.is_ascii_digit())),
            _ => false,
        }
    }

    fn from_ast(
        context: &RewriteContext<'_>,
        expr: &ast::Expr,
        is_method_call_receiver: bool,
    ) -> Option<(ChainItemKind, Span)> {
        let full = expr_span(expr);
        let (kind, span) = match expr {
            ast::Expr::MethodCallExpr(call) => {
                let types = call
                    .generic_arg_list()
                    .map(|args| {
                        args.generic_args()
                            .filter(|a| !matches!(a, ast::GenericArg::AssocTypeArg(_)))
                            .filter_map(|a| SegmentParam::from_generic_arg(&a))
                            .collect()
                    })
                    .unwrap_or_default();
                let receiver = call.receiver()?;
                let span = mk_sp(expr_span(&receiver).hi(), full.hi());
                let args = call.arg_list()?.args().collect();
                (
                    ChainItemKind::MethodCall(call.name_ref()?, types, args),
                    span,
                )
            }
            ast::Expr::FieldExpr(field) => {
                let nested = field.expr()?;
                let name = field.name_ref()?;
                let kind = if Self::is_tup_field_access(expr) {
                    ChainItemKind::TupleField(name.clone(), Self::is_tup_field_access(&nested))
                } else {
                    ChainItemKind::StructField(name.clone())
                };
                let span = mk_sp(expr_span(&nested).hi(), name.span().hi());
                (kind, span)
            }
            ast::Expr::AwaitExpr(a) => {
                let nested = a.expr()?;
                let span = mk_sp(expr_span(&nested).hi(), full.hi());
                (ChainItemKind::Await, span)
            }
            _ => {
                return Some((
                    ChainItemKind::Parent {
                        expr: expr.clone(),
                        parens: is_method_call_receiver && should_add_parens(expr),
                    },
                    full,
                ));
            }
        };

        // Remove comments from the span.
        let lo = context.snippet_provider.span_before(span, ".");
        Some((kind, mk_sp(lo, span.hi())))
    }
}

impl Rewrite for ChainItem {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let shape = shape.sub_width(self.tries)?;
        let rewrite = match self.kind {
            ChainItemKind::Parent {
                ref expr,
                parens: true,
            } => {
                // rustfmt's `rewrite_paren` on a non-parenthesised expression: `(expr)`.
                let inner = expr.rewrite(context, shape.offset_left(1)?.sub_width(1)?)?;
                format!("({inner})")
            }
            ChainItemKind::Parent {
                ref expr,
                parens: false,
            } => expr.rewrite(context, shape)?,
            ChainItemKind::MethodCall(ref name, ref types, ref exprs) => {
                Self::rewrite_method_call(name, types, exprs, self.span, context, shape)?
            }
            ChainItemKind::StructField(ref ident) => format!(".{}", ident.syntax().text()),
            ChainItemKind::TupleField(ref ident, nested) => format!(
                "{}.{}",
                if nested && context.config.style_edition() <= StyleEdition::Edition2021 {
                    " "
                } else {
                    ""
                },
                ident.syntax().text()
            ),
            ChainItemKind::Await => ".await".to_owned(),
            ChainItemKind::Comment(ref comment, _) => {
                rewrite_comment(comment, false, shape, context.config)?
            }
        };
        Some(format!("{rewrite}{}", "?".repeat(self.tries)))
    }
}

impl ChainItem {
    fn new(context: &RewriteContext<'_>, expr: &SubExpr, tries: usize) -> Option<ChainItem> {
        let (kind, span) =
            ChainItemKind::from_ast(context, &expr.expr, expr.is_method_call_receiver)?;
        Some(ChainItem { kind, tries, span })
    }

    fn comment(span: Span, comment: String, pos: CommentPosition) -> ChainItem {
        ChainItem {
            kind: ChainItemKind::Comment(comment, pos),
            tries: 0,
            span,
        }
    }

    fn is_comment(&self) -> bool {
        matches!(self.kind, ChainItemKind::Comment(..))
    }

    fn rewrite_method_call(
        method_name: &ast::NameRef,
        types: &[SegmentParam],
        args: &[ast::Expr],
        span: Span,
        context: &RewriteContext<'_>,
        shape: Shape,
    ) -> Option<String> {
        let type_str = if types.is_empty() {
            String::new()
        } else {
            let type_list = types
                .iter()
                .map(|ty| ty.rewrite(context, shape))
                .collect::<Option<Vec<_>>>()?;

            format!("::<{}>", type_list.join(", "))
        };
        let callee_str = format!(".{}{}", method_name.syntax().text(), type_str);
        let args = args.iter().cloned().map(OverflowableItem::Expr).collect();
        rewrite_call(context, &callee_str, args, span, shape)
    }
}

#[derive(Debug)]
struct Chain {
    parent: ChainItem,
    children: Vec<ChainItem>,
}

impl Chain {
    fn from_ast(top: SubExpr, context: &RewriteContext<'_>) -> Option<Chain> {
        let subexpr_list = Self::make_subexpr_list(top, context);

        // Un-parse the expression tree into ChainItems
        let mut rev_children = vec![];
        let mut sub_tries = 0;
        for subexpr in &subexpr_list {
            if subexpr.is_try() {
                sub_tries += 1;
            } else {
                rev_children.push(ChainItem::new(context, subexpr, sub_tries)?);
                sub_tries = 0;
            }
        }

        fn is_tries(s: &str) -> bool {
            s.chars().all(|c| c == '?')
        }

        fn is_post_comment(s: &str) -> bool {
            let Some(comment_start_index) = s.chars().position(|c| c == '/') else {
                return false;
            };
            match s.chars().position(|c| c == '\n') {
                None => true,
                Some(newline_index) => comment_start_index < newline_index,
            }
        }

        fn handle_post_comment(
            post_comment_span: Span,
            post_comment_snippet: &str,
            prev_span_end: &mut BytePos,
            children: &mut Vec<ChainItem>,
        ) {
            let white_spaces: &[_] = &[' ', '\t'];
            if post_comment_snippet
                .trim_matches(white_spaces)
                .starts_with('\n')
            {
                // No post comment.
                return;
            }
            let trimmed_snippet = trim_tries(post_comment_snippet);
            if is_post_comment(&trimmed_snippet) {
                children.push(ChainItem::comment(
                    post_comment_span,
                    trimmed_snippet.trim().to_owned(),
                    CommentPosition::Back,
                ));
                *prev_span_end = post_comment_span.hi();
            }
        }

        let parent = rev_children.pop()?;
        let mut children = vec![];
        let mut prev_span_end = parent.span.hi();
        let mut iter = rev_children.into_iter().rev().peekable();
        if let Some(first_chain_item) = iter.peek() {
            let comment_span = mk_sp(prev_span_end, first_chain_item.span.lo());
            let comment_snippet = context.snippet(comment_span);
            if !is_tries(comment_snippet.trim()) {
                handle_post_comment(
                    comment_span,
                    comment_snippet,
                    &mut prev_span_end,
                    &mut children,
                );
            }
        }
        while let Some(chain_item) = iter.next() {
            let comment_snippet = context.snippet(chain_item.span);
            // FIXME: Figure out the way to get a correct span when converting `try!` to `?`.
            let handle_comment =
                !(context.config.use_try_shorthand() || is_tries(comment_snippet.trim()));

            // Pre-comment
            if handle_comment {
                let pre_comment_span = mk_sp(prev_span_end, chain_item.span.lo());
                let pre_comment_snippet = trim_tries(context.snippet(pre_comment_span));
                let (pre_comment, _) = extract_pre_comment(&pre_comment_snippet);
                match pre_comment {
                    Some(ref comment) if !comment.is_empty() => {
                        children.push(ChainItem::comment(
                            pre_comment_span,
                            comment.to_owned(),
                            CommentPosition::Top,
                        ));
                    }
                    _ => (),
                }
            }

            prev_span_end = chain_item.span.hi();
            children.push(chain_item);

            // Post-comment
            if !handle_comment || iter.peek().is_none() {
                continue;
            }

            let next_lo = iter.peek()?.span.lo();
            let post_comment_span = mk_sp(prev_span_end, next_lo);
            let post_comment_snippet = context.snippet(post_comment_span);
            handle_post_comment(
                post_comment_span,
                post_comment_snippet,
                &mut prev_span_end,
                &mut children,
            );
        }

        Some(Chain { parent, children })
    }

    /// Returns a Vec of the prefixes of the chain.
    /// E.g., for input `a.b.c` we return [`a.b.c`, `a.b`, 'a']
    fn make_subexpr_list(top: SubExpr, context: &RewriteContext<'_>) -> Vec<SubExpr> {
        let mut subexpr_list = vec![top];

        while let Some(subexpr) = subexpr_list
            .last()
            .and_then(|last| Self::pop_expr_chain(last, context))
        {
            subexpr_list.push(subexpr);
        }

        subexpr_list
    }

    /// Returns the expression's subexpression, if it exists. When the subexpr
    /// is a try! macro, we'll convert it to shorthand when the option is set.
    fn pop_expr_chain(expr: &SubExpr, context: &RewriteContext<'_>) -> Option<SubExpr> {
        if expr.synthetic_try {
            return Some(Self::convert_try(expr.expr.clone(), false, context));
        }
        match &expr.expr {
            ast::Expr::MethodCallExpr(call) => {
                Some(Self::convert_try(call.receiver()?, true, context))
            }
            ast::Expr::FieldExpr(f) => Some(Self::convert_try(f.expr()?, false, context)),
            ast::Expr::TryExpr(t) => Some(Self::convert_try(t.expr()?, false, context)),
            ast::Expr::AwaitExpr(a) => Some(Self::convert_try(a.expr()?, false, context)),
            _ => None,
        }
    }

    fn convert_try(
        expr: ast::Expr,
        is_method_call_receiver: bool,
        context: &RewriteContext<'_>,
    ) -> SubExpr {
        if let ast::Expr::MacroExpr(mac) = &expr
            && context.config.use_try_shorthand()
            && let Some(inner) = mac
                .macro_call()
                .and_then(|call| convert_try_mac(&call, context))
        {
            return SubExpr {
                expr: inner,
                is_method_call_receiver,
                synthetic_try: true,
            };
        }
        SubExpr::new(expr, is_method_call_receiver)
    }
}

impl Rewrite for Chain {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let mut formatter = ChainFormatterBlock::new(self);

        formatter.format_root(&self.parent, context, shape)?;
        if let Some(result) = formatter.pure_root() {
            return wrap_str(result, context.config.max_width(), shape);
        }

        // Decide how to layout the rest of the chain.
        let child_shape = formatter.child_shape(context, shape);

        formatter.format_children(context, child_shape)?;
        formatter.format_last_child(context, shape, child_shape)?;

        let result = formatter.join_rewrites(context, child_shape)?;
        wrap_str(result, context.config.max_width(), shape)
    }
}

/// Data and behaviour that is shared by both chain formatters. The concrete
/// formatters can delegate much behaviour to `ChainFormatterShared`.
struct ChainFormatterShared<'a> {
    /// The current working set of child items.
    children: &'a [ChainItem],
    /// The current rewrites of items (includes trailing `?`s, but not any way to
    /// connect the rewrites together).
    rewrites: Vec<String>,
    /// Whether the chain can fit on one line.
    fits_single_line: bool,
    /// The number of children in the chain. This is not equal to `self.children.len()`
    /// because `self.children` will change size as we process the chain.
    child_count: usize,
    /// Whether elements are allowed to overflow past the max_width limit
    allow_overflow: bool,
}

impl<'a> ChainFormatterShared<'a> {
    fn new(chain: &'a Chain) -> ChainFormatterShared<'a> {
        ChainFormatterShared {
            children: &chain.children,
            rewrites: Vec::with_capacity(chain.children.len() + 1),
            fits_single_line: false,
            child_count: chain.children.len(),
            allow_overflow: false,
        }
    }

    fn pure_root(&mut self) -> Option<String> {
        if self.children.is_empty() {
            debug_assert_eq!(self.rewrites.len(), 1);
            self.rewrites.pop()
        } else {
            None
        }
    }

    fn format_children(&mut self, context: &RewriteContext<'_>, child_shape: Shape) -> Option<()> {
        for item in &self.children[..self.children.len() - 1] {
            let rewrite = format_chain_item(item, context, child_shape, self.allow_overflow)?;
            self.rewrites.push(rewrite);
        }
        Some(())
    }

    /// Rewrite the last child. The last child of a chain requires special treatment. We need to
    /// know whether 'overflowing' the last child make a better formatting:
    ///
    /// A chain with overflowing the last child:
    /// ```text
    /// parent.child1.child2.last_child(
    ///     a,
    ///     b,
    ///     c,
    /// )
    /// ```
    ///
    /// A chain without overflowing the last child (in vertical layout):
    /// ```text
    /// parent
    ///     .child1
    ///     .child2
    ///     .last_child(a, b, c)
    /// ```
    ///
    /// In particular, overflowing is effective when the last child is a method with a multi-lined
    /// block-like argument (e.g., closure):
    /// ```text
    /// parent.child1.child2.last_child(|a, b, c| {
    ///     let x = foo(a, b, c);
    ///     let y = bar(a, b, c);
    ///
    ///     // ...
    ///
    ///     result
    /// })
    /// ```
    fn format_last_child(
        &mut self,
        may_extend: bool,
        context: &RewriteContext<'_>,
        shape: Shape,
        child_shape: Shape,
    ) -> Option<()> {
        let last = self.children.last()?;
        let extendable = may_extend && last_line_extendable(&self.rewrites[0]);
        let prev_last_line_width = last_line_width(&self.rewrites[0]);

        // Total of all items excluding the last.
        let almost_total = if extendable {
            prev_last_line_width
        } else {
            self.rewrites.iter().map(|rw| unicode_str_width(rw)).sum()
        } + last.tries;
        let one_line_budget = if self.child_count == 1 {
            shape.width
        } else {
            min(shape.width, context.config.chain_width())
        }
        .saturating_sub(almost_total);

        let all_in_one_line = !self.children.iter().any(ChainItem::is_comment)
            && self.rewrites.iter().all(|s| !s.contains('\n'))
            && one_line_budget > 0;
        let last_shape = if all_in_one_line {
            shape.sub_width(last.tries)?
        } else if extendable {
            child_shape.sub_width(last.tries)?
        } else {
            child_shape.sub_width(shape.rhs_overhead(context.config) + last.tries)?
        };

        let mut last_subexpr_str = None;
        if all_in_one_line || extendable {
            // First we try to 'overflow' the last child and see if it looks better than using
            // vertical layout.
            let one_line_shape = last_shape.offset_left(almost_total);

            if let Some(one_line_shape) = one_line_shape
                && let Some(rw) = last.rewrite(context, one_line_shape)
            {
                // We allow overflowing here only if both of the following conditions match:
                // 1. The entire chain fits in a single line except the last child.
                // 2. `last_child_str.lines().count() >= 5`.
                let line_count = rw.lines().count();
                let could_fit_single_line = first_line_width(&rw) <= one_line_budget;
                if could_fit_single_line && line_count >= 5 {
                    last_subexpr_str = Some(rw);
                    self.fits_single_line = all_in_one_line;
                } else {
                    // We could not know whether overflowing is better than using vertical
                    // layout, just by looking at the overflowed rewrite. Now we rewrite the
                    // last child on its own line, and compare two rewrites to choose which is
                    // better.
                    let last_shape =
                        child_shape.sub_width(shape.rhs_overhead(context.config) + last.tries)?;
                    match last.rewrite(context, last_shape) {
                        Some(ref new_rw) if !could_fit_single_line => {
                            last_subexpr_str = Some(new_rw.clone());
                        }
                        Some(ref new_rw) if new_rw.lines().count() >= line_count => {
                            last_subexpr_str = Some(rw);
                            self.fits_single_line = could_fit_single_line && all_in_one_line;
                        }
                        Some(new_rw) => {
                            last_subexpr_str = Some(new_rw);
                        }
                        _ => {
                            last_subexpr_str = Some(rw);
                            self.fits_single_line = could_fit_single_line && all_in_one_line;
                        }
                    }
                }
            }
        }

        let last_subexpr_str = match last_subexpr_str {
            Some(s) => s,
            None => last.rewrite(context, last_shape)?,
        };
        self.rewrites.push(last_subexpr_str);
        Some(())
    }

    fn join_rewrites(&self, context: &RewriteContext<'_>, child_shape: Shape) -> Option<String> {
        let connector = if self.fits_single_line {
            // Yay, we can put everything on one line.
            Cow::from("")
        } else {
            // Use new lines.
            if context.force_one_line_chain.get() {
                return None;
            }
            child_shape.to_string_with_newline(context.config)
        };

        let mut rewrite_iter = self.rewrites.iter();
        let mut result = rewrite_iter.next()?.clone();
        let children_iter = self.children.iter();
        let iter = rewrite_iter.zip(children_iter);

        for (rewrite, chain_item) in iter {
            match chain_item.kind {
                ChainItemKind::Comment(_, CommentPosition::Back) => result.push(' '),
                ChainItemKind::Comment(_, CommentPosition::Top) => result.push_str(&connector),
                _ => result.push_str(&connector),
            }
            result.push_str(rewrite);
        }

        Some(result)
    }
}

/// Formats a chain using block indent.
struct ChainFormatterBlock<'a> {
    shared: ChainFormatterShared<'a>,
    root_ends_with_block: bool,
}

impl<'a> ChainFormatterBlock<'a> {
    fn new(chain: &'a Chain) -> ChainFormatterBlock<'a> {
        ChainFormatterBlock {
            shared: ChainFormatterShared::new(chain),
            root_ends_with_block: false,
        }
    }

    /// Parent is the first item in the chain, e.g., `foo` in `foo.bar.baz()`.
    /// Root is the parent plus any other chain items placed on the first line to
    /// avoid an orphan. E.g.,
    /// ```text
    /// foo.bar
    ///     .baz()
    /// ```
    /// If `bar` were not part of the root, then foo would be orphaned and 'float'.
    fn format_root(
        &mut self,
        parent: &ChainItem,
        context: &RewriteContext<'_>,
        shape: Shape,
    ) -> Option<()> {
        let mut root_rewrite: String = parent.rewrite(context, shape)?;

        let mut root_ends_with_block = parent.kind.is_block_like(context, &root_rewrite);
        let tab_width = context.config.tab_spaces().saturating_sub(shape.offset);

        while root_rewrite.len() <= tab_width && !root_rewrite.contains('\n') {
            let item = &self.shared.children[0];
            if let ChainItemKind::Comment(..) = item.kind {
                break;
            }
            let shape = shape.offset_left(root_rewrite.len())?;
            match &item.rewrite(context, shape) {
                Some(rewrite) => root_rewrite.push_str(rewrite),
                None => break,
            }

            root_ends_with_block = last_line_extendable(&root_rewrite);

            self.shared.children = &self.shared.children[1..];
            if self.shared.children.is_empty() {
                break;
            }
        }
        self.shared.rewrites.push(root_rewrite);
        self.root_ends_with_block = root_ends_with_block;
        Some(())
    }

    fn child_shape(&self, context: &RewriteContext<'_>, shape: Shape) -> Shape {
        get_block_child_shape(self.root_ends_with_block, context, shape)
    }

    fn format_children(&mut self, context: &RewriteContext<'_>, child_shape: Shape) -> Option<()> {
        self.shared.format_children(context, child_shape)
    }

    fn format_last_child(
        &mut self,
        context: &RewriteContext<'_>,
        shape: Shape,
        child_shape: Shape,
    ) -> Option<()> {
        self.shared
            .format_last_child(true, context, shape, child_shape)
    }

    fn join_rewrites(&self, context: &RewriteContext<'_>, child_shape: Shape) -> Option<String> {
        self.shared.join_rewrites(context, child_shape)
    }

    fn pure_root(&mut self) -> Option<String> {
        self.shared.pure_root()
    }
}

/// Removes try operators (`?`s) that appear in the given string. If removing
/// them leaves an empty line, remove that line as well unless it is the first
/// line (we need the first newline for detecting pre/post comment).
fn trim_tries(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut line_buffer = String::with_capacity(s.len());
    for (kind, _, c) in CharClasses::new(s) {
        match c {
            '\n' => {
                if result.is_empty() || !line_buffer.trim().is_empty() {
                    result.push_str(&line_buffer);
                    result.push('\n')
                }
                line_buffer.clear();
            }
            '?' if kind == FullCodeCharKind::Normal => continue,
            c => line_buffer.push(c),
        }
    }
    if !line_buffer.trim().is_empty() {
        result.push_str(&line_buffer);
    }
    result
}

/// Whether a method call's receiver needs parenthesis, like
/// ```rust,ignore
/// || .. .method();
/// || 1.. .method();
/// 1. .method();
/// ```
/// Which all need parenthesis or a space before `.method()`.
fn should_add_parens(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Literal(lit) => lit_ends_in_dot(lit),
        ast::Expr::ClosureExpr(cl) => match cl.body() {
            // Any half-open range (`..`, `a..`, `..b`, `a..b`).
            Some(ast::Expr::RangeExpr(r)) => matches!(r.op_kind(), Some(ast::RangeOp::Exclusive)),
            Some(ast::Expr::Literal(lit)) => lit_ends_in_dot(&lit),
            _ => false,
        },
        _ => false,
    }
}
