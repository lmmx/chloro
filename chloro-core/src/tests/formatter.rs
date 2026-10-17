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
fn is_unchanged_compares_as_rustfmt_does() {
    let auto = crate::Config::default();
    let mut unix = crate::Config::default();
    unix.set("newline_style", "Unix").unwrap();
    let lf = "fn a() {}\nfn b() {}\n";
    // Under `Auto`, the source as rustc reads it: `\r\n` and a byte order mark do not count.
    let crlf = "fn a() {}\r\nfn b() {}\r\n";
    let mixed = "fn a() {}\nfn b() {}\r\n";
    for source in [crlf, mixed, "\u{feff}fn a() {}\nfn b() {}\n", lf] {
        let formatted = format_source(source);
        assert_eq!(formatted, lf);
        assert!(crate::is_unchanged(source, &formatted, &auto), "{source:?}");
        let formatted = crate::format_source_with_config(source, &unix);
        assert_eq!(crate::is_unchanged(source, &formatted, &unix), source == lf);
    }
    // A difference other than line endings counts.
    let unformatted = "fn a(){}\r\nfn b() {}\r\n";
    assert!(!crate::is_unchanged(
        unformatted,
        &format_source(unformatted),
        &auto
    ));
    // An output equal to the source is unchanged under any style.
    let mut off = unix.clone();
    off.set("disable_all_formatting", "true").unwrap();
    assert!(crate::is_unchanged(
        crlf,
        &crate::format_source_with_config(crlf, &off),
        &off
    ));
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
    // Newline-terminated: with no newline in the input, `newline_style = Auto` takes the
    // platform's line ending (as rustfmt does), which on Windows is `\r\n`.
    let input = "fn foo() { let x = 1; }\n";
    let once = format_source(input);
    let twice = format_source(&once);
    assert_eq!(once, twice);
}
