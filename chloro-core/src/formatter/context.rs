//! The rewrite context and the `Rewrite` trait (rustfmt's `rewrite.rs`).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

use ra_ap_syntax::{NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode};
use rowan::Language;

use super::config::Settings;
use super::macro_args::ParsedMacroArgs;
use super::shape::Shape;
use super::span::{BytePos, SnippetProvider, Span};

/// Parsed macro arguments keyed by token tree range and the `vec!` flag. rustfmt rewrites
/// a macro call once per candidate shape; the arguments are parsed once.
pub(crate) type MacroArgsCache =
    HashMap<(BytePos, BytePos, bool), Option<ParsedMacroArgs>, FxBuildHasher>;

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

/// What a memoized rewrite reads besides its node: the shape and the context flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MemoKey {
    /// Address of the node's green node and its offset: rowan's identity of a node. The
    /// entry keeps the node alive, so the address is not reused while the key is stored.
    green: usize,
    offset: u32,
    /// The rewrite function and its other arguments.
    tag: u8,
    shape: Shape,
    flags: u8,
}

/// A memoized rewrite: its result and the effects it had on the run, replayed on a hit.
struct MemoEntry {
    _node: SyntaxNode,
    result: Option<String>,
    macro_failure: bool,
    lost_comment: bool,
    skipped_range: Vec<(usize, usize)>,
}

/// State shared by every context of one formatting run (rustfmt's `FormatReport` and
/// `skipped_range`).
#[derive(Default)]
pub(crate) struct RunState {
    /// Line ranges (1-based, inclusive) left as in the input: `#[rustfmt::skip]` items and
    /// macro calls that could not be parsed.
    pub(crate) skipped_range: RefCell<Vec<(usize, usize)>>,
    /// Set when a rewrite dropped a comment and fell back to the original text. Only
    /// consulted while formatting a macro definition body.
    pub(crate) lost_comment: Cell<bool>,
    pub(crate) macro_args: RefCell<MacroArgsCache>,
    /// Rewrites already computed in this run; see [`RewriteContext::memoize`].
    memo: RefCell<HashMap<MemoKey, MemoEntry, FxBuildHasher>>,
    /// `padding[k]` is a whitespace token of 2^k spaces; see [`RunState::padding`].
    padding: RefCell<Vec<rowan::GreenToken>>,
    /// Start offsets of the source lines, computed on first use.
    line_starts: OnceCell<Vec<BytePos>>,
}

impl RunState {
    /// Whitespace tokens of `len` bytes in total, shared between calls: at most one token
    /// of each power of two.
    pub(crate) fn padding(
        &self,
        len: usize,
    ) -> Vec<NodeOrToken<rowan::GreenNode, rowan::GreenToken>> {
        let mut pads = self.padding.borrow_mut();
        let mut out = Vec::new();
        let mut k = 0;
        while len >> k != 0 {
            if pads.len() == k {
                let kind = RustLanguage::kind_to_raw(SyntaxKind::WHITESPACE);
                pads.push(rowan::GreenToken::new(kind, &" ".repeat(1 << k)));
            }
            if len & (1 << k) != 0 {
                out.push(NodeOrToken::Token(pads[k].clone()));
            }
            k += 1;
        }
        out
    }

    /// Drops the memoized rewrites (see [`RewriteContext::memoize`]); only their memory is
    /// lost, as every later rewrite can be computed again.
    pub(crate) fn clear_memo(&self) {
        self.memo.borrow_mut().clear();
    }

    /// 1-based line number of `pos` in `src`, the source this run formats.
    pub(crate) fn line_of(&self, src: &str, pos: BytePos) -> usize {
        let starts = self.line_starts.get_or_init(|| {
            std::iter::once(0)
                .chain(
                    src.bytes()
                        .enumerate()
                        .filter(|&(_, b)| b == b'\n')
                        .map(|(i, _)| i as BytePos + 1),
                )
                .collect()
        });
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
    pub(crate) macro_rewrite_failure: Cell<bool>,
    pub(crate) is_macro_def: bool,
    pub(crate) run: Rc<RunState>,
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
        run: Rc<RunState>,
    ) -> Self {
        RewriteContext {
            config,
            snippet_provider,
            inside_macro: Rc::new(Cell::new(false)),
            is_if_else_block: Cell::new(false),
            is_loop_block: Cell::new(false),
            force_one_line_chain: Cell::new(false),
            macro_rewrite_failure: Cell::new(false),
            is_macro_def: false,
            run,
        }
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
    pub(crate) fn memoize(
        &self,
        node: &SyntaxNode,
        tag: u8,
        shape: Shape,
        rewrite: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        let green = node.green();
        let key = MemoKey {
            green: &*green as *const _ as *const u8 as usize,
            offset: node.text_range().start().into(),
            tag,
            shape,
            flags: self.flags(),
        };
        drop(green);
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
            return entry.result.clone();
        }

        let failure_before = self.macro_rewrite_failure.replace(false);
        let lost_before = self.run.lost_comment.replace(false);
        let skipped_before = self.run.skipped_range.borrow().len();
        let result = rewrite();
        let macro_failure = self.macro_rewrite_failure.get();
        let lost_comment = self.run.lost_comment.get();
        self.macro_rewrite_failure
            .set(failure_before || macro_failure);
        self.run.lost_comment.set(lost_before || lost_comment);
        if self.flags() == key.flags {
            let skipped_range = self.run.skipped_range.borrow()[skipped_before..].to_vec();
            self.run.memo.borrow_mut().insert(
                key,
                MemoEntry {
                    _node: node.clone(),
                    result: result.clone(),
                    macro_failure,
                    lost_comment,
                    skipped_range,
                },
            );
        }
        result
    }
}
