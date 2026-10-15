//! The rewrite context and the `Rewrite` trait (rustfmt's `rewrite.rs`).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

use crate::cst::{AstNode, Node, Tree, TreeArena, ast};

use super::comment::find_any_byte;
use super::config::{RustfmtVersion, Settings};
use super::macro_args::ParsedMacroArgs;
use super::nodes::{Block, CachedBlock};
use super::shape::Shape;
use super::skip::SkipContext;
use super::span::{BytePos, SnippetProvider, Span};

/// Parsed macro arguments keyed by token tree range and the `vec!` flag. rustfmt rewrites
/// a macro call once per candidate shape; the arguments are parsed once.
pub(crate) type MacroArgsCache<'a> =
    HashMap<(BytePos, BytePos, bool), Option<ParsedMacroArgs<'a>>, FxBuildHasher>;

/// FxHash (rustc's hasher): the memo keys are a few machine words, for which SipHash's
/// setup cost dominates.
#[derive(Default, Clone, Copy)]
pub(crate) struct FxHasher(u64);

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }
    fn write_u8(&mut self, n: u8) {
        self.write_u64(u64::from(n));
    }
    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type FxBuildHasher = BuildHasherDefault<FxHasher>;

/// The nesting depth from which [`RewriteContext::memoize`] looks rewrites up and stores
/// them, counted in rewrites in progress: an expression rewritten while `MEMO_MIN_DEPTH` or
/// more others are being rewritten. Shallower rewrites are computed every time.
///
/// On ordinary code a rewrite is rarely asked for twice with the same shape, and storing
/// every result costs about as much as the hits save; the hits come from layout attempts
/// that rewrite a subexpression again at the same shape, and they gather inside nested
/// expressions, where the exponential nesting is too. Measured in EXP-028, one process per
/// file: storing every rewrite cost 6.8 M instructions on the 49-file fixture sample and its
/// hits saved 6.7 M (on the registry sample 4.0 M and 0.3 M); from depth 6, 1,801 of 9,766
/// lookups, which found 188 of the 315 entries found, the two samples cost 3.9 M and 2.9 M
/// less than with every rewrite stored, and no cliff shape costs more.
const MEMO_MIN_DEPTH: u32 = 6;

/// What a memoized rewrite reads besides its node: the shape and the context flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MemoKey {
    /// The node's tree (the file or a macro-argument fragment, numbered per run) and its
    /// index in it: the identity of a node. Trees live for the run, so no key is reused.
    tree: u32,
    index: u32,
    /// The rewrite function and its other arguments.
    tag: u8,
    shape: Shape,
    flags: u8,
}

/// A memoized rewrite: its result and the effects it had on the run, replayed on a hit.
struct MemoEntry {
    result: Option<String>,
    macro_failure: bool,
    lost_comment: bool,
    skipped_range: Vec<(usize, usize)>,
}

/// State shared by every context of one formatting run (rustfmt's `FormatReport` and
/// `skipped_range`), and the owner of the run's syntax trees.
///
/// The file's tree is numbered 0; every macro-argument fragment the run parses is laid
/// out as a tree of its own ([`Tree::from_steps_at`]), numbered from 1, and appended to the
/// arena, whose lifetime `'a` every handle in the run borrows. The memo and the
/// macro-argument cache hold such handles.
pub(crate) struct RunState<'a> {
    /// Line ranges (1-based, inclusive) left as in the input: `#[rustfmt::skip]` items and
    /// macro calls that could not be parsed.
    pub(crate) skipped_range: RefCell<Vec<(usize, usize)>>,
    /// Set when a rewrite dropped a comment and fell back to the original text. Only
    /// consulted while formatting a macro definition body.
    pub(crate) lost_comment: Cell<bool>,
    /// Set when the rustfmt release reproduced panics on this input (F6 under
    /// `rustfmt_version = "1.8.0"`): rustfmt then writes nothing, so the file is returned
    /// as it is.
    pub(crate) format_failure: Cell<bool>,
    /// Set when the run formats a whole file (`format_source_with_config` and
    /// `parallel.rs`), not a snippet (`format_snippet`: a macro definition body). A
    /// discarded candidate can set `macro_rewrite_failure`, `lost_comment` or
    /// `skipped_range`, which only a snippet run reads (`format_snippet`), and under
    /// `rustfmt_version = "1.8.0"` `format_failure`, which every run reads; see
    /// [`RewriteContext::may_skip_dead_candidates`].
    pub(crate) whole_file: Cell<bool>,
    pub(crate) macro_args: RefCell<MacroArgsCache<'a>>,
    /// Rewrites already computed in this run; see [`RewriteContext::memoize`].
    memo: RefCell<HashMap<MemoKey, MemoEntry, FxBuildHasher>>,
    /// Blocks already built in this run, by the identity of the block expression (tree id
    /// and index); see [`RunState::block_of`].
    blocks: RefCell<HashMap<(u32, u32), CachedBlock, FxBuildHasher>>,
    /// Start offsets of the source lines that follow a `\n` in the first `.1` bytes of the
    /// source, after the start of the first line; extended on demand by
    /// [`RunState::line_of`].
    line_starts: RefCell<(Vec<BytePos>, usize)>,
    /// The fragment trees of the run, in the order they were parsed.
    arena: &'a TreeArena<'a>,
    /// The id of the next tree to be built ([`RunState::next_tree_id`]).
    next_tree_id: Cell<u32>,
    /// The number of [`RewriteContext::memoize`] rewrites in progress.
    memo_depth: Cell<u32>,
    /// Set while a debug build checks a memo hit against the rewrite itself.
    memo_checking: Cell<bool>,
}

impl<'a> RunState<'a> {
    pub(crate) fn new(arena: &'a TreeArena<'a>) -> RunState<'a> {
        RunState {
            skipped_range: RefCell::default(),
            lost_comment: Cell::default(),
            format_failure: Cell::default(),
            whole_file: Cell::default(),
            macro_args: RefCell::default(),
            memo: RefCell::default(),
            blocks: RefCell::default(),
            line_starts: RefCell::default(),
            arena,
            next_tree_id: Cell::new(1),
            memo_depth: Cell::new(0),
            memo_checking: Cell::new(false),
        }
    }

    /// The id for the next tree built in this run: 1, 2, ... (the file's tree is 0).
    pub(crate) fn next_tree_id(&self) -> u32 {
        let id = self.next_tree_id.get();
        self.next_tree_id.set(id + 1);
        id
    }

    /// Keeps `tree` for the rest of the run and returns it by reference.
    pub(crate) fn keep_tree(&self, tree: Tree<'a>) -> &'a Tree<'a> {
        self.arena.alloc(tree)
    }

    /// Drops the memoized rewrites (see [`RewriteContext::memoize`]); only their memory is
    /// lost, as every later rewrite can be computed again.
    pub(crate) fn clear_memo(&self) {
        self.memo.borrow_mut().clear();
        self.blocks.borrow_mut().clear();
    }

    /// [`Block::from_block_expr`], computed once per block expression in a run: a block is
    /// rewritten several times (each attempt at a layout of the expression around it), and
    /// its statements are the same each time. The cache holds the block without handles
    /// ([`CachedBlock`]) and rebuilds it over the caller's tree, so it carries no lifetime.
    pub(crate) fn block_of<'t>(&self, block: &ast::BlockExpr<'t>) -> Option<Block<'t>> {
        let node = block.syntax();
        let key = (node.tree_id(), node.index());
        if let Some(cached) = self.blocks.borrow().get(&key) {
            return Some(Block::from_cached(cached, node.tree()));
        }
        let built = Block::from_block_expr(block)?;
        self.blocks.borrow_mut().insert(key, built.to_cached());
        Some(built)
    }

    /// 1-based line number of `pos` in `src`, the source this run formats.
    pub(crate) fn line_of(&self, src: &str, pos: BytePos) -> usize {
        let mut line_starts = self.line_starts.borrow_mut();
        let (starts, scanned) = &mut *line_starts;
        if starts.is_empty() {
            starts.push(0);
        }
        // Every line start at or before `pos` follows a `\n` before `pos`, so the count below
        // needs the source scanned only up to `pos`.
        let end = (pos as usize).min(src.len());
        if *scanned < end {
            let bytes = &src.as_bytes()[..end];
            let mut i = *scanned;
            loop {
                i = find_any_byte(bytes, i, *b"\n");
                if i == end {
                    break;
                }
                starts.push(i as BytePos + 1);
                i += 1;
            }
            *scanned = end;
        }
        starts.partition_point(|&start| start <= pos)
    }
}

/// Rewrites a syntax element into a string that fits `shape`, or `None` if it cannot.
pub(crate) trait Rewrite {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String>;
}

impl<T: Rewrite> Rewrite for Box<T> {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        (**self).rewrite(context, shape)
    }
}

#[derive(Clone)]
pub(crate) struct RewriteContext<'a> {
    pub(crate) config: &'a Settings,
    pub(crate) snippet_provider: SnippetProvider<'a>,
    pub(crate) inside_macro: Rc<Cell<bool>>,
    /// When `is_if_else_block` is true, unindent the comment on top
    /// of the `else` or `else if`.
    pub(crate) is_if_else_block: Cell<bool>,
    /// When `is_loop_block` is true, we can more aggressively end the
    /// last statement of the block with a semicolon.
    pub(crate) is_loop_block: Cell<bool>,
    /// When rewriting chain, veto going multi line except the last element
    pub(crate) force_one_line_chain: Cell<bool>,
    /// Used for `format_snippet`
    pub(crate) macro_rewrite_failure: Rc<Cell<bool>>,
    pub(crate) is_macro_def: bool,
    /// Macro and attribute names left as written (rustfmt's `skip_context`); `None` when
    /// there are none, which is every context outside an item with a
    /// `#[rustfmt::skip::macros(..)]` or `#[rustfmt::skip::attributes(..)]` attribute.
    pub(crate) skip_context: Option<Rc<SkipContext>>,
    pub(crate) run: Rc<RunState<'a>>,
}

pub(crate) struct InsideMacroGuard {
    is_nested_macro_context: bool,
    inside_macro_ref: Rc<Cell<bool>>,
}

impl InsideMacroGuard {
    pub(crate) fn is_nested(&self) -> bool {
        self.is_nested_macro_context
    }
}

impl Drop for InsideMacroGuard {
    fn drop(&mut self) {
        self.inside_macro_ref.replace(self.is_nested_macro_context);
    }
}

impl<'a> RewriteContext<'a> {
    pub(crate) fn new(
        config: &'a Settings,
        snippet_provider: SnippetProvider<'a>,
        run: Rc<RunState<'a>>,
    ) -> Self {
        RewriteContext {
            config,
            snippet_provider,
            inside_macro: Rc::new(Cell::new(false)),
            is_if_else_block: Cell::new(false),
            is_loop_block: Cell::new(false),
            force_one_line_chain: Cell::new(false),
            macro_rewrite_failure: Rc::new(Cell::new(false)),
            is_macro_def: false,
            skip_context: None,
            run,
        }
    }

    /// Whether `#[rustfmt::skip::macros(..)]` lists `name` here.
    pub(crate) fn skips_macro(&self, name: &str) -> bool {
        self.skip_context
            .as_ref()
            .is_some_and(|s| s.macros.skip(name))
    }

    /// Whether `#[rustfmt::skip::attributes(..)]` lists `name` here.
    pub(crate) fn skips_attribute(&self, name: &str) -> bool {
        self.skip_context
            .as_ref()
            .is_some_and(|s| s.attributes.skip(name))
    }

    pub(crate) fn snippet(&self, span: Span) -> &'a str {
        self.snippet_provider.snippet(span)
    }

    /// Records the lines of `span` as left unformatted.
    pub(crate) fn add_skipped_span(&self, span: Span) {
        let src = self.snippet_provider.entire_snippet();
        let range = (
            self.run.line_of(src, span.lo()),
            self.run.line_of(src, span.hi()),
        );
        self.run.skipped_range.borrow_mut().push(range);
    }

    /// Always `true`: `indent_style` is an unstable rustfmt option and chloro implements
    /// only its default, `Block`.
    pub(crate) fn use_block_indent(&self) -> bool {
        true
    }

    pub(crate) fn budget(&self, used_width: usize) -> usize {
        self.config.max_width().saturating_sub(used_width)
    }

    pub(crate) fn inside_macro(&self) -> bool {
        self.inside_macro.get()
    }

    pub(crate) fn enter_macro(&self) -> InsideMacroGuard {
        let is_nested_macro_context = self.inside_macro.replace(true);
        InsideMacroGuard {
            is_nested_macro_context,
            inside_macro_ref: self.inside_macro.clone(),
        }
    }

    /// Whether a candidate rewrite whose text cannot change the result may be left
    /// uncomputed. Rendering it can still set flags on the run (`macro_rewrite_failure`,
    /// `lost_comment`, `skipped_range`, and `format_failure` under
    /// `rustfmt_version = "1.8.0"`); those reach the output only in a snippet run or under
    /// 1.8.0, so the skip is allowed only in a whole-file run under 1.9.0.
    pub(crate) fn may_skip_dead_candidates(&self) -> bool {
        self.run.whole_file.get() && self.config.rustfmt_version() == RustfmtVersion::V1_9_0
    }

    pub(crate) fn leave_macro(&self) {
        self.inside_macro.replace(false);
    }

    pub(crate) fn is_if_else_block(&self) -> bool {
        self.is_if_else_block.get()
    }

    pub(crate) fn is_loop_block(&self) -> bool {
        self.is_loop_block.get()
    }

    fn flags(&self) -> u8 {
        u8::from(self.inside_macro.get())
            | u8::from(self.is_if_else_block.get()) << 1
            | u8::from(self.is_loop_block.get()) << 2
            | u8::from(self.force_one_line_chain.get()) << 3
            | u8::from(self.is_macro_def) << 4
    }

    /// Runs `rewrite` for `node`, or returns its result from an earlier call with the same
    /// node, `tag`, shape and context flags.
    ///
    /// rustfmt tries several layouts for most constructs and rewrites every subexpression
    /// once per attempt, so the work grows exponentially with nesting. A rewrite is a
    /// function of the node, the shape and the context flags, apart from three effects on
    /// the run, which are only written while formatting and read once it is done: the
    /// macro failure flag, the lost comment flag and the skipped line ranges. These are
    /// recorded with the result and replayed on a hit, so memoization changes no output.
    /// A rewrite that leaves a context flag changed is not stored.
    ///
    /// Rewrites nested less than [`MEMO_MIN_DEPTH`] deep are neither looked up nor stored.
    /// Which rewrites are stored changes no output, as a stored result is the one the
    /// rewrite returns for its key; the levels above the threshold are rewritten once per
    /// attempt of the levels around them, a factor that does not grow with the nesting below.
    pub(crate) fn memoize(
        &self,
        node: Node<'_>,
        tag: u8,
        shape: Shape,
        rewrite: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        let depth = self.run.memo_depth.get();
        if depth < MEMO_MIN_DEPTH {
            self.run.memo_depth.set(depth + 1);
            let result = rewrite();
            self.run.memo_depth.set(depth);
            return result;
        }
        let key = MemoKey {
            tree: node.tree_id(),
            index: node.index(),
            tag,
            shape,
            flags: self.flags(),
        };
        if let Some(entry) = self.run.memo.borrow().get(&key) {
            if entry.macro_failure {
                self.macro_rewrite_failure.set(true);
            }
            if entry.lost_comment {
                self.run.lost_comment.set(true);
            }
            self.run
                .skipped_range
                .borrow_mut()
                .extend_from_slice(&entry.skipped_range);
            if cfg!(debug_assertions) && !self.run.memo_checking.get() {
                // The check only reads the table, so the borrow may stay.
                self.check_memo_hit(depth, entry, rewrite);
            }
            return entry.result.clone();
        }

        let failure_before = self.macro_rewrite_failure.replace(false);
        let lost_before = self.run.lost_comment.replace(false);
        let skipped_before = self.run.skipped_range.borrow().len();
        self.run.memo_depth.set(depth + 1);
        let result = rewrite();
        self.run.memo_depth.set(depth);
        let macro_failure = self.macro_rewrite_failure.get();
        let lost_comment = self.run.lost_comment.get();
        self.macro_rewrite_failure
            .set(failure_before || macro_failure);
        self.run.lost_comment.set(lost_before || lost_comment);
        // A debug build's hit check rewrites without storing (`check_memo_hit`).
        if self.flags() == key.flags && !(cfg!(debug_assertions) && self.run.memo_checking.get()) {
            let skipped_range = self.run.skipped_range.borrow()[skipped_before..].to_vec();
            self.run.memo.borrow_mut().insert(
                key,
                MemoEntry {
                    result: result.clone(),
                    macro_failure,
                    lost_comment,
                    skipped_range,
                },
            );
        }
        result
    }

    /// Debug builds: a memo hit equals the rewrite computed again, result and effects. The
    /// check rewrites with the memo read but not written and checks no hit inside it, so it
    /// costs one rewrite per hit over memoized subexpressions; the run's effects are
    /// restored afterwards.
    #[cold]
    fn check_memo_hit(
        &self,
        depth: u32,
        entry: &MemoEntry,
        rewrite: impl FnOnce() -> Option<String>,
    ) {
        let failure_before = self.macro_rewrite_failure.replace(false);
        let lost_before = self.run.lost_comment.replace(false);
        let skipped_before = self.run.skipped_range.borrow().len();
        self.run.memo_checking.set(true);
        self.run.memo_depth.set(depth + 1);
        let result = rewrite();
        self.run.memo_depth.set(depth);
        self.run.memo_checking.set(false);
        let macro_failure = self.macro_rewrite_failure.replace(failure_before);
        let lost_comment = self.run.lost_comment.replace(lost_before);
        let skipped: Vec<_> = self
            .run
            .skipped_range
            .borrow_mut()
            .drain(skipped_before..)
            .collect();
        assert!(
            result == entry.result
                && macro_failure == entry.macro_failure
                && lost_comment == entry.lost_comment
                && skipped == entry.skipped_range,
            "a memo hit differs from the rewrite: {:?} against {result:?}",
            entry.result
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::Tree;
    use crate::formatter::nodes::{self, StmtKind};
    use ra_ap_parser::Edition;

    #[test]
    fn line_of_counts_the_lines_up_to_the_position() {
        let src = "use a;\n\nuse b;\r\nfn f() {}\n\n";
        let len = src.len() as BytePos;
        // Positions out of order, repeated, and past the end of the source.
        let positions: Vec<BytePos> = (0..=len + 2)
            .rev()
            .chain(0..=len + 2)
            .chain([7, 3, 20, 0, len, 9])
            .collect();
        let arena = TreeArena::new();
        let run = RunState::new(&arena);
        for pos in positions {
            let expected = src.as_bytes()[..(pos as usize).min(src.len())]
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
                + 1;
            assert_eq!(run.line_of(src, pos), expected, "{pos}");
        }
        // A run whose first query is in the middle of the source.
        let arena = TreeArena::new();
        let run = RunState::new(&arena);
        for pos in [12, 0, 5, 30, 8] {
            let expected = src.as_bytes()[..(pos as usize).min(src.len())]
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
                + 1;
            assert_eq!(run.line_of(src, pos), expected, "{pos}");
        }
    }

    /// A rewrite's result and effects: macro failure, lost comment, skipped ranges.
    type Answer = (Option<String>, bool, bool, Vec<(usize, usize)>);

    /// The memo for the test below: a function of the node, tag, shape and context flags
    /// that rewrites later nodes first, some at a narrower or offset shape and some with a
    /// context flag set (as layout attempts do), and sets the run's effects for some keys.
    struct Workload<'n, 't> {
        nodes: &'n [Node<'t>],
        /// Through `memoize` (the memo under test) or through `reference`.
        memoized: bool,
        /// A memo that stores every rewrite, with its effects.
        reference: RefCell<HashMap<MemoKey, Answer>>,
        /// Rewrites computed by `memoize` at a depth it memoizes, per key since the last clear.
        computed: RefCell<HashMap<MemoKey, u32>>,
        /// Rewrites asked of `memoize`, and computed below the depth it memoizes.
        lookups: Cell<u32>,
        shallow: Cell<u32>,
    }

    impl Workload<'_, '_> {
        fn rewrite(
            &self,
            context: &RewriteContext<'_>,
            i: usize,
            tag: u8,
            shape: Shape,
        ) -> Option<String> {
            let node = self.nodes[i];
            let key = MemoKey {
                tree: node.tree_id(),
                index: node.index(),
                tag,
                shape,
                flags: context.flags(),
            };
            if self.memoized {
                let depth = context.run.memo_depth.get();
                self.lookups.set(self.lookups.get() + 1);
                return context.memoize(node, tag, shape, || {
                    if depth < MEMO_MIN_DEPTH {
                        self.shallow.set(self.shallow.get() + 1);
                    } else if !context.run.memo_checking.get() {
                        let mut computed = self.computed.borrow_mut();
                        let n = computed.entry(key).or_default();
                        *n += 1;
                        assert_eq!(*n, 1, "{i} computed again at depth {depth}");
                    }
                    self.body(context, i, tag, shape)
                });
            }
            let hit = self.reference.borrow().get(&key).cloned();
            if let Some((result, failure, lost, skipped)) = hit {
                if failure {
                    context.macro_rewrite_failure.set(true);
                }
                if lost {
                    context.run.lost_comment.set(true);
                }
                context.run.skipped_range.borrow_mut().extend(skipped);
                return result;
            }
            let failure_before = context.macro_rewrite_failure.replace(false);
            let lost_before = context.run.lost_comment.replace(false);
            let skipped_before = context.run.skipped_range.borrow().len();
            let result = self.body(context, i, tag, shape);
            let failure = context.macro_rewrite_failure.get();
            let lost = context.run.lost_comment.get();
            context.macro_rewrite_failure.set(failure_before || failure);
            context.run.lost_comment.set(lost_before || lost);
            let skipped = context.run.skipped_range.borrow()[skipped_before..].to_vec();
            self.reference
                .borrow_mut()
                .insert(key, (result.clone(), failure, lost, skipped));
            result
        }

        fn body(
            &self,
            context: &RewriteContext<'_>,
            i: usize,
            tag: u8,
            shape: Shape,
        ) -> Option<String> {
            let mut h = FxHasher::default();
            h.write_usize(i);
            h.write_u8(tag);
            h.write_usize(shape.width);
            h.write_usize(shape.offset);
            h.write_u8(context.flags());
            let h = h.finish() >> 7;
            let mut out = format!(
                "{i}.{tag}.{}.{}.{}",
                shape.width,
                shape.offset,
                context.flags()
            );
            if h % 7 == 0 {
                context.macro_rewrite_failure.set(true);
            }
            if h % 11 == 0 {
                context.run.lost_comment.set(true);
            }
            if h % 5 == 0 {
                context
                    .run
                    .skipped_range
                    .borrow_mut()
                    .push((i, (h % 9) as usize));
            }
            for k in 0..(h % 4) {
                let bits = h >> (8 * k + 3);
                let j = i + 1 + (bits % 4) as usize;
                if j >= self.nodes.len() {
                    break;
                }
                let child = Shape {
                    width: shape.width.saturating_sub(((bits >> 2) % 3) as usize),
                    offset: shape.offset + ((bits >> 4) % 2) as usize,
                    ..shape
                };
                let result = if (bits >> 5) % 5 == 0 {
                    let saved = context.is_if_else_block.replace(true);
                    let result = self.rewrite(context, j, tag, child);
                    context.is_if_else_block.set(saved);
                    result
                } else {
                    self.rewrite(context, j, (bits >> 7) as u8 % 2, child)
                };
                // A digest of the child's result keeps the strings short.
                match result {
                    Some(r) => {
                        let mut d = FxHasher::default();
                        d.write(r.as_bytes());
                        out.push_str(&format!("({:x})", d.finish() >> 48));
                    }
                    None if h % 3 == 0 => return None,
                    None => out.push('-'),
                }
            }
            Some(out)
        }
    }

    /// `memoize` answers every rewrite as a memo that stores every rewrite does, result and
    /// replayed effects (macro failure, lost comment, skipped ranges), on random nests of
    /// rewrites over a tree's nodes with clears in between; it never computes a key twice
    /// between clears at a depth it memoizes, and leaves the depth as it found it.
    #[test]
    fn memoize_answers_as_a_memo_storing_every_rewrite() {
        use crate::formatter::config::Settings;
        use crate::formatter::shape::Indent;
        use crate::formatter::span::SnippetProvider;
        let src =
            "fn f() { a(b(c, d), e.f(g, [h, i]), (j, k)); if l { m(n, o) } else { p.q().r(s) } }\n";
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let nodes: Vec<Node> = tree.root().descendants().collect();
        assert!(nodes.len() > 60, "{}", nodes.len());
        let settings = Settings::default();
        let arena = TreeArena::new();
        let run = Rc::new(RunState::new(&arena));
        let context = RewriteContext::new(&settings, SnippetProvider::new(src), run.clone());
        let workload = |memoized| Workload {
            nodes: &nodes,
            memoized,
            reference: RefCell::default(),
            computed: RefCell::default(),
            lookups: Cell::new(0),
            shallow: Cell::new(0),
        };
        let (memo, reference) = (workload(true), workload(false));
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        let mut computed = 0;
        for step in 0..200 {
            if next(20) == 0 {
                run.clear_memo();
                reference.reference.borrow_mut().clear();
                computed += memo.computed.borrow().values().sum::<u32>();
                memo.computed.borrow_mut().clear();
            }
            let i = next(nodes.len() as u64) as usize;
            let tag = next(2) as u8;
            let shape = Shape {
                width: 10 + next(4) as usize,
                indent: Indent::new(4 * next(2) as usize, 0),
                offset: next(3) as usize,
            };
            let mut answers: Vec<Answer> = Vec::new();
            for w in [&memo, &reference] {
                context.macro_rewrite_failure.set(false);
                run.lost_comment.set(false);
                run.skipped_range.borrow_mut().clear();
                let result = w.rewrite(&context, i, tag, shape);
                answers.push((
                    result,
                    context.macro_rewrite_failure.get(),
                    run.lost_comment.get(),
                    run.skipped_range.borrow().clone(),
                ));
                assert_eq!(run.memo_depth.get(), 0);
            }
            assert_eq!(answers[0], answers[1], "step {step}");
        }
        computed += memo.computed.borrow().values().sum::<u32>();
        let (lookups, shallow) = (memo.lookups.get(), memo.shallow.get());
        // Rewrites were computed above and below the threshold, and found in the memo.
        let hits = lookups - computed - shallow;
        assert!(
            computed > 1_000 && shallow > 500 && hits > 1_000,
            "{lookups} rewrites: {computed} computed at memoized depths, {shallow} above, {hits} found"
        );
    }

    /// `block_of` answers what `Block::from_block_expr` answers on every block of a source
    /// with nested, empty, `unsafe` and labeled blocks, and a second call returns the same
    /// block without building it again.
    #[test]
    fn block_of_matches_from_block_expr_and_is_cached() {
        let src = "fn f() { let a = { 1 }; unsafe { a; } 'l: { break 'l; } if a {} else { b } }\n";
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let arena = TreeArena::new();
        let run = RunState::new(&arena);
        type Key<'t> = (
            Vec<(Span, Option<Node<'t>>, bool)>,
            Span,
            nodes::BlockRules,
            Option<ast::StmtList<'t>>,
        );
        fn key<'t>(b: &Block<'t>) -> Key<'t> {
            let stmts: Vec<_> = b
                .stmts
                .iter()
                .map(|s| {
                    let node = match &s.kind {
                        StmtKind::Let(l) => Some(l.syntax()),
                        StmtKind::Item(i) => Some(i.syntax()),
                        StmtKind::Expr(e) | StmtKind::Semi(e) => Some(e.syntax()),
                        StmtKind::MacCall(m) => Some(m.syntax()),
                        StmtKind::Empty => None,
                    };
                    (s.span, node, matches!(s.kind, StmtKind::Semi(_)))
                })
                .collect();
            (stmts, b.span, b.rules, b.stmt_list)
        }
        let mut blocks = 0;
        for b in tree.root().descendants().filter_map(ast::BlockExpr::cast) {
            let fresh = Block::from_block_expr(&b).unwrap();
            assert_eq!(
                key(&run.block_of(&b).unwrap()),
                key(&fresh),
                "{}",
                b.syntax()
            );
            assert_eq!(
                key(&run.block_of(&b).unwrap()),
                key(&fresh),
                "{}",
                b.syntax()
            );
            blocks += 1;
        }
        assert_eq!(run.blocks.borrow().len(), blocks);
        assert_eq!(blocks, 6);
        run.clear_memo();
        assert!(run.blocks.borrow().is_empty());
    }
}
