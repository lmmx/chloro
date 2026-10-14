fn main() {
    bar(|| 'b: { let x = 1; break 'b x; });
}
