use super::*;
use insta::assert_snapshot;

#[test]
fn format_use_items_multiline_nested_mods() {
    let input = "use hir_def::{DefWithBodyId, GenericParamId, SyntheticSyntax, expr_store::{ExprOrPatPtr, ExpressionStoreSourceMap, hir_assoc_type_binding_to_ast, hir_generic_arg_to_ast, hir_segment_to_ast_segment}, hir::ExprOrPatId};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir_def::{
        expr_store::{
            hir_assoc_type_binding_to_ast, hir_generic_arg_to_ast, hir_segment_to_ast_segment,
            ExprOrPatPtr, ExpressionStoreSourceMap,
        },
        hir::ExprOrPatId,
        DefWithBodyId, GenericParamId, SyntheticSyntax,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_ast_module() {
    let input = "use syntax::{AstNode, AstPtr, SyntaxError, SyntaxNodePtr, TextRange, ast::{self, HasGenericArgs}, match_ast};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use syntax::{
        ast::{self, HasGenericArgs},
        match_ast, AstNode, AstPtr, SyntaxError, SyntaxNodePtr, TextRange,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_one_group() {
    let input = "use hir::{db::ExpandDatabase, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics, Symbol};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        db::ExpandDatabase, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer,
        HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics,
        Symbol,
    };
    ");
}

/// The singleton nested group gets its braces removed and it isn't treated as a group.
#[test]
fn format_use_items_multiline_nested_db_singleton_not_a_group() {
    let input = "use hir::{db::{ExpandDatabase}, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics, Symbol};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        db::ExpandDatabase, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer,
        HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics,
        Symbol,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_db_self() {
    let input = "use hir::{db::{self, ExpandDatabase}, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics, Symbol};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        db::{self, ExpandDatabase},
        sym,
        symbols::FileSymbol,
        AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId,
        InFile, LocalSource, ModuleSource, Semantics, Symbol,
    };
    ");
}

/// Shortened identifier version of `format_use_items_multiline_nested_db_self`
#[test]
fn format_use_items_multiline_nested_db_self_abbreviated() {
    let input =
        "use hir::{db::{s, E}, sym, symbols::F, Ha, Has, Hi, Hir, Sy, A, C, F, H, I, L, M, S};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        db::{s, E},
        sym,
        symbols::F,
        Ha, Has, Hi, Hir, Sy, A, C, F, H, I, L, M, S,
    };
    ");
}

/// Less identifiers in root level than `format_use_items_multiline_nested_db_self_abbreviated`
#[test]
fn format_use_items_multiline_nested_db_self_abbreviated_reduced() {
    let input = "use hir::{db::{s, E}, sym, symbols::F, Ha};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        db::{s, E},
        sym,
        symbols::F,
        Ha,
    };
    ");
}
