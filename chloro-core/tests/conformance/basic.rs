use crate::helpers::{compare_with_rustfmt, load_fixture};
use insta::assert_snapshot;

#[test]
fn compare_asterism_app_state() {
    let code = load_fixture("asterism/app_state");
    let result = compare_with_rustfmt(&code, "asterism/app_state");

    // Snapshot both outputs
    assert_snapshot!("app_state_chloro", result.chloro);
    assert_snapshot!("app_state_rustfmt", result.rustfmt);
}

#[test]
fn compare_asterism_app_state_min() {
    let code = load_fixture("asterism/app_state_min");
    let result = compare_with_rustfmt(&code, "asterism/app_state_min");

    // Snapshot both outputs
    assert_snapshot!("app_state_min_chloro", result.chloro);
    assert_snapshot!("app_state_min_rustfmt", result.rustfmt);
}

#[test]
fn compare_asterism_config() {
    let code = load_fixture("asterism/config");
    compare_with_rustfmt(&code, "asterism/config");
}

#[test]
fn compare_asterism_edit_plan() {
    let code = load_fixture("asterism/edit_plan");
    compare_with_rustfmt(&code, "asterism/edit_plan");
}
