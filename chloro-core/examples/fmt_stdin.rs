//! Formats Rust source read from stdin and writes the result to stdout, like
//! `rustfmt --edition 2024`. Arguments are rustfmt options as `key=value`. For side-by-side
//! checks:
//!
//! ```text
//! cargo run -q --release -p chloro-core --example fmt_stdin -- max_width=80 < file.rs \
//!     | diff <(rustfmt --edition 2024 --config max_width=80 < file.rs) -
//! ```

use std::io::{Read, Write};

fn main() {
    let mut config = chloro_core::formatter::config::Config::default();
    for option in std::env::args().skip(1) {
        let (key, value) = option.split_once('=').expect("options are key=value");
        config.set(key, value).expect("supported option");
    }
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .expect("stdin is UTF-8");
    std::io::stdout()
        .write_all(chloro_core::formatter::format_source_with_config(&source, &config).as_bytes())
        .expect("stdout is writable");
}
