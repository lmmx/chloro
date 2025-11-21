use chloro_core::format_source;

#[test]
fn test_simple_enum() {
    let input = "enum Color { Red, Green, Blue }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_enum_with_data() {
    let input = "enum Option<T> { Some(T), None }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_enum_with_struct_variants() {
    let input = "enum Message { Move { x: i32, y: i32 }, Write(String), Quit }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_public_enum() {
    let input = "pub enum Result<T, E> { Ok(T), Err(E) }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_enum_with_discriminants() {
    let input = "enum Status { Active = 1, Inactive = 0 }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_enum_with_generics() {
    let input = "enum Tree<T> { Leaf(T), Branch(Box<Tree<T>>, Box<Tree<T>>) }";
    insta::assert_snapshot!(format_source(input));
}
