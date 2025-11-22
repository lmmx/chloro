//! Builtin macros and attributes

#[macro_use]
pub mod quote;
mod attr_macro;
mod derive_macro;
mod fn_macro;

pub use self::{
    attr_macro::{BuiltinAttrExpander, derive_macro::{BuiltinDeriveExpander, find_builtin_attr,
    find_builtin_derive}, find_builtin_macro, fn_macro::{
        BuiltinFnLikeExpander,
    include_input_to_file_id, pseudo_derive_attr_expansion}, EagerExpander, },
};
