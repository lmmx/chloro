//! E7: a flat syntax array with a generated typed layer, measured against rowan.
//!
//! ```text
//! cargo run --release -p chloro-core --example flat_cst -- MODE [ROOT] [-n ROUNDS] [-p]
//! ```
//!
//! Every mode runs over every `.rs` file under ROOT (default: the conformance fixtures) that
//! lexes and parses without error (the files the formatter formats), and reports the best of
//! `-n` rounds (default 1). `-p` builds the flat tree with its parent column.
//!
//! - `events`: lexing and parsing to parser events, no tree (the floor under both builds);
//! - `build-rowan`: events plus the rowan tree, built as `formatting::parse` builds it
//!   (`GreenNodeBuilder`, a new `NodeCache` per file, `SyntaxNode::new_root`);
//! - `build-flat`: events plus the flat tree (`cst::Tree::from_steps`);
//! - `query-red`: `build-rowan`, then the query workload over red nodes and the
//!   `ra_ap_syntax::ast` accessors;
//! - `query-flat`: `build-flat`, then the same workload (same source text, `workload.rs`)
//!   over flat nodes and the generated `ast.rs`;
//! - `parent-column` / `parent-search`: `build-flat`, then `parent()` of every node, from
//!   the column or by backward search;
//! - `check`: builds both trees for every file, compares them element by element (kind,
//!   range, token text against the source slice) and compares the two workloads' checksums;
//! - `fragments`: for every macro call with `(`/`[`/`{` delimiters, re-parses the token
//!   tree interior as an expression into a fragment tree at the interior's file offset
//!   (`cst::Tree::from_steps_at`) and checks that every fragment element's text is the file's
//!   text at its range; counts fragments and fragment elements;
//! - `mem FILE`: retained and peak heap bytes of each tree for one file.
//!
//! The only `unsafe` in this example is the counting allocator for `mem` (see `Counting`).

pub mod ast;
pub mod cst;
pub mod ext;

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use ra_ap_parser::{Edition, LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::{RustLanguage, SyntaxKind, SyntaxNode};
use rowan::{GreenNodeBuilder, Language};

const EDITION: Edition = Edition::Edition2024;

// ---------------------------------------------------------------- counting allocator

/// Forwards to the system allocator and counts live and peak bytes. `unsafe impl` is what
/// `GlobalAlloc` requires; the safety argument is that every method forwards its arguments
/// unchanged to `System`, whose contract this type therefore inherits, and only adds
/// relaxed atomic bookkeeping that touches no allocated memory.
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn note_alloc(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note_alloc(layout.size());
        // SAFETY: same layout, forwarded to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: `ptr` was returned by `System.alloc` with this layout (every allocation
        // of this program goes through `Counting`, which forwards to `System`).
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        note_alloc(new_size);
        // SAFETY: as for `dealloc`; `new_size` is forwarded unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn live_bytes() -> usize {
    LIVE.load(Ordering::Relaxed)
}

fn reset_peak() {
    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
}

fn peak_bytes() -> usize {
    PEAK.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------- builds

fn events(src: &str) -> Option<(LexedStr<'_>, ra_ap_parser::Output)> {
    let lexed = LexedStr::new(EDITION, src);
    if lexed.errors().next().is_some() {
        return None;
    }
    let input = lexed.to_input(EDITION);
    let output = TopEntryPoint::SourceFile.parse(&input, EDITION);
    Some((lexed, output))
}

/// The rowan tree as `formatting::parse` builds it (`TreeBuilder`).
fn build_rowan(lexed: &LexedStr<'_>, output: &ra_ap_parser::Output) -> Option<SyntaxNode> {
    let mut builder = GreenNodeBuilder::new();
    let mut has_error = false;
    lexed.intersperse_trivia(output, &mut |step| match step {
        StrStep::Token { kind, text } => builder.token(RustLanguage::kind_to_raw(kind), text),
        StrStep::Enter { kind } => builder.start_node(RustLanguage::kind_to_raw(kind)),
        StrStep::Exit => builder.finish_node(),
        StrStep::Error { .. } => has_error = true,
    });
    if has_error {
        return None;
    }
    Some(SyntaxNode::new_root(builder.finish()))
}

fn build_flat<'s>(
    src: &'s str,
    lexed: &LexedStr<'_>,
    output: &ra_ap_parser::Output,
    parents: bool,
) -> Option<cst::Tree<'s>> {
    match cst::Tree::from_steps(src, lexed, output, parents) {
        cst::Built::Tree(t) => Some(t),
        _ => None,
    }
}

/// `true` when the formatter would format the file: it lexes and parses without error.
fn parses(src: &str) -> bool {
    let Some((lexed, output)) = events(src) else {
        return false;
    };
    let mut has_error = false;
    lexed.intersperse_trivia(&output, &mut |step| {
        if let StrStep::Error { .. } = step {
            has_error = true
        }
    });
    !has_error
}

// ---------------------------------------------------------------- the workload, twice

/// Shared by both instantiations: the checksum and counters the workload fills.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acc {
    pub hash: u64,
    /// Typed accessor calls made (child, children-element, token).
    pub accessor_calls: u64,
    /// Elements visited by `children_with_tokens` loops.
    pub elements: u64,
}

impl Acc {
    #[inline]
    fn mix(&mut self, v: u64) {
        self.hash = (self.hash ^ v).wrapping_mul(0x100_0000_01b3).rotate_left(7);
    }
}

/// The workload over rowan's red nodes and `ra_ap_syntax`'s accessors, as the formatter
/// reads them today: typed accessors on `SyntaxNode`, green reads for `rustc_span` and
/// `has_token` (chloro's `span.rs` and `nodes.rs`).
///
/// `needless_borrow` and `clone_on_copy`: `workload.rs` is written once for both substrates;
/// `&x.syntax()` and `n.clone()` are needed for rowan's `&SyntaxNode` and cost nothing on
/// the `Copy` flat handles.
#[allow(clippy::needless_borrow, clippy::clone_on_copy)]
mod red {
    pub use ra_ap_syntax::ast::{
        self, AstNode, HasArgList, HasGenericArgs, HasLoopBody, RangeItem,
    };
    pub use ra_ap_syntax::{NodeOrToken, SyntaxNode as Node, SyntaxToken as Token};
    use ra_ap_syntax::{RustLanguage, SyntaxKind};
    use rowan::Language;

    pub fn is_outer_doc_text(text: &str) -> bool {
        (text.starts_with("///") && !text.starts_with("////"))
            || (text.starts_with("/**") && !text.starts_with("/***") && !text.starts_with("/**/"))
    }

    /// chloro's `span::rustc_span`: the node's range without the leading trivia
    /// rust-analyzer attached to it, read from the green children.
    pub fn rustc_span(node: &Node) -> (u32, u32) {
        let r = node.text_range();
        let (mut lo, hi) = (u32::from(r.start()), u32::from(r.end()));
        for child in node.green().children() {
            match child {
                NodeOrToken::Node(_) => return (lo, hi),
                NodeOrToken::Token(t) => {
                    let significant = match RustLanguage::kind_from_raw(t.kind()) {
                        SyntaxKind::WHITESPACE => false,
                        SyntaxKind::COMMENT => is_outer_doc_text(t.text()),
                        _ => true,
                    };
                    if significant {
                        return (lo, hi);
                    }
                    lo += u32::from(t.text_len());
                }
            }
        }
        (lo, hi)
    }

    /// chloro's `nodes::has_token`: a direct child token of `kind`, read from the green tree.
    pub fn has_token(node: &Node, kind: SyntaxKind) -> bool {
        node.green().children().any(
            |c| matches!(c, NodeOrToken::Token(t) if RustLanguage::kind_from_raw(t.kind()) == kind),
        )
    }

    pub fn token_text(t: &Token) -> &str {
        t.text()
    }

    include!("workload.rs");
}

/// The same workload source over the flat tree and the generated layer.
#[allow(clippy::needless_borrow, clippy::clone_on_copy)]
mod flat {
    pub use crate::ast::{self, HasArgList, HasGenericArgs, HasLoopBody};
    pub use crate::cst::{AstNode, Node, NodeOrToken, Token};
    pub use crate::ext::RangeItem;
    use ra_ap_syntax::SyntaxKind;

    pub use super::red::is_outer_doc_text;

    pub fn rustc_span(node: &Node) -> (u32, u32) {
        let r = node.text_range();
        let (mut lo, hi) = (u32::from(r.start()), u32::from(r.end()));
        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Node(_) => return (lo, hi),
                NodeOrToken::Token(t) => {
                    let significant = match t.kind() {
                        SyntaxKind::WHITESPACE => false,
                        SyntaxKind::COMMENT => is_outer_doc_text(t.text()),
                        _ => true,
                    };
                    if significant {
                        return (lo, hi);
                    }
                    lo += u32::from(t.text_range().len());
                }
            }
        }
        (lo, hi)
    }

    pub fn has_token(node: &Node, kind: SyntaxKind) -> bool {
        node.children_with_tokens()
            .any(|c| matches!(c, NodeOrToken::Token(t) if t.kind() == kind))
    }

    pub fn token_text<'t>(t: &Token<'t>) -> &'t str {
        t.text()
    }

    include!("workload.rs");
}

// ---------------------------------------------------------------- harness

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

fn load(root: &Path) -> Vec<(PathBuf, String)> {
    files_under(root)
        .into_iter()
        .filter_map(|p| {
            let s = std::fs::read_to_string(&p).ok()?;
            let s = s
                .strip_prefix('\u{feff}')
                .unwrap_or(&s)
                .replace("\r\n", "\n");
            parses(&s).then_some((p, s))
        })
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("mode");
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    let mut rounds = 1usize;
    let mut parents = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" => rounds = args.next().and_then(|n| n.parse().ok()).expect("-n N"),
            "-p" => parents = true,
            _ => root = arg.into(),
        }
    }

    if mode == "mem" {
        mem(&root);
        return;
    }

    let sources = load(&root);
    let bytes: usize = sources.iter().map(|(_, s)| s.len()).sum();
    let mut best = f64::MAX;
    let mut checksum = 0u64;
    let mut acc_total = Acc::default();
    for _ in 0..rounds {
        let start = Instant::now();
        let mut acc = Acc::default();
        for (_, src) in &sources {
            match mode.as_str() {
                "events" => {
                    let (_, output) = events(src).unwrap();
                    checksum = checksum.wrapping_add(output.iter().count() as u64);
                }
                "build-rowan" => {
                    let (lexed, output) = events(src).unwrap();
                    let root = build_rowan(&lexed, &output).unwrap();
                    checksum = checksum.wrapping_add(u64::from(u32::from(root.text_range().end())));
                }
                "build-flat" => {
                    let (lexed, output) = events(src).unwrap();
                    let tree = build_flat(src, &lexed, &output, parents).unwrap();
                    checksum = checksum.wrapping_add(tree.len() as u64);
                }
                "query-red" => {
                    let (lexed, output) = events(src).unwrap();
                    let root = build_rowan(&lexed, &output).unwrap();
                    red::run(&root, &mut acc);
                }
                "query-flat" => {
                    let (lexed, output) = events(src).unwrap();
                    let tree = build_flat(src, &lexed, &output, parents).unwrap();
                    flat::run(&tree.root(), &mut acc);
                }
                "parent-column" | "parent-search" => {
                    let (lexed, output) = events(src).unwrap();
                    let tree = build_flat(src, &lexed, &output, mode == "parent-column").unwrap();
                    for n in tree.nodes() {
                        let p = if mode == "parent-column" {
                            n.parent().map(|p| p.index())
                        } else {
                            tree.parent_by_search_of(n.index())
                        };
                        checksum = checksum.wrapping_add(u64::from(p.unwrap_or(u32::MAX)));
                    }
                }
                "check" => {
                    check(src, &mut acc);
                }
                "fragments" => {
                    let (lexed, output) = events(src).unwrap();
                    let tree = build_flat(src, &lexed, &output, false).unwrap();
                    fragments(src, &tree, &mut acc);
                }
                _ => panic!("unknown mode {mode}"),
            }
        }
        best = best.min(start.elapsed().as_secs_f64());
        acc_total = acc;
    }
    let mb = bytes as f64 / 1e6;
    println!(
        "{mode}{}: {} files, {mb:.2} MB: {best:.3}s, {:.2} MB/s (best of {rounds}; checksum {checksum}; workload {acc_total:?})",
        if parents { "+parents" } else { "" },
        sources.len(),
        mb / best
    );
}

/// Both trees for `src`, compared element by element and by workload checksum.
fn check(src: &str, acc: &mut Acc) {
    let (lexed, output) = events(src).unwrap();
    let root = build_rowan(&lexed, &output).unwrap();
    let tree = build_flat(src, &lexed, &output, true).unwrap();
    // Same preorder: kinds, ranges, and token texts equal to the source slice.
    let mut n = 0u32;
    for (i, e) in root.preorder_with_tokens().enumerate() {
        let rowan::WalkEvent::Enter(el) = e else {
            continue;
        };
        let _ = i;
        let fe = tree.elem(n);
        assert_eq!(el.kind(), fe.kind, "kind at element {n}");
        assert_eq!(
            el.text_range(),
            cst::range_of_elem(&fe),
            "range at element {n}"
        );
        match el {
            ra_ap_syntax::NodeOrToken::Node(_) => assert!(fe.is_node, "node at {n}"),
            ra_ap_syntax::NodeOrToken::Token(t) => {
                assert!(!fe.is_node, "token at {n}");
                let slice = &src[fe.start as usize..(fe.start + fe.len) as usize];
                assert_eq!(t.text(), slice, "token text at {n}");
            }
        }
        n += 1;
    }
    assert_eq!(n as usize, tree.len(), "element count");
    // Parent column against rowan's parent, and the search against the column.
    for node in tree.nodes() {
        let by_search = tree.parent_by_search_of(node.index());
        assert_eq!(
            node.parent().map(|p| p.index()),
            by_search,
            "parent of {node:?}"
        );
    }
    for (el, fl) in root.descendants().zip(tree.nodes()) {
        assert_eq!(
            el.parent().map(|p| p.text_range()),
            fl.parent().map(|p| p.text_range()),
            "parent range of {el:?}"
        );
    }
    // Same workload answers.
    let mut a = Acc::default();
    let mut b = Acc::default();
    red::run(&root, &mut a);
    flat::run(&tree.root(), &mut b);
    assert_eq!(a, b, "workload checksums differ");
    acc.hash = acc.hash.wrapping_add(a.hash);
    acc.accessor_calls += a.accessor_calls;
    acc.elements += a.elements;
}

/// Re-parses every macro call's token tree interior as an expression into a fragment tree at
/// its file offset, as `macro_args::MacroTokens::build` + `splice` do for each argument (here
/// the whole interior, without the comma splitting), and checks offsets and texts.
fn fragments(src: &str, tree: &cst::Tree<'_>, acc: &mut Acc) {
    use cst::AstNode;
    for n in tree.nodes().filter(|n| n.kind() == SyntaxKind::MACRO_CALL) {
        let call = ast::MacroCall::cast(n).unwrap();
        let Some(tt) = call.token_tree() else {
            continue;
        };
        let (Some(open), Some(close)) = (tt.syntax().first_token(), tt.syntax().last_token())
        else {
            continue;
        };
        let base = u32::from(open.text_range().end());
        let end = u32::from(close.text_range().start());
        if end <= base {
            continue;
        }
        let interior = &src[base as usize..end as usize];
        let lexed = LexedStr::new(EDITION, interior);
        if lexed.errors().next().is_some() {
            continue;
        }
        let input = lexed.to_input(EDITION);
        let output = TopEntryPoint::Expr.parse(&input, EDITION);
        let frag = match cst::Tree::from_steps_at(src, &lexed, &output, base, false) {
            cst::Built::Tree(t) => t,
            _ => continue,
        };
        acc.elements += 1;
        for el in frag.root().descendants_with_tokens() {
            acc.accessor_calls += 1;
            let r = el.text_range();
            assert!(
                u32::from(r.start()) >= base && u32::from(r.end()) <= end,
                "fragment element outside the token tree"
            );
            if let cst::NodeOrToken::Token(t) = el {
                assert_eq!(t.text(), &src[usize::from(r.start())..usize::from(r.end())]);
                acc.mix(u64::from(u32::from(r.start())));
            }
        }
        // The fragment root covers the interior, and the file tree knows nothing of it.
        assert!(frag.root().parent().is_none());
    }
}

/// Retained and peak heap bytes of each tree for one file.
fn mem(file: &Path) {
    let src = std::fs::read_to_string(file).unwrap();
    let src = src
        .strip_prefix('\u{feff}')
        .unwrap_or(&src)
        .replace("\r\n", "\n");
    assert!(parses(&src));
    println!("{}: {} bytes of source", file.display(), src.len());

    let before = live_bytes();
    reset_peak();
    let (lexed, output) = events(&src).unwrap();
    let events_peak = peak_bytes() - before;
    let events_live = live_bytes() - before;
    println!("events: live {events_live} bytes (LexedStr + Input + Output), peak {events_peak}");

    for parents in [false, true] {
        let before = live_bytes();
        reset_peak();
        let tree = build_flat(&src, &lexed, &output, parents).unwrap();
        let peak = peak_bytes() - before;
        let retained = live_bytes() - before;
        println!(
            "flat{}: {} elements, retained {retained} bytes ({} per element, {} per source byte), peak during build {peak}",
            if parents { "+parents" } else { "" },
            tree.len(),
            retained as f64 / tree.len() as f64,
            retained as f64 / src.len() as f64,
        );
        drop(tree);
    }

    let before = live_bytes();
    reset_peak();
    let root = build_rowan(&lexed, &output).unwrap();
    let peak = peak_bytes() - before;
    let retained = live_bytes() - before;
    let elements = root
        .preorder_with_tokens()
        .filter(|e| matches!(e, rowan::WalkEvent::Enter(_)))
        .count();
    println!(
        "rowan: {elements} elements, retained {retained} bytes ({} per element, {} per source byte), peak during build {peak}",
        retained as f64 / elements as f64,
        retained as f64 / src.len() as f64,
    );
    // Red nodes: one allocation per handle.
    let before = live_bytes();
    let handles: Vec<SyntaxNode> = root.descendants().collect();
    println!(
        "rowan: {} red node handles held at once cost {} bytes ({} each)",
        handles.len(),
        live_bytes() - before,
        (live_bytes() - before) as f64 / handles.len() as f64
    );
    drop(handles);
    let _ = SyntaxKind::EOF;
}
