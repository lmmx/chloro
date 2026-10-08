//! Reordering of items (rustfmt's `reorder.rs`).
//!
//! Consecutive `use` declarations (with `reorder_imports`), `extern crate` declarations
//! (with `reorder_imports`) and out-of-line `mod` declarations (with `reorder_modules`)
//! are sorted. A blank line ends a group: items are only sorted within their group.
//! `group_imports` is an unstable rustfmt option and stays at `Preserve`.

use std::cmp::Ordering;

use ra_ap_syntax::ast::{self, AstNode, HasName};

use super::config::{Settings, StyleEdition};
use super::context::RewriteContext;
use super::imports::UseTree;
use super::items::{is_mod_decl, rewrite_extern_crate, rewrite_mod};
use super::lists::{ListFormatting, ListItem, itemize_list, write_list};
use super::nodes::{contains_skip, outer_attributes};
use super::shape::Shape;
use super::sort::version_sort;
use super::span::{Span, Spanned, mk_sp};
use super::visitor::FmtVisitor;

/// The symbol of an identifier: rustc compares `r#type` as `type`.
fn symbol(text: String) -> String {
    match text.strip_prefix("r#") {
        Some(name) => name.to_owned(),
        None => text,
    }
}

fn mod_name(item: &ast::Item) -> String {
    match item {
        ast::Item::Module(m) => m.name().map(|n| symbol(n.syntax().text().to_string())),
        _ => None,
    }
    .unwrap_or_default()
}

/// `extern crate name as alias;`: the crate name and the optional alias.
fn extern_crate_names(item: &ast::Item) -> (String, Option<String>) {
    match item {
        ast::Item::ExternCrate(e) => (
            e.name_ref()
                .map(|n| symbol(n.syntax().text().to_string()))
                .unwrap_or_default(),
            e.rename().map(|r| {
                r.name()
                    .map_or_else(|| "_".to_owned(), |n| symbol(n.syntax().text().to_string()))
            }),
        ),
        _ => (String::new(), None),
    }
}

/// Choose the ordering between the given two items.
fn compare_items(a: &ast::Item, b: &ast::Item, context: &RewriteContext<'_>) -> Ordering {
    let style_edition = context.config.style_edition();
    let cmp = |a: &str, b: &str| {
        if style_edition <= StyleEdition::Edition2024 {
            a.cmp(b)
        } else {
            version_sort(a, b)
        }
    };
    match (a, b) {
        (ast::Item::Module(_), ast::Item::Module(_)) => cmp(&mod_name(a), &mod_name(b)),
        (ast::Item::ExternCrate(_), ast::Item::ExternCrate(_)) => {
            // `extern crate foo as bar;`
            //               ^^^ Comparing this.
            let (a_name, a_alias) = extern_crate_names(a);
            let (b_name, b_alias) = extern_crate_names(b);
            let result = cmp(&a_name, &b_name);
            if result != Ordering::Equal {
                return result;
            }

            // `extern crate foo as bar;`
            //                      ^^^ Comparing this.
            match (a_alias, b_alias) {
                (Some(..), None) => Ordering::Greater,
                (None, Some(..)) => Ordering::Less,
                (None, None) => Ordering::Equal,
                (Some(a), Some(b)) => cmp(&a, &b),
            }
        }
        _ => Ordering::Equal,
    }
}

fn wrap_reorderable_items(
    context: &RewriteContext<'_>,
    list_items: &[ListItem],
    shape: Shape,
) -> Option<String> {
    let fmt = ListFormatting::new(shape, context.config)
        .separator("")
        .align_comments(false);
    write_list(list_items, &fmt)
}

fn rewrite_reorderable_item(
    context: &RewriteContext<'_>,
    item: &ast::Item,
    shape: Shape,
) -> Option<String> {
    match item {
        ast::Item::ExternCrate(e) => rewrite_extern_crate(context, e, shape),
        ast::Item::Module(m) => rewrite_mod(context, m, shape),
        _ => None,
    }
}

/// Rewrite a list of items with reordering. Every item in `items` must have the same
/// kind.
fn rewrite_reorderable_items(
    context: &RewriteContext<'_>,
    reorderable_items: &[ast::Item],
    shape: Shape,
    span: Span,
) -> Option<String> {
    match &reorderable_items[0] {
        ast::Item::Use(_) => {
            let mut normalized_items: Vec<_> = reorderable_items
                .iter()
                .filter_map(|item| match item {
                    ast::Item::Use(u) => UseTree::from_use_item(context, u, true),
                    _ => None,
                })
                .collect();
            // Add comments before sorting.
            let list_items: Vec<ListItem> = itemize_list(
                context.snippet_provider,
                normalized_items.iter(),
                "",
                ";",
                |item| item.span().lo(),
                |item| item.span().hi(),
                |_item| Some(String::new()),
                span.lo(),
                span.hi(),
                false,
            )
            .collect();
            for (item, list_item) in normalized_items.iter_mut().zip(list_items) {
                item.list_item = Some(list_item);
            }

            if context.config.reorder_imports() {
                normalized_items.sort();
            }

            // 4 = "use ", 1 = ";"
            let nested_shape = shape.offset_left(4)?.sub_width(1)?;
            let item_vec: Vec<_> = normalized_items
                .into_iter()
                .map(|use_tree| {
                    let item = use_tree.rewrite_top_level(context, nested_shape);
                    match use_tree.list_item {
                        Some(list_item) => ListItem { item, ..list_item },
                        None => ListItem::from_item(item),
                    }
                })
                .collect();
            wrap_reorderable_items(context, &item_vec, nested_shape)
        }
        _ => {
            let list_items = itemize_list(
                context.snippet_provider,
                reorderable_items.iter(),
                "",
                ";",
                |item| item.span().lo(),
                |item| item.span().hi(),
                |item| rewrite_reorderable_item(context, item, shape),
                span.lo(),
                span.hi(),
                false,
            );

            let mut item_pair_vec: Vec<_> = list_items.zip(reorderable_items.iter()).collect();
            item_pair_vec.sort_by(|a, b| compare_items(a.1, b.1, context));
            let item_vec: Vec<_> = item_pair_vec.into_iter().map(|pair| pair.0).collect();

            wrap_reorderable_items(context, &item_vec, shape)
        }
    }
}

fn contains_macro_use_attr(item: &ast::Item) -> bool {
    outer_attributes(item.syntax())
        .iter()
        .any(|a| a.has_name("macro_use"))
}

/// A simplified version of `ast::ItemKind`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum ReorderableItemKind {
    ExternCrate,
    Mod,
    Use,
    /// An item that cannot be reordered. Either has an unreorderable item kind
    /// or an `macro_use` attribute.
    Other,
}

impl ReorderableItemKind {
    fn from(item: &ast::Item) -> Self {
        if contains_macro_use_attr(item) || contains_skip(&outer_attributes(item.syntax())) {
            return ReorderableItemKind::Other;
        }
        match item {
            ast::Item::ExternCrate(..) => ReorderableItemKind::ExternCrate,
            ast::Item::Module(..) if is_mod_decl(item) => ReorderableItemKind::Mod,
            ast::Item::Use(..) => ReorderableItemKind::Use,
            _ => ReorderableItemKind::Other,
        }
    }

    fn is_same_item_kind(self, item: &ast::Item) -> bool {
        ReorderableItemKind::from(item) == self
    }

    fn is_reorderable(self, config: &Settings) -> bool {
        match self {
            ReorderableItemKind::ExternCrate => config.reorder_imports(),
            ReorderableItemKind::Mod => config.reorder_modules(),
            ReorderableItemKind::Use => config.reorder_imports(),
            ReorderableItemKind::Other => false,
        }
    }
}

impl FmtVisitor<'_> {
    /// Line range (1-based, inclusive) of `span`.
    fn line_range(&self, span: Span) -> (usize, usize) {
        (self.line_of(span.lo()), self.line_of(span.hi()))
    }

    /// Format items with the same item kind and reorder them. Items separated by an empty
    /// line are not reordered together.
    fn walk_reorderable_items(
        &mut self,
        items: &[ast::Item],
        item_kind: ReorderableItemKind,
    ) -> usize {
        let mut last = self.line_range(items[0].span());
        let item_length = items
            .iter()
            .take_while(|ppi| {
                item_kind.is_same_item_kind(ppi) && {
                    let current = self.line_range(ppi.span());
                    let in_same_group = current.0 < last.1 + 2;
                    last = current;
                    in_same_group
                }
            })
            .count();
        let items = &items[..item_length];

        let lo = items[0].span().lo();
        let hi = items[item_length - 1].span().hi();
        let span = mk_sp(lo, hi);
        let rw = rewrite_reorderable_items(&self.get_context(), items, self.shape(), span);
        self.push_rewrite(span, rw);

        item_length
    }

    /// Visits and format the given items. Items are reordered If they are
    /// consecutive and reorderable.
    pub(crate) fn visit_items_with_reordering(&mut self, mut items: &[ast::Item]) {
        while !items.is_empty() {
            // If the next item is a `use`, `extern crate` or `mod`, then extract it and any
            // subsequent items that have the same item kind to be reordered within
            // `walk_reorderable_items`. Otherwise, just format the next item for output.
            let item_kind = ReorderableItemKind::from(&items[0]);
            if item_kind.is_reorderable(self.config) {
                let visited_items_num = self.walk_reorderable_items(items, item_kind);
                items = &items[visited_items_num..];
            } else {
                self.visit_item(&items[0]);
                items = &items[1..];
            }
        }
    }
}
