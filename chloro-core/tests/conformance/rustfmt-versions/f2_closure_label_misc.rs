fn main() {
    let f = || 'b: {
        let x = compute();
        if x { break 'b 1 }
        2
    };
    foo(|| 'b: { 1 }, 2);
    foo(1, |x| 'outer: { x });
    foo(|| 'b: { #[cfg(a)] 1 });
}
