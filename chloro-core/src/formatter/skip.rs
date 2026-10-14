//! Names of macros and attributes left unformatted (rustfmt's `skip.rs`):
//! `#[rustfmt::skip::macros(name, ..)]` and `#[rustfmt::skip::attributes(name, ..)]` on an
//! item, or as inner attributes of a file or module, apply to the code inside the item.

use std::collections::HashSet;

use crate::cst::ast::{self, AstNode};

use super::attr::{MetaItemInner, meta_item_list};
use super::nodes::{Attribute, child};

/// Which macro calls and attributes to leave as written. Only item attributes update it
/// (`FmtVisitor::visit_item`), and nested visitors inherit it. Contexts hold it as an
/// `Option<Rc<SkipContext>>`, `None` when no names are listed.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct SkipContext {
    pub(crate) macros: SkipNameContext,
    pub(crate) attributes: SkipNameContext,
}

impl SkipContext {
    pub(crate) fn update_with_attrs(&mut self, attrs: &[Attribute]) {
        self.macros.extend(get_skip_names("macros", attrs));
        self.attributes.extend(get_skip_names("attributes", attrs));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.macros.is_empty() && self.attributes.is_empty()
    }
}

/// Whether `attr` may be a `#[rustfmt::skip::<kind>(..)]` attribute: its path starts with
/// `rustfmt`. A filter without allocation in front of [`SkipContext::update_with_attrs`],
/// which compares the whole path.
pub(crate) fn may_list_skip_names(attr: &Attribute<'_>) -> bool {
    let Attribute::Normal(normal) = attr else {
        return false;
    };
    child::<ast::Meta>(normal.syntax())
        .and_then(|meta| meta.path())
        .is_some_and(|path| path.syntax().text().starts_with("rustfmt"))
}

/// rustfmt's `SkipNameContext`. The `All` variant comes only from the unstable
/// `skip_macro_invocations` option, fixed at its default (no names) in chloro.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct SkipNameContext(HashSet<String>);

impl SkipNameContext {
    fn extend(&mut self, names: Vec<String>) {
        self.0.extend(names);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn skip(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

/// The names listed by `#[rustfmt::skip::<kind>(..)]` attributes: each single-segment path
/// (`MetaItemInner::ident`), whether a word, a list or a name-value item.
fn get_skip_names(kind: &str, attrs: &[Attribute]) -> Vec<String> {
    let path = format!("rustfmt::skip::{kind}");
    let mut skip_names = vec![];
    for attr in attrs {
        let Attribute::Normal(normal) = attr else {
            continue;
        };
        if attr.path_text().as_deref() != Some(path.as_str()) {
            continue;
        }
        for inner in meta_item_list(normal).unwrap_or_default() {
            if let MetaItemInner::MetaItem(item) = inner
                && !item.path.contains("::")
            {
                skip_names.push(item.path);
            }
        }
    }
    skip_names
}
