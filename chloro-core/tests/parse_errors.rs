//! The `try_format_source_*` functions: the syntax error for which the formatter returns a
//! source unchanged, reported where rustc reports it; every other source is formatted as the
//! functions without `try_` format it.

use chloro_core::formatter::config::Config;
use chloro_core::{
    CachedUnit, ParseError, UnitCache, UnitKey, format_source_with_config,
    try_format_source_with_config, try_format_source_with_config_parallel,
    try_format_source_with_unit_cache,
};

/// A unit cache that holds nothing.
struct NoUnits;

impl UnitCache for NoUnits {
    fn get(&self, _: &UnitKey<'_>) -> Option<CachedUnit> {
        None
    }

    fn insert(&self, _: &UnitKey<'_>, _: CachedUnit) {}
}

/// The error for `source` under the default options, the same from all three functions, for
/// a source that `format_source_with_config` returns unchanged.
fn error(source: &str) -> ParseError {
    let config = Config::default();
    assert_eq!(format_source_with_config(source, &config), source);
    let error = try_format_source_with_config(source, &config).unwrap_err();
    assert_eq!(
        try_format_source_with_config_parallel(source, &config, 4).unwrap_err(),
        error
    );
    assert_eq!(
        try_format_source_with_unit_cache(source, &config, 4, &NoUnits).unwrap_err(),
        error
    );
    error
}

/// `(line, column, width, message)` of `error`.
fn parts(error: &ParseError) -> (usize, usize, usize, &str) {
    (error.line(), error.column(), error.width(), error.message())
}

#[test]
fn a_syntax_error_is_reported_at_the_token_the_parser_could_not_accept() {
    let e = error("fn main() {\n    let x = ;\n}\n");
    assert_eq!(parts(&e), (2, 13, 1, "expected expression, found `;`"));
    assert_eq!(e.to_string(), "2:13: expected expression, found `;`");
    // rust-analyzer's `SEMICOLON` reads as the token, and a keyword found as rustc words it.
    let e = error("fn main() {\n    let x = 1 2;\n}\n");
    assert_eq!(parts(&e), (2, 15, 1, "expected `;`, found `2`"));
    let e = error("fn f() {}\n}\n");
    assert_eq!(parts(&e), (2, 1, 1, "unmatched `}`"));
    // The message stays on one line when the token found spans lines.
    let e = error("fn main() {\n    let x = 1 \"a\nb\";\n}\n");
    assert_eq!(parts(&e), (2, 15, 2, "expected `;`, found `\"a`"));
}

#[test]
fn a_token_missing_at_the_end_of_a_line_or_file_is_reported_after_the_token_before() {
    let e = error("fn main() {\n    let x = 1\n    let y = 2;\n}\n");
    assert_eq!(parts(&e), (2, 14, 1, "expected `;`, found keyword `let`"));
    let e = error("struct S {\n    a: u8\n    b: u8,\n}\n");
    assert_eq!(parts(&e), (2, 10, 1, "expected `,`, found `b`"));
    let e = error("fn f(\n");
    assert_eq!(parts(&e), (1, 6, 1, "expected `)`, found `<eof>`"));
}

#[test]
fn a_lexer_error_covers_its_token_to_the_end_of_the_line() {
    let e = error("fn main() {\n    let s = \"abc;\n}\n");
    assert_eq!((e.line(), e.column(), e.width()), (2, 13, 5));
    assert!(e.message().contains("terminate the string literal"), "{e}");
}

#[test]
fn syntax_rustc_rejects_is_reported_at_the_rejected_construct() {
    let e = error("fn f() { x <- y; }\n");
    assert_eq!(parts(&e), (1, 10, 6, "rustc's parser rejects this syntax"));
    // A doc comment that documents nothing, at the end of the file.
    let e = error("fn f() {}\n/// nothing\n");
    assert_eq!(parts(&e), (2, 1, 11, "rustc's parser rejects this syntax"));
}

#[test]
fn columns_count_characters_of_the_normalised_source() {
    // A tab and a two-byte character are one column each.
    let e = error("fn main() {\n\tlet é = ;\n}\n");
    assert_eq!((e.line(), e.column()), (2, 10));
    // `\r\n` reads as `\n`, and a byte order mark takes no column.
    let e = error("\u{feff}fn main() {\r\n    let x = ;\r\n}\r\n");
    assert_eq!((e.line(), e.column()), (2, 13));
    let bom = error("\u{feff}fn main( {}\n");
    assert_eq!(bom, error("fn main( {}\n"));
    assert_eq!((bom.line(), bom.column()), (1, 10));
}

#[test]
fn every_other_source_is_formatted_as_without_try() {
    let config = Config::default();
    let ok = |source: &str, config: &Config| {
        let expected = format_source_with_config(source, config);
        assert_eq!(
            try_format_source_with_config(source, config),
            Ok(expected.clone())
        );
        assert_eq!(
            try_format_source_with_config_parallel(source, config, 4),
            Ok(expected.clone())
        );
        assert_eq!(
            try_format_source_with_unit_cache(source, config, 4, &NoUnits),
            Ok(expected)
        );
    };
    ok("fn main(){let x=1;}\nfn g(){}\n", &config);
    ok("fn main() {\n    let x = 1;\n}\n", &config);
    // Syntax rust-analyzer's parser rejects and rustc's accepts (`rustc_syntax`).
    ok("fn f(x: Box<(Copy) + Send>) {}\n", &config);
    // Returned unchanged without an error: an inner `#![rustfmt::skip]`, and
    // `disable_all_formatting`, under which nothing is parsed.
    ok("#![rustfmt::skip]\nfn main(){let x=1;}\n", &config);
    let mut off = Config::default();
    off.set("disable_all_formatting", "true").unwrap();
    ok("fn main() {\n    let x = ;\n}\n", &off);
    // A formatting failure that is not a parse error: rustfmt 1.8.0 panics on this input
    // (F6) and writes nothing, and the source comes back unchanged.
    let mut v1_8 = Config::default();
    v1_8.set("rustfmt_version", "1.8.0").unwrap();
    let source = "fn f() {}\n// c\n\n\u{2029}\n\nfn g() {}\n";
    assert_eq!(format_source_with_config(source, &v1_8), source);
    ok(source, &v1_8);
}
