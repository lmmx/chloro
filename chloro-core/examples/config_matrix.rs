//! Compares chloro with `rustfmt --edition 2024 --config <option>` for non-default values of
//! rustfmt's stable options, over a sample of the conformance fixtures.
//!
//! ```text
//! cargo run --release -p chloro-core --example config_matrix                 # built-in matrix
//! cargo run --release -p chloro-core --example config_matrix -- max_width=80  # one option
//! cargo run --release -p chloro-core --example config_matrix -- -n 400 tab_spaces=2
//! cargo run --release -p chloro-core --example config_matrix -- --root ~/.cargo/registry/src default
//! cargo run --release -p chloro-core --example config_matrix -- rustfmt_version=1.8.0
//! cargo run --release -p chloro-core --example config_matrix -- --rustfmt "rustup run 1.88.0 rustfmt" rustfmt_version=1.8.0,edition=2021
//! ```
//!
//! The first five differing files of each option are listed; `-v` lists all of them.
//!
//! `default` compares rustfmt's default configuration. `--root` selects another directory
//! of Rust sources, such as the cargo registry, as a corpus chloro was not developed on.
//! An argument may combine options with commas (`edition=2021,max_width=80`).
//!
//! `rustfmt_version=<v>` is chloro's option and is not passed to rustfmt: the comparison
//! runs against the rustfmt named by `--rustfmt <command>` (program and arguments), which
//! defaults to `rustup run 1.88.0 rustfmt` for 1.8.0 and to the `rustfmt` on `PATH`
//! otherwise. A file that rustfmt rejects (or panics on) is compared against its input,
//! as the snapshots do, so the count covers every file of the corpus.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use chloro_core::formatter::config::Config;

const MATRIX: &[&str] = &[
    "max_width=80",
    "max_width=120",
    "hard_tabs=true",
    "tab_spaces=2",
    "use_small_heuristics=Max",
    "use_small_heuristics=Off",
    "fn_call_width=40",
    "chain_width=40",
    "struct_lit_width=0",
    "array_width=30",
    "single_line_if_else_max_width=0",
    "single_line_let_else_max_width=0",
    "reorder_imports=false",
    "reorder_modules=false",
    "remove_nested_parens=false",
    "short_array_element_width_threshold=4",
    "match_arm_leading_pipes=Always",
    "match_arm_leading_pipes=Preserve",
    "fn_params_layout=Vertical",
    "fn_params_layout=Compressed",
    "match_block_trailing_comma=true",
    "merge_derives=false",
    "use_try_shorthand=true",
    "use_field_init_shorthand=true",
    "force_explicit_abi=false",
    "newline_style=Windows",
    "style_edition=2021",
    "style_edition=2015",
    "edition=2021",
    "edition=2018",
    "edition=2015",
];

fn fixtures(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

/// The settings of one matrix entry: chloro's `Config` and the rustfmt command line.
struct Entry {
    config: Config,
    /// Program and arguments of the rustfmt to compare against.
    command: Vec<String>,
    /// Arguments after the program: `--edition <e>` and `--config` pairs.
    args: Vec<String>,
    /// `rustfmt_version` is set: a file rustfmt rejects counts as unchanged input.
    versioned: bool,
}

fn entry(option: &str, rustfmt_command: Option<&[String]>) -> Entry {
    let mut config = Config::default();
    let mut edition = "2024".to_owned();
    let mut configs = Vec::new();
    let mut versioned = false;
    for item in option.split(',').filter(|o| *o != "default") {
        let (key, value) = item.split_once('=').expect("option is key=value");
        config.set(key, value).expect("option is supported");
        match key {
            // rustfmt's `--edition` flag overrides the `edition` option.
            "edition" => edition = value.to_owned(),
            // chloro's option: rustfmt has none, the release is chosen by the command.
            "rustfmt_version" => versioned = true,
            _ => configs.push(item.to_owned()),
        }
    }
    let command = match rustfmt_command {
        Some(c) => c.to_vec(),
        None if config.rustfmt_version.as_str() == "1.8.0" => {
            ["rustup", "run", "1.88.0", "rustfmt"]
                .map(String::from)
                .to_vec()
        }
        None => vec!["rustfmt".to_owned()],
    };
    let mut args = vec!["--edition".to_owned(), edition];
    for c in configs {
        args.push("--config".to_owned());
        args.push(c);
    }
    Entry {
        config,
        command,
        args,
        versioned,
    }
}

fn rustfmt(source: &str, entry: &Entry) -> Option<String> {
    let (program, prefix) = entry.command.split_first().expect("rustfmt command");
    let mut child = Command::new(program)
        .args(prefix)
        .args(&entry.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run {program}: {e}"));
    let written = child
        .stdin
        .take()
        .expect("stdin")
        .write_all(source.as_bytes());
    let output = child.wait_with_output().ok()?;
    if written.is_err() || !output.status.success() {
        return entry.versioned.then(|| source.to_owned());
    }
    String::from_utf8(output.stdout).ok()
}

fn run(
    option: &str,
    rustfmt_command: Option<&[String]>,
    files: &[PathBuf],
) -> (usize, usize, Vec<String>) {
    let entry = entry(option, rustfmt_command);
    let config = &entry.config;

    let next = AtomicUsize::new(0);
    let identical = AtomicUsize::new(0);
    let compared = AtomicUsize::new(0);
    let differing = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(i) else { break };
                    let Ok(source) = std::fs::read_to_string(path) else {
                        continue;
                    };
                    let Some(expected) = rustfmt(&source, &entry) else {
                        continue;
                    };
                    compared.fetch_add(1, Ordering::Relaxed);
                    let actual = chloro_core::formatter::format_source_with_config(&source, config);
                    if actual == expected {
                        identical.fetch_add(1, Ordering::Relaxed);
                    } else {
                        differing.lock().unwrap().push(path.display().to_string());
                    }
                }
            });
        }
    });
    let mut differing = differing.into_inner().unwrap();
    differing.sort();
    (identical.into_inner(), compared.into_inner(), differing)
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut sample = 200;
    let verbose = args.iter().any(|a| a == "-v");
    args.retain(|a| a != "-v");
    if let Some(i) = args.iter().position(|a| a == "-n") {
        sample = args[i + 1].parse().expect("-n takes a number");
        args.drain(i..=i + 1);
    }
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    if let Some(i) = args.iter().position(|a| a == "--root") {
        root = PathBuf::from(&args[i + 1]);
        args.drain(i..=i + 1);
    }
    let mut rustfmt_command: Option<Vec<String>> = None;
    if let Some(i) = args.iter().position(|a| a == "--rustfmt") {
        rustfmt_command = Some(args[i + 1].split_whitespace().map(String::from).collect());
        args.drain(i..=i + 1);
    }
    let options: Vec<&str> = if args.is_empty() {
        MATRIX.to_vec()
    } else {
        args.iter().map(String::as_str).collect()
    };

    let all = fixtures(&root);
    let step = (all.len() / sample.max(1)).max(1);
    let files: Vec<PathBuf> = all.into_iter().step_by(step).collect();

    for option in options {
        let (identical, compared, differing) = run(option, rustfmt_command.as_deref(), &files);
        println!("{option:<40} {identical:>4}/{compared:<4}");
        for path in differing.iter().take(if verbose { usize::MAX } else { 5 }) {
            println!("    {path}");
        }
    }
}
