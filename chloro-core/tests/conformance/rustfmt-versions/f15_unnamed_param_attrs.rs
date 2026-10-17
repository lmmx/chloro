extern "C" { fn a(x: i32, #[attr] ...) -> i32; }
extern "C" { fn b(x: i32, #[a] #[b] ...); }
extern "C" { fn c(x: i32, #[attr] /* c */ ...); }
unsafe extern "C" fn d(x: i32, #[attr] args: ...) {}
type E = fn(#[attr] u8, u16);
type F = unsafe extern "C" fn(#[attr] u8, #[attr] ...);
extern "C" { fn g(x: i32, #[cfg(feature = "a_long_feature_name")] #[attr_that_is_quite_long] ...) -> i32; }
