//! Cost of reading source into a tree, without formatting: the floor under any formatter
//! built on rust-analyzer's parser.
//!
//! ```text
//! cargo run --release -p chloro-core --example parse_floor -- MODE [ROOT] [-n ROUNDS]
//! ```
//!
//! Every mode runs the same lexer and parser for edition 2024 over every `.rs` file under
//! ROOT (default: the conformance fixtures) and reports the best of `-n` rounds (default 5):
//!
//! - `lex`: `LexedStr::new` only;
//! - `events`: lexing and parsing to parser events, no tree;
//! - `rowan`: events with trivia attached into a rowan tree by `SyntaxTreeBuilder`, as
//!   `formatting::parse` builds it (a new `NodeCache` per file);
//! - `rowan-shared`: as `rowan`, with one `NodeCache` reused for every file of a round;
//! - `rowan-walk`: as `rowan`, then a walk over every red node and token;
//! - `flat`: events with trivia attached by the same `intersperse_trivia`, stored in two
//!   flat arrays (nodes and tokens) reused between files, no rowan tree.
//!
//! `rowan-shared` keeps the cache between rounds, so run it with `-n 1` for a fair
//! comparison.

use std::cell::RefCell;
use std::path::Path;
use std::time::Instant;

use ra_ap_parser::{Edition, LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxTreeBuilder, WalkEvent};

const EDITION: Edition = Edition::Edition2024;

fn rowan_tree(src: &str) -> SyntaxNode {
    let lexed = LexedStr::new(EDITION, src);
    let output = TopEntryPoint::SourceFile.parse(&lexed.to_input(EDITION), EDITION);
    let mut builder = SyntaxTreeBuilder::default();
    lexed.intersperse_trivia(&output, &mut |step| match step {
        StrStep::Token { kind, text } => builder.token(kind, text),
        StrStep::Enter { kind } => builder.start_node(kind),
        StrStep::Exit => builder.finish_node(),
        StrStep::Error { .. } => {}
    });
    builder.finish().syntax_node()
}

fn rowan_shared(src: &str, cache: &mut rowan::NodeCache) -> usize {
    let lexed = LexedStr::new(EDITION, src);
    let output = TopEntryPoint::SourceFile.parse(&lexed.to_input(EDITION), EDITION);
    let mut builder = rowan::GreenNodeBuilder::with_cache(cache);
    lexed.intersperse_trivia(&output, &mut |step| match step {
        StrStep::Token { kind, text } => builder.token(rowan::SyntaxKind(kind as u16), text),
        StrStep::Enter { kind } => builder.start_node(rowan::SyntaxKind(kind as u16)),
        StrStep::Exit => builder.finish_node(),
        StrStep::Error { .. } => {}
    });
    builder.finish().children().len()
}

/// A node of the flat tree: its kind, its parent and next sibling (`u32::MAX` for none) and
/// the range of its tokens in `Flat::tokens`.
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct FlatNode {
    kind: SyntaxKind,
    parent: u32,
    next_sibling: u32,
    first_token: u32,
    end_token: u32,
}

/// A token of the flat tree: its kind and its byte range in the source.
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct FlatToken {
    kind: SyntaxKind,
    start: u32,
    len: u32,
}

#[derive(Default)]
struct Flat {
    nodes: Vec<FlatNode>,
    tokens: Vec<FlatToken>,
    open: Vec<u32>,
    last_child: Vec<u32>,
}

impl Flat {
    fn build(&mut self, src: &str) -> usize {
        self.nodes.clear();
        self.tokens.clear();
        self.open.clear();
        self.last_child.clear();
        let lexed = LexedStr::new(EDITION, src);
        let output = TopEntryPoint::SourceFile.parse(&lexed.to_input(EDITION), EDITION);
        let base = src.as_ptr() as usize;
        let Flat {
            nodes,
            tokens,
            open,
            last_child,
        } = self;
        lexed.intersperse_trivia(&output, &mut |step| match step {
            StrStep::Token { kind, text } => tokens.push(FlatToken {
                kind,
                start: (text.as_ptr() as usize - base) as u32,
                len: text.len() as u32,
            }),
            StrStep::Enter { kind } => {
                let id = nodes.len() as u32;
                if let Some(last) = last_child.last_mut() {
                    if *last != u32::MAX {
                        nodes[*last as usize].next_sibling = id;
                    }
                    *last = id;
                }
                nodes.push(FlatNode {
                    kind,
                    parent: open.last().copied().unwrap_or(u32::MAX),
                    next_sibling: u32::MAX,
                    first_token: tokens.len() as u32,
                    end_token: 0,
                });
                open.push(id);
                last_child.push(u32::MAX);
            }
            StrStep::Exit => {
                let id = open.pop().expect("balanced events");
                last_child.pop();
                nodes[id as usize].end_token = tokens.len() as u32;
            }
            StrStep::Error { .. } => {}
        });
        self.nodes.len() + self.tokens.len()
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .expect("mode: lex|events|rowan|rowan-shared|rowan-walk|flat");
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    let mut rounds = 5;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" => {
                rounds = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .expect("-n takes a number")
            }
            _ => root = arg.into(),
        }
    }
    let mut files: Vec<_> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    let sources: Vec<String> = files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();
    let bytes: usize = sources.iter().map(String::len).sum();

    let cache = RefCell::new(rowan::NodeCache::default());
    let mut flat = Flat::default();
    let mut best = f64::MAX;
    let mut checksum = 0usize;
    for _ in 0..rounds {
        let start = Instant::now();
        for src in &sources {
            checksum += match mode.as_str() {
                "lex" => LexedStr::new(EDITION, src).len(),
                "events" => {
                    let lexed = LexedStr::new(EDITION, src);
                    let input = lexed.to_input(EDITION);
                    TopEntryPoint::SourceFile
                        .parse(&input, EDITION)
                        .iter()
                        .count()
                }
                "rowan" => rowan_tree(src).green().children().len(),
                "rowan-shared" => rowan_shared(src, &mut cache.borrow_mut()),
                "rowan-walk" => rowan_tree(src)
                    .preorder_with_tokens()
                    .filter(|e| matches!(e, WalkEvent::Enter(NodeOrToken::Token(_))))
                    .count(),
                "flat" => flat.build(src),
                _ => panic!("unknown mode {mode}"),
            };
        }
        best = best.min(start.elapsed().as_secs_f64());
    }
    let mb = bytes as f64 / 1e6;
    println!(
        "{mode}: {} files, {mb:.1} MB: {best:.3}s, {:.2} MB/s (best of {rounds}; checksum {checksum})",
        sources.len(),
        mb / best
    );
}
