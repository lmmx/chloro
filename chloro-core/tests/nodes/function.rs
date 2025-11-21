use chloro_core::format_source;

#[test]
fn test_simple_function() {
    let input = "fn foo() {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_function_with_params() {
    let input = "fn add(x: i32, y: i32) -> i32 { x + y }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_public_function() {
    let input = "pub fn public_func() {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_async_function() {
    let input = "async fn async_work() {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_const_function() {
    let input = "const fn const_compute() -> i32 { 42 }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_unsafe_function() {
    let input = "unsafe fn unsafe_op() {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_generic_function() {
    let input = "fn generic<T>(x: T) -> T { x }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_function_with_where_clause() {
    let input = "fn constrained<T>(x: T) -> T where T: Clone { x.clone() }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_function_with_lifetime() {
    let input = "fn with_lifetime<'a>(x: &'a str) -> &'a str { x }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_function_declaration_no_body() {
    let input = "fn declared();";
    insta::assert_snapshot!(format_source(input));
}
