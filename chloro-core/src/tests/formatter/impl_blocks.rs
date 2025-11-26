use super::*;

#[test]
fn impl_methods_have_blank_lines() {
    let input = r#"impl Foo {
    pub fn method_a(&self) -> i32 {
        1
    }
    pub fn method_b(&self) -> i32 {
        2
    }
}
"#;
    let output = format_source(input);
    // Methods should be separated by blank lines
    assert!(output.contains("}\n\n    pub fn method_b"));
}

#[test]
fn impl_preserves_comments_on_methods() {
    let input = r#"impl Foo {
    // This method is consumed by external tools. Don't remove, please.
    pub fn expression_types(&self) -> impl Iterator<Item = i32> {
        std::iter::empty()
    }
}
"#;
    let output = format_source(input);
    assert!(output.contains("// This method is consumed by external tools"));
}
