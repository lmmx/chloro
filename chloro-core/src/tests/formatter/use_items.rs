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
