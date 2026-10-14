fn main() {
    foo!(|| 'a: { break 'a 1; });
    foo!(|| 'a: { 1 });
}
