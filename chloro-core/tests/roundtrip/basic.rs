use crate::helpers::{assert_idempotent, load_fixture, run_roundtrip};
use insta::assert_snapshot;

#[test]
fn roundtrip_asterism_app_state() {
    let code = load_fixture("asterism/app_state");
    let result = run_roundtrip(&code);

    // Snapshot the formatted output
    assert_snapshot!("app_state_formatted", result.formatted_once);

    // Assert idempotency
    assert!(result.is_idempotent, "Formatting should be idempotent");
}

#[test]
fn roundtrip_asterism_config() {
    let code = load_fixture("asterism/config");
    assert_idempotent(&code);
}

#[test]
fn roundtrip_asterism_edit_plan() {
    let code = load_fixture("asterism/edit_plan");
    assert_idempotent(&code);
}
