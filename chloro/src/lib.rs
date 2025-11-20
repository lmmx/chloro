#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![forbid(unsafe_code)]

//! # chloro
//!
//! A minimal Rust code formatter.
//!
//! This crate provides a simple, fast formatter for Rust source code,
//! with both library and CLI interfaces.

// Re-export the core formatting functionality
pub use chloro_core::{chloro_debug, format_source};
