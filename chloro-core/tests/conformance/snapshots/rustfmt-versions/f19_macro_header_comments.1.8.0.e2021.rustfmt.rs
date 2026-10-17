macro_rules! foo {
    () => {};
}
macro_rules! bar {
    () => {};
}
macro_rules! baz {
    () => {};
}
macro_rules! r#qux {
    () => {};
}
pub macro m1() {}
pub(crate) macro m2() {}
macro m3($x:expr) {
    $x
}
pub(in crate::macro_mod) macro m4() {}
mod m {
    macro_rules! inner {
        () => {};
    }
}
#[macro_export]
macro_rules! attr {
    () => {};
}
