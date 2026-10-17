extern "C" {
    fn a(x: i32, ...) -> i32;
}
extern "C" {
    fn b(x: i32, ...);
}
extern "C" {
    fn c(x: i32, ...);
}
unsafe extern "C" fn d(x: i32, #[attr] args: ...) {}
type E = fn(u8, u16);
type F = unsafe extern "C" fn(u8, ...);
extern "C" {
    fn g(x: i32, ...) -> i32;
}
