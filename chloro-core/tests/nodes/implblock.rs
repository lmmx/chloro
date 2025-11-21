use chloro_core::format_source;

#[test]
fn test_simple_impl() {
    let input = "impl Foo { fn new() -> Self { Self } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_impl_with_methods() {
    let input = "impl Calculator { fn add(&self, x: i32, y: i32) -> i32 { x + y } fn multiply(&self, x: i32, y: i32) -> i32 { x * y } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_impl_with_generics() {
    let input = "impl<T> Container<T> { fn new(value: T) -> Self { Self { value } } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_trait_impl() {
    let input = "impl Display for Person { fn fmt(&self, f: &mut Formatter) -> Result { write!(f, \"{}\", self.name) } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_impl_with_where_clause() {
    let input = "impl<T> Container<T> where T: Clone { fn duplicate(&self) -> T { self.value.clone() } }";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_unsafe_impl() {
    let input = "unsafe impl Send for MyType {}";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_empty_impl() {
    let input = "impl Foo {}";
    insta::assert_snapshot!(format_source(input));
}
