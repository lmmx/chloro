use super::*;
use insta::assert_snapshot;

#[test]
fn format_use_items_multiline_nested_mods() {
    let input = "use hir_def::{DefWithBodyId, GenericParamId, SyntheticSyntax, expr_store::{ExprOrPatPtr, ExpressionStoreSourceMap, hir_assoc_type_binding_to_ast, hir_generic_arg_to_ast, hir_segment_to_ast_segment}, hir::ExprOrPatId};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir_def::{
        DefWithBodyId, GenericParamId, SyntheticSyntax,
        expr_store::{
            ExprOrPatPtr, ExpressionStoreSourceMap, hir_assoc_type_binding_to_ast,
            hir_generic_arg_to_ast, hir_segment_to_ast_segment,
        },
        hir::ExprOrPatId,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_ast_module() {
    let input = "use syntax::{AstNode, AstPtr, SyntaxError, SyntaxNodePtr, TextRange, ast::{self, HasGenericArgs}, match_ast};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use syntax::{
        AstNode, AstPtr, SyntaxError, SyntaxNodePtr, TextRange,
        ast::{self, HasGenericArgs},
        match_ast,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_one_group() {
    let input = "use hir::{db::ExpandDatabase, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics, Symbol};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId,
        InFile, LocalSource, ModuleSource, Semantics, Symbol, db::ExpandDatabase, sym,
        symbols::FileSymbol,
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
        AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId,
        InFile, LocalSource, ModuleSource, Semantics, Symbol, db::ExpandDatabase, sym,
        symbols::FileSymbol,
    };
    ");
}

#[test]
fn format_use_items_multiline_nested_db_self() {
    let input = "use hir::{db::{self, ExpandDatabase}, sym, symbols::FileSymbol, AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId, InFile, LocalSource, ModuleSource, Semantics, Symbol};";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use hir::{
        AssocItem, Crate, FieldSource, HasContainer, HasCrate, HasSource, HirDisplay, HirFileId,
        InFile, LocalSource, ModuleSource, Semantics, Symbol,
        db::{self, ExpandDatabase},
        sym,
        symbols::FileSymbol,
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
        A, C, F, H, Ha, Has, Hi, Hir, I, L, M, S, Sy,
        db::{E, s},
        sym,
        symbols::F,
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
        Ha,
        db::{E, s},
        sym,
        symbols::F,
    };
    ");
}

#[test]
fn preserve_blank_line_between_mod_and_use() {
    // With blank line - should preserve it
    let input = r#"mod foo;

use bar::Baz;
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
mod foo;

use bar::Baz;
"#);
}

#[test]
fn preserve_no_blank_line_between_mod_and_use() {
    // Without blank line - should not add one
    let input = r#"mod foo;
use bar::Baz;
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
mod foo;
use bar::Baz;
"#);
}

#[test]
fn test_sort_imports_simple() {
    let input = "use a::{A, Ab, Ac, a, ab, ac, self};";
    let output = format_source(input);
    assert_snapshot!(output, @"use a::{self, A, Ab, Ac, a, ab, ac};");
}

#[test]
fn test_sort_imports_with_line_width() {
    let input = r#"use a::{
    A, AA, AAA, AAAA, AAAZ, AAAa, AAAz, AAZ, AAZA, AAZZ,
    AAZa, AAZz, AAa, AAaA, AAaZ, AAaa, AAaz, AAz, AAzA,
    AAzZ, AAza, AAzz, AZ, AZA, AZAA, AZAZ, AZAa, AZAz, AZZ,
    AZZA, AZZZ, AZZa, AZZz, AZa, AZaA, AZaZ, AZaa, AZaz,
    AZz, AZzA, AZzZ, AZza, AZzz, Aa, AaA, AaAA, AaAZ, AaAa,
    AaAz, AaZ, AaZA, AaZZ, AaZa, AaZz, Aaa, AaaA, AaaZ,
    Aaaa, Aaaz, Aaz, AazA, AazZ, Aaza, Aazz, Az, AzA, AzAA,
    AzAZ, AzAa, AzAz, AzZ, AzZA, AzZZ, AzZa, AzZz, Aza,
    AzaA, AzaZ, Azaa, Azaz, Azz, AzzA, AzzZ, Azza, Azzz, Z,
    ZA, ZAA, ZAAA, ZAAZ, ZAAa, ZAAz, ZAZ, ZAZA, ZAZZ, ZAZa,
    ZAZz, ZAa, ZAaA, ZAaZ, ZAaa, ZAaz, ZAz, ZAzA, ZAzZ,
    ZAza, ZAzz, ZZ, ZZA, ZZAA, ZZAZ, ZZAa, ZZAz, ZZZ, ZZZA,
    ZZZZ, ZZZa, ZZZz, ZZa, ZZaA, ZZaZ, ZZaa, ZZaz, ZZz,
    ZZzA, ZZzZ, ZZza, ZZzz, Za, ZaA, ZaAA, ZaAZ, ZaAa, ZaAz,
    ZaZ, ZaZA, ZaZZ, ZaZa, ZaZz, Zaa, ZaaA, ZaaZ, Zaaa,
    Zaaz, Zaz, ZazA, ZazZ, Zaza, Zazz, Zz, ZzA, ZzAA, ZzAZ,
    ZzAa, ZzAz, ZzZ, ZzZA, ZzZZ, ZzZa, ZzZz, Zza, ZzaA,
    ZzaZ, Zzaa, Zzaz, Zzz, ZzzA, ZzzZ, Zzza, Zzzz, a, aA,
    aAA, aAAA, aAAZ, aAAa, aAAz, aAZ, aAZA, aAZZ, aAZa,
    aAZz, aAa, aAaA, aAaZ, aAaa, aAaz, aAz, aAzA, aAzZ,
    aAza, aAzz, aZ, aZA, aZAA, aZAZ, aZAa, aZAz, aZZ, aZZA,
    aZZZ, aZZa, aZZz, aZa, aZaA, aZaZ, aZaa, aZaz, aZz,
    aZzA, aZzZ, aZza, aZzz, aa, aaA, aaAA, aaAZ, aaAa, aaAz,
    aaZ, aaZA, aaZZ, aaZa, aaZz, aaa, aaaA, aaaZ, aaaa,
    aaaz, aaz, aazA, aazZ, aaza, aazz, az, azA, azAA, azAZ,
    azAa, azAz, azZ, azZA, azZZ, azZa, azZz, aza, azaA,
    azaZ, azaa, azaz, azz, azzA, azzZ, azza, azzz, z, zA,
    zAA, zAAA, zAAZ, zAAa, zAAz, zAZ, zAZA, zAZZ, zAZa,
    zAZz, zAa, zAaA, zAaZ, zAaa, zAaz, zAz, zAzA, zAzZ,
    zAza, zAzz, zZ, zZA, zZAA, zZAZ, zZAa, zZAz, zZZ, zZZA,
    zZZZ, zZZa, zZZz, zZa, zZaA, zZaZ, zZaa, zZaz, zZz,
    zZzA, zZzZ, zZza, zZzz, za, zaA, zaAA, zaAZ, zaAa, zaAz,
    zaZ, zaZA, zaZZ, zaZa, zaZz, zaa, zaaA, zaaZ, zaaa,
    zaaz, zaz, zazA, zazZ, zaza, zazz, zz, zzA, zzAA, zzAZ,
    zzAa, zzAz, zzZ, zzZA, zzZZ, zzZa, zzZz, zza, zzaA,
    zzaZ, zzaa, zzaz, zzz, zzzA, zzzZ, zzza, zzzz, _A, _Aa,
    _AaA, _Aaa, __A, __Aa, ___A, ___a, __a, __aA, _a, _aA,
    _aAA
};"#;

    let output = format_source(input);
    assert_snapshot!(output, @r"
    use a::{
        ___A, ___a, __A, __Aa, __a, __aA, _A, _Aa, _AaA, _Aaa, _a, _aA, _aAA, A, AA, AAA, AAAA, AAAZ,
        AAAa, AAAz, AAZ, AAZA, AAZZ, AAZa, AAZz, AAa, AAaA, AAaZ, AAaa, AAaz, AAz, AAzA, AAzZ, AAza,
        AAzz, AZ, AZA, AZAA, AZAZ, AZAa, AZAz, AZZ, AZZA, AZZZ, AZZa, AZZz, AZa, AZaA, AZaZ, AZaa,
        AZaz, AZz, AZzA, AZzZ, AZza, AZzz, Aa, AaA, AaAA, AaAZ, AaAa, AaAz, AaZ, AaZA, AaZZ, AaZa,
        AaZz, Aaa, AaaA, AaaZ, Aaaa, Aaaz, Aaz, AazA, AazZ, Aaza, Aazz, Az, AzA, AzAA, AzAZ, AzAa,
        AzAz, AzZ, AzZA, AzZZ, AzZa, AzZz, Aza, AzaA, AzaZ, Azaa, Azaz, Azz, AzzA, AzzZ, Azza, Azzz, Z,
        ZA, ZAA, ZAAA, ZAAZ, ZAAa, ZAAz, ZAZ, ZAZA, ZAZZ, ZAZa, ZAZz, ZAa, ZAaA, ZAaZ, ZAaa, ZAaz, ZAz,
        ZAzA, ZAzZ, ZAza, ZAzz, ZZ, ZZA, ZZAA, ZZAZ, ZZAa, ZZAz, ZZZ, ZZZA, ZZZZ, ZZZa, ZZZz, ZZa,
        ZZaA, ZZaZ, ZZaa, ZZaz, ZZz, ZZzA, ZZzZ, ZZza, ZZzz, Za, ZaA, ZaAA, ZaAZ, ZaAa, ZaAz, ZaZ,
        ZaZA, ZaZZ, ZaZa, ZaZz, Zaa, ZaaA, ZaaZ, Zaaa, Zaaz, Zaz, ZazA, ZazZ, Zaza, Zazz, Zz, ZzA,
        ZzAA, ZzAZ, ZzAa, ZzAz, ZzZ, ZzZA, ZzZZ, ZzZa, ZzZz, Zza, ZzaA, ZzaZ, Zzaa, Zzaz, Zzz, ZzzA,
        ZzzZ, Zzza, Zzzz, a, aA, aAA, aAAA, aAAZ, aAAa, aAAz, aAZ, aAZA, aAZZ, aAZa, aAZz, aAa, aAaA,
        aAaZ, aAaa, aAaz, aAz, aAzA, aAzZ, aAza, aAzz, aZ, aZA, aZAA, aZAZ, aZAa, aZAz, aZZ, aZZA,
        aZZZ, aZZa, aZZz, aZa, aZaA, aZaZ, aZaa, aZaz, aZz, aZzA, aZzZ, aZza, aZzz, aa, aaA, aaAA,
        aaAZ, aaAa, aaAz, aaZ, aaZA, aaZZ, aaZa, aaZz, aaa, aaaA, aaaZ, aaaa, aaaz, aaz, aazA, aazZ,
        aaza, aazz, az, azA, azAA, azAZ, azAa, azAz, azZ, azZA, azZZ, azZa, azZz, aza, azaA, azaZ,
        azaa, azaz, azz, azzA, azzZ, azza, azzz, z, zA, zAA, zAAA, zAAZ, zAAa, zAAz, zAZ, zAZA, zAZZ,
        zAZa, zAZz, zAa, zAaA, zAaZ, zAaa, zAaz, zAz, zAzA, zAzZ, zAza, zAzz, zZ, zZA, zZAA, zZAZ,
        zZAa, zZAz, zZZ, zZZA, zZZZ, zZZa, zZZz, zZa, zZaA, zZaZ, zZaa, zZaz, zZz, zZzA, zZzZ, zZza,
        zZzz, za, zaA, zaAA, zaAZ, zaAa, zaAz, zaZ, zaZA, zaZZ, zaZa, zaZz, zaa, zaaA, zaaZ, zaaa,
        zaaz, zaz, zazA, zazZ, zaza, zazz, zz, zzA, zzAA, zzAZ, zzAa, zzAz, zzZ, zzZA, zzZZ, zzZa,
        zzZz, zza, zzaA, zzaZ, zzaa, zzaz, zzz, zzzA, zzzZ, zzza, zzzz,
    };
    ");
}

#[test]
fn test_import_order_super_before_crate() {
    let input = r#"use crate::foo;
use super::bar;"#;

    let output = format_source(input);
    assert_snapshot!(output, @r"
    use super::bar;
    use crate::foo;
    ");
}

#[test]
fn test_use_reordering() {
    let input = r#"use std::*;
use a;
use b;
use c;
use z;
use crate::e;
use crate::f;
use super::a;
use self::*;
use alloc::*;
use core::*;
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
use self::*;
use super::a;
use crate::e;
use crate::f;
use a;
use alloc::*;
use b;
use c;
use core::*;
use std::*;
use z;
");
}
