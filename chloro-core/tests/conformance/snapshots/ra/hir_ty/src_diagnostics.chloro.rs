//! Type inference-based diagnostics.

mod decl_check;
mod expr;
mod match_check;
mod unsafe_check;

pub use crate::diagnostics::{
    decl_check::{CaseType, expr::{
        BodyValidationDiagnostic, incorrect_case},
    missing_unsafe, record_literal_missing_fields, record_pattern_missing_fields,
    unsafe_check::{
        InsideUnsafeBlock, unsafe_operations, unsafe_operations_for_body,
    IncorrectCase, UnsafetyReason, }, },
};
