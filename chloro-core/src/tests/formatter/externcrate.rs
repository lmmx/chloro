use super::*;
use insta::assert_snapshot;

#[test]
fn preserve_extern_crate_with_rename() {
    let input = r#"extern crate ra_ap_rustc_type_ir as rustc_type_ir;

mod attrs;
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    extern crate ra_ap_rustc_type_ir as rustc_type_ir;

    mod attrs;
    "#);
}

#[test]
fn preserve_extern_crate_simple() {
    let input = r#"extern crate alloc;"#;
    let output = format_source(input);
    assert_snapshot!(output, @"extern crate alloc;");
}

#[test]
fn preserve_extern_crate_with_visibility() {
    let input = r#"pub extern crate core;"#;
    let output = format_source(input);
    assert_snapshot!(output, @"pub extern crate core;");
}

#[test]
fn preserve_extern_crate_with_doc_comment() {
    let input = r#"/// Re-export for convenience
extern crate alloc;"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    /// Re-export for convenience
    extern crate alloc;
    "#);
}

#[test]
fn preserve_extern_crate_with_attribute() {
    let input = r#"#[macro_use]
extern crate log;"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    #[macro_use]
    extern crate log;
    "#);
}
