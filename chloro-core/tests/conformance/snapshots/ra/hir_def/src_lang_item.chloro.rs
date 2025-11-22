//! Collects lang items: items marked with `#[lang = "..."]` attribute.
//!
//! This attribute to tell the compiler about semi built-in std library
//! features, such as Fn family of traits.

use hir_expand::name::Name;
use intern::{Symbol, sym};
use rustc_hash::FxHashMap;
use triomphe::Arc;

use crate::{
    db::DefDatabase, expr_store::path::Path,
    nameres::{assoc::TraitItems, crate_def_map, crate_local_def_map}, AdtId, AssocItemId, AttrDefId,
    Crate, EnumId, EnumVariantId, FunctionId, ImplId, ModuleDefId, StaticId, StructId, TraitId,
    TypeAliasId, UnionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangItemTarget {
    EnumId(EnumId),
    Function(FunctionId),
    ImplDef(ImplId),
    Static(StaticId),
    Struct(StructId),
    Union(UnionId),
    TypeAlias(TypeAliasId),
    Trait(TraitId),
    EnumVariant(EnumVariantId),
}

impl LangItemTarget {
    pub fn as_enum(self) -> Option<EnumId> {
        match self {
            LangItemTarget::EnumId(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_function(self) -> Option<FunctionId> {
        match self {
            LangItemTarget::Function(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_impl_def(self) -> Option<ImplId> {
        match self {
            LangItemTarget::ImplDef(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_static(self) -> Option<StaticId> {
        match self {
            LangItemTarget::Static(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_struct(self) -> Option<StructId> {
        match self {
            LangItemTarget::Struct(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_trait(self) -> Option<TraitId> {
        match self {
            LangItemTarget::Trait(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_enum_variant(self) -> Option<EnumVariantId> {
        match self {
            LangItemTarget::EnumVariant(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_type_alias(self) -> Option<TypeAliasId> {
        match self {
            LangItemTarget::TypeAlias(id) => Some(id),
            _ => None,
        }
    }

    pub fn as_adt(self) -> Option<AdtId> {
        match self {
            LangItemTarget::Union(it) => Some(it.into()),
            LangItemTarget::EnumId(it) => Some(it.into()),
            LangItemTarget::Struct(it) => Some(it.into()),
            _ => None,
        }
    }
}

/// Salsa query. This will look for lang items in a specific crate.
#[salsa_macros::tracked(returns(ref))]
pub fn crate_lang_items(
    db: &dyn DefDatabase,
    krate: Crate,
) -> Option<Box<LangItems>> {
    let _p = tracing::info_span!("crate_lang_items_query").entered();
    let mut lang_items = LangItems::default();
    let crate_def_map = crate_def_map(db, krate);
    for (_, module_data) in crate_def_map.modules() {
        for impl_def in module_data.scope.impls() {
            lang_items.collect_lang_item(db, impl_def, LangItemTarget::ImplDef);
            for &(_, assoc) in impl_def.impl_items(db).items.iter() {
                match assoc {
                    AssocItemId::FunctionId(f) => {
                        lang_items.collect_lang_item(db, f, LangItemTarget::Function)
                    }
                    AssocItemId::TypeAliasId(t) => {
                        lang_items.collect_lang_item(db, t, LangItemTarget::TypeAlias)
                    }
                    AssocItemId::ConstId(_) => (),
                }
            }
        }

        for def in module_data.scope.declarations() {
            match def {
                ModuleDefId::TraitId(trait_) => {
                    lang_items.collect_lang_item(db, trait_, LangItemTarget::Trait);
                    TraitItems::query(db, trait_).items.iter().for_each(|&(_, assoc_id)| {
                        match assoc_id {
                            AssocItemId::FunctionId(f) => {
                                lang_items.collect_lang_item(db, f, LangItemTarget::Function);
                            }
                            AssocItemId::TypeAliasId(alias) => {
                                lang_items.collect_lang_item(db, alias, LangItemTarget::TypeAlias)
                            }
                            AssocItemId::ConstId(_) => {}
                        }
                    });
                }
                ModuleDefId::AdtId(AdtId::EnumId(e)) => {
                    lang_items.collect_lang_item(db, e, LangItemTarget::EnumId);
                    e.enum_variants(db).variants.iter().for_each(|&(id, _, _)| {
                        lang_items.collect_lang_item(db, id, LangItemTarget::EnumVariant);
                    });
                }
                ModuleDefId::AdtId(AdtId::StructId(s)) => {
                    lang_items.collect_lang_item(db, s, LangItemTarget::Struct);
                }
                ModuleDefId::AdtId(AdtId::UnionId(u)) => {
                    lang_items.collect_lang_item(db, u, LangItemTarget::Union);
                }
                ModuleDefId::FunctionId(f) => {
                    lang_items.collect_lang_item(db, f, LangItemTarget::Function);
                }
                ModuleDefId::StaticId(s) => {
                    lang_items.collect_lang_item(db, s, LangItemTarget::Static);
                }
                ModuleDefId::TypeAliasId(t) => {
                    lang_items.collect_lang_item(db, t, LangItemTarget::TypeAlias);
                }
                _ => {}
            }
        }
    }
    if lang_items.items.is_empty() { None } else { Some(Box::new(lang_items)) }
}

/// Salsa query. Look for a lang item, starting from the specified crate and recursively
/// traversing its dependencies.
#[salsa_macros::tracked]
pub fn lang_item(
    db: &dyn DefDatabase,
    start_crate: Crate,
    item: LangItem,
) -> Option<LangItemTarget> {
    let _p = tracing::info_span!("lang_item_query").entered();
    if let Some(target) =
        crate_lang_items(db, start_crate).as_ref().and_then(|it| it.items.get(&item).copied())
    {
        return Some(target);
    }
    // Our `CrateGraph` eagerly inserts sysroot dependencies like `core` or `std` into dependencies
    // even if the target crate has `#![no_std]`, `#![no_core]` or shadowed sysroot dependencies
    // like `dependencies.std.path = ".."`. So we use `extern_prelude()` instead of
    // `CrateData.dependencies` here, which has already come through such sysroot complexities
    // while nameres.
    //
    // See https://github.com/rust-lang/rust-analyzer/pull/20475 for details.
    crate_local_def_map(db, start_crate).local(db).extern_prelude().find_map(|(_, (krate, _))| {
        // Some crates declares themselves as extern crate like `extern crate self as core`.
        // Ignore these to prevent cycles.
        if krate.krate == start_crate { None } else { lang_item(db, krate.krate, item) }
    })
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct LangItems {
    items: FxHashMap<LangItem, LangItemTarget>,
}

impl LangItems {
    pub fn target(
        &self,
        item: LangItem,
    ) -> Option<LangItemTarget> {
        self.items.get(&item).copied()
    }

    fn collect_lang_item<T>(
        &mut self,
        db: &dyn DefDatabase,
        item: T,
        constructor: fn(T) -> LangItemTarget,
    )
    where
        T: Into<AttrDefId> + Copy, {
        let _p = tracing::info_span!("collect_lang_item").entered();
        if let Some(lang_item) = lang_attr(db, item.into()) {
            self.items.entry(lang_item).or_insert_with(|| constructor(item));
        }
    }
}

pub(crate) fn lang_attr(
    db: &dyn DefDatabase,
    item: AttrDefId,
) -> Option<LangItem> {
    db.attrs(item).lang_item()
}

pub(crate) fn notable_traits_in_deps(
    db: &dyn DefDatabase,
    krate: Crate,
) -> Arc<[Arc<[TraitId]>]> {
    let _p = tracing::info_span!("notable_traits_in_deps", ?krate).entered();
    Arc::from_iter(
        db.transitive_deps(krate).into_iter().filter_map(|krate| db.crate_notable_traits(krate)),
    )
}

pub(crate) fn crate_notable_traits(
    db: &dyn DefDatabase,
    krate: Crate,
) -> Option<Arc<[TraitId]>> {
    let _p = tracing::info_span!("crate_notable_traits", ?krate).entered();
    let mut traits = Vec::new();
    let crate_def_map = crate_def_map(db, krate);
    for (_, module_data) in crate_def_map.modules() {
        for def in module_data.scope.declarations() {
            if let ModuleDefId::TraitId(trait_) = def
                && db.attrs(trait_.into()).has_doc_notable_trait()
            {
                traits.push(trait_);
            }
        }
    }
    if traits.is_empty() { None } else { Some(traits.into_iter().collect()) }
}

pub enum GenericRequirement {
    None,
    Minimum(usize),
    Exact(usize),
}

macro_rules! language_item_table {
    (
        $( $(#[$attr:meta])* $variant:ident, $module:ident :: $name:ident, $method:ident, $target:expr, $generics:expr; )*
    ) => {

        /// A representation of all the valid language items in Rust.
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub enum LangItem {
            $(
                #[doc = concat!("The `", stringify!($name), "` lang item.")]
                $(#[$attr])*
                $variant,
            )*
        }

        impl LangItem {
            pub fn name(self) -> &'static str {
                match self {
                    $( LangItem::$variant => stringify!($name), )*
                }
            }

            /// Opposite of [`LangItem::name`]
            pub fn from_symbol(sym: &Symbol) -> Option<Self> {
                match sym {
                    $(sym if *sym == $module::$name => Some(LangItem::$variant), )*
                    _ => None,
                }
            }
        }
    }
}

impl LangItem {
    pub fn resolve_function(
        self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<FunctionId> {
        lang_item(db, start_crate, self).and_then(|t| t.as_function())
    }

    pub fn resolve_trait(
        self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<TraitId> {
        lang_item(db, start_crate, self).and_then(|t| t.as_trait())
    }

    pub fn resolve_adt(
        self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<AdtId> {
        lang_item(db, start_crate, self).and_then(|t| t.as_adt())
    }

    pub fn resolve_enum(
        self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<EnumId> {
        lang_item(db, start_crate, self).and_then(|t| t.as_enum())
    }

    pub fn resolve_type_alias(
        self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<TypeAliasId> {
        lang_item(db, start_crate, self).and_then(|t| t.as_type_alias())
    }

    /// Opposite of [`LangItem::name`]
    pub fn from_name(name: &hir_expand::name::Name) -> Option<Self> {
        Self::from_symbol(name.symbol())
    }

    pub fn path(
        &self,
        db: &dyn DefDatabase,
        start_crate: Crate,
    ) -> Option<Path> {
        let t = lang_item(db, start_crate, *self)?;
        Some(Path::LangItem(t, None))
    }

    pub fn ty_rel_path(
        &self,
        db: &dyn DefDatabase,
        start_crate: Crate,
        seg: Name,
    ) -> Option<Path> {
        let t = lang_item(db, start_crate, *self)?;
        Some(Path::LangItem(t, Some(seg)))
    }
}
//  Variant name,            Name,                     Getter method name,         Target                  Generic requirements;
/// Trait injected by `#[derive(PartialEq)]`, (i.e. "Partial EQ").
/// Trait injected by `#[derive(Eq)]`, (i.e. "Total EQ"; no, I will not apologize).
/// The associated item of the [`DiscriminantKind`] trait.
// language items relating to transmutability
// A number of panic-related lang items. The `panic` item corresponds to divide-by-zero and
// various panic cases with `match`. The `panic_bounds_check` item is for indexing arrays.
//
// The `begin_unwind` lang item has a predefined symbol name and is sort of a "weak lang item"
// in the sense that a crate is not required to have it defined to use it, but a final product
// is required to define it somewhere. Additionally, there are restrictions on crates that use
// a weak lang item, but do not have it defined.
/// libstd panic entry point. Necessary for const eval to be able to catch it
// Lang items needed for `format_args!()`.
/// Align offset for stride != 1; must not panic.
// Language items from AST lowering
// FIXME(swatinem): the following lang items are used for async lowering and
// should become obsolete eventually.
