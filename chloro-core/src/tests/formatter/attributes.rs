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
