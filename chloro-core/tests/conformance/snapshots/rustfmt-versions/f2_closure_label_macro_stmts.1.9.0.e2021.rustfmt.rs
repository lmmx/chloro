fn main() {
    m!(x, || 'a: {
        let y = 1;
        y
    });
    m!(|| 'a: {
        // comment
        1
    });
}
