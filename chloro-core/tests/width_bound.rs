//! The one-line width bound (`formatter/width.rs`) against the rendered text, over whole
//! corpora.
//!
//! Debug builds check, on every expression rewrite, that a one-line result is at least as
//! wide as the node's bound and, for a node the fit predicate accepts, at most one column
//! wider than its shape; and every attempt that the bound lets the formatter skip is rendered there and
//! checked to fail. This test formats every conformance fixture under the default options
//! and under the options that drop or rewrite tokens (`use_field_init_shorthand`,
//! `use_try_shorthand`, rustfmt 1.8.0), and the nested-closure
//! cliff shape of `benchmarks/cliffs/gen.py` at depths 2-16, so any of those checks that
//! fails fails the test. `CHLORO_WIDTH_BOUND_ROOT=<dir>` adds every `.rs` file under a
//! directory (the registry corpus) under the default options.

use std::path::{Path, PathBuf};

use chloro_core::formatter::config::Config;
use chloro_core::formatter::format_source_with_config;

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    files
}

fn config(options: &[(&str, &str)]) -> Config {
    let mut config = Config::default();
    for (name, value) in options {
        config.set(name, value).unwrap();
    }
    config
}

/// Formats `source`, naming `what` if a check panics.
fn format(source: &str, config: &Config, what: &dyn std::fmt::Display) {
    let result = std::panic::catch_unwind(|| format_source_with_config(source, config));
    assert!(result.is_ok(), "width bound check failed on {what}");
}

#[test]
fn width_bound_holds_on_the_fixtures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    let files = rust_files(&root);
    assert!(files.len() > 1000, "{} fixtures found", files.len());
    let configs = [
        config(&[]),
        config(&[
            ("use_field_init_shorthand", "true"),
            ("use_try_shorthand", "true"),
        ]),
        config(&[("rustfmt_version", "1.8.0")]),
    ];
    for path in &files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        for config in &configs {
            format(&source, config, &path.display());
        }
    }
}

/// `closures-expr` of the cliff generator: `run_0(|a_0| run_1(|a_1| .. finish(a, c)))`.
fn closures_expr(depth: usize) -> String {
    let mut out = String::new();
    for c in 0..8 {
        let mut body = format!("finish(a_{}, {c})", depth - 1);
        for d in (0..depth).rev() {
            body = format!("run_{d}(|a_{d}| {body})");
        }
        out.push_str(&format!("fn f_{c}() {{ {body}; }}\n"));
    }
    out
}

#[test]
fn width_bound_holds_on_nested_closures() {
    let config = Config::default();
    for depth in 2..=16 {
        let source = closures_expr(depth);
        format(&source, &config, &format!("closures-expr at depth {depth}"));
        // Formatting the output again checks the formatted shape too.
        let formatted = format_source_with_config(&source, &config);
        format(
            &formatted,
            &config,
            &format!("closures-expr at depth {depth}, formatted"),
        );
    }
}

#[test]
fn width_bound_holds_on_an_extra_corpus() {
    let Some(root) = std::env::var_os("CHLORO_WIDTH_BOUND_ROOT") else {
        return;
    };
    let config = Config::default();
    for path in rust_files(Path::new(&root)) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        format(&source, &config, &path.display());
    }
}
