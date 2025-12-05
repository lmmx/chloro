//! Tests to ensure chloro can format its own code correctly.
//! These are regression tests for bugs found when running `chloro -w` on chloro-core.

use insta::assert_snapshot;

use super::*;

#[test]
fn preserve_comments_with_their_statements_in_block() {
    // Comments should stay with the statement they precede, not all move to top
    let input = r#"fn foo() {
    {
        // Comment 1
        // Comment 2
        let x = 1;

        // Comment 3
        if x > 0 {
            return false;
        }

        // Comment 4
        true
    }
}
"#;
    let output = format_source(input);
    // Comments should stay with their statements, blank lines preserved
    assert_snapshot!(output, @r#"
    fn foo() {
        {
            // Comment 1
            // Comment 2
            let x = 1;

            // Comment 3
            if x > 0 {
                return false;
            }

            // Comment 4
            true
        }
    }
    "#);
}

#[test]
fn preserve_blank_lines_between_statements_in_block() {
    // From node.rs: blank lines between `let` and `if` statements should be preserved
    let input = r#"fn foo(prev: Option<i32>, curr: i32) -> bool {
    {
        let Some(prev) = prev else {
            return false;
        };

        // No blank line between consecutive uses
        if prev == 1 && curr == 1 {
            return false;
        }

        // No blank line between consecutive mod declarations
        if prev == 2 && curr == 2 {
            return false;
        }

        // Blank line between different top-level items
        true
    }
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_blank_line_after_variable_declaration() {
    // Blank line after `let mut last_kind` should be preserved
    let input = r#"fn foo() {
    let mut last_kind: Option<i32> = None;
    let mut prev_was_standalone_comment = false;

    for item in items {
        println!("{}", item);
    }
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_blank_line_after_match_arm_comment() {
    // Blank line after `}` and before `match` should be preserved
    let input = r#"fn foo() {
    for (i, comment) in comments.iter().enumerate() {
        println!("{}", comment);
    }

    match item {
        1 => println!("one"),
        _ => {}
    }
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_end_of_line_comment_on_enum_variant() {
    // End-of-line comments should stay on the same line
    let input = r#"pub enum ImportGroup {
    Internal(InternalKind), // self::, super::, crate::, - sorted first
    External,               // everything else (including std, core, alloc)
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_blank_line_between_comment_block_and_function() {
    // Blank line between a comment block and a function should be preserved
    let input = r#"// /// Check if a string contains any lowercase ASCII characters
// fn has_lowercase(s: &str) -> bool {
//     s.as_bytes().iter().any(|&b| b.is_ascii_lowercase())
// }

pub fn sort_key(s: &str) -> bool {
    true
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_comment_between_statements_not_moved_to_top() {
    // Comments should stay with the statement they precede, not move to top of block
    let input = r#"fn foo() {
    {
        let x = 1;

        // This comment is about the if statement
        if x > 0 {
            return true;
        }

        // This comment is about the match
        match x {
            1 => true,
            _ => false,
        }
    }
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

#[test]
fn preserve_blank_line_before_close_brace_comment() {
    // From function.rs: blank line before `buf.close_brace_ln` should be preserved
    let input = r#"fn format_something() {
    if condition {
        do_something();
    }

    buf.close_brace_ln(indent);
}
"#;
    let output = format_source(input);
    assert_snapshot!(output);
}

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
    assert_snapshot!(output, @r"
    use ra_ap_syntax::{
        AstNode, SyntaxNode,
        ast::{self, HasVisibility},
    };

    pub mod grouping;
    pub mod sort;

    use crate::formatter::config::MAX_WIDTH;
    use crate::formatter::write_indent;

    pub fn format_use();
    ");
}
