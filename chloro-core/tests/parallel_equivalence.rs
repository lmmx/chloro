//! The parallel item formatter gives the sequential formatter's output.
//!
//! The CLI formats a file of 128 KiB or more with `format_source_with_config_parallel` when it
//! has more than one thread and with `format_source_with_config` otherwise, and its cache
//! records a fixed point under whichever path observed it. A hit is therefore right only while
//! the two paths agree byte for byte (finding 3 of
//! `docs/journal/2026-10-14-autoresearch/experiments/review-cache-parallel-cst.md`). This
//! test holds them to it on every eleventh conformance fixture, under the default options and
//! with the reordering of imports and modules off (the two paths treat a reorderable group as
//! one unit, and this exercises the groups' other shape), at two and four threads.

use std::path::Path;

use chloro_core::formatter::config::Config;
use chloro_core::{format_source_with_config, format_source_with_config_parallel};

#[test]
fn parallel_items_match_the_sequential_formatter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    let mut files: Vec<_> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    assert!(
        files.len() > 1000,
        "{} fixtures found under {}",
        files.len(),
        root.display()
    );

    let mut no_reorder = Config::default();
    no_reorder.set("reorder_imports", "false").unwrap();
    no_reorder.set("reorder_modules", "false").unwrap();
    let configs = [
        ("default", Config::default()),
        ("no reordering", no_reorder),
    ];

    let mut checked = 0;
    for path in files.iter().step_by(11) {
        let source = std::fs::read_to_string(path).unwrap();
        for (name, config) in &configs {
            let sequential = format_source_with_config(&source, config);
            for threads in [2, 4] {
                let parallel = format_source_with_config_parallel(&source, config, threads);
                assert!(
                    parallel == sequential,
                    "{} differs between {threads} threads and the sequential formatter ({name})",
                    path.strip_prefix(&root).unwrap().display()
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 100, "{checked} files checked");
}
