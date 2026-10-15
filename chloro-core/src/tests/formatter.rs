use crate::format_source;

mod attributes;
mod comments;
mod controlflow;
mod externcrate;
mod functions;
mod impl_blocks;
mod macros;
mod method_chains;
mod parallel;
mod rustfmt_parity;
mod self_format;
mod struct_literals;
mod structs;
mod use_items;

#[test]
fn format_simple_function() {
    let input = "fn main(){println!(\"hello\");}";
    let output = format_source(input);
    assert!(output.contains("fn main()"));
}

#[test]
fn parse_only_parses_as_formatting_does() {
    let config = crate::Config::default();
    let src = "fn main() {\n    let x = 1;\n}\n";
    let tree = crate::cst::Tree::parse(src, ra_ap_parser::Edition::Edition2024).map(|t| t.len());
    assert!(tree.is_some_and(|n| n > 1));
    assert_eq!(crate::parse_only(src, &config), tree);
    // Normalised as formatting normalises it: no byte order mark, `\r\n` read as `\n`.
    let crlf = "\u{feff}fn main() {\r\n    let x = 1;\r\n}\r\n";
    assert_eq!(crate::parse_only(crlf, &config), tree);
    // `None` where formatting returns the source without formatting it.
    let broken = "fn main( {";
    assert_eq!(crate::parse_only(broken, &config), None);
    assert_eq!(format_source(broken), broken);
    let mut off = crate::Config::default();
    off.set("disable_all_formatting", "true").unwrap();
    assert_eq!(crate::parse_only(src, &off), None);
    // A skipped file is parsed: the attribute is read from the tree.
    assert!(crate::parse_only("#![rustfmt::skip]\nfn  f() {}\n", &config).is_some());
}

#[test]
fn format_is_idempotent() {
    let input = "fn foo() { let x = 1; }";
    let once = format_source(input);
    let twice = format_source(&once);
    assert_eq!(once, twice);
}
