fn main() {
    bar(|| 'b: {
        #[attr]
        1
    });
    let x = || 1;
    let y = move || 'b: {
        break 'b;
    };
}
