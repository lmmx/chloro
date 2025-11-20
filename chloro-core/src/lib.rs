/// chloro-core: minimal Rust code formatter
pub mod debug;

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
