pub mod comparison;
pub mod fixture;
pub mod roundtrip;
pub mod rustfmt;

pub use comparison::ComparisonResult;
pub use fixture::load_fixture;

use chloro_core::format_source;

#[ctor::ctor]
fn init_debug() {
    chloro_core::debug::set_debug(true);
}

pub fn compare_with_rustfmt(code: &str, name: &str) -> ComparisonResult {
    eprintln!();
    eprintln!("============================================================");
    eprintln!("COMPARING: {}", name);
    eprintln!("============================================================");

    let chloro = format_source(code);

    // Format with rustfmt
    let rustfmt = rustfmt::format_with_rustfmt(code).unwrap_or_else(|e| {
        eprintln!("Warning: rustfmt failed: {}", e);
        code.to_string()
    });

    let result = ComparisonResult {
        original: code.to_string(),
        chloro,
        rustfmt,
    };

    result.write_comparison_files(name);
    result
}
