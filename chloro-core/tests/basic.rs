use rustfmt_minimal::format_source;

#[test]
fn test_function_formatting() {
    let input = "fn main(){let x=1;}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_struct_formatting() {
    let input = "pub struct Foo{x:i32,y:String}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_enum_formatting() {
    let input = "pub enum Result{Ok(i32),Err(String)}";
    insta::assert_snapshot!(format_source(input));
}
