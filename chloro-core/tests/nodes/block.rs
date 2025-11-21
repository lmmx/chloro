use chloro_core::format_source;

#[test]
fn test_empty_block() {
    let input = "fn foo() {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_block_with_statement() {
    let input = "fn foo() { let x = 1; }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_block_with_multiple_statements() {
    let input = "fn foo() { let x = 1; let y = 2; let z = x + y; }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_block_with_expression() {
    let input = "fn add() -> i32 { 1 + 1 }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_nested_blocks() {
    let input = "fn foo() { { let x = 1; } { let y = 2; } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_block_with_if() {
    let input = "fn check(x: i32) { if x > 0 { println!(\"positive\"); } }";
    insta::assert_snapshot!(format_source(input));
}
