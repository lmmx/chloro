use chloro_core::format_source;

#[test]
fn test_simple_module() {
    let input = "mod utils;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_public_module() {
    let input = "pub mod api;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_inline_module() {
    let input = "mod helpers { fn helper() {} }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_nested_modules() {
    let input = "mod outer { mod inner { fn nested() {} } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_module_with_multiple_items() {
    let input = "pub mod lib { pub fn a() {} pub fn b() {} pub struct C; }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_module_with_use_statements() {
    let input = "mod utils { use std::collections::HashMap; fn helper() {} }";
    insta::assert_snapshot!(format_source(input));
}
