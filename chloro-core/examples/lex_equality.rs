//! The token-equality harness of `cst::lex` (EXP-031): for every text, chloro's lexer front
//! end gives the tokens `LexedStr::new` gives and the input `LexedStr::to_input` gives, or
//! leaves the text to `LexedStr` (`None`) exactly when `LexedStr` reports a lexer error or
//! lexes a shebang or frontmatter.
//!
//! ```text
//! cargo run --release -p chloro-core --example lex_equality -- [--root DIR]... [--random N] [--mutations N]
//! ```
//!
//! For each `.rs` file under each `--root` (default: the conformance fixtures), in editions
//! 2021 and 2024, then for `--random N` random texts (default 10,000) and `--mutations N`
//! random edits of fixture files (default 10,000), each in all four editions, it compares:
//!
//! - every token's kind and start offset, and the end offset, with `LexedStr`'s;
//! - the input, as the sequence of `Input` calls (`push`, `push_ident` with its contextual
//!   kind, `was_joint`) against a transcription of `to_input` over `LexedStr`;
//! - the parser's output (`TopEntryPoint::SourceFile`) over both inputs, step by step;
//! - when `lex` answers `None`: that `LexedStr` has an error or the text a script prefix.
//!
//! It prints the counts and every mismatch, and exits with 1 if there was one.

use std::path::{Path, PathBuf};

use chloro_core::cst::lex::{self, InputSink};
use ra_ap_parser::{Edition, LexedStr, SyntaxKind, TopEntryPoint};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Push(SyntaxKind),
    Ident(SyntaxKind),
    Joint,
}

#[derive(Default)]
struct Ops(Vec<Op>);

impl InputSink for Ops {
    fn push(&mut self, kind: SyntaxKind) {
        self.0.push(Op::Push(kind));
    }
    fn push_ident(&mut self, contextual_kind: SyntaxKind) {
        self.0.push(Op::Ident(contextual_kind));
    }
    fn was_joint(&mut self) {
        self.0.push(Op::Joint);
    }
}

/// `LexedStr::to_input` (ra_ap_parser 0.0.307, `shortcuts.rs`), transcribed to record its
/// calls.
fn reference_ops(lexed: &LexedStr<'_>, edition: Edition) -> Vec<Op> {
    let mut res = Ops::default();
    let mut was_joint = false;
    for i in 0..lexed.len() {
        let kind = lexed.kind(i);
        if kind.is_trivia() {
            was_joint = false
        } else if kind == SyntaxKind::IDENT {
            let token_text = lexed.text(i);
            res.push_ident(
                SyntaxKind::from_contextual_keyword(token_text, edition)
                    .unwrap_or(SyntaxKind::IDENT),
            )
        } else {
            if was_joint {
                res.was_joint();
            }
            res.push(kind);
            if kind == SyntaxKind::FLOAT_NUMBER {
                if !lexed.text(i).ends_with('.') {
                    res.was_joint();
                } else {
                    was_joint = false;
                }
            } else {
                was_joint = true;
            }
        }
    }
    res.0
}

#[derive(Default)]
struct Counts {
    texts: u64,
    tokens: u64,
    own: u64,
    left_errors: u64,
    left_prefix: u64,
    mismatches: u64,
}

fn steps(output: &ra_ap_parser::Output) -> Vec<String> {
    output.iter().map(|s| format!("{s:?}")).collect()
}

/// Compares the two front ends on `text`; returns a description of the first difference.
fn compare(text: &str, edition: Edition, counts: &mut Counts) -> Option<String> {
    counts.texts += 1;
    let lexed = LexedStr::new(edition, text);
    counts.tokens += lexed.len() as u64;
    let mut ops = Ops::default();
    let Some(tokens) = lex::lex_into(edition, text, &mut ops, 16) else {
        if lexed.errors().next().is_some() {
            counts.left_errors += 1;
            return None;
        }
        if lex::starts_with_script_prefix(text) {
            counts.left_prefix += 1;
            return None;
        }
        return Some("lex gave up on a text LexedStr lexes without errors".to_owned());
    };
    counts.own += 1;
    if lexed.errors().next().is_some() {
        return Some("lex lexed a text LexedStr reports errors in".to_owned());
    }
    use lex::Lexed;
    if tokens.len() != lexed.len() {
        return Some(format!("{} tokens, LexedStr {}", tokens.len(), lexed.len()));
    }
    for i in 0..=lexed.len() {
        if tokens.text_start(i) != lexed.text_start(i) {
            return Some(format!("start of token {i}"));
        }
        if i < lexed.len() && tokens.kind(i) != lexed.kind(i) {
            return Some(format!(
                "kind of token {i} ({:?}): {:?}, LexedStr {:?}",
                lexed.text(i),
                tokens.kind(i),
                lexed.kind(i)
            ));
        }
    }
    let reference = reference_ops(&lexed, edition);
    if ops.0 != reference {
        let at = ops
            .0
            .iter()
            .zip(&reference)
            .take_while(|(a, b)| a == b)
            .count();
        return Some(format!(
            "input differs at call {at}: {:?}, to_input {:?}",
            ops.0.get(at),
            reference.get(at)
        ));
    }
    let (_, input) = lex::lex(edition, text).expect("lexed above");
    let ours = TopEntryPoint::SourceFile.parse(&input, edition);
    let theirs = TopEntryPoint::SourceFile.parse(&lexed.to_input(edition), edition);
    if steps(&ours) != steps(&theirs) {
        return Some("parser output differs".to_owned());
    }
    None
}

fn check(label: &str, text: &str, edition: Edition, counts: &mut Counts) {
    if let Some(diff) = compare(text, edition, counts) {
        counts.mismatches += 1;
        if counts.mismatches <= 50 {
            println!("MISMATCH {label} {edition:?}: {diff}\n  text: {text:?}");
        }
    }
}

/// xorshift64*: a fixed, dependency-free sequence.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Pieces of text the random texts are made of: single characters of every lexical class
/// and the openings and closings the lexer treats specially.
const PIECES: &[&str] = &[
    "a",
    "z",
    "_",
    "r",
    "b",
    "c",
    "e",
    "E",
    "x",
    "u",
    "S",
    "Self",
    "self",
    "async",
    "dyn",
    "union",
    "builtin",
    "gen",
    "try",
    "macro_rules",
    "raw",
    "default",
    "fn",
    "let",
    "0",
    "1",
    "9",
    "0x",
    "0b",
    "0o",
    "1e",
    "1E+",
    "1e-",
    "1.",
    "1.0",
    "2.5e3",
    "1_000",
    "0xff_u8",
    "1f32",
    ".",
    "..",
    "...",
    ";",
    ",",
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    "@",
    "#",
    "#!",
    "#![",
    "!",
    "~",
    "?",
    ":",
    "::",
    ":::",
    "$",
    "=",
    "==",
    "<",
    ">",
    "-",
    "->",
    "--",
    "---",
    "&",
    "&&",
    "|",
    "+",
    "*",
    "/",
    "^",
    "%",
    "'",
    "'a",
    "'1",
    "'\\''",
    "'\\u{1F600}'",
    "'ab'",
    "b'",
    "b'\\x7f'",
    "b'é'",
    "\"",
    "\\",
    "\\n",
    "\\u{",
    "\\u{110000}",
    "\\x",
    "\\xff",
    "\\0",
    "\\\n",
    "\\\r\n",
    "r\"",
    "r#\"",
    "\"#",
    "r##\"",
    "\"##",
    "r###\"",
    "\"###",
    "r####",
    "br\"",
    "br#\"",
    "b\"",
    "c\"",
    "cr\"",
    "cr#\"",
    "rb\"",
    "r#ident",
    "r#",
    "#\"",
    "##",
    "k#",
    "'r#a",
    "/*",
    "*/",
    "/**",
    "/*!",
    "//",
    "///",
    "//!",
    "////",
    "/***",
    "/**/",
    "\n",
    "\r\n",
    "\r",
    " ",
    "  ",
    "\t",
    "\u{b}",
    "\u{c}",
    "\u{85}",
    "\u{2028}",
    "\u{2029}",
    "\u{200e}",
    "\u{200f}",
    "\u{3000}",
    "\u{a0}",
    "\0",
    "é",
    "ß",
    "日本",
    "ℕ",
    "𝔸",
    "\u{301}",
    "🦀",
    "😀",
    "❤",
    "\u{fe0f}",
    "\u{200d}",
    "a🦀",
    "\u{feff}",
    "---\n",
    "--- cargo\n",
    "#!/usr/bin/env x\n",
];

/// Whole tokens and spacing, so that texts made of them mostly lex without errors and take
/// `lex`'s own path.
const CLEAN: &[&str] = &[
    " ",
    "\n",
    "\t",
    "\r\n",
    "x",
    "_",
    "_x",
    "Self",
    "self",
    "union",
    "async",
    "dyn",
    "gen",
    "try",
    "auto",
    "default",
    "builtin",
    "raw",
    "macro_rules",
    "format_args",
    "r#type",
    "r#x",
    "'a",
    "'_",
    "'static",
    "'r#a",
    "'x'",
    "'\\n'",
    "'\\u{1F600}'",
    "b'x'",
    "b'\\xff'",
    "1",
    "1.0",
    "1.",
    "1e3",
    "1.5E-3",
    "0x1F",
    "0b10u8",
    "1_000i64",
    "2.0f32",
    "\"s\"",
    "\"a\\nb\\\"c\"",
    "\"\\u{10FFFF}\"",
    "\"line\\\n  next\"",
    "\"\r\"",
    "r\"x\"",
    "r#\"a\"b\"#",
    "r###\"x\"##\"###",
    "b\"x\"",
    "br#\"x\"#",
    "c\"x\"",
    "cr\"x\"",
    "\"x\"suffix",
    "// c\n",
    "/// d\n",
    "//! i\n",
    "/* a */",
    "/* a /* b */ c */",
    "/** d */",
    "/*! i */",
    "#",
    "#[a]",
    "#![a]",
    "!",
    ".",
    "..",
    "..=",
    ";",
    ",",
    "::",
    ":",
    "->",
    "=>",
    "=",
    "==",
    "!=",
    "<",
    ">",
    "<=",
    ">>=",
    "&",
    "&&",
    "|",
    "||",
    "+",
    "-",
    "*",
    "/",
    "%",
    "^",
    "@",
    "$",
    "~",
    "?",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    "é",
    "日本",
    "x.0.1",
    "t.0 .1",
    "x.0.",
];

fn random_text(rng: &mut Rng) -> String {
    let n = rng.below(40);
    let pieces = if rng.below(2) == 0 { PIECES } else { CLEAN };
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(pieces[rng.below(pieces.len())]);
    }
    s
}

/// A random edit of `src`: pieces inserted, ranges deleted, or a window of it.
fn mutate(src: &str, rng: &mut Rng) -> String {
    let mut s = src.to_owned();
    let boundary = |s: &str, rng: &mut Rng| {
        let mut i = rng.below(s.len() + 1);
        while !s.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    for _ in 0..1 + rng.below(4) {
        let at = boundary(&s, rng);
        match rng.below(3) {
            0 => s.insert_str(at, PIECES[rng.below(PIECES.len())]),
            1 => {
                let mut end = (at + 1 + rng.below(16)).min(s.len());
                while !s.is_char_boundary(end) {
                    end += 1;
                }
                s.replace_range(at..end, "");
            }
            _ => {
                let mut end = (at + rng.below(4096)).min(s.len());
                while !s.is_char_boundary(end) {
                    end += 1;
                }
                s = s[at..end].to_owned();
            }
        }
    }
    s
}

fn rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() && p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn report(label: &str, c: &Counts) {
    println!(
        "{label}: {} texts, {} tokens; lexed by lex {}, left to LexedStr {} (lexer errors {}, script prefix {}); mismatches {}",
        c.texts,
        c.tokens,
        c.own,
        c.left_errors + c.left_prefix,
        c.left_errors,
        c.left_prefix,
        c.mismatches
    );
}

fn main() {
    let mut roots = Vec::new();
    let mut random = 10_000usize;
    let mut mutations = 10_000usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().expect("a value");
        match a.as_str() {
            "--root" => roots.push(PathBuf::from(value())),
            "--random" => random = value().parse().expect("a number"),
            "--mutations" => mutations = value().parse().expect("a number"),
            other => panic!("unknown argument {other}"),
        }
    }
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    if roots.is_empty() {
        roots.push(fixtures.clone());
    }
    let mut failed = false;
    for root in &roots {
        let mut c = Counts::default();
        let files = rs_files(root);
        let mut unreadable = 0;
        for f in &files {
            let Ok(src) = std::fs::read_to_string(f) else {
                unreadable += 1;
                continue;
            };
            for edition in [Edition::Edition2021, Edition::Edition2024] {
                check(&f.display().to_string(), &src, edition, &mut c);
            }
        }
        println!(
            "{}: {} files ({unreadable} not UTF-8)",
            root.display(),
            files.len()
        );
        report("  files x 2 editions", &c);
        failed |= c.mismatches > 0;
    }
    let editions = [
        Edition::Edition2015,
        Edition::Edition2018,
        Edition::Edition2021,
        Edition::Edition2024,
    ];
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut c = Counts::default();
    for i in 0..random {
        let text = random_text(&mut rng);
        for edition in editions {
            check(&format!("random {i}"), &text, edition, &mut c);
        }
    }
    report("random texts x 4 editions", &c);
    failed |= c.mismatches > 0;
    let sources: Vec<String> = rs_files(&fixtures)
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .collect();
    let mut c = Counts::default();
    for i in 0..mutations {
        let src = &sources[rng.below(sources.len())];
        let text = mutate(src, &mut rng);
        for edition in editions {
            check(&format!("mutation {i}"), &text, edition, &mut c);
        }
    }
    report("fixture mutations x 4 editions", &c);
    failed |= c.mismatches > 0;
    if failed {
        std::process::exit(1);
    }
}
