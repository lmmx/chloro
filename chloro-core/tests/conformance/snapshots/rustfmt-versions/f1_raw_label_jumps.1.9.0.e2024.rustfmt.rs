fn main() {
    'r#match: for _ in 0..1 {
        if true {
            continue 'r#match;
        } else {
            break 'r#match;
        }
    }
    let v = 'r#a: { break 'r#a 5 };
}
