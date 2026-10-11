#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![forbid(unsafe_code)]

//! # chloro
//!
//! A Rust code formatter reproducing `rustfmt --edition 2024`, with library and CLI
//! interfaces.
//!
//! ```
//! let formatted = chloro::format_source("fn main(){let x=1;}");
//! assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");
//!
//! let mut config = chloro::Config::default();
//! config.set("tab_spaces", "2").unwrap();
//! let formatted = chloro::format_source_with_config("fn main(){let x=1;}", &config);
//! assert_eq!(formatted, "fn main() {\n  let x = 1;\n}\n");
//! ```

// Re-export the core formatting functionality
pub use chloro_core::{Config, chloro_debug, format_source, format_source_with_config};
