//! `format_source_with_config_parallel` against `format_source_with_config`.

use crate::{Config, format_source_with_config, format_source_with_config_parallel};

/// A file with every kind of text the stitching visitor handles between units: a `#!`
/// line, inner attributes, blank-line groups of `use`, comments between items, a
/// `#[rustfmt::skip]` item, a skipped `mod x;`, a `macro_rules!`, a `use` that formats to
/// nothing, doc comments, and a trailing comment.
const MIXED: &str = r#"#!/usr/bin/env cargo
#![allow(dead_code)]
//! Crate docs.

use std::collections::HashMap;
use std::fmt;
use b::{};

use   a::c;
use a::b;

// A comment between items.
extern crate alloc;

   /* block comment before a struct */
struct   S { a : u32, b: HashMap<u32,u32> }

#[rustfmt::skip]
fn   skipped( a:u32 )  ->  u32 { a   + 1 }

#[rustfmt::skip]
mod   skipped_mod;

mod z;
mod a;

macro_rules! m {
    ($x:expr) => { $x+1 };
}

/// Documented.
#[derive(Debug)]
pub fn documented<T:fmt::Debug>(t:T) { let x = m!(1);   println!("{:?}", t); }

enum E { A, B(u32), C { x: u8 } }


impl S { fn new() -> Self { S { a: 0, b: HashMap::new() } } }
struct A; struct B;
const C: u32 = 1; // trailing comment on the same line

pub mod inline { use super::S; pub fn f() -> S { S::new() } }

m!(2); // trailing comment at the end
"#;

fn check(source: &str, config: &Config) {
    let expected = format_source_with_config(source, config);
    for threads in [1, 4] {
        let got = format_source_with_config_parallel(source, config, threads);
        assert_eq!(got, expected, "{threads} threads");
    }
}

#[test]
fn mixed_file_matches_the_sequential_output() {
    check(MIXED, &Config::default());
    // Every `use` goes through `format_import`, including the removed one.
    let mut config = Config::default();
    config.set("reorder_imports", "false").unwrap();
    check(MIXED, &config);
}

#[test]
fn files_with_fewer_than_two_units_match() {
    for source in [
        "",
        "\n\n",
        "fn f() {}\n",
        "// only a comment\n",
        "#![rustfmt::skip]\nfn  f(){}\n",
    ] {
        check(source, &Config::default());
    }
    assert_eq!(
        format_source_with_config_parallel("fn  f(){}\n", &Config::default(), 4),
        "fn f() {}\n"
    );
}

#[test]
fn unparsable_source_is_returned_unchanged() {
    let source = "fn f( {\n";
    assert_eq!(
        format_source_with_config_parallel(source, &Config::default(), 4),
        source
    );
}

#[test]
fn skipped_items_and_removed_imports_at_the_edges() {
    // A skipped `mod x;` first and last; a removed import first and last.
    let sources = [
        "#[rustfmt::skip]\nmod   a;\nfn f(){}\n#[rustfmt::skip]\nmod   b;\n",
        "use a::{};\nfn f(){}\nuse b::{};\n",
        "use a::{};\n\nuse b::{};\nfn  f(){}\n",
        "fn f(){}\n\n\n\n#[rustfmt::skip]\nfn g(){}\n\n\n\nfn h(){}\n",
        "struct A; /* c */ struct B;\n/** doc */ struct C;\n",
    ];
    let mut no_reorder = Config::default();
    no_reorder.set("reorder_imports", "false").unwrap();
    for source in sources {
        check(source, &Config::default());
        check(source, &no_reorder);
    }
}

#[test]
fn crlf_and_newline_style_match() {
    let source = "\u{feff}fn main() {\r\n    let x=\"a\r\nb\";\r\n}\r\nfn  g(){}\r\n";
    check(source, &Config::default());
    let mut config = Config::default();
    config.set("newline_style", "Windows").unwrap();
    check(source, &config);
}

#[test]
fn fixture_sample_matches_with_four_threads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/conformance/fixtures/rust-analyzer");
    let mut files: Vec<_> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    let config = Config::default();
    let mut checked = 0;
    for path in files.iter().step_by(20) {
        let source = std::fs::read_to_string(path).unwrap();
        let expected = format_source_with_config(&source, &config);
        for threads in [2, 4] {
            let got = format_source_with_config_parallel(&source, &config, threads);
            assert_eq!(got, expected, "{threads} threads: {}", path.display());
        }
        checked += 1;
    }
    assert!(checked > 50, "{checked} files checked");
}
