//! Print the syntax tree chloro formats from, for a file (or stdin):
//! `cargo run --example dump_tree -- FILE`. Nodes are `KIND@start..end`, tokens also show
//! their text, as rowan's debug output does. A file that does not lex or parse prints
//! `error: no tree`.
use std::io::Read;

use chloro_core::cst::{NodeOrToken, Tree};
use ra_ap_parser::Edition;

fn main() {
    let mut src = String::new();
    match std::env::args().nth(1) {
        Some(path) => src = std::fs::read_to_string(path).expect("readable file"),
        None => {
            std::io::stdin().read_to_string(&mut src).expect("stdin");
        }
    }
    let Some(tree) = Tree::parse(&src, Edition::CURRENT) else {
        println!("error: no tree");
        return;
    };
    let mut depth = Vec::new();
    for (i, elem) in tree.root().descendants_with_tokens().enumerate() {
        let i = i as u32;
        while depth.last().is_some_and(|&end| i >= end) {
            depth.pop();
        }
        let indent = "  ".repeat(depth.len());
        let r = elem.text_range();
        match elem {
            NodeOrToken::Node(n) => {
                println!("{indent}{:?}@{}..{}", n.kind(), r.start(), r.end());
                depth.push(i + n.subtree_len());
            }
            NodeOrToken::Token(t) => {
                println!(
                    "{indent}{:?}@{}..{} {:?}",
                    t.kind(),
                    r.start(),
                    r.end(),
                    t.text()
                );
            }
        }
    }
}
