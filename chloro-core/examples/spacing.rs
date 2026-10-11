//! Measures how much of its input a formatter lays out, rather than copies as written, and
//! builds corpora on which two formatters give the same output.
//!
//! ```text
//! cargo run --release -p chloro-core --example spacing -- split SRC DST
//! cargo run --release -p chloro-core --example spacing -- perturb SRC DST
//! cargo run --release -p chloro-core --example spacing -- survival ORIG PERT OUT_ORIG OUT_PERT [REF]
//! cargo run --release -p chloro-core --example spacing -- agree PERT OUT_A OUT_B DST [--rustfmt]
//! ```
//!
//! - `split` writes every top-level item of every file under SRC that rust-analyzer parses
//!   without error to its own file under DST (leading comments and attributes are inside
//!   rust-analyzer's item nodes). DST is flat: `a/b.rs` item 3 is `a__b.rs__0003.rs`.
//! - `perturb` copies every `.rs` file under SRC to DST with every whitespace token that
//!   is a single space doubled. Comments, literals and newlines are unchanged.
//! - `survival` counts, over the files of PERT (a `perturb` copy of ORIG), the runs of two
//!   or more spaces between non-space characters in each formatter output, minus the count
//!   in the output for the unperturbed input. The result is the number of doubled spaces
//!   the formatter left as written, against the number injected. Files whose perturbed
//!   output equals the perturbed input (returned unchanged: parse error, `rustfmt::skip`)
//!   are left out; with REF, another formatter's OUT_PERT, the files left out are those REF
//!   returned unchanged, so that two formatters are compared on one set of files.
//! - `agree` copies to DST every file of PERT whose two outputs are equal, differ from the
//!   input and, with `--rustfmt`, equal `rustfmt --edition 2024` reading the file on stdin.
//!
//! OUT_* directories are produced by `examples/fmt_dir.rs` with the same relative paths.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ra_ap_parser::{Edition, LexedStr};
use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind, ast};

fn rs_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("file has a parent")).expect("create dir");
    std::fs::write(path, text).expect("write file");
}

/// Runs of two or more spaces between non-space characters.
fn double_spaces(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' && i > 0 && !bytes[i - 1].is_ascii_whitespace() {
            let start = i;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            if i - start >= 2 && i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                count += 1;
            }
        } else {
            i += 1;
        }
    }
    count
}

fn split(src: &Path, dst: &Path) {
    let (mut files, mut items) = (0, 0);
    for path in rs_files(src) {
        let text = read(&path);
        let parse = SourceFile::parse(&text, Edition::Edition2024);
        if !parse.errors().is_empty() {
            continue;
        }
        files += 1;
        let name = path
            .strip_prefix(src)
            .unwrap()
            .to_string_lossy()
            .replace('/', "__");
        let file_items = parse.tree().syntax().children().filter_map(ast::Item::cast);
        for (i, item) in file_items.enumerate() {
            write(
                &dst.join(format!("{name}__{i:04}.rs")),
                &format!("{}\n", item.syntax()),
            );
            items += 1;
        }
    }
    println!("{files} files, {items} items");
}

fn perturb(src: &Path, dst: &Path) {
    for path in rs_files(src) {
        let text = read(&path);
        let lexed = LexedStr::new(Edition::Edition2024, &text);
        let mut out = String::with_capacity(text.len() * 2);
        for i in 0..lexed.len() {
            let token = lexed.text(i);
            let single_space = lexed.kind(i) == SyntaxKind::WHITESPACE && token == " ";
            out.push_str(if single_space { "  " } else { token });
        }
        write(&dst.join(path.strip_prefix(src).unwrap()), &out);
    }
}

fn survival(orig: &Path, pert: &Path, out_orig: &Path, out_pert: &Path, reference: &Path) {
    let (mut files, mut injected, mut survived) = (0, 0i64, 0i64);
    for path in rs_files(pert) {
        let rel = path.strip_prefix(pert).unwrap();
        let input = read(&path);
        let output = read(&out_pert.join(rel));
        if read(&reference.join(rel)) == input {
            continue;
        }
        files += 1;
        injected += double_spaces(&input) as i64 - double_spaces(&read(&orig.join(rel))) as i64;
        survived +=
            double_spaces(&output) as i64 - double_spaces(&read(&out_orig.join(rel))) as i64;
    }
    println!(
        "{files} files: {injected} doubled spaces injected, {survived} left as written ({:.1}%)",
        100.0 * survived as f64 / injected as f64
    );
}

fn rustfmt(input: &str) -> Option<String> {
    use std::io::Write;
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(input.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).ok())
        .flatten()
}

fn agree(pert: &Path, out_a: &Path, out_b: &Path, dst: &Path, check_rustfmt: bool) {
    let files = rs_files(pert);
    let equal: Vec<(&PathBuf, String, String)> = files
        .iter()
        .filter_map(|path| {
            let rel = path.strip_prefix(pert).unwrap();
            let (input, a) = (read(path), read(&out_a.join(rel)));
            (a == read(&out_b.join(rel)) && a != input).then_some((path, input, a))
        })
        .collect();
    // One rustfmt process per file; spread over the available cores.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = equal.len().div_ceil(threads).max(1);
    let kept: usize = std::thread::scope(|s| {
        let handles: Vec<_> = equal
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut kept = 0;
                    for (path, input, a) in part {
                        if check_rustfmt && rustfmt(input).as_deref() != Some(a.as_str()) {
                            continue;
                        }
                        kept += 1;
                        write(&dst.join(path.strip_prefix(pert).unwrap()), input);
                    }
                    kept
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("worker")).sum()
    });
    println!(
        "{} files: {} with equal outputs, {kept} copied to {}",
        files.len(),
        equal.len(),
        dst.display()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let p = |i: usize| Path::new(args.get(i).map(String::as_str).expect("missing argument"));
    match args.first().map(String::as_str) {
        Some("split") => split(p(1), p(2)),
        Some("perturb") => perturb(p(1), p(2)),
        Some("survival") => survival(
            p(1),
            p(2),
            p(3),
            p(4),
            p(if args.len() > 5 { 5 } else { 4 }),
        ),
        Some("agree") => agree(
            p(1),
            p(2),
            p(3),
            p(4),
            args.iter().any(|a| a == "--rustfmt"),
        ),
        _ => panic!("usage: spacing split|perturb|survival|agree ..."),
    }
}
