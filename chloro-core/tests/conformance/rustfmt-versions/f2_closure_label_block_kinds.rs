fn main() {
    bar(|| 'b: { unsafe { 1 } });
    bar(|| 'b: { if x { 1 } else { 2 } });
    bar(|| 'b: { match x { _ => 1 } });
}
