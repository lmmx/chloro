use super::*;

use insta::assert_snapshot;

#[test]
fn if_let_chain_multiline() {
    let input = r#"fn foo() {
    if let Some(x) = opt
        && let Some(y) = other
    {
        println!("{} {}", x, y);
    }
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        if let Some(x) = opt
            && let Some(y) = other
        {
            println!("{} {}", x, y);
        }
    }
    "#);
}

#[test]
fn if_let_chain_triple() {
    let input = r#"fn foo() {
    if let Some(x) = a
        && let Some(y) = b
        && let Some(z) = c
    {
        println!("{}", x);
    }
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        if let Some(x) = a
            && let Some(y) = b
            && let Some(z) = c
        {
            println!("{}", x);
        }
    }
    "#);
}

#[test]
fn if_let_simple_stays_one_line() {
    // Simple if-let without chain stays on one line
    let input = r#"fn foo() {
    if let Some(x) = opt {
        println!("{}", x);
    }
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        if let Some(x) = opt {
            println!("{}", x);
        }
    }
    "#);
}

#[test]
fn if_let_with_bool_condition() {
    // let + bool condition is also a chain
    let input = r#"fn foo() {
    if let Some(x) = opt
        && x > 0
    {
        println!("{}", x);
    }
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        if let Some(x) = opt
            && x > 0
        {
            println!("{}", x);
        }
    }
    "#);
}
