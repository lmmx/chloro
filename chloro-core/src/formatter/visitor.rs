//! The formatting visitor (rustfmt's `visitor.rs`).
//!
//! [`FmtVisitor`] walks items and statements in source order, appending formatted text to
//! `buffer`. `last_pos` is the source position up to which input has been consumed; the
//! text between `last_pos` and the next node (whitespace, comments, or code a rewrite
//! gave up on) is written by [`FmtVisitor::format_missing`] (see `missed_spans.rs`).

use std::cell::Cell;
use std::rc::Rc;

use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::ast::{self, AstNode, HasModuleItem, HasName, HasVisibility};

use super::comment::{CodeCharKind, CommentCodeSlices, contains_comment, rewrite_comment};
use super::config::{Settings, StyleEdition};
use super::context::{Rewrite, RewriteContext, RunState};
use super::items::{
    ItemVisitorKind, StaticParts, StructParts, format_impl, format_trait, format_trait_alias,
    is_mod_decl, is_use_item, item_span, rewrite_extern_crate, rewrite_type_alias,
};
use super::macros::{
    MacroDef, MacroPosition, mac_span, macro_style, rewrite_macro, rewrite_macro_def,
};
use super::nodes::{
    AttrStyle, Attribute, Block, StmtKind, contains_skip, inner_attributes, outer_attributes,
    span_without_attrs,
};
use super::overflow::Delimiter;
use super::shape::{Indent, Shape};
use super::span::{BytePos, SnippetProvider, Span, Spanned, mk_sp, node_range_span};
use super::stmt::Stmt;
use super::utils::{count_newlines, format_visibility, last_line_width, starts_with_newline};

pub(crate) struct FmtVisitor<'a> {
    /// Receives `macro_rewrite_failure` when the visitor is dropped (rustfmt's
    /// `parent_context`).
    parent_macro_failure: Option<&'a Cell<bool>>,
    pub(crate) buffer: String,
    pub(crate) last_pos: BytePos,
    pub(crate) block_indent: Indent,
    pub(crate) config: &'a Settings,
    pub(crate) is_if_else_block: bool,
    pub(crate) is_loop_block: bool,
    pub(crate) snippet_provider: SnippetProvider<'a>,
    pub(crate) line_number: usize,
    pub(crate) macro_rewrite_failure: bool,
    pub(crate) is_macro_def: bool,
    pub(crate) run: Rc<RunState>,
}

impl Drop for FmtVisitor<'_> {
    fn drop(&mut self) {
        if let Some(parent) = self.parent_macro_failure
            && self.macro_rewrite_failure
        {
            parent.set(true);
        }
    }
}

/// The outer and inner attributes of an item, as rustc's `item.attrs`.
fn item_attrs(item: &ast::Item) -> Vec<Attribute> {
    let mut attrs = outer_attributes(item.syntax());
    let container = match item {
        ast::Item::Module(m) => m.item_list().map(|l| l.syntax().clone()),
        ast::Item::Impl(i) => i.assoc_item_list().map(|l| l.syntax().clone()),
        ast::Item::Trait(t) => t.assoc_item_list().map(|l| l.syntax().clone()),
        ast::Item::ExternBlock(e) => e.extern_item_list().map(|l| l.syntax().clone()),
        ast::Item::Fn(f) => f
            .body()
            .and_then(|b| b.stmt_list())
            .map(|l| l.syntax().clone()),
        _ => None,
    };
    if let Some(container) = container {
        attrs.extend(inner_attributes(&container));
    }
    attrs
}

impl<'a> FmtVisitor<'a> {
    pub(crate) fn new(
        config: &'a Settings,
        snippet_provider: SnippetProvider<'a>,
        run: Rc<RunState>,
    ) -> FmtVisitor<'a> {
        FmtVisitor {
            parent_macro_failure: None,
            buffer: String::with_capacity(snippet_provider.entire_snippet().len() * 2),
            last_pos: 0,
            block_indent: Indent::empty(),
            config,
            is_if_else_block: false,
            is_loop_block: false,
            snippet_provider,
            line_number: 0,
            macro_rewrite_failure: false,
            is_macro_def: false,
            run,
        }
    }

    pub(crate) fn from_context(ctx: &'a RewriteContext<'_>) -> FmtVisitor<'a> {
        let mut visitor = FmtVisitor::new(ctx.config, ctx.snippet_provider, ctx.run.clone());
        visitor.buffer = String::with_capacity(256);
        visitor.is_macro_def = ctx.is_macro_def;
        visitor.parent_macro_failure = Some(&ctx.macro_rewrite_failure);
        visitor
    }

    pub(crate) fn shape(&self) -> Shape {
        Shape::indented(self.block_indent, self.config)
    }

    pub(crate) fn next_span(&self, hi: BytePos) -> Span {
        mk_sp(self.last_pos, hi)
    }

    pub(crate) fn snippet(&self, span: Span) -> &'a str {
        self.snippet_provider.snippet(span)
    }

    /// 1-based line number of `pos`.
    pub(crate) fn line_of(&self, pos: BytePos) -> usize {
        self.run
            .line_of(self.snippet_provider.entire_snippet(), pos)
    }

    fn visit_stmt(&mut self, stmt: &Stmt, include_empty_semi: bool) {
        if stmt.is_empty() {
            // If the statement is empty, just skip over it. Before that, make sure any comment
            // snippet preceding the semicolon is picked up.
            let snippet = self.snippet(mk_sp(self.last_pos, stmt.span().lo()));
            let original_starts_with_newline = snippet
                .find(|c| c != ' ')
                .is_some_and(|i| starts_with_newline(&snippet[i..]));
            let snippet = snippet.trim();
            if !snippet.is_empty() {
                // rustfmt preserves redundant semicolons on items in statement position.
                if include_empty_semi {
                    self.format_missing(stmt.span().hi());
                } else {
                    if original_starts_with_newline {
                        self.push_str("\n");
                    }

                    self.push_str(&self.block_indent.to_string(self.config));
                    self.push_str(snippet);
                }
            } else if include_empty_semi {
                self.push_str(";");
            }
            self.last_pos = stmt.span().hi();
            return;
        }

        let node = stmt.as_ast_node();
        match &node.kind {
            StmtKind::Item(item) => {
                self.visit_item(item);
                self.last_pos = stmt.span().hi();
            }
            StmtKind::Let(..) | StmtKind::Expr(..) | StmtKind::Semi(..) => {
                let attrs = node.attrs();
                if contains_skip(&attrs) {
                    let main_span = match &node.kind {
                        StmtKind::Let(l) => span_without_attrs(l.syntax()),
                        StmtKind::Expr(e) | StmtKind::Semi(e) => span_without_attrs(e.syntax()),
                        _ => stmt.span(),
                    };
                    self.push_skipped_with_span(&attrs, stmt.span(), main_span);
                } else {
                    let shape = self.shape();
                    let rewrite = self.with_context(|ctx| stmt.rewrite(ctx, shape));
                    self.push_rewrite(stmt.span(), rewrite)
                }
            }
            StmtKind::MacCall(mac) => {
                let attrs = node.attrs();
                if self.visit_attrs(&attrs, AttrStyle::Outer) {
                    self.push_skipped_with_span(&attrs, stmt.span(), mac_span(mac));
                } else {
                    self.visit_mac(mac, MacroPosition::Statement);
                }
                self.format_missing(stmt.span().hi());
            }
            StmtKind::Empty => (),
        }
    }

    /// Remove spaces between the opening brace and the first statement or the inner attribute
    /// of the block.
    fn trim_spaces_after_opening_brace(&mut self, b: &Block, inner_attrs: Option<&[Attribute]>) {
        if let Some(first_stmt) = b.stmts.first() {
            let hi = inner_attrs
                .and_then(|attrs| {
                    attrs
                        .iter()
                        .find(|a| a.style() == AttrStyle::Inner)
                        .map(|attr| attr.span().lo())
                })
                .unwrap_or_else(|| first_stmt.span.lo());
            let missing_span = self.next_span(hi);
            let snippet = self.snippet(missing_span);
            let len = CommentCodeSlices::new(snippet)
                .next()
                .and_then(|(kind, _, s)| {
                    if kind == CodeCharKind::Normal {
                        s.rfind('\n')
                    } else {
                        None
                    }
                });
            if let Some(len) = len {
                self.last_pos += len as BytePos;
            }
        }
    }

    pub(crate) fn visit_block(
        &mut self,
        b: &Block,
        inner_attrs: Option<&[Attribute]>,
        has_braces: bool,
    ) {
        // Check if this block has braces.
        let brace_compensation: BytePos = if has_braces { 1 } else { 0 };

        self.last_pos += brace_compensation;
        self.block_indent = self.block_indent.block_indent(self.config);
        self.push_str("{");
        self.trim_spaces_after_opening_brace(b, inner_attrs);

        // Format inner attributes if available.
        if let Some(attrs) = inner_attrs {
            self.visit_attrs(attrs, AttrStyle::Inner);
        }

        self.walk_block_stmts(b);

        if let Some(stmt) = b.stmts.last()
            && self.add_semi_on_last_block_stmt(stmt)
        {
            self.push_str(";");
        }

        // Ignore the closing brace.
        let missing_span = self.next_span(b.span.hi() - brace_compensation);
        self.close_block(missing_span, self.unindent_comment_on_closing_brace(b));
        self.last_pos = b.span.hi();
    }

    pub(crate) fn close_block(&mut self, span: Span, unindent_comment: bool) {
        let config = self.config;

        let mut last_hi = span.lo();
        let mut unindented = false;
        let mut prev_ends_with_newline = false;
        let mut extra_newline = false;

        let skip_normal = |s: &str| {
            let trimmed = s.trim();
            trimmed.is_empty() || trimmed.chars().all(|c| c == ';')
        };

        let comment_snippet = self.snippet(span);

        let align_to_right = if unindent_comment && contains_comment(comment_snippet) {
            let first_lines = comment_snippet.split('/').next().unwrap_or("");
            last_line_width(first_lines) > last_line_width(comment_snippet)
        } else {
            false
        };

        for (kind, offset, sub_slice) in CommentCodeSlices::new(comment_snippet) {
            match kind {
                CodeCharKind::Comment => {
                    if !unindented && unindent_comment && !align_to_right {
                        unindented = true;
                        self.block_indent = self.block_indent.block_unindent(config);
                    }
                    let span_in_between = mk_sp(last_hi, span.lo() + offset as BytePos);
                    let snippet_in_between = self.snippet(span_in_between);
                    let mut comment_on_same_line = !snippet_in_between.contains('\n');

                    let mut comment_shape =
                        Shape::indented(self.block_indent, config).comment(config);
                    if self.config.style_edition() >= StyleEdition::Edition2024
                        && comment_on_same_line
                    {
                        self.push_str(" ");
                        // put the first line of the comment on the same line as the
                        // block's last line
                        match sub_slice.find('\n') {
                            None => {
                                self.push_str(sub_slice);
                            }
                            Some(offset) if offset + 1 == sub_slice.len() => {
                                self.push_str(&sub_slice[..offset]);
                            }
                            Some(offset) => {
                                let first_line = &sub_slice[..offset];
                                self.push_str(first_line);
                                self.push_str(&self.block_indent.to_string_with_newline(config));

                                // put the other lines below it, shaping it as needed
                                let other_lines = &sub_slice[offset + 1..];
                                let comment_str =
                                    rewrite_comment(other_lines, false, comment_shape, config);
                                match comment_str {
                                    Some(ref s) => self.push_str(s),
                                    None => self.push_str(other_lines),
                                }
                            }
                        }
                    } else {
                        if comment_on_same_line {
                            // 1 = a space before `//`
                            let offset_len = 1 + last_line_width(&self.buffer)
                                .saturating_sub(self.block_indent.width());
                            match comment_shape
                                .visual_indent(offset_len)
                                .sub_width(offset_len)
                            {
                                Some(shp) => comment_shape = shp,
                                None => comment_on_same_line = false,
                            }
                        };

                        if comment_on_same_line {
                            self.push_str(" ");
                        } else {
                            if count_newlines(snippet_in_between) >= 2 || extra_newline {
                                self.push_str("\n");
                            }
                            self.push_str(&self.block_indent.to_string_with_newline(config));
                        }

                        let comment_str = rewrite_comment(sub_slice, false, comment_shape, config);
                        match comment_str {
                            Some(ref s) => self.push_str(s),
                            None => self.push_str(sub_slice),
                        }
                    }
                }
                CodeCharKind::Normal if skip_normal(sub_slice) => {
                    extra_newline = prev_ends_with_newline && sub_slice.contains('\n');
                    continue;
                }
                CodeCharKind::Normal => {
                    self.push_str(&self.block_indent.to_string_with_newline(config));
                    self.push_str(sub_slice.trim());
                }
            }
            prev_ends_with_newline = sub_slice.ends_with('\n');
            extra_newline = false;
            last_hi = span.lo() + (offset + sub_slice.len()) as BytePos;
        }
        if unindented {
            self.block_indent = self.block_indent.block_indent(self.config);
        }
        self.block_indent = self.block_indent.block_unindent(self.config);
        self.push_str(&self.block_indent.to_string_with_newline(config));
        self.push_str("}");
    }

    fn unindent_comment_on_closing_brace(&self, b: &Block) -> bool {
        self.is_if_else_block && !b.stmts.is_empty()
    }

    pub(crate) fn visit_item(&mut self, item: &ast::Item) {
        let attrs = item_attrs(item);
        let span = item_span(item.syntax());
        let full_span = item.span();

        let should_visit_node_again = match item {
            // For use/extern crate items, skip rewriting attributes but check for a skip attribute.
            ast::Item::Use(..) | ast::Item::ExternCrate(..) => {
                if contains_skip(&attrs) {
                    self.push_skipped_with_span(&attrs, full_span, full_span);
                    false
                } else {
                    true
                }
            }
            // Module is not inline, but should be skipped.
            ast::Item::Module(..) if is_mod_decl(item) && contains_skip(&attrs) => false,
            _ => {
                if self.visit_attrs(&attrs, AttrStyle::Outer) {
                    self.push_skipped_with_span(&attrs, full_span, full_span);
                    false
                } else {
                    true
                }
            }
        };

        if !should_visit_node_again {
            return;
        }

        match item {
            ast::Item::Use(u) => self.format_import(u),
            ast::Item::Impl(i) => {
                let block_indent = self.block_indent;
                let rw = self.with_context(|ctx| format_impl(ctx, i, block_indent));
                self.push_rewrite(span, rw);
            }
            ast::Item::Trait(t) if t.eq_token().is_some() => {
                let shape = Shape::indented(self.block_indent, self.config);
                let rw = self.with_context(|ctx| format_trait_alias(ctx, t, shape));
                self.push_rewrite(span, rw);
            }
            ast::Item::Trait(t) => {
                let block_indent = self.block_indent;
                let rw = self.with_context(|ctx| format_trait(ctx, t, block_indent));
                self.push_rewrite(span, rw);
            }
            ast::Item::ExternCrate(e) => {
                let shape = self.shape();
                let rw = self.with_context(|ctx| rewrite_extern_crate(ctx, e, shape));
                self.push_rewrite(full_span, rw);
            }
            ast::Item::Struct(s) => match StructParts::from_struct(s) {
                Some(parts) => self.visit_struct(&parts),
                None => self.push_rewrite(span, None),
            },
            ast::Item::Union(u) => match StructParts::from_union(u) {
                Some(parts) => self.visit_struct(&parts),
                None => self.push_rewrite(span, None),
            },
            ast::Item::Enum(e) => {
                self.format_missing_with_indent(span.lo());
                self.visit_enum(e, span);
                self.last_pos = span.hi();
            }
            ast::Item::Module(m) => {
                self.format_missing_with_indent(span.lo());
                self.format_mod(m, span, &attrs);
            }
            ast::Item::MacroCall(mac) => {
                self.visit_mac(mac, MacroPosition::Item);
            }
            ast::Item::ExternBlock(fm) => {
                self.format_missing_with_indent(span.lo());
                self.format_foreign_mod(fm, span);
            }
            ast::Item::Static(s) => match StaticParts::from_static(s) {
                Some(parts) => self.visit_static(&parts),
                None => self.push_rewrite(span, None),
            },
            ast::Item::Const(c) => match StaticParts::from_const(c) {
                Some(parts) => self.visit_static(&parts),
                None => self.push_rewrite(span, None),
            },
            ast::Item::Fn(f) => {
                if f.body().is_some() {
                    self.visit_fn(f, span);
                } else {
                    let indent = self.block_indent;
                    let rewrite = self.rewrite_required_fn(indent, f, span);
                    self.push_rewrite(span, rewrite);
                }
            }
            ast::Item::TypeAlias(ta) => {
                let indent = self.block_indent;
                let rewrite = self.with_context(|ctx| {
                    rewrite_type_alias(ta, ctx, indent, ItemVisitorKind::Item, span)
                });
                self.push_rewrite(span, rewrite);
            }
            ast::Item::AsmExpr(..) => {
                let snippet = Some(self.snippet(span).to_owned());
                self.push_rewrite(span, snippet);
            }
            ast::Item::MacroRules(m) => {
                self.visit_macro_def(&MacroDef::Rules(m.clone()), span);
            }
            ast::Item::MacroDef(m) => {
                self.visit_macro_def(&MacroDef::Def(m.clone()), span);
            }
        }
    }

    fn visit_macro_def(&mut self, def: &MacroDef, span: Span) {
        let (shape, indent) = (self.shape(), self.block_indent);
        let rewrite = self.with_context(|ctx| rewrite_macro_def(ctx, shape, indent, def, span));
        self.push_rewrite(span, rewrite);
    }

    /// An associated item of a trait or impl.
    pub(crate) fn visit_assoc_item(&mut self, ai: &ast::AssocItem) {
        let in_trait = ai
            .syntax()
            .parent()
            .and_then(|list| list.parent())
            .is_some_and(|p| p.kind() == SyntaxKind::TRAIT);
        let visitor_kind = if in_trait {
            ItemVisitorKind::AssocTraitItem
        } else {
            ItemVisitorKind::AssocImplItem
        };
        let skip_span = item_span(ai.syntax());
        let mut attrs = outer_attributes(ai.syntax());
        if let ast::AssocItem::Fn(f) = ai
            && let Some(list) = f.body().and_then(|b| b.stmt_list())
        {
            attrs.extend(inner_attributes(list.syntax()));
        }

        if self.visit_attrs(&attrs, AttrStyle::Outer) {
            self.push_skipped_with_span(&attrs, skip_span, skip_span);
            return;
        }

        match ai {
            ast::AssocItem::Const(c) => match StaticParts::from_const(c) {
                Some(parts) => self.visit_static(&parts),
                None => self.push_rewrite(skip_span, None),
            },
            ast::AssocItem::Fn(f) => {
                if f.body().is_some() {
                    self.visit_fn(f, skip_span);
                } else {
                    let indent = self.block_indent;
                    let rewrite = self.rewrite_required_fn(indent, f, skip_span);
                    self.push_rewrite(skip_span, rewrite);
                }
            }
            ast::AssocItem::TypeAlias(ta) => {
                let indent = self.block_indent;
                let rewrite = self.with_context(|ctx| {
                    rewrite_type_alias(ta, ctx, indent, visitor_kind, skip_span)
                });
                self.push_rewrite(skip_span, rewrite);
            }
            ast::AssocItem::MacroCall(mac) => {
                self.visit_mac(mac, MacroPosition::Item);
            }
        }
    }

    fn visit_mac(&mut self, mac: &ast::MacroCall, pos: MacroPosition) {
        // 1 = ;
        let shape = self.shape().saturating_sub_width(1);
        let rewrite = self.with_context(|ctx| rewrite_macro(mac, ctx, shape, pos));
        // The span of a macro call does not include the trailing semicolon. This determines
        // the correct span to ensure scenarios with whitespace between the delimiters and
        // trailing semi (i.e. `foo!(abc)     ;`) are formatted correctly.
        let span = mac_span(mac);
        let (span, rewrite) = match macro_style(mac) {
            Delimiter::Bracket | Delimiter::Parenthesis if MacroPosition::Item == pos => {
                let search_span = mk_sp(span.hi(), self.snippet_provider.end_pos());
                let hi = self.snippet_provider.span_before(search_span, ";");
                let target_span = mk_sp(span.lo(), hi + 1);
                let rewrite = rewrite.map(|rw| {
                    if !rw.ends_with(';') {
                        format!("{rw};")
                    } else {
                        rw
                    }
                });
                (target_span, rewrite)
            }
            _ => (span, rewrite),
        };

        self.push_rewrite(span, rewrite);
    }

    pub(crate) fn push_str(&mut self, s: &str) {
        self.line_number += count_newlines(s);
        self.buffer.push_str(s);
    }

    fn push_rewrite_inner(&mut self, span: Span, rewrite: Option<String>) {
        if let Some(ref s) = rewrite {
            self.push_str(s);
        } else {
            let snippet = self.snippet(span);
            self.push_str(snippet.trim());
        }
        self.last_pos = span.hi();
    }

    pub(crate) fn push_rewrite(&mut self, span: Span, rewrite: Option<String>) {
        self.format_missing_with_indent(span.lo());
        self.push_rewrite_inner(span, rewrite);
    }

    pub(crate) fn push_skipped_with_span(
        &mut self,
        attrs: &[Attribute],
        item_span: Span,
        main_span: Span,
    ) {
        self.format_missing_with_indent(item_span.lo());
        // do not take into account the lines with attributes as part of the skipped range
        let attrs_end = attrs
            .iter()
            .map(|attr| self.line_of(attr.span().hi()))
            .max()
            .unwrap_or(1);
        let first_line = self.line_of(main_span.lo());
        // Statement can start after some newlines and/or spaces
        // or it can be on the same line as the last attribute.
        // So here we need to take a minimum between the two.
        let lo = std::cmp::min(attrs_end + 1, first_line);
        self.push_rewrite_inner(item_span, None);
        let hi = self.line_number + 1;
        self.run.skipped_range.borrow_mut().push((lo, hi));
    }

    /// Returns `true` if the following item should be skipped (it has a skip attribute);
    /// otherwise writes the attributes of `style` and returns `false`.
    pub(crate) fn visit_attrs(&mut self, attrs: &[Attribute], style: AttrStyle) -> bool {
        if contains_skip(attrs) {
            return true;
        }

        let attrs: Vec<_> = attrs
            .iter()
            .filter(|a| a.style() == style)
            .cloned()
            .collect();
        let (Some(first), Some(last)) = (attrs.first(), attrs.last()) else {
            return false;
        };

        let span = mk_sp(first.span().lo(), last.span().hi());
        let rewrite = attrs[..].rewrite(&self.get_context(), self.shape());
        self.push_rewrite(span, rewrite);

        false
    }

    fn walk_mod_items(&mut self, items: &[ast::Item]) {
        self.visit_items_with_reordering(items);
    }

    fn walk_stmts(&mut self, stmts: &[Stmt], include_current_empty_semi: bool) {
        if stmts.is_empty() {
            return;
        }

        // Extract leading `use ...;`.
        let items: Vec<ast::Item> = stmts
            .iter()
            .take_while(|stmt| stmt.to_item().is_some_and(is_use_item))
            .filter_map(|stmt| stmt.to_item().cloned())
            .collect();

        if items.is_empty() {
            self.visit_stmt(&stmts[0], include_current_empty_semi);

            // rustfmt preserves redundant semicolons after items in statement position: rustc
            // parses `struct A {};` as an item followed by an empty statement.
            let include_next_empty = stmts.len() > 1
                && matches!(
                    (&stmts[0].as_ast_node().kind, &stmts[1].as_ast_node().kind),
                    (StmtKind::Item(_), StmtKind::Empty)
                );

            self.walk_stmts(&stmts[1..], include_next_empty);
        } else {
            self.visit_items_with_reordering(&items);
            self.walk_stmts(&stmts[items.len()..], false);
        }
    }

    fn walk_block_stmts(&mut self, b: &Block) {
        self.walk_stmts(&Stmt::from_ast_nodes(&b.stmts), false)
    }

    fn format_mod(&mut self, m: &ast::Module, s: Span, attrs: &[Attribute]) {
        let vis_str = format_visibility(m.visibility().as_ref());
        self.push_str(&vis_str);
        if m.syntax()
            .children_with_tokens()
            .any(|el| el.kind() == SyntaxKind::UNSAFE_KW)
        {
            self.push_str("unsafe ");
        }
        self.push_str("mod ");
        let name = m
            .name()
            .map(|n| n.syntax().text().to_string())
            .unwrap_or_default();
        self.push_str(&name);

        if let Some(item_list) = m.item_list() {
            let inner_span = node_range_span(item_list.syntax());
            // `brace_style` is fixed at `SameLineWhere`.
            self.push_str(" {");
            // Hackery to account for the closing }.
            let mod_lo = self.snippet_provider.span_after(s, "{");
            let body_snippet = self.snippet(mk_sp(mod_lo, inner_span.hi() - 1));
            let body_snippet = body_snippet.trim();
            if body_snippet.is_empty() {
                self.push_str("}");
            } else {
                self.last_pos = mod_lo;
                self.block_indent = self.block_indent.block_indent(self.config);
                self.visit_attrs(attrs, AttrStyle::Inner);
                let items: Vec<ast::Item> = item_list.items().collect();
                self.walk_mod_items(&items);
                let missing_span = self.next_span(inner_span.hi() - 1);
                self.close_block(missing_span, false);
            }
            self.last_pos = inner_span.hi();
        } else {
            self.push_str(";");
            self.last_pos = s.hi();
        }
    }

    /// Formats the items of a source file.
    pub(crate) fn format_separate_mod(&mut self, file: &ast::SourceFile, end_pos: BytePos) {
        self.block_indent = Indent::empty();
        self.visit_attrs(&inner_attributes(file.syntax()), AttrStyle::Inner);
        let items: Vec<ast::Item> = file.items().collect();
        self.walk_mod_items(&items);
        self.format_missing_with_indent(end_pos);
    }

    pub(crate) fn skip_empty_lines(&mut self, end_pos: BytePos) {
        while let Some(pos) = self
            .snippet_provider
            .opt_span_after(self.next_span(end_pos), "\n")
        {
            if let Some(snippet) = self.snippet_provider.span_to_snippet(self.next_span(pos)) {
                if snippet.trim().is_empty() {
                    self.last_pos = pos;
                } else {
                    return;
                }
            }
        }
    }

    pub(crate) fn with_context<T>(&mut self, f: impl Fn(&RewriteContext<'_>) -> T) -> T {
        let context = self.get_context();
        let result = f(&context);

        self.macro_rewrite_failure |= context.macro_rewrite_failure.get();
        result
    }

    pub(crate) fn get_context(&self) -> RewriteContext<'a> {
        let mut context = RewriteContext::new(self.config, self.snippet_provider, self.run.clone());
        context.is_macro_def = self.is_macro_def;
        context
    }

    fn add_semi_on_last_block_stmt(&self, stmt: &super::nodes::Stmt) -> bool {
        let StmtKind::Expr(expr) = &stmt.kind else {
            return false;
        };

        if self.is_macro_def {
            return false;
        }

        match expr {
            ast::Expr::ReturnExpr(..) | ast::Expr::ContinueExpr(..) | ast::Expr::BreakExpr(..) => {
                self.config.trailing_semicolon()
            }
            // Other expressions only get a semicolon with `style_edition = 2027`, which is
            // not stable.
            _ => false,
        }
    }
}
