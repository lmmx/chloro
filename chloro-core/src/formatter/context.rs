//! The rewrite context and the `Rewrite` trait (rustfmt's `rewrite.rs`).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::config::Settings;
use super::macro_args::ParsedMacroArgs;
use super::shape::Shape;
use super::span::{BytePos, SnippetProvider, Span};

/// Parsed macro arguments keyed by token tree range and the `vec!` flag. rustfmt rewrites
/// a macro call once per candidate shape; the arguments are parsed once.
pub(crate) type MacroArgsCache = HashMap<(BytePos, BytePos, bool), Option<ParsedMacroArgs>>;

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
    /// Start offsets of the source lines, computed on first use.
    line_starts: OnceCell<Vec<BytePos>>,
}

impl RunState {
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
}
