//! Print rust-analyzer's syntax tree for a file (or stdin): `cargo run --example dump_tree -- FILE`.
use ra_ap_syntax::{Edition, SourceFile};
use std::io::Read;

fn main() {
    let mut src = String::new();
    match std::env::args().nth(1) {
        Some(path) => src = std::fs::read_to_string(path).expect("readable file"),
        None => {
            std::io::stdin().read_to_string(&mut src).expect("stdin");
        }
    }
    let parse = SourceFile::parse(&src, Edition::CURRENT);
    print!("{:#?}", parse.syntax_node());
    for e in parse.errors() {
        println!("error: {e:?}");
    }
}
