use super::*;
use insta::assert_snapshot;

#[test]
fn struct_literal_receiver_in_call_arg() {
    let input = r#"fn foo() {
    acc.push(
        UnresolvedModule {
            decl: InFile::new(file_id, decl),
            candidates: candidates.clone(),
        }
        .into(),
    )
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    fn foo() {
        acc.push(
            UnresolvedModule {
                decl: InFile::new(file_id, decl),
                candidates: candidates.clone(),
            }
            .into(),
        )
    }
    ");
}

#[test]
fn method_call_after_multiline_struct() {
    let input = r#"fn foo() {
    Foo {
        a: 1,
        b: 2,
    }
    .bar()
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    fn foo() {
        Foo { a: 1, b: 2 }.bar()
    }
    ");
}

#[test]
fn method_call_after_inline_struct() {
    let input = r#"fn foo() { Foo { a: 1 }.bar() }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        Foo { a: 1 }.bar()
    }
    "#);
}

#[test]
fn into_after_multiline_struct_in_arg() {
    let input = r#"fn foo() {
    acc.push(
        UnresolvedModule {
            decl: InFile::new(file_id, decl),
            candidates: candidates.clone(),
        }
        .into(),
    )
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        acc.push(
            UnresolvedModule {
                decl: InFile::new(file_id, decl),
                candidates: candidates.clone(),
            }
            .into(),
        )
    }
    "#);
}
