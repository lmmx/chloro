use crate::helpers::load_fixture;
use chloro_core::format_source;
use rust_format::{Formatter, RustFmt, Config, Edition};

#[test]
fn compare_with_rustfmt_app_state() {
    let code = load_fixture("asterism/app_state");

    let chloro_output = format_source(&code);
    let rustfmt = RustFmt::from_config(
        Config::new_str().edition(Edition::Rust2024)
    );
    let rustfmt_output = rustfmt
        .format_str(&code)
        .expect("rustfmt failed");

    insta::assert_snapshot!(
        "chloro_vs_rustfmt_diff",
        format!(
            "=== CHLORO ===\n{}\n\n=== RUSTFMT ===\n{}",
            chloro_output, rustfmt_output
        )
    );
}
