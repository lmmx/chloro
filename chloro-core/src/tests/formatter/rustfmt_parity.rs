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

/// rustc's `parse_ty_tuple_or_parens` reads `(Path) + ..` and `(?Bound) + ..` as a bare trait
/// object; the parentheses of a path stay in its bound's span, those of a one-bound trait
/// object do not. rust-analyzer's parser rejects both, and `(?Sized)` on its own; chloro
/// parses them with `dyn` inserted (`rustc_syntax`).
#[test]
fn parenthesized_first_bound_of_bare_trait_object() {
    let input = "fn f<T: (Copy) + (?Sized) + (for<'a> Trait<'a>)>() {}

fn main() {
    let _: Box<(Copy) + (?Sized) + (for<'a> Trait<'a>)>;
    let _: Box<(?Sized) + (for<'a> Trait<'a>) + (Copy)>;
    let _: Box<(for<'a> Trait<'a>) + (Copy) + (?Sized)>;
    let _: Box<( ?Sized )>;
    let _: (Copy)+Send = x;
}
";
    let rustfmt = "fn f<T: (Copy) + (?Sized) + (for<'a> Trait<'a>)>() {}

fn main() {
    let _: Box<(Copy) + (?Sized) + (for<'a> Trait<'a>)>;
    let _: Box<?Sized + (for<'a> Trait<'a>) + (Copy)>;
    let _: Box<for<'a> Trait<'a> + (Copy) + (?Sized)>;
    let _: Box<(?Sized)>;
    let _: (Copy) + Send = x;
}
";
    assert_rustfmt(input, rustfmt);
}

/// rustc's `break_up_float` reads a float literal without `.` after a `.` as one field name
/// (`DestructuredFloat::Single`); rust-analyzer's tree builder reports it and wraps it in an
/// `ERROR` node. The file is formatted, with the field name as written.
#[test]
fn float_literal_field_name() {
    let input = "fn f() {
let a = s.1e0;
    let b = s . 1E1_ . 2;
}
";
    let rustfmt = "fn f() {
    let a = s.1e0;
    let b = s.1E1_.2;
}
";
    assert_rustfmt(input, rustfmt);
}

/// `#[rustfmt::skip::macros(name)]` and `#[rustfmt::skip::attributes(name)]` on an item (or
/// as inner attributes of a module) leave the macro calls and attributes with that
/// single-segment name inside the item as written (rustfmt's `SkipContext`);
/// `attributes(derive)` also keeps derives from being merged.
#[test]
fn skip_macros_and_attributes() {
    let input = r#"#[rustfmt::skip::macros(foo)]
fn f() {
    foo!(  a,b  );
    bar!(  a,b  );
    a::foo!(  a,b  );
}

#[rustfmt::skip::attributes(custom, derive)]
#[derive(Debug)]
#[derive(Clone)]
struct S {
    #[custom(  a,b  )]
    #[other(  a,b  )]
    x: u8,
}

#[derive(Debug)]
#[derive(Clone)]
struct T;

mod m {
    #![rustfmt::skip::macros(qux)]
    fn h() {
        qux!(  a,b  );
        foo!(  a,b  );
    }
}

#[rustfmt::skip::macros(mac)]
impl X {
    fn i() {
        mac!(  a,b  );
        let x = vec![  1,2  ];
    }
}

fn j() {
    mac!(  a,b  );
    let y = foo!(  a,b  ) + 1;
}
"#;
    let rustfmt = r#"#[rustfmt::skip::macros(foo)]
fn f() {
    foo!(  a,b  );
    bar!(a, b);
    a::foo!(a, b);
}

#[rustfmt::skip::attributes(custom, derive)]
#[derive(Debug)]
#[derive(Clone)]
struct S {
    #[custom(  a,b  )]
    #[other(a, b)]
    x: u8,
}

#[derive(Debug, Clone)]
struct T;

mod m {
    #![rustfmt::skip::macros(qux)]
    fn h() {
        qux!(  a,b  );
        foo!(a, b);
    }
}

#[rustfmt::skip::macros(mac)]
impl X {
    fn i() {
        mac!(  a,b  );
        let x = vec![1, 2];
    }
}

fn j() {
    mac!(a, b);
    let y = foo!(a, b) + 1;
}
"#;
    assert_rustfmt(input, rustfmt);
}

/// A const block item, `const { .. }` in a file or module, is rustc's `ItemKind::ConstBlock`;
/// rust-analyzer's parser rejects it (`rustc_syntax` builds a `CONST` item). rustfmt drops
/// a visibility, and formats the block with `get_context`, so a macro call it cannot format
/// inside a `macro_rules!` body does not leave the whole body as written (facet-core 0.30.0
/// `src/macros.rs`; the misplaced `if` line is rustfmt's output).
#[test]
fn const_block_items() {
    let input = r#"#[a]
pub const {   a( ) ; }

mod m {
    const { 1 }
}

macro_rules! value_vtable {
    ($type_name:ty) => {
        const {
            builder::<$type_name>()
                .display(|| {
                    if $crate::spez::impls!($type_name: core::fmt::Display) {
                        Some(|data, f| { f(data) })
                    } else {
                        None
                    }
                })
                .build()
        }
    };
}
"#;
    let rustfmt = r#"#[a]
const {
    a();
}

mod m {
    const { 1 }
}

macro_rules! value_vtable {
    ($type_name:ty) => {
        const {
            builder::<$type_name>()
                .display(|| {
            if $crate::spez::impls!($type_name: core::fmt::Display) {
                        Some(|data, f| f(data))
                    } else {
                        None
                    }
                })
                .build()
        }
    };
}
"#;
    assert_rustfmt(input, rustfmt);
}

/// rustc's parser accepts an `impl Trait` type as the self type of an impl (AST validation
/// rejects it later); rust-analyzer's parser ends the impl at the second `impl`.
#[test]
fn impl_trait_self_type() {
    let input = "impl   impl   NotType   {}
impl Trait2 for impl A + B {  fn f( ) { } }
";
    let rustfmt = "impl impl NotType {}
impl Trait2 for impl A + B {
    fn f() {}
}
";
    assert_rustfmt(input, rustfmt);
}
