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

/// `[value; count]` is rustc's `ExprKind::Repeat`, not `ExprKind::Array`: rustfmt's
/// `is_block_expr` (chain roots), `can_flatten_block_around_this` (match arm bodies) and
/// `can_be_overflowed_expr` (last call argument) list only `Array`.
#[test]
fn repeat_array_is_not_block_like() {
    let input = "fn main() {
    let a = [aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb].into();
    let x = foo.bar([0u8; 1024]).baz([aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb]);
    match x {
        A => { [aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb] }
    }
}
";
    let rustfmt = "fn main() {
    let a = [aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb]
        .into();
    let x = foo.bar([0u8; 1024]).baz(
        [aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb],
    );
    match x {
        A => {
            [aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;
                bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb]
        }
    }
}
";
    assert_rustfmt(input, rustfmt);
}

/// rustc's lexer makes a doc comment a token. In a macro call's arguments it can begin only
/// an item (`Parser::nonterminal_may_begin_with`), and after an argument it is neither a
/// separator nor the end, so rustfmt leaves such a call as written. Before an item argument,
/// the doc comment is the item's attribute.
#[test]
fn doc_comment_tokens_in_macro_args() {
    let input = "m!(
    /// doc
    A,   B);
m!(a,   /// doc
);
m!(a   /// doc
, b);
m!(struct   A; /// doc
 struct   B;);
";
    let rustfmt = "m!(
    /// doc
    A,   B);
m!(a,   /// doc
);
m!(a   /// doc
, b);
m!(
    struct A;
    /// doc
    struct B;
);
";
    assert_rustfmt(input, rustfmt);
}
