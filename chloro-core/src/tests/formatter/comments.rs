use super::*;
use insta::assert_snapshot;

#[test]
fn preserve_fixme_comments() {
    let input = r#"struct Foo {
    // FIXME: This should be fixed
    field: i32,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    struct Foo {
        // FIXME: This should be fixed
        field: i32,
    }
    "#);
}

#[test]
fn preserve_inline_comments_in_enum() {
    let input = r#"enum Foo {
    // FIXME: We should use this when appropriate.
    Yes,
    No,
}
"#;
    let output = format_source(input);
    println!("{}", output);
    assert!(output.contains("// FIXME: We should use this when appropriate."));
}

#[test]
fn preserve_comments_before_enum_variant() {
    let input = r#"enum PointerCast {
    /// Go from a fn-item type to a fn-pointer type.
    ReifyFnPointer,
    /// Go from a safe fn pointer to an unsafe fn pointer.
    UnsafeFnPointer,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    enum PointerCast {
        /// Go from a fn-item type to a fn-pointer type.
        ReifyFnPointer,
        /// Go from a safe fn pointer to an unsafe fn pointer.
        UnsafeFnPointer,
    }
    "#);
}

#[test]
fn preserve_comments_before_mod() {
    let input = r#"// src/formatter/node/common.rs
// Shared helpers used by node formatters.
pub mod comments;
pub mod fields;
"#;
    let output = format_source(input);
    assert!(output.contains("// src/formatter/node/common.rs"));
    assert!(output.contains("// Shared helpers used by node formatters."));
}

#[test]
fn no_spurious_blank_line_after_comment() {
    let input = r#"fn foo() {
    // Comment from rustc:
    // Even though coercion casts provide type hints, we check casts after fallback for
    // backwards compatibility.
    let x = 1;
}
"#;
    let output = format_source(input);
    // Should NOT have double newline after the comment block
    assert!(!output.contains("// backwards compatibility.\n\n"));
}

#[test]
fn preserve_allow_attribute_after_doc_comment() {
    let input = r#"enum PointerCast {
    /// Go from `*const [T; N]` to `*const T`
    #[allow(dead_code)]
    ArrayToPointer,
}
"#;
    let output = format_source(input);
    // The doc comment should come before the attribute
    assert!(output.contains("/// Go from `*const [T; N]` to `*const T`\n    #[allow(dead_code)]"));
}

#[test]
fn preserve_trailing_comment_whitespace_alignment_in_struct() {
    let input = r#"struct Foo {
    pub method_refs: bool,
    pub refs_adt: bool,   // for Struct, Enum, Union and Trait
    pub refs_trait: bool, // for Struct, Enum, Union and Trait
    pub enum_variant_refs: bool,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    struct Foo {
        pub method_refs: bool,
        pub refs_adt: bool,   // for Struct, Enum, Union and Trait
        pub refs_trait: bool, // for Struct, Enum, Union and Trait
        pub enum_variant_refs: bool,
    }
    "#);
}

#[test]
fn preserve_module_inner_doc_comments() {
    let input = r#"//! This is a module-level doc comment.
//! It describes what this module does.
//! And spans multiple lines.

fn foo() {}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    //! This is a module-level doc comment.
    //! It describes what this module does.
    //! And spans multiple lines.

    fn foo() {}
    "#);
}

#[test]
fn preserve_module_doc_comment_before_items() {
    let input = r#"//! Adapted from a rustc test.
//! The Rust parser handles this case.

use std::io;

fn main() {}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    //! Adapted from a rustc test.
    //! The Rust parser handles this case.

    use std::io;

    fn main() {}
    "#);
}

#[test]
fn preserve_doc_comment_on_function() {
    let input = r#"/// This function does something important.
/// It has multiple lines of documentation.
fn important_function() {}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    /// This function does something important.
    /// It has multiple lines of documentation.
    fn important_function() {}
    "#);
}
