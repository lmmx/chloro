//! Formatting the top-level items of one file on several threads.
//!
//! rustfmt formats a file by walking its items in order with one visitor. The output of a
//! top-level item depends on the item's own text and the configuration, not on its
//! neighbours (`docs/journal/2026-10-10-item-independence.md`); what does depend on the
//! neighbours is the text *between* items (comments, blank lines, a skipped item copied
//! as written) and the sorting of a group of `use`, `mod` or `extern crate` items. This
//! module keeps that part sequential and formats the items themselves in parallel:
//!
//! - **Units.** A unit is one top-level item, or one group of consecutive reorderable
//!   items (the group `reorder.rs` sorts together); the units are the sequence
//!   `visit_items_with_reordering` walks.
//! - **Workers.** The file is parsed once into one immutable tree, which the workers share
//!   (`Tree` is `Sync`; a handle is a reference and an index). Each worker takes units
//!   from a shared queue (longest first) and formats each one with a visitor of its own,
//!   starting from the state the sequential visitor would have at that item: `last_pos` at
//!   the unit's start, an empty block indent and a [`RunState`] of its own (memo, macro
//!   argument cache, line table, and an arena for the macro-argument trees it parses).
//! - **The gap before each unit.** The sequential visitor writes the text between the
//!   previous item and this one from inside the item's own code (its first
//!   `format_missing*` call), and that call reads the output written so far (whether it is
//!   empty, how many newlines it ends with). A worker therefore does not write the gap:
//!   its visitor records the first such call as a [`GapOp`] and skips it. The stitching
//!   visitor on the main thread replays the recorded call on the real buffer, appends the
//!   unit's text and moves `last_pos` to where the worker left it.
//! - **Shared state.** `macro_rewrite_failure`, `lost_comment` and `format_failure` are or-ed
//!   together.
//!   The skipped line ranges a worker records for a skipped top-level item are in output
//!   lines of the worker's buffer and are shifted by the line the unit starts at in the
//!   file; ranges in source lines are kept as they are.
//!
//! The library's `format_source` and `format_source_with_config` do not use this module.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::cst::TreeArena;
use crate::cst::ast::{self, AstNode, HasModuleItem};

use super::config::Settings;
use super::context::RunState;
use super::formatting::Formatted;
use super::nodes::{AttrStyle, Attribute, inner_attributes};
use super::shape::Indent;
use super::span::{BytePos, SnippetProvider, Spanned};
use super::unit_cache::{CachedUnit, UnitCache, UnitKey};
use super::visitor::FmtVisitor;

/// The first `format_missing*` call a unit makes, which writes the text between the
/// previous item and this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GapOp {
    /// `format_missing_with_indent(end)`.
    WithIndent(BytePos),
    /// `format_missing(end)`.
    NoIndent(BytePos),
    /// `format_missing_no_indent(end)`.
    WithoutIndent(BytePos),
    /// `format_missing_before_removed_import(lo)`: the gap before a `use` item that
    /// formats to nothing (`imports.rs`).
    RemovedImport(BytePos),
}

/// What a worker's visitor records while formatting one unit.
#[derive(Default)]
pub(crate) struct UnitRecord {
    /// The first gap call, recorded instead of written.
    pub(crate) gap: Option<GapOp>,
    /// Indices into the run's `skipped_range` of the ranges whose end is an output line
    /// number of the worker's buffer (`push_skipped_with_span`).
    pub(crate) output_line_ranges: Vec<usize>,
    /// Text was written before the first gap call, so the record cannot be replayed in
    /// order; `format_file` formats the file sequentially instead.
    pub(crate) broken: bool,
}

/// The output of one unit.
pub(crate) struct UnitOutput {
    pub(crate) gap: Option<GapOp>,
    pub(crate) text: String,
    /// The visitor's `last_pos` after the unit.
    pub(crate) last_pos: BytePos,
    pub(crate) macro_failure: bool,
    pub(crate) lost_comment: bool,
    pub(crate) format_failure: bool,
    pub(crate) skipped_range: Vec<(usize, usize)>,
    pub(crate) output_line_ranges: Vec<usize>,
    pub(crate) broken: bool,
}

/// A unit cache and the context bytes of the file's keys (`unit_cache.rs`).
pub(crate) struct CacheUse<'c> {
    pub(crate) cache: &'c dyn UnitCache,
    /// `unit_cache::key_context` of the configuration; the file's skip names are appended.
    pub(crate) context: Vec<u8>,
}

/// Formats `file` (parsed from `source`) with its top-level items spread over `threads`
/// threads. Equivalent to `format_separate_mod` followed by the trailing-newline fix-up
/// of `format_text`.
///
/// With a unit cache, every file takes the units path (whatever `threads` is): each unit is
/// looked up first, only the units not found are formatted (on the calling thread when
/// `threads <= 1` or a single unit is missing), and each one formatted is stored.
pub(crate) fn format_file(
    source: &str,
    config: &Settings,
    file: &ast::SourceFile<'_>,
    threads: usize,
    cache: Option<CacheUse<'_>>,
) -> Option<Formatted> {
    let arena = TreeArena::new();
    let run = Rc::new(RunState::new(&arena));
    run.whole_file.set(true);
    let snippet_provider = SnippetProvider::new(source);
    let end_pos = snippet_provider.end_pos();
    let mut main = FmtVisitor::new(config, snippet_provider, run.clone());
    main.last_pos = 0;
    main.skip_empty_lines(end_pos);

    // `format_separate_mod`, with the walk over the items replaced.
    main.block_indent = Indent::empty();
    let file_attrs = inner_attributes(file.syntax());
    main.update_skip_context(&file_attrs);
    main.visit_attrs(&file_attrs, AttrStyle::Inner);
    let items: Vec<ast::Item<'_>> = file.items().collect();
    let units = main.unit_ranges(&items);

    let outputs = match cache {
        Some(cache) if !units.is_empty() => {
            let mut context = cache.context;
            // The file's skip names, as every worker starts with them
            // (`update_skip_context(file_attrs)` in `format_units`).
            match main.skip_context.as_deref() {
                Some(skip) => skip.write_key(&mut context),
                None => super::skip::SkipContext::default().write_key(&mut context),
            }
            Some(format_units_cached(
                source,
                config,
                &file_attrs,
                &items,
                &units,
                threads,
                cache.cache,
                &context,
                &|pos| main.line_of(pos),
            ))
        }
        _ if threads <= 1 || units.len() < 2 => None,
        _ => {
            let all: Vec<usize> = (0..units.len()).collect();
            let mut outputs: Vec<Option<UnitOutput>> = (0..units.len()).map(|_| None).collect();
            for (u, out) in format_units(source, config, &file_attrs, &items, &units, &all, threads)
            {
                outputs[u] = Some(out);
            }
            Some(
                outputs
                    .into_iter()
                    .map(|o| o.expect("every unit is formatted"))
                    .collect(),
            )
        }
    };

    match outputs {
        None => main.visit_items_with_reordering(&items),
        Some(outputs) => {
            // Every record is checked before anything is written to `main`: a unit that
            // wrote text before its first gap call cannot be replayed in order, so the file
            // is then formatted by the sequential walk, which gives the same output (`main`
            // holds only what `format_separate_mod` writes before the items). No input
            // tried reaches this; debug builds stop here so that one would be found.
            let ok = stitchable(&items, &units, &outputs);
            debug_assert!(ok, "a unit's output cannot be stitched");
            if ok {
                stitch(&mut main, &run, &outputs);
            } else {
                main.visit_items_with_reordering(&items);
            }
        }
    }
    main.format_missing_with_indent(end_pos);

    let mut text = std::mem::take(&mut main.buffer);
    let macro_failure = main.macro_rewrite_failure;
    drop(main);
    if run.format_failure.get() {
        return None;
    }
    text.push('\n');
    super::formatting::truncate_trailing_newlines(&mut text);
    let skipped_range = run.skipped_range.borrow().clone();
    Some(Formatted {
        text,
        echoed: false,
        macro_failure,
        lost_comment: run.lost_comment.get(),
        skipped_range,
    })
}

/// Writes the unit outputs onto `main`'s buffer in order: each unit's gap call replayed,
/// then its text, as the sequential walk writes them.
fn stitch(main: &mut FmtVisitor<'_>, run: &RunState<'_>, outputs: &[UnitOutput]) {
    for out in outputs {
        let Some(gap) = out.gap else {
            // The unit wrote nothing and consumed nothing (a skipped `mod x;`, checked by
            // `stitchable`): its text is copied with the gap before the next unit.
            continue;
        };
        match gap {
            GapOp::WithIndent(end) => main.format_missing_with_indent(end),
            GapOp::NoIndent(end) => main.format_missing(end),
            GapOp::WithoutIndent(end) => main.format_missing_no_indent(end),
            GapOp::RemovedImport(lo) => main.format_missing_before_removed_import(lo),
        }
        let line_base = main.line_number();
        main.push_str(&out.text);
        main.last_pos = out.last_pos;
        main.macro_rewrite_failure |= out.macro_failure;
        if out.lost_comment {
            run.lost_comment.set(true);
        }
        if out.format_failure {
            run.format_failure.set(true);
        }
        let mut skipped = run.skipped_range.borrow_mut();
        for (i, &(lo, hi)) in out.skipped_range.iter().enumerate() {
            if out.output_line_ranges.contains(&i) {
                skipped.push((lo, hi + line_base));
            } else {
                skipped.push((lo, hi));
            }
        }
    }
}

/// The source range of a unit: from its first item's start (outer attributes included)
/// to its last item's end.
fn unit_span(items: &[ast::Item<'_>], (first, len): (usize, usize)) -> (BytePos, BytePos) {
    (items[first].span().lo(), items[first + len - 1].span().hi())
}

/// Every unit's output, in unit order: from `cache` where it holds the unit, formatted
/// (and stored) otherwise.
#[allow(clippy::too_many_arguments)]
fn format_units_cached(
    source: &str,
    config: &Settings,
    file_attrs: &[Attribute<'_>],
    items: &[ast::Item<'_>],
    units: &[(usize, usize)],
    threads: usize,
    cache: &dyn UnitCache,
    context: &[u8],
    line_of: &dyn Fn(BytePos) -> usize,
) -> Vec<UnitOutput> {
    let mut outputs: Vec<Option<UnitOutput>> = (0..units.len()).map(|_| None).collect();
    let mut missing = Vec::new();
    for (u, &unit) in units.iter().enumerate() {
        let (lo, hi) = unit_span(items, unit);
        let hit = source.get(lo as usize..hi as usize).and_then(|text| {
            cache
                .get(&UnitKey::new(context, text))
                .and_then(|cached| cached.to_output(source, lo, hi, || line_of(lo)))
        });
        match hit {
            Some(out) => outputs[u] = Some(out),
            None => missing.push(u),
        }
    }
    for (u, out) in format_units(source, config, file_attrs, items, units, &missing, threads) {
        let (lo, hi) = unit_span(items, units[u]);
        if let Some(text) = source.get(lo as usize..hi as usize)
            && let Some(cached) = CachedUnit::from_output(&out, source, lo, hi, || line_of(lo))
        {
            cache.insert(&UnitKey::new(context, text), cached);
        }
        outputs[u] = Some(out);
    }
    outputs
        .into_iter()
        .map(|o| o.expect("every unit is looked up or formatted"))
        .collect()
}

/// Formats the units `todo` (indices into `units`) on up to `threads` threads, the calling
/// thread being one of them; on the calling thread alone when `threads <= 1` or there is at
/// most one unit. The outputs are paired with their unit's index, in no particular order.
fn format_units(
    source: &str,
    config: &Settings,
    file_attrs: &[Attribute<'_>],
    items: &[ast::Item<'_>],
    units: &[(usize, usize)],
    todo: &[usize],
    threads: usize,
) -> Vec<(usize, UnitOutput)> {
    // Longest units first, so that the end of the run is not one thread on the largest
    // item while the others idle.
    let mut order: Vec<usize> = todo.to_vec();
    let unit_len = |&(first, len): &(usize, usize)| {
        items[first + len - 1].span().hi() - items[first].span().lo()
    };
    order.sort_by_key(|&u| std::cmp::Reverse(unit_len(&units[u])));
    let next = AtomicUsize::new(0);

    // One worker: a visitor of its own and a `RunState` for all the units it takes.
    let work = || {
        let arena = TreeArena::new();
        let run = Rc::new(RunState::new(&arena));
        run.whole_file.set(true);
        let mut visitor = FmtVisitor::new(config, SnippetProvider::new(source), run.clone());
        // The file's `rustfmt::skip::macros`/`attributes` names, as the sequential
        // visitor has them at a top-level item.
        visitor.update_skip_context(file_attrs);
        // The buffer starts with one newline, as the sequential buffer does after the gap
        // before an item: the unit's text is what follows it.
        visitor.buffer = String::from("\n");
        let mut done = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            let Some(&u) = order.get(i) else { break };
            let (first, len) = units[u];
            let out = format_unit(&mut visitor, &run, &items[first..first + len]);
            done.push((u, out));
        }
        done
    };

    let threads = threads.min(order.len());
    if threads <= 1 {
        return work();
    }
    // The calling thread takes units beside `threads - 1` spawned ones instead of waiting in
    // `join`: one thread fewer to start, and no thread holding a slot while it sleeps
    // (EXP-029).
    std::thread::scope(|s| {
        let handles: Vec<_> = (1..threads).map(|_| s.spawn(work)).collect();
        let mut done = work();
        for h in handles {
            // A worker's panic is re-raised with its own payload, so the message the
            // sequential path would give is the one reported.
            done.extend(
                h.join()
                    .unwrap_or_else(|payload| std::panic::resume_unwind(payload)),
            );
        }
        done
    })
}

/// Formats one unit with `visitor`, in the state the sequential visitor has at a
/// top-level item.
fn format_unit<'a>(
    visitor: &mut FmtVisitor<'a>,
    run: &RunState<'a>,
    unit: &[ast::Item<'a>],
) -> UnitOutput {
    debug_assert_eq!(visitor.buffer, "\n");
    visitor.last_pos = unit[0].span().lo();
    visitor.block_indent = Indent::empty();
    visitor.is_if_else_block = false;
    visitor.is_loop_block = false;
    visitor.reset_line_number();
    visitor.macro_rewrite_failure = false;
    visitor.unit = Some(UnitRecord::default());
    run.lost_comment.set(false);
    run.format_failure.set(false);
    run.skipped_range.borrow_mut().clear();

    visitor.visit_items_with_reordering(unit);

    let record = visitor.unit.take().expect("set above");
    UnitOutput {
        gap: record.gap,
        text: visitor.buffer.split_off(1),
        last_pos: visitor.last_pos,
        macro_failure: visitor.macro_rewrite_failure,
        lost_comment: run.lost_comment.get(),
        format_failure: run.format_failure.get(),
        skipped_range: std::mem::take(&mut *run.skipped_range.borrow_mut()),
        output_line_ranges: record.output_line_ranges,
        broken: record.broken,
    }
}

/// Whether the unit outputs can be stitched onto the main buffer: every unit recorded
/// its first gap call, or wrote nothing and consumed nothing (a skipped `mod x;`, whose
/// text the gap before the next unit copies), and none wrote text before its first gap
/// call (`UnitRecord::broken`).
fn stitchable(items: &[ast::Item<'_>], units: &[(usize, usize)], outputs: &[UnitOutput]) -> bool {
    units.iter().zip(outputs).all(|(unit, out)| {
        !out.broken
            && (out.gap.is_some()
                || (out.text.is_empty() && out.last_pos == items[unit.0].span().lo()))
    })
}
