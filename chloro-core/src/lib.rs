//! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
pub mod debug;
pub mod formatter;

pub use formatter::config::Config;
pub use formatter::{format_source, format_source_with_config};

/// Macro for debug output in chloro.
///
/// Prints to stderr only if debug output is enabled via the atomic flag (tests do this using ctor)
/// or the `CHLORO_DEBUG` environment variable at startup.
#[macro_export]
macro_rules! chloro_debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_enabled() {
            eprintln!("[CHLORO DEBUG] {}", format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests;
