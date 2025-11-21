//! Builtin attributes resolved by nameres.
//!
//! The actual definitions were copied from rustc's `compiler/rustc_feature/src/builtin_attrs.rs`.
//!
//! It was last synchronized with upstream commit c3def263a44e07e09ae6d57abfc8650227fb4972.
//!
//! The macros were adjusted to only expand to the attribute name, since that is all we need to do
//! name resolution, and `BUILTIN_ATTRIBUTES` is almost entirely unchanged from the original, to
//! ease updating.

use std::sync::OnceLock;

use intern::Symbol;
use rustc_hash::FxHashMap;
pub struct BuiltinAttribute {
    pub name: &'static str,
    pub template: AttributeTemplate,
}

/// A template that the attribute input must match.
/// Only top-level shape (`#[attr]` vs `#[attr(...)]` vs `#[attr = ...]`) is considered now.
#[derive(Clone, Copy)]
pub struct AttributeTemplate {
    pub word: bool,
    pub list: Option<&'static str>,
    pub name_value_str: Option<&'static str>,
}

pub fn find_builtin_attr_idx(name: &Symbol) -> Option<usize> {
    static BUILTIN_LOOKUP_TABLE: OnceLock<FxHashMap<Symbol, usize>> = OnceLock::new();
    BUILTIN_LOOKUP_TABLE
        .get_or_init(|| {
            INERT_ATTRIBUTES
                .iter()
                .map(|attr| attr.name)
                .enumerate()
                .map(|(a, b)| (Symbol::intern(b), a))
                .collect()
        })
        .get(name)
        .copied()
}

/// A convenience macro for constructing attribute templates.
/// E.g., `template!(Word, List: "description")` means that the attribute
/// supports forms `#[attr]` and `#[attr(description)]`.
macro_rules! template {
    (Word) => { template!(@ true, None, None) };
    (List: $descr: expr) => { template!(@ false, Some($descr), None) };
    (NameValueStr: $descr: expr) => { template!(@ false, None, Some($descr)) };
    (Word, List: $descr: expr) => { template!(@ true, Some($descr), None) };
    (Word, NameValueStr: $descr: expr) => { template!(@ true, None, Some($descr)) };
    (List: $descr1: expr, NameValueStr: $descr2: expr) => {
        template!(@ false, Some($descr1), Some($descr2))
    };
    (Word, List: $descr1: expr, NameValueStr: $descr2: expr) => {
        template!(@ true, Some($descr1), Some($descr2))
    };
    (@ $word: expr, $list: expr, $name_value_str: expr) => {
        AttributeTemplate {
            word: $word, list: $list, name_value_str: $name_value_str
        }
    };
}

macro_rules! ungated {
    ($attr:ident, $typ:expr, $tpl:expr, $duplicates:expr $(, @only_local: $only_local:expr)? $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

macro_rules! gated {
    ($attr:ident, $typ:expr, $tpl:expr, $duplicates:expr $(, @only_local: $only_local:expr)?, $gate:ident, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
    ($attr:ident, $typ:expr, $tpl:expr, $duplicates:expr $(, @only_local: $only_local:expr)?, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

macro_rules! rustc_attr {
    (TEST, $attr:ident, $typ:expr, $tpl:expr, $duplicate:expr $(, @only_local: $only_local:expr)? $(,)?) => {
        rustc_attr!(
            $attr,
            $typ,
            $tpl,
            $duplicate,
            $(@only_local: $only_local,)?
            concat!(
                "the `#[",
                stringify!($attr),
                "]` attribute is just used for rustc unit tests \
                and will never be stable",
            ),
        )
    };
    ($attr:ident, $typ:expr, $tpl:expr, $duplicates:expr $(, @only_local: $only_local:expr)?, $msg:expr $(,)?) => {
        BuiltinAttribute { name: stringify!($attr), template: $tpl }
    };
}

#[allow(unused_macros)]
macro_rules! experimental {
    ($attr:ident) => {
        concat!("the `#[", stringify!($attr), "]` attribute is an experimental feature")
    };
}

/// Attributes that have a special meaning to rustc or rustdoc.
#[rustfmt::skip]
// ==========================================================================
// Stable attributes:
// ==========================================================================
// Conditional compilation:
// Testing:
// FIXME(Centril): This can be used on stable but shouldn't.
// Macros:
// Deprecated synonym for `macro_use`.
// Lints:
// Crate properties:
// crate_id is deprecated
// ABI, linking, symbols, and FFI
// Limits:
// Entry point:
// Modules, prelude, and resolution:
// Runtime
// RFC 2070
// Code generation:
// Debugging
// ==========================================================================
// Unstable attributes:
// ==========================================================================
// Linking:
// Testing:
// RFC #1268
// RFC 2412
// RFC 2632
// lang-team MCP 147
// `#[collapse_debuginfo]`
// RFC 2397
// `#[cfi_encoding = ""]`
// ==========================================================================
// Internal attributes: Stability, deprecation, and unsafe:
// ==========================================================================
// DuplicatesOk since it has its own validation
// ==========================================================================
// Internal attributes: Type system related:
// ==========================================================================
// ==========================================================================
// Internal attributes: Runtime related:
// ==========================================================================
// ==========================================================================
// Internal attributes, Linkage:
// ==========================================================================
// ==========================================================================
// Internal attributes, Macro related:
// ==========================================================================
// ==========================================================================
// Internal attributes, Diagnostics related:
// ==========================================================================
// Enumerates "identity-like" conversion methods to suggest on type mismatch.
// Prevents field reads in the marked trait or method to be considered
// during dead code analysis.
// Used by the `rustc::potential_query_instability` lint to warn methods which
// might not be stable during incremental compilation.
// Used by the `rustc::untracked_query_information` lint to warn methods which
// might break incremental compilation.
// Used by the `rustc::untranslatable_diagnostic` and `rustc::diagnostic_outside_of_impl` lints
// to assist in changes to diagnostic APIs.
// Used by the `rustc::bad_opt_access` lint to identify `DebuggingOptions` and `CodegenOptions`
// types (as well as any others in future).
// Used by the `rustc::bad_opt_access` lint on fields
// types (as well as any others in future).
// ==========================================================================
// Internal attributes, Const related:
// ==========================================================================
// Do not const-check this function's body. It will always get replaced during CTFE via `hook_special_const_fn`.
// Ensure the argument to this function is &&str during const-check.
// ==========================================================================
// Internal attributes, Layout related:
// ==========================================================================
// ==========================================================================
// Internal attributes, Misc:
// ==========================================================================
// name: sym::rustc_diagnostic_item,
// FIXME: This can be `true` once we always use `tcx.is_diagnostic_item`.
// only_local: false,
// type_: Normal,
// duplicates: ErrorFollowing,
// gate: Gated(
// Stability::Unstable,
// sym::rustc_attrs,
// "diagnostic items compiler internal support for linting",
// cfg_fn!(rustc_attrs),
// ),
// Used in resolve:
// ==========================================================================
// Internal attributes, Testing:
// ==========================================================================
/* doesn't matter*/
