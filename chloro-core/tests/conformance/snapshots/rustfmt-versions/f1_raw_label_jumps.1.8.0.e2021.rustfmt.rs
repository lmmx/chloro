fn main() {
    'r#match: for _ in 0..1 {
        if true {
            continue 'match;
        } else {
            break 'match;
        }
    }
    let v = 'r#a: { break 'a 5 };
}
