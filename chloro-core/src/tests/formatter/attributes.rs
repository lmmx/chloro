use super::*;
use insta::assert_snapshot;

#[test]
fn preserve_inner_attribute_allow() {
    let input = "#![allow(non_camel_case_types)]\n";
    let output = format_source(input);
    assert_snapshot!(output, @"#![allow(non_camel_case_types)]
");
}

#[test]
fn preserve_inner_attribute_feature() {
    let input = "#![feature(test)]\n";
    let output = format_source(input);
    assert_snapshot!(output, @"#![feature(test)]
");
}

#[test]
fn preserve_outer_attribute_derive() {
    let input = "#[derive(Debug, Clone)]\nstruct Foo;\n";
    let output = format_source(input);
    assert_snapshot!(output, @r"
    #[derive(Debug, Clone)]
    struct Foo;
    ");
}

#[test]
fn preserve_nested_attribute_content() {
    let input = r#"#[cfg(all(feature = "foo", target_os = "linux"))]
fn foo() {}
"#;
    let output = format_source(input);
    assert!(output.contains(r#"#[cfg(all(feature = "foo", target_os = "linux"))]"#));
}

#[test]
fn preserve_extern_block_with_function() {
    let input = r#"extern "C" { fn printf(format: *const i8, ...) -> i32; }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    extern "C" {
        fn printf(format: *const i8, ...) -> i32;
    }
    "#);
}

#[test]
fn preserve_extern_block_multiline() {
    let input = r#"extern "C" {
    fn printf(format: *const i8, ...) -> i32;
    fn exit(code: i32) -> !;
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    extern "C" {
        fn printf(format: *const i8, ...) -> i32;
        fn exit(code: i32) -> !;
    }
    "#);
}

#[test]
fn preserve_unsafe_extern_block() {
    let input = r#"unsafe extern "C" {}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"unsafe extern "C" {}"#);
}

#[test]
fn preserve_extern_block_with_attributed_variadic() {
    let input = r#"extern "C" { fn printf(format: *const i8, #[attr] ...) -> i32; }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    extern "C" {
        fn printf(format: *const i8, #[attr] ...) -> i32;
    }
    "#);
}

#[test]
fn preserve_inner_attribute_full_content() {
    let input = r#"#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![recursion_limit = "128"]
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]
    #![allow(unreachable_code)]
    #![recursion_limit = "128"]
    "#);
}

#[test]
fn preserve_multiple_inner_attributes_with_blank_line() {
    let input = r#"#![allow(non_camel_case_types)]
#![allow(dead_code)]

#![recursion_limit = "128"]
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]

    #![recursion_limit = "128"]
    "#);
}
