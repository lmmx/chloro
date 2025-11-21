use chloro_core::format_source;

#[test]
fn test_unit_struct() {
    let input = "struct Unit;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_tuple_struct() {
    let input = "struct Point(i32, i32);";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_tuple_struct_with_visibility() {
    let input = "pub struct Point(pub i32, pub i32);";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_named_fields_struct() {
    let input = "struct Person { name: String, age: u32 }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_public_struct_with_fields() {
    let input = "pub struct Config { pub port: u16, host: String }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_generic_struct() {
    let input = "struct Container<T> { value: T }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_struct_with_lifetime() {
    let input = "struct Borrowed<'a> { data: &'a str }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_struct_with_where_clause() {
    let input = "struct Constrained<T> where T: Clone { value: T }";
    insta::assert_snapshot!(format_source(input));
}
