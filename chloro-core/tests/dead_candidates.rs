//! The last child of a chain (`chains.rs`, `format_last_child`): when its one-line rewrite
//! fits the budget and has at most one line, the vertical rewrite is not rendered in a
//! whole-file run under `rustfmt_version = "1.9.0"`, and is rendered as before in a macro
//! definition body (a snippet run) and under 1.8.0. The expected outputs are those of
//! `rustfmt 1.9.0` and `rustup run 1.88.0 rustfmt` (1.8.0) at edition 2024, which agree on
//! these inputs.

use chloro_core::formatter::config::Config;
use chloro_core::formatter::format_source_with_config;

fn check(source: &str, expected: &str) {
    for version in ["1.9.0", "1.8.0"] {
        let mut config = Config::default();
        config.set("rustfmt_version", version).unwrap();
        config.set("edition", "2024").unwrap();
        assert_eq!(
            format_source_with_config(source, &config),
            expected,
            "rustfmt_version={version}"
        );
    }
}

/// The one-line rewrite fits and is kept: the vertical candidate is dead.
#[test]
fn last_child_fits_on_one_line() {
    let source = "fn one_line() {\n    let value = receiver.first().second(argument).third(other_argument)?;\n}\n";
    check(source, source);
}

/// The chain is wider than `chain_width`, so the last child is not tried on the chain's
/// line, and the chain goes vertical.
#[test]
fn chain_wider_than_chain_width() {
    check(
        "fn one_line() {\n    let value = receiver.first_method().second_method(argument_one, argument_two);\n}\n",
        "fn one_line() {\n    let value = receiver\n        .first_method()\n        .second_method(argument_one, argument_two);\n}\n",
    );
}

/// The one-line rewrite's first line misses the budget: the vertical candidate is
/// rendered and compared.
#[test]
fn last_child_does_not_fit_on_one_line() {
    check(
        "fn too_wide() {\n    let value = receiver.first_method().second_method(argument_number_one, argument_number_two, argument_number_three);\n}\n",
        "fn too_wide() {\n    let value = receiver.first_method().second_method(\n        argument_number_one,\n        argument_number_two,\n        argument_number_three,\n    );\n}\n",
    );
}

/// The one-line rewrite fits but has three lines (fewer than five): the vertical candidate
/// is rendered and loses on its line count.
#[test]
fn last_child_fits_with_several_lines() {
    let source = "fn multi_line_fits() {\n    let value = receiver.first_method().second_method(|x| {\n        let y = x + 1;\n        y\n    });\n}\n";
    check(source, source);
}

/// Inside a macro definition body (a snippet run) the vertical candidate is still
/// rendered; the output is the same.
#[test]
fn chains_in_a_macro_definition_body() {
    check(
        "macro_rules! in_body {\n    ($e:expr) => {\n        let value = $e.first_method().second_method(argument_one, argument_two);\n        let other = receiver.first_method().second_method(argument_number_one, argument_number_two, argument_number_three);\n    };\n}\n",
        "macro_rules! in_body {\n    ($e:expr) => {\n        let value = $e.first_method().second_method(argument_one, argument_two);\n        let other = receiver.first_method().second_method(\n            argument_number_one,\n            argument_number_two,\n            argument_number_three,\n        );\n    };\n}\n",
    );
}
