//! Cases where chloro's output differed from rustfmt 1.9.0 (`rustfmt --edition 2024`).
//! Each expected output is rustfmt's output for the input.

use super::*;

/// Formats `input` and compares the result with rustfmt's output.
fn assert_rustfmt(input: &str, rustfmt: &str) {
    assert_eq!(format_source(input), rustfmt);
}

/// rustfmt's `FmtVisitor::from_context` builds the visitor of a nested block with
/// `is_macro_def = false`, so only statements directly in a macro definition body keep a
/// trailing `return`, `break` or `continue` without the semicolon.
#[test]
fn macro_def_nested_block_trailing_return_gets_semicolon() {
    let input = "macro_rules! m {
    () => {
        if a {
            return 1
        }
        loop {
            break
        }
        return 2
    };
}
";
    let rustfmt = "macro_rules! m {
    () => {
        if a {
            return 1;
        }
        loop {
            break;
        }
        return 2
    };
}
";
    assert_rustfmt(input, rustfmt);
}
