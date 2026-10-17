macro_rules! /* c */ foo { () => {} }
macro_rules!   bar { () => {} }
macro_rules! // line
baz { () => {} }
macro_rules!r#qux { () => {} }
pub /* a */ macro m1() {}
pub(crate) macro // b
m2 { () => {} }
macro /* c */ m3($x:expr) { $x }
pub(in crate::macro_mod) macro m4() {}
mod m {
    macro_rules! /* deep */ inner { () => {} }
}
#[macro_export]
macro_rules! /*x*/ attr { () => {} }
