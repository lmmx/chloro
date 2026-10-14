//! `Config::rustfmt_version`: one test per difference class between rustfmt 1.8.0 and
//! 1.9.0 (`docs/journal/2026-10-14-autoresearch/rustfmt-versions.md`), with the class's
//! minimal reproducer and the output of each release as `rustup run 1.88.0 rustfmt` and
//! `rustfmt 1.9.0` print it. The fixture suite under `tests/conformance/rustfmt-versions`
//! holds the longer cases.

use chloro_core::formatter::config::{Config, RustfmtVersion};
use chloro_core::formatter::format_source_with_config;

fn format(source: &str, version: &str, edition: &str) -> String {
    let mut config = Config::default();
    config.set("rustfmt_version", version).unwrap();
    config.set("edition", edition).unwrap();
    format_source_with_config(source, &config)
}

/// Asserts the 1.9.0 and 1.8.0 outputs of `source` at edition 2024.
fn check(source: &str, v1_9_0: &str, v1_8_0: &str) {
    check_at("2024", source, v1_9_0, v1_8_0);
}

fn check_at(edition: &str, source: &str, v1_9_0: &str, v1_8_0: &str) {
    assert_eq!(
        format(source, "1.9.0", edition),
        v1_9_0,
        "rustfmt_version=1.9.0"
    );
    assert_eq!(
        format(source, "1.8.0", edition),
        v1_8_0,
        "rustfmt_version=1.8.0"
    );
}

#[test]
fn config_accepts_1_8_0_and_1_9_0_and_rejects_1_7_0() {
    let mut config = Config::default();
    assert_eq!(config.rustfmt_version, RustfmtVersion::V1_9_0);
    config.set("rustfmt_version", "1.8.0").unwrap();
    assert_eq!(config.rustfmt_version, RustfmtVersion::V1_8_0);
    config.set("rustfmt_version", "1.9.0").unwrap();
    assert_eq!(config.rustfmt_version, RustfmtVersion::V1_9_0);
    let err = config.set("rustfmt_version", "1.7.0").unwrap_err();
    assert!(err.contains("rustfmt 1.7.0 is not supported"), "{err}");
    assert!(err.contains("Style Guide 2024"), "{err}");
    assert!(err.contains("supported: 1.8.0, 1.9.0"), "{err}");
    assert_eq!(config.rustfmt_version, RustfmtVersion::V1_9_0);
}

#[test]
fn f1_raw_labels_in_break_and_continue() {
    let source = "fn main() {\n    'r#if: {\n        break 'r#if;\n    }\n    'r#loop: loop {\n        continue 'r#loop;\n    }\n}\n";
    check(
        source,
        source,
        "fn main() {\n    'r#if: {\n        break 'if;\n    }\n    'r#loop: loop {\n        continue 'loop;\n    }\n}\n",
    );
}

#[test]
fn f2_labelled_block_as_closure_body() {
    check(
        "fn main() {\n    bar(|| 'b: { 1 });\n}\n",
        "fn main() {\n    bar(|| 'b: { 1 });\n}\n",
        "fn main() {\n    bar(|| 1);\n}\n",
    );
    // A body that does not fit without a block: rustfmt 1.8.0 starts the block one byte
    // into the label and copies the rest of the label as code.
    check(
        "fn main() {\n    bar(|| 'b: { some_function_with_a_long_name(argument_one, argument_two, argument_three, argument_four, argument_five) });\n}\n",
        "fn main() {\n    bar(|| 'b: {\n        some_function_with_a_long_name(\n            argument_one,\n            argument_two,\n            argument_three,\n            argument_four,\n            argument_five,\n        )\n    });\n}\n",
        "fn main() {\n    bar(|| {        b: {        some_function_with_a_long_name(\n            argument_one,\n            argument_two,\n            argument_three,\n            argument_four,\n            argument_five,\n        )\n    });\n}\n",
    );
    // Inside a macro call the block is never unwrapped, so every labelled body takes
    // that path.
    check(
        "fn main() {\n    foo!(|| 'a: { break 'a 1; });\n    foo!(|| 'a: { 1 });\n}\n",
        "fn main() {\n    foo!(|| 'a: {\n        break 'a 1;\n    });\n    foo!(|| 'a: { 1 });\n}\n",
        "fn main() {\n    foo!(|| {        a: {        break 'a 1;\n    });\n    foo!(|| { 1 });\n}\n",
    );
}

#[test]
fn f3_use_self_self() {
    check(
        "use self::self;\nuse a::{self::self};\nuse self::self as b;\n",
        "use self::self;\nuse self::self as b;\nuse a::self::self;\n",
        "use self;\nuse self::self as b;\nuse a::self;\n",
    );
}

#[test]
fn f4_super_let() {
    check(
        "fn main() {\n    super let x = 1;\n    super let y: i32 = 2;\n}\n",
        "fn main() {\n    super let x = 1;\n    super let y: i32 = 2;\n}\n",
        "fn main() {\n    let x = 1;\n    let y: i32 = 2;\n}\n",
    );
}

#[test]
fn f5_underscore_import_order_before_2024() {
    let source = "use b::{Ab, _x};\nuse a::{ab, _x, Ab};\nuse libc::{NCCS, _POSIX_VDISABLE};\nuse crate::Parser;\nuse crate::_tutorial::chapter_7;\n";
    check_at(
        "2021",
        source,
        "use crate::_tutorial::chapter_7;\nuse crate::Parser;\nuse a::{_x, ab, Ab};\nuse b::{_x, Ab};\nuse libc::{_POSIX_VDISABLE, NCCS};\n",
        "use crate::Parser;\nuse crate::_tutorial::chapter_7;\nuse a::{_x, ab, Ab};\nuse b::{Ab, _x};\nuse libc::{NCCS, _POSIX_VDISABLE};\n",
    );
    // Under style edition 2024 the releases agree (version sort).
    let sorted = "use crate::_tutorial::chapter_7;\nuse crate::Parser;\nuse a::{_x, Ab, ab};\nuse b::{_x, Ab};\nuse libc::{_POSIX_VDISABLE, NCCS};\n";
    check(source, sorted, sorted);
}

#[test]
fn f6_unicode_whitespace_in_blank_lines() {
    // rustfmt 1.8.0 panics (`byte index 23 is not a char boundary`) and writes nothing.
    let source = "// compile-flags: x\n\n\n\u{2029}\n";
    check(source, "// compile-flags: x\n", source);
    // A comment after the gap resets the offset before any slice: no panic.
    let source = "// a\n\n\n\u{2029}\n// b\nfn f() {}\n";
    check(
        source,
        "// a\n\n// b\nfn f() {}\n",
        "// a\n\n// b\nfn f() {}\n",
    );
    // Code after the gap: the panic.
    let source = "fn f() {}\n// c\n\n\u{2029}\n\nfn g() {}\n";
    check(source, "fn f() {}\n// c\n\nfn g() {}\n", source);
}

#[test]
fn p1_let_chains_before_edition_2024() {
    let source = "fn f() {\n    if a && let Some(x) = b {\n        x\n    }\n}\n";
    check_at("2021", source, source, source);
    check_at(
        "2021",
        "fn f() { if a && let Some(x) = b { x } }\n",
        "fn f() { if a && let Some(x) = b { x } }\n",
        source,
    );
    check_at(
        "2024",
        "fn f() { if a && let Some(x) = b { x } }\n",
        source,
        source,
    );
}

#[test]
fn p2_const_trait_bounds() {
    let source = "fn f<T: [const] Trait>() {}\nfn g() where T: [const] Trait {}\n";
    check(
        source,
        "fn f<T: [const] Trait>() {}\nfn g()\nwhere\n    T: [const] Trait,\n{\n}\n",
        source,
    );
}

#[test]
fn p8_negative_inherent_impl() {
    let source = "impl !Send for Foo {}\nimpl   !Foo {}\n";
    check(source, source, "impl !Send for Foo {}\nimpl !Foo {}\n");
}
