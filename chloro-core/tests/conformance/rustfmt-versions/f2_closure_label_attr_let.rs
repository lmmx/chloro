fn main() {
    bar(|| 'b: { #[attr] 1 });
    let x = || 'b: { 1 };
    let y = move || 'b: { break 'b; };
}
