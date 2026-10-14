fn main() {
    bar(|| -> i32 {
        'b: {
            1
        }
    });
}
