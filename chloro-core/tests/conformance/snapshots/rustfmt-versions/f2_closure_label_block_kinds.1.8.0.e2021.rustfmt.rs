fn main() {
    bar(|| unsafe { 1 });
    bar(|| if x { 1 } else { 2 });
    bar(|| match x {
        _ => 1,
    });
}
