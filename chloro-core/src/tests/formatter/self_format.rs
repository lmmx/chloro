//! Tests to ensure chloro can format its own code correctly.
//! These are regression tests for bugs found when running `chloro -w` on chloro-core.

use super::*;
use insta::assert_snapshot;

#[test]
fn preserve_section_comments_in_match() {
    // Comments like "// === Section ===" should be preserved
    let input = r#"fn foo(x: i32) -> i32 {
    match x {
        // === Simple / Pass-through ===
        1 => 1,

        // === Wrapping expressions ===
        2 => 2,

        _ => 0,
    }
}
"#;
    let output = format_source(input);
    assert!(
        output.contains("// === Simple / Pass-through ==="),
        "Missing section comment. Output:\n{}",
        output
    );
    assert!(
        output.contains("// === Wrapping expressions ==="),
        "Missing section comment. Output:\n{}",
        output
    );
}

#[test]
fn preserve_inline_explanatory_comments_in_match() {
    let input = r#"fn foo(x: i32) {
    match x {
        1 => {
            // Preserve macro definitions as-is for now
            println!("one");
        }
        _ => {}
    }
}
"#;
    let output = format_source(input);
    assert!(
        output.contains("// Preserve macro definitions as-is for now"),
        "Missing inline comment. Output:\n{}",
        output
    );
}

#[test]
fn do_not_move_cfg_test_mod() {
    // #[cfg(test)] mod tests; should stay at the bottom, not be moved up
    let input = r#"pub mod formatter;

pub use formatter::format_source;

#[cfg(test)]
mod tests;
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    pub mod formatter;

    pub use formatter::format_source;

    #[cfg(test)]
    mod tests;
    ");
}

#[test]
fn do_not_expand_empty_match_arms() {
    let input = r#"fn foo(x: i32) {
    match x {
        1 => println!("one"),
        _ => {}
    }
}
"#;
    let output = format_source(input);
    // Should keep _ => {} on one line, not expand it
    assert!(
        output.contains("_ => {}"),
        "Empty match arm was expanded. Output:\n{}",
        output
    );
}

#[test]
fn preserve_blank_line_in_file_before_commented_code() {
    let input = r#"pub fn is_enabled() -> bool {
    true
}

// /// Automatically enable debug output for tests
// #[ctor::ctor]
// fn init_debug() {
//     init_from_env();
// }
"#;
    let output = format_source(input);
    // Should preserve blank line between function and commented code
    assert!(
        output.contains("}\n\n// /// Automatically"),
        "Missing blank line before comments. Output:\n{}",
        output
    );
}

#[test]
fn preserve_mod_declarations_without_extra_blank_lines() {
    // Mod declarations should stay grouped together without blank lines between them
    let input = r#"use ra_ap_syntax::{
    AstNode, SyntaxNode,
    ast::{self, HasVisibility},
};

pub mod grouping;
pub mod sort;

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

pub fn format_use();
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
use ra_ap_syntax::{
    ast::{self, HasVisibility},
    AstNode, SyntaxNode,
};

pub mod grouping;
pub mod sort;

use crate::formatter::config::MAX_WIDTH;
use crate::formatter::write_indent;

pub fn format_use();
"#);
}
