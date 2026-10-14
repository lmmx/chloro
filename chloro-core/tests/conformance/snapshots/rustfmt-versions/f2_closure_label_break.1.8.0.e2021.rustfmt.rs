fn main() {
    bar(|| 'b: {
        break 'b 1;
    });
}
