//! `use` declarations (rustfmt's `imports.rs`).
//!
//! Imports are translated into [`UseTree`], a path of [`UseSegment`]s, normalised
//! (`a::{b}` becomes `a::b`, `a::self` becomes `a`, nested lists are sorted and
//! deduplicated) and then sorted and printed. Merging and splitting imports
//! (`imports_granularity`) is an unstable rustfmt option and is not implemented: rustfmt's
//! default, `Preserve`, leaves the import structure as written.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};

use super::comment::combine_strs_with_missing_comments;
use super::config::{Edition, StyleEdition};
use super::context::{Rewrite, RewriteContext};
use super::lists::{
    DefinitiveListTactic, ListFormatting, ListItem, Separator, definitive_tactic, itemize_list,
    write_list,
};
use super::nodes::{Attribute, outer_attributes};
use super::shape::Shape;
use super::sort::version_sort;
use super::span::{BytePos, Span, Spanned, mk_sp, node_range_span};
use super::utils::format_visibility;
use super::visitor::FmtVisitor;

impl FmtVisitor<'_> {
    pub(crate) fn format_import(&mut self, item: &ast::Use) {
        // The span includes the attributes, which `rewrite_top_level` formats.
        let span = item.span();
        let shape = self.shape();
        let context = self.get_context();
        let rw = UseTree::from_use_item(&context, item, false)
            .and_then(|tree| tree.rewrite_top_level(&context, shape));
        match rw {
            Some(ref s) if s.is_empty() => {
                // Format up to last newline
                let prev_span = mk_sp(self.last_pos, span.lo());
                let trimmed_snippet = self.snippet(prev_span).trim_end();
                let span_end = self.last_pos + trimmed_snippet.len() as BytePos;
                self.format_missing(span_end);
                // We have an excessive newline from the removed import.
                if self.buffer.ends_with('\n') {
                    self.buffer.pop();
                    self.line_number -= 1;
                }
                self.last_pos = span.hi();
            }
            Some(ref s) => {
                self.format_missing_with_indent(span.lo());
                self.push_str(s);
                self.last_pos = span.hi();
            }
            None => {
                self.format_missing_with_indent(span.lo());
                self.format_missing(span.hi());
            }
        }
    }
}

// Ordering of imports
//
// Imports are ordered by translating them to this representation and then sorting.
// `self` and `super` sort before other imports, then identifier imports, then glob
// imports, then lists of imports. Aliases are only taken into account when the imports
// are otherwise identical.

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) enum UseSegmentKind {
    Ident(String, Option<String>),
    Slf(Option<String>),
    Super(Option<String>),
    Crate(Option<String>),
    Glob,
    List(Vec<UseTree>),
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub(crate) struct UseSegment {
    pub(crate) kind: UseSegmentKind,
    pub(crate) style_edition: StyleEdition,
}

#[derive(Clone)]
pub(crate) struct UseTree {
    pub(crate) path: Vec<UseSegment>,
    pub(crate) span: Span,
    // Comment information within nested use tree.
    pub(crate) list_item: Option<ListItem>,
    // Additional fields for top level use items.
    visibility: Option<ast::Visibility>,
    attrs: Option<Vec<Attribute>>,
}

impl PartialEq for UseTree {
    fn eq(&self, other: &UseTree) -> bool {
        self.path == other.path
    }
}
impl Eq for UseTree {}

impl std::hash::Hash for UseTree {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Spanned for UseTree {
    fn span(&self) -> Span {
        let lo = match &self.attrs {
            Some(attrs) => attrs.first().map_or(self.span.lo(), |a| a.span().lo()),
            None => self.span.lo(),
        };
        mk_sp(lo, self.span.hi())
    }
}

impl UseSegment {
    // Clone a version of self with any top-level alias removed.
    fn remove_alias(&self) -> UseSegment {
        let kind = match self.kind {
            UseSegmentKind::Ident(ref s, _) => UseSegmentKind::Ident(s.clone(), None),
            UseSegmentKind::Slf(_) => UseSegmentKind::Slf(None),
            UseSegmentKind::Super(_) => UseSegmentKind::Super(None),
            UseSegmentKind::Crate(_) => UseSegmentKind::Crate(None),
            _ => return self.clone(),
        };
        UseSegment {
            kind,
            style_edition: self.style_edition,
        }
    }

    fn from_name(context: &RewriteContext<'_>, name: &str, modsep: bool) -> Option<UseSegment> {
        if name.is_empty() {
            return None;
        }
        let kind = match name {
            "self" => UseSegmentKind::Slf(None),
            "super" => UseSegmentKind::Super(None),
            "crate" => UseSegmentKind::Crate(None),
            _ => {
                let mod_sep = if modsep { "::" } else { "" };
                UseSegmentKind::Ident(format!("{mod_sep}{name}"), None)
            }
        };

        Some(UseSegment {
            kind,
            style_edition: context.config.style_edition(),
        })
    }
}

impl fmt::Debug for UseTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Debug for UseSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl fmt::Display for UseSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl fmt::Debug for UseSegmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for UseSegmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UseSegmentKind::Glob => write!(f, "*"),
            UseSegmentKind::Ident(ref s, Some(ref alias)) => write!(f, "{s} as {alias}"),
            UseSegmentKind::Ident(ref s, None) => write!(f, "{s}"),
            UseSegmentKind::Slf(..) => write!(f, "self"),
            UseSegmentKind::Super(..) => write!(f, "super"),
            UseSegmentKind::Crate(..) => write!(f, "crate"),
            UseSegmentKind::List(ref list) => {
                write!(f, "{{")?;
                for (i, item) in list.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl fmt::Display for UseTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.path.iter().enumerate() {
            if i != 0 {
                write!(f, "::")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}

/// The identifier texts of a path's segments, as written, and whether the path starts
/// with `::`.
fn path_segment_names(path: &ast::Path) -> (Vec<String>, bool) {
    let mut names = Vec::new();
    let mut global = false;
    // Segments are nested: `a::b::c` is Path(Path(Path(a), b), c).
    let mut segments: Vec<ast::PathSegment> = Vec::new();
    let mut cur = Some(path.clone());
    while let Some(p) = cur {
        if let Some(seg) = p.segment() {
            segments.push(seg);
        }
        cur = p.qualifier();
    }
    segments.reverse();
    for (i, seg) in segments.iter().enumerate() {
        if i == 0 && seg.coloncolon_token().is_some() {
            global = true;
        }
        let text = seg
            .name_ref()
            .map_or_else(String::new, |n| n.syntax().text().to_string());
        names.push(text);
    }
    (names, global)
}

impl UseTree {
    // Rewrite use tree with `use ` and a trailing `;`.
    pub(crate) fn rewrite_top_level(
        &self,
        context: &RewriteContext<'_>,
        shape: Shape,
    ) -> Option<String> {
        let vis = self
            .visibility
            .as_ref()
            .map_or(Cow::from(""), |vis| format_visibility(Some(vis)));
        let use_str = self
            .rewrite(context, shape.offset_left(vis.len())?)
            .map(|s| {
                if s.is_empty() {
                    s
                } else {
                    format!("{vis}use {s};")
                }
            })?;
        match self.attrs {
            Some(ref attrs) if !attrs.is_empty() => {
                let attr_str = attrs.rewrite(context, shape)?;
                let lo = attrs.last()?.span().hi();
                let hi = self.span.lo();
                let span = mk_sp(lo, hi);

                let allow_extend = if attrs.len() == 1 {
                    let line_len = attr_str.len() + 1 + use_str.len();
                    !attrs[0].is_doc_comment()
                        && context.config.inline_attribute_width() >= line_len
                } else {
                    false
                };

                combine_strs_with_missing_comments(
                    context,
                    &attr_str,
                    &use_str,
                    span,
                    shape,
                    allow_extend,
                )
            }
            _ => Some(use_str),
        }
    }

    /// The tree of a `use` item, normalised when the item takes part in reordering.
    pub(crate) fn from_use_item(
        context: &RewriteContext<'_>,
        item: &ast::Use,
        normalize: bool,
    ) -> Option<UseTree> {
        let attrs = outer_attributes(item.syntax());
        let tree = UseTree::from_ast(
            context,
            &item.use_tree()?,
            None,
            item.visibility(),
            Some(super::items::item_span(item.syntax()).lo()),
            if attrs.is_empty() { None } else { Some(attrs) },
        );
        Some(if normalize { tree.normalize() } else { tree })
    }

    fn from_ast(
        context: &RewriteContext<'_>,
        a: &ast::UseTree,
        list_item: Option<ListItem>,
        visibility: Option<ast::Visibility>,
        opt_lo: Option<BytePos>,
        attrs: Option<Vec<Attribute>>,
    ) -> UseTree {
        let a_span = node_range_span(a.syntax());
        let span = match opt_lo {
            Some(lo) => mk_sp(lo, a_span.hi()),
            None => a_span,
        };
        let mut result = UseTree {
            path: vec![],
            span,
            list_item,
            visibility,
            attrs,
        };

        // rustc's prefix path: `::` alone (`use ::*`, `use ::{a}`) is a global empty path.
        let (names, path_global) = a.path().map(|p| path_segment_names(&p)).unwrap_or_default();
        let bare_root = a.path().is_none() && a.coloncolon_token().is_some();
        let is_global = path_global || bare_root;
        // Number of rustc prefix segments, counting the `{{root}}` segment of a global path.
        let prefix_len = names.len() + usize::from(is_global);

        let leading_modsep = context.config.edition() >= Edition::Edition2018 && is_global;

        let mut modsep = leading_modsep;

        for name in &names {
            if let Some(use_segment) = UseSegment::from_name(context, name, modsep) {
                result.path.push(use_segment);
                modsep = false;
            }
        }

        let style_edition = context.config.style_edition();

        if a.star_token().is_some() {
            // in case of a global path and the glob starts at the root, e.g., "::*"
            if prefix_len == 1 && leading_modsep {
                result.path.push(UseSegment {
                    kind: UseSegmentKind::Ident(String::new(), None),
                    style_edition,
                });
            }
            result.path.push(UseSegment {
                kind: UseSegmentKind::Glob,
                style_edition,
            });
        } else if let Some(list) = a.use_tree_list() {
            let trees: Vec<ast::UseTree> = list.use_trees().collect();
            // Extract comments between nested use items.
            // This needs to be done before sorting use items.
            let items = itemize_list(
                context.snippet_provider,
                trees.iter(),
                "}",
                ",",
                |tree| node_range_span(tree.syntax()).lo(),
                |tree| node_range_span(tree.syntax()).hi(),
                |_| Some(String::new()), // We only need comments for now.
                context.snippet_provider.span_after(a_span, "{"),
                a_span.hi(),
                false,
            );

            // in case of a global path and the nested list starts at the root,
            // e.g., "::{foo, bar}"
            if prefix_len == 1 && leading_modsep {
                result.path.push(UseSegment {
                    kind: UseSegmentKind::Ident(String::new(), None),
                    style_edition,
                });
            }
            let kind = UseSegmentKind::List(
                trees
                    .iter()
                    .zip(items)
                    .map(|(t, list_item)| {
                        Self::from_ast(context, t, Some(list_item), None, None, None)
                    })
                    .collect(),
            );
            result.path.push(UseSegment {
                kind,
                style_edition,
            });
        } else if let Some(last) = names.last() {
            // If the path has leading double colons and is composed of only 2 segments, then we
            // bypass the imported ident which would lose the path root, e.g., `that` in
            // `::that`. The span of the prefix contains the leading colons.
            let name = if prefix_len == 2 && leading_modsep {
                a.path()
                    .map(|p| context.snippet(node_range_span(p.syntax())).to_owned())
                    .unwrap_or_default()
            } else {
                last.clone()
            };
            let alias = a.rename().and_then(|rename| {
                let alias = match rename.name() {
                    Some(n) => n.syntax().text().to_string(),
                    // for impl-only-use
                    None => "_".to_owned(),
                };
                if alias == *last { None } else { Some(alias) }
            });
            let kind = match name.as_ref() {
                "self" => UseSegmentKind::Slf(alias),
                "super" => UseSegmentKind::Super(alias),
                "crate" => UseSegmentKind::Crate(alias),
                _ => UseSegmentKind::Ident(name, alias),
            };

            // `name` is already in result.
            result.path.pop();
            result.path.push(UseSegment {
                kind,
                style_edition,
            });
        }
        result
    }

    // Do the adjustments that rustfmt does elsewhere to use paths.
    pub(crate) fn normalize(mut self) -> UseTree {
        let Some(mut last) = self.path.pop() else {
            return self;
        };
        let mut normalize_sole_list = false;
        let mut aliased_self = false;

        // Remove foo::{} or self without attributes.
        match last.kind {
            _ if self.attrs.is_some() => (),
            UseSegmentKind::List(ref list) if list.is_empty() => {
                self.path = vec![];
                return self;
            }
            UseSegmentKind::Slf(None) if self.path.is_empty() && self.visibility.is_some() => {
                self.path = vec![];
                return self;
            }
            _ => (),
        }

        // Normalise foo::self -> foo.
        if let UseSegmentKind::Slf(None) = last.kind
            && let Some(second_last) = self.path.pop()
        {
            if matches!(second_last.kind, UseSegmentKind::Slf(_)) {
                self.path.push(second_last);
            } else {
                self.path.push(second_last);
                return self;
            }
        }

        // Normalise foo::self as bar -> foo as bar.
        if let UseSegmentKind::Slf(_) = last.kind
            && let Some(UseSegment {
                kind: UseSegmentKind::Ident(_, None),
                ..
            }) = self.path.last()
        {
            aliased_self = true;
        }

        if aliased_self
            && let Some(UseSegment {
                kind: UseSegmentKind::Ident(_, old_rename),
                ..
            }) = self.path.last_mut()
            && let UseSegmentKind::Slf(Some(rename)) = last.kind.clone()
        {
            *old_rename = Some(rename);
            return self;
        }

        // Normalise foo::{bar} -> foo::bar
        if let UseSegmentKind::List(ref list) = last.kind
            && list.len() == 1
            && list[0].to_string() != "self"
            && !list[0].has_comment()
        {
            normalize_sole_list = true;
        }

        if normalize_sole_list && let UseSegmentKind::List(list) = last.kind {
            for seg in &list[0].path {
                self.path.push(seg.clone());
            }
            return self.normalize();
        }

        // Recursively normalize elements of a list use (including sorting the list).
        if let UseSegmentKind::List(list) = last.kind {
            let mut list = list.into_iter().map(UseTree::normalize).collect::<Vec<_>>();
            list.sort();
            list.dedup();
            last = UseSegment {
                kind: UseSegmentKind::List(list),
                style_edition: last.style_edition,
            };
        }

        self.path.push(last);
        self
    }

    fn has_comment(&self) -> bool {
        self.list_item.as_ref().is_some_and(ListItem::has_comment)
    }
}

impl PartialOrd for UseSegment {
    fn partial_cmp(&self, other: &UseSegment) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialOrd for UseTree {
    fn partial_cmp(&self, other: &UseTree) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for UseSegment {
    fn cmp(&self, other: &UseSegment) -> Ordering {
        use self::UseSegmentKind::*;

        fn is_upper_snake_case(s: &str) -> bool {
            s.chars()
                .all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
        }

        match (&self.kind, &other.kind) {
            (Slf(a), Slf(b)) | (Super(a), Super(b)) | (Crate(a), Crate(b)) => match (a, b) {
                (Some(sa), Some(sb)) => {
                    if self.style_edition >= StyleEdition::Edition2024 {
                        version_sort(sa.trim_start_matches("r#"), sb.trim_start_matches("r#"))
                    } else {
                        a.cmp(b)
                    }
                }
                (_, _) => a.cmp(b),
            },
            (Glob, Glob) => Ordering::Equal,
            (Ident(pia, aa), Ident(pib, ab)) => {
                let (ia, ib) = if self.style_edition >= StyleEdition::Edition2024 {
                    (pia.trim_start_matches("r#"), pib.trim_start_matches("r#"))
                } else {
                    (pia.as_str(), pib.as_str())
                };

                let ident_ord = if self.style_edition >= StyleEdition::Edition2024 {
                    version_sort(ia, ib)
                } else {
                    fn sorting_key(ident: &str) -> (bool, bool, &str) {
                        // snake_case < CamelCase < UPPER_SNAKE_CASE
                        (
                            is_upper_snake_case(ident),
                            ident.starts_with(char::is_uppercase),
                            ident,
                        )
                    }

                    sorting_key(ia).cmp(&sorting_key(ib))
                };

                if ident_ord != Ordering::Equal {
                    return ident_ord;
                }
                match (aa, ab) {
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (Some(aas), Some(abs)) => {
                        if self.style_edition >= StyleEdition::Edition2024 {
                            version_sort(aas.trim_start_matches("r#"), abs.trim_start_matches("r#"))
                        } else {
                            aas.cmp(abs)
                        }
                    }
                    (None, None) => Ordering::Equal,
                }
            }
            (List(a), List(b)) => a.iter().cmp(b.iter()),
            (Slf(_), _) => Ordering::Less,
            (_, Slf(_)) => Ordering::Greater,
            (Super(_), _) => Ordering::Less,
            (_, Super(_)) => Ordering::Greater,
            (Crate(_), _) => Ordering::Less,
            (_, Crate(_)) => Ordering::Greater,
            (Ident(..), _) => Ordering::Less,
            (_, Ident(..)) => Ordering::Greater,
            (Glob, _) => Ordering::Less,
            (_, Glob) => Ordering::Greater,
        }
    }
}
impl Ord for UseTree {
    fn cmp(&self, other: &UseTree) -> Ordering {
        for (a, b) in self.path.iter().zip(other.path.iter()) {
            let ord = a.cmp(b);
            // The comparison without aliases is a hack to avoid situations like
            // comparing `a::b` to `a as c` - where the latter should be ordered
            // first since it is shorter.
            if ord != Ordering::Equal && a.remove_alias().cmp(&b.remove_alias()) != Ordering::Equal
            {
                return ord;
            }
        }

        self.path.len().cmp(&other.path.len())
    }
}

/// `imports_indent` is fixed at `Block` and `imports_layout` at `Mixed`.
fn rewrite_nested_use_tree(
    context: &RewriteContext<'_>,
    use_tree_list: &[UseTree],
    shape: Shape,
) -> Option<String> {
    let mut list_items = Vec::with_capacity(use_tree_list.len());
    let nested_shape = shape
        .block_indent(context.config.tab_spaces())
        .with_max_width(context.config)
        .sub_width(1)?;
    for use_tree in use_tree_list {
        if let Some(mut list_item) = use_tree.list_item.clone() {
            list_item.item = use_tree.rewrite(context, nested_shape);
            list_items.push(list_item);
        } else {
            list_items.push(ListItem::from_str(use_tree.rewrite(context, nested_shape)?));
        }
    }
    let has_nested_list = use_tree_list.iter().any(|use_segment| {
        use_segment
            .path
            .last()
            .is_some_and(|last_segment| matches!(last_segment.kind, UseSegmentKind::List(..)))
    });

    let remaining_width = if has_nested_list {
        0
    } else {
        shape.width.saturating_sub(2)
    };

    let tactic = definitive_tactic(
        &list_items,
        context.config.imports_layout(),
        Separator::Comma,
        remaining_width,
    );

    let ends_with_newline = tactic != DefinitiveListTactic::Horizontal;
    let trailing_separator = if ends_with_newline {
        context.config.trailing_comma()
    } else {
        super::lists::SeparatorTactic::Never
    };
    let fmt = ListFormatting::new(nested_shape, context.config)
        .tactic(tactic)
        .trailing_separator(trailing_separator)
        .ends_with_newline(ends_with_newline)
        .preserve_newline(true)
        .nested(has_nested_list);

    let list_str = write_list(&list_items, &fmt)?;

    let result = if list_str.contains('\n')
        || list_str.len() > remaining_width
        || tactic == DefinitiveListTactic::Vertical
    {
        format!(
            "{{\n{}{}\n{}}}",
            nested_shape.indent.to_string(context.config),
            list_str,
            shape.indent.to_string(context.config)
        )
    } else {
        format!("{{{list_str}}}")
    };

    Some(result)
}

impl Rewrite for UseSegment {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        Some(match self.kind {
            UseSegmentKind::Ident(ref ident, Some(ref rename)) => {
                format!("{ident} as {rename}")
            }
            UseSegmentKind::Ident(ref ident, None) => ident.clone(),
            UseSegmentKind::Slf(Some(ref rename)) => format!("self as {rename}"),
            UseSegmentKind::Slf(None) => "self".to_owned(),
            UseSegmentKind::Super(Some(ref rename)) => format!("super as {rename}"),
            UseSegmentKind::Super(None) => "super".to_owned(),
            UseSegmentKind::Crate(Some(ref rename)) => format!("crate as {rename}"),
            UseSegmentKind::Crate(None) => "crate".to_owned(),
            UseSegmentKind::Glob => "*".to_owned(),
            UseSegmentKind::List(ref use_tree_list) => rewrite_nested_use_tree(
                context,
                use_tree_list,
                // 1 = "{" and "}"
                shape.offset_left(1)?.sub_width(1)?,
            )?,
        })
    }
}

impl Rewrite for UseTree {
    // This does NOT format attributes and visibility or add a trailing `;`.
    fn rewrite(&self, context: &RewriteContext<'_>, mut shape: Shape) -> Option<String> {
        let mut result = String::with_capacity(256);
        let mut iter = self.path.iter().peekable();
        while let Some(segment) = iter.next() {
            let segment_str = segment.rewrite(context, shape)?;
            result.push_str(&segment_str);
            if iter.peek().is_some() {
                result.push_str("::");
                // 2 = "::"
                shape = shape.offset_left(2 + segment_str.len())?;
            }
        }
        Some(result)
    }
}
