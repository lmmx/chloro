//! Formats every `.rs` file under SRC into the same relative path under DST.
//!
//! ```text
//! cargo run --release -p chloro-core --example fmt_dir -- SRC DST
//! ```
//!
//! Uses only `chloro_core::format_source`, so the file can be copied into a checkout of an
//! older commit (such as the proof of concept, `c4d74ee`) to produce that version's output
//! for `examples/spacing.rs`.

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let (src, dst) = (args.next().expect("SRC"), args.next().expect("DST"));
    for entry in walkdir::WalkDir::new(&src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let out = Path::new(&dst).join(path.strip_prefix(&src).expect("under SRC"));
        std::fs::create_dir_all(out.parent().expect("file has a parent")).expect("create DST");
        std::fs::write(out, chloro_core::format_source(&text)).expect("write output");
    }
}
