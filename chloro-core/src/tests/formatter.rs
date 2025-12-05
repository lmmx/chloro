use crate::format_source;

mod attributes;
mod comments;
mod functions;
mod impl_blocks;
mod macros;
mod self_format;
mod struct_literals;
mod structs;
mod use_items;

#[test]
fn format_simple_function() {
    let input = "fn main(){println!(\"hello\");}";
    let output = format_source(input);
    assert!(output.contains("fn main()"));
}

#[test]
fn format_is_idempotent() {
    let input = "fn foo() { let x = 1; }";
    let once = format_source(input);
    let twice = format_source(&once);
    assert_eq!(once, twice);
}
