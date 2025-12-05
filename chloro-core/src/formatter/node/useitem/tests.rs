use super::*;
use insta::assert_snapshot;
use ra_ap_syntax::{Edition, SourceFile};

#[test]
fn test_format_use_with_submodule_grouping() {
    // This test ensures that different submodules are kept on separate lines
    // as rustfmt does, matching the exact formatting from the issue
    let input = r#"
use hir_def::{
    attr::AttrsWithOwner,
    expr_store::path::Path,
    item_scope::ItemInNs,
    per_ns::Namespace,
    resolver::{HasResolver, Resolver, TypeNs},
    AssocItemId, AttrDefId, ModuleDefId,
};
"#;

    let parse = SourceFile::parse(input, Edition::CURRENT);
    let root = parse.syntax_node();

    let mut formatted = String::new();
    for item in root.descendants() {
        if let Some(use_node) = ast::Use::cast(item) {
            format_use(use_node.syntax(), &mut formatted, 0);
            break;
        }
    }

    // Should match rustfmt's output: each submodule on its own line(s),
    // separated by blank lines
    assert_snapshot!(formatted, @r"
    use hir_def::{
        AssocItemId, AttrDefId, ModuleDefId,
        attr::AttrsWithOwner,
        expr_store::path::Path,
        item_scope::ItemInNs,
        per_ns::Namespace,
        resolver::{HasResolver, Resolver, TypeNs},
    };
    ");
}

#[test]
fn test_format_use_multiple_items_same_submodule() {
    let input = r#"use foo::{bar::A, bar::B, bar::C, baz::D};"#;

    let parse = SourceFile::parse(input, Edition::CURRENT);
    let root = parse.syntax_node();

    let mut formatted = String::new();
    for item in root.descendants() {
        if let Some(use_node) = ast::Use::cast(item) {
            format_use(use_node.syntax(), &mut formatted, 0);
            break;
        }
    }

    // Items from same submodule should be together, different submodules separated
    assert_snapshot!(formatted, @"use foo::{bar::A, bar::B, bar::C, baz::D};");
}

#[test]
fn test_format_use_root_level_items_only() {
    let input = r#"use foo::{A, B, C, D, E, F, G};"#;

    let parse = SourceFile::parse(input, Edition::CURRENT);
    let root = parse.syntax_node();

    let mut formatted = String::new();
    for item in root.descendants() {
        if let Some(use_node) = ast::Use::cast(item) {
            format_use(use_node.syntax(), &mut formatted, 0);
            break;
        }
    }

    // All root-level items should be grouped together on same line if they fit
    assert_snapshot!(formatted, @"use foo::{A, B, C, D, E, F, G};");
}

#[test]
fn test_parse_items_with_nested_braces() {
    let input = "event::Event, SyntaxKind::{self, TokenSet, EOF}, input::Input";
    let items = parse_items_with_nested_braces(input);

    assert_eq!(
        items,
        vec![
            "event::Event",
            "SyntaxKind::{self, TokenSet, EOF}",
            "input::Input"
        ]
    );
}

#[test]
fn test_format_use_with_nested_braces() {
    // Items with nested braces should not be split by submodule grouping
    let input = r#"use crate::{
    event::Event, input::Input, Edition, SyntaxKind::{self, TokenSet, EOF, ERROR, T, TOMBSTONE},
};"#;

    let parse = SourceFile::parse(input, Edition::CURRENT);
    let root = parse.syntax_node();

    let mut formatted = String::new();
    for item in root.descendants() {
        if let Some(use_node) = ast::Use::cast(item) {
            format_use(use_node.syntax(), &mut formatted, 0);
            break;
        }
    }

    // Should not add blank lines between items when nested braces are present
    assert_snapshot!(formatted, @r"
    use crate::{
        Edition,
        SyntaxKind::{self, EOF, ERROR, T, TOMBSTONE, TokenSet},
        event::Event,
        input::Input,
    };
    ");
}
