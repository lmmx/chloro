# 2026-10-10: First-attempt costs and indentation reuse (X9b, X9c)

Follows [2026-10-10-formatter-work-measurements.md](2026-10-10-formatter-work-measurements.md).
Two experiments from that entry, run in the order agreed:
- **X9b:** the cost of first rewrites by node kind, in both versions, on the same items;
- **X9c:** whether a rewrite result can be reused at another indentation.

Neither changes repository code. Both ran on scratch copies; the patches are in the
appendix:
- `x9b_port.diff` and `x9c.diff` apply to `2093831`;
- `x9b_poc.diff` applies to `c4d74ee`.

Every instrumented build reproduced its uninstrumented outputs:
- the port's builds against the hashes recorded at `bcef0f9`: 0 of 1238 fixtures changed;
- the proof of concept's build against hashes of `c4d74ee`'s own output, recorded for this
  check: 0 of 1238 changed.

Labels as before: **measured**, **read**, **hypothesis**.

## Host change

The container was moved between the previous entry and this one:
- before: kernel `6.18.44-fc-v114`, "Intel(R) Xeon(R) Processor @ 2.80GHz";
- now: the same kernel string, "Intel(R) Xeon(R) Processor @ 2.10GHz".

Wall-clock figures from the two hosts are not comparable. Instruction counts are. Measured
on the new host, `bench -n 5` on the 1238 fixtures:

| build | throughput |
|---|---|
| `c4d74ee` (proof of concept) | 10.22–11.12 MB/s |
| `bcef0f9` (port before performance work) | 3.75 MB/s |
| `9202b42` (port now) | 5.01–5.25 MB/s |

- The ratio between the proof of concept and the port is 2.0–2.1x on this host, against
  2.3–2.5x on the old one.
- The phase-1 and previous-entry wall-clock numbers stay valid for the old host.
- **Hypothesis:** the narrower ratio on the newer core fits the instruction-cache pressure
  hypothesis of the change-of-direction entry, since the port has the larger code
  footprint. Not established.

Formatted against perturbed input on the new host (`bench -n 5`, measured, two passes).
The perturbed copy is the fixtures with every single space doubled.

| build | formatted fixtures | perturbed fixtures |
|---|---|---|
| `c4d74ee` | 10.52–11.12 MB/s | 10.43–10.99 MB/s |
| `9202b42` | 5.10–5.25 MB/s | 4.89–4.94 MB/s |

Doubled spaces are a mild perturbation, and fully unformatted input is not measured.

## X9b: exclusive cost per node kind, both versions

### Method

The same module (`x9b.rs`) goes into both versions:
- **Frames.** It keeps a stack of frames, one per node being formatted. rdtsc cycles
  between two stack events are charged to the frame on top, so a frame's cost excludes its
  child frames.
- **Allocations** made while a frame is on top are charged to it, through the driver's
  global allocator.
- **Port only:** each frame is also marked *repeat* when the node was rewritten before in
  the file (as in X9). Repeat marking is inherited by nested frames.

Frames are opened at:
- **Port:**
  - `format_expr`'s uncached rewrites;
  - `Rewrite for ast::Pat` and `Rewrite for ast::Type`;
  - `FmtVisitor::visit_item` and `visit_assoc_item`;
  - `visit_stmt`, as a pseudo kind `(statement)`;
  - `formatting::parse`, as `(parse)`.
- **Proof of concept:**
  - `format_node`;
  - `try_format_expr_inner`;
  - `SourceFile::parse`, as `(parse)`.
- Work outside any frame is charged to `(outside any node frame)`. The port's seen-set
  bookkeeping runs under `(instrumentation)`.

### Results

The corpus is the 7262-item agreement corpus of the change-of-direction entry: perturbed
items on which both versions give rustfmt's output. Five rounds; figures are per round
(measured).

| kind | PoC frames | PoC Mcycles | port first frames | port first Mcycles | port repeat Mcycles | cycles per frame, PoC | cycles per frame, port first |
|---|---|---|---|---|---|---|---|
| `(parse)` | 7262 | 430.1 | 7262 | 382.1 | | 59,231 | 52,623 |
| `(outside any node frame)` | 7262 | 133.6 | 7262 | 150.7 | | 18,395 | 20,747 |
| `MACRO_EXPR` | 182 | 0.63 | 1682 | 83.4 | 0.18 | 3,439 | 49,559 |
| `FN` | 2500 | 59.6 | 2500 | 45.3 | | 23,826 | 18,109 |
| `CALL_EXPR` | 501 | 2.89 | 3029 | 26.4 | 0.15 | 5,762 | 8,706 |
| `LITERAL` | 760 | 0.38 | 5385 | 21.6 | 0.13 | 503 | 4,014 |
| `ARRAY_EXPR` | 3 | 0.01 | 1555 | 17.3 | 0.27 | 1,855 | 11,128 |
| `STRUCT` | 417 | 8.86 | 417 | 11.6 | | 21,252 | 27,851 |
| `(statement)` | | | 3119 | 10.9 | | | 3,492 |
| `PATH_TYPE` | 0 | 0 | 2743 | 6.21 | 0.14 | | 2,262 |
| `IMPL` | 523 | 5.20 | 523 | 5.47 | | 9,951 | 10,468 |
| `ENUM` | 109 | 2.54 | 109 | 5.26 | | 23,260 | 48,281 |
| `PATH_EXPR` | 776 | 0.95 | 3509 | 3.50 | 0.20 | 1,223 | 996 |
| `(instrumentation)` | | | 18667 | 3.11 | 0.04 | | 166 |
| `TYPE_ALIAS` | 141 | 1.74 | 141 | 1.54 | | 12,368 | 10,948 |
| `MODULE` | 514 | 1.54 | 7 | 1.02 | | 2,995 | 146,174 |
| `USE` | 3466 | 66.4 | 0 | | | 19,146 | |
| `SOURCE_FILE` | 7262 | 9.96 | 0 | | | 1,371 | |

Totals per round:

| | cycles | allocations |
|---|---|---|
| proof of concept | 726.9 M | 1,424,603 |
| port | 783.0 M | 1,568,969 |

Cycles excluding `(parse)` are 296.8 M against 400.9 M (1.35x).

The `fn` items alone (2051 items, 1.23 MB): 256.8 M against 334.8 M cycles per round;
excluding parse, 96.4 M against 218.7 M (2.27x). The change-of-direction entry measured
2.3x in instructions on the same items.

### Findings (measured)

1. **The agreement corpus does not hold the work equal.** On the same items, the port
   formats many more nodes than the proof of concept:

   | node kind | port | proof of concept |
   |---|---|---|
   | `MACRO_EXPR` | 1682 | 182 |
   | `CALL_EXPR` | 3029 | 501 |
   | `LITERAL` | 5385 | 760 |
   | `PATH_TYPE` | 2743 | 0 |
   | `ARRAY_EXPR` | 1555 | 3 |

   The proof of concept copies statements, types and macro calls as written. On these items
   the copy is rustfmt's output, because the perturbation only doubles existing single
   spaces: code containing no single space between tokens, such as
   `check(r#"..."#);`, stays unperturbed. The change-of-direction entry's reading of the
   2.3–2.4x gap as pure overhead was wrong, and that entry now carries a correction.
2. **Per node of the same kind, the port is not uniformly more expensive.** Per first-attempt
   frame, against the proof of concept:

   | node kind | ratio |
   |---|---|
   | `PATH_EXPR` | 0.81x |
   | `TYPE_ALIAS` | 0.89x |
   | `IMPL` | 1.05x |
   | `STRUCT` | 1.31x |
   | `CALL_EXPR` | 1.51x |
   | `ENUM` | 2.08x |

   Outliers:
   - `LITERAL` at 8x: `rewrite_literal` measures multi-line literals line by line (X1);
   - `ARRAY_EXPR` at 6x, on 3 proof-of-concept frames only;
   - `MACRO_EXPR`, which the proof of concept copies, while the port re-parses and formats
     the arguments at 49,559 cycles per call.
3. **Item and top-level glue costs about the same.** The port's `FN` frame costs less than
   the proof of concept's: the proof of concept's `FN` frame includes body glue that the
   port charges to `(statement)` frames. `use` and `mod` handling sits in the port's
   `(outside any node frame)` (imports go through `reorder.rs`, which opens no frame). The
   port's outside-frame total (150.7 M) is below the proof of concept's outside, `USE`,
   `MODULE` and `SOURCE_FILE` frames combined (211.4 M).
4. **Parsing.** The proof of concept's `SourceFile::parse` costs 13% more cycles per round
   than the port's `formatting::parse` on the whole corpus, and 38% more on the `fn` items.
   `SourceFile::parse` does not run rust-analyzer's validation (read,
   `ra_ap_syntax-0.0.307/src/lib.rs:229-236`). The difference is not explained here.
5. **Repeats are small on this corpus.** Repeat frames total under 1.2 M cycles of the port's
   783 M. The agreement corpus has almost no layout search, so it cannot show the repeat
   cost X9 measured on whole files.

### Limits

- **Overhead.** rdtsc and the frame stack add a fixed cost to every frame in both versions,
  which narrows ratios. The seen-set bookkeeping is 0.4% of the port's cycles; the
  per-frame overhead is not measured.
- **Frame boundaries differ.** The proof of concept's `FN` frame and the port's `FN` plus
  `(statement)` frames do not cover the same work, so item rows compare only roughly.
- **Corpus.** The agreement corpus is test-heavy and simple. A per-kind comparison on
  layout-heavy code still needs a corpus on which the proof of concept formats every
  node, and no such corpus exists.

## X9c: reuse at another indentation

### Method (`x9c.diff`)

- **When a candidate exists.** On every uncached `format_expr` rewrite of a node that was
  rewritten before in the file with the same expression type and context flags, a
  candidate is built from the latest earlier result.
- **Building the candidate.** Lines after the first are shifted by the difference between
  the two shapes' indentation.
- **When it is attempted.** All of these must hold:
  - the earlier rewrite had no side effects (macro failure, lost comment, skipped range);
  - the shift is possible (enough leading spaces to remove);
  - the candidate passes `filtered_str_fits` for the new shape, the check rustfmt's
    `wrap_str` applies.
- **Comparison.** In shadow mode, the fresh rewrite is still computed and returned. Every
  attempted candidate is compared with it.
- **Counts.** Attempts, agreements, disagreements and each fallback reason are counted per
  node kind.
- **Saved cycles.** The cycles of fresh rewrites whose candidate agreed are summed as a
  union of intervals, so nested reuse is not counted twice.

Three candidate rules were tested in turn (measured on the fixtures):

| rule | attempted | disagreed |
|---|---|---|
| fits the new shape | 113,140 | 9,670 (8.5%) |
| fits, and the earlier shape's first-line width is at least the new one's | 74,065 | 1,392 (1.9%) |
| fits, and the new shape is dominated (first-line width no larger, indent no smaller) | 67,934 | 1,286 (1.9%) |
| as above, and the node contains no multi-line string literal | 67,052 | 414 (0.62%) |

The last rule on the registry (5659 files): 142,295 attempted, 138 disagreed (0.10%).

Disagreements by kind under the last rule:

| node kind | fixtures | registry |
|---|---|---|
| `METHOD_CALL_EXPR` | 310 | 87 |
| `TRY_EXPR` | 32 | 9 |
| `BLOCK_EXPR` | 27 | 13 |
| `CLOSURE_EXPR` | 14 | 2 |
| `FIELD_EXPR` | 7 | 0 |
| `BIN_EXPR` | 6 | 1 |
| `MATCH_EXPR` | 6 of 968 | 1 of 483 |
| `CALL_EXPR` | 3 | 7 |
| `REF_EXPR` | 1 | 5 |
| `MACRO_EXPR` | 0 | 1 |
| `ARRAY_EXPR`, `RECORD_EXPR`, `TUPLE_EXPR` | 0 | 0 |

Cycles of fresh rewrites whose candidate agreed, under the last rule (union):

| corpus | agreed | disagreed |
|---|---|---|
| fixtures | 8.7% of `format_source` | 0.3% |
| registry | 2.2% | 0.0% |

Without the dominance condition, agreeing rewrites cover 12.6% of `format_source` on the
fixtures and 3.8% on the registry, but with 18–22% disagreement.

### Findings (measured)

1. **Fitting is not a correctness condition.** rustfmt takes the first layout that fits. A
   result laid out over several lines in a narrow shape is not rustfmt's choice in a wider
   shape, where a more compact layout fits (sample: `Some(\n current_section_idx,\n)`
   against `Some(current_section_idx)`).
2. **The width of the first line is not enough.** Later lines are budgeted by
   `max_width` minus their indentation. Moving a result left gives its inner lines more
   room even when the first line has less (sample: a struct literal field that fits on one
   line at indent 4 but not at indent 28).
3. **Shifting changes multi-line string literals.** Raw strings and `\` continuations move
   with the shift, so their content changes.
4. **Chains are not monotonic in the width given.** Under a dominated (tighter) shape, rustfmt
   chose a more compact layout than under the wider one: it overflowed the last call's
   closure argument instead of breaking the chain vertically (sample in the patch output,
   `METHOD_CALL_EXPR`). No rule based on width alone makes chain reuse exact. Most of the
   remaining disagreements are on chains, or on nodes that contain chains (`TRY_EXPR`,
   `BLOCK_EXPR`, `CLOSURE_EXPR`).
5. **No rule tested is exact.** Applying the last rule would change the output for 414
   fixture rewrites and 138 registry rewrites. The reuse mode (`X9C_REUSE=1`) was
   therefore not run for output, conformance or speed.
6. **The reusable share is smaller than X9's 10.4%** "differ only in indentation" share.
   Under the last rule, at most 8.7% (fixtures) and 2.2% (registry) of `format_source`
   would be saved, before the cost of building and checking candidates and before
   excluding the kinds that disagree.

Hypotheses (not established):
- A rule restricted to kinds without chains in their subtree could be exact on these
  corpora. The kinds with no disagreement (`ARRAY_EXPR`, `RECORD_EXPR`, `TUPLE_EXPR`) are
  not the kinds that carry the repeat cost (`match`, chains, closures).
- `match` has 7 disagreements in 1451 attempts. Whether those come from chains inside arms
  was not checked.

## What these results change

- **Indentation reuse:** the case for it is weak. It is not exact under any tested rule,
  and its ceiling on the registry is 2.2%.
- **First attempts:** the port's per-node first-attempt costs are within 0.8–2.1x of the
  proof of concept's for comparable kinds, with outliers for literal measuring and macro
  arguments.
- **The remaining gap:** most of the gap the agreement corpus showed comes from nodes the
  proof of concept never formats. That is not overhead of the port.
- **What the proof of concept's speed rests on:** copying text that is already in its
  final form. A conformant formatter can only do that once it knows the text is already
  canonical. Whether that can be decided more cheaply than by formatting is an open
  question (below). It is not tested.

## Open questions

- **Recognising canonical text.** Can a node's source text be recognised as rustfmt's output
  more cheaply than by formatting it? On already-formatted input — the common case for a
  formatter run in CI or on save — that would let a conformant formatter copy, as the
  proof of concept does. A test would need a recogniser that is exact (never accepts
  non-canonical text). Its cost and coverage on the fixtures, which are already formatted,
  and on perturbed input would then be measured. **Hypothesis.**
- **The two parse paths.** Why does `SourceFile::parse` cost 13–38% more cycles than the
  port's parse on the same files?
- **Literal measuring.** How much of `LITERAL`'s 8x is the `filtered_str_fits` line scan on
  multi-line literals? A width that the lexer's token already determines would remove it.
  Not tested.
- **Representations.** X3/X4 — typed accessors on another representation — stay as
  proposed.

## Current State

- `format_expr` memoizes rewrites by exact shape, expression type and context flags
  (chloro-core/src/formatter/expr.rs:109-123, chloro-core/src/formatter/context.rs:271)
- `rewrite_literal` measures every line of a multi-line string literal through
  `filtered_str_fits` (chloro-core/src/formatter/expr.rs:1251,
  chloro-core/src/formatter/utils.rs:156-182)
- Macro call arguments are re-parsed and formatted wherever a macro call appears
  (chloro-core/src/formatter/macro_args.rs)
- The formatter code is unchanged from `9202b42`

## Missing

- No exact rule for reusing a rewrite result at another indentation — chains change layout
  family under a tighter shape
- No recogniser for source text that is already in rustfmt's output form

## Appendix: patches

- `x9b_port.diff` applies to `2093831`.
- `x9b_poc.diff` applies to `c4d74ee`. Copy `chloro-core/examples/bench.rs` from this branch
  into the checkout before building the proof of concept's bench.
- Run as `target/release/examples/x9b DIR ROUNDS`: TSV of key, kind, repeat, frames, cycles,
  allocations, bytes.
- `x9c.diff` applies to `2093831`. Run as `target/release/examples/x9c DIR`; the variable
  `X9C_KIND=<KIND>` limits the printed disagreement samples to one node kind.

<details>
<summary><code>x9b_port.diff</code> — X9b, port</summary>

```diff
diff --git a/chloro-core/src/formatter/expr.rs b/chloro-core/src/formatter/expr.rs
index f3f2db2..1e8eea9 100644
--- a/chloro-core/src/formatter/expr.rs
+++ b/chloro-core/src/formatter/expr.rs
@@ -115,9 +115,11 @@ pub(crate) fn format_expr(
     // Paths and literals have no subexpressions: rewriting one again costs less than
     // storing it.
     if matches!(expr, ast::Expr::PathExpr(_) | ast::Expr::Literal(_)) {
+        let _f = crate::x9b::node_frame(expr.syntax(), 0, true);
         return format_expr_uncached(expr, expr_type, context, shape);
     }
     context.memoize(expr.syntax(), expr_type as u8, shape, || {
+        let _f = crate::x9b::node_frame(expr.syntax(), 0, true);
         format_expr_uncached(expr, expr_type, context, shape)
     })
 }
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..9683356 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -38,6 +38,7 @@ pub(crate) struct Formatted {
 /// rust-analyzer's semantic validation (e.g. `crate` in the middle of a path), which
 /// rustc's parser does not do, and would cost a second traversal of the tree.
 fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
+    let _f = crate::x9b::enter(crate::x9b::PARSE, false);
     let lexed = LexedStr::new(edition, source);
     if lexed.errors().next().is_some() {
         return None;
diff --git a/chloro-core/src/formatter/patterns.rs b/chloro-core/src/formatter/patterns.rs
index bbd6dca..9c1bd44 100644
--- a/chloro-core/src/formatter/patterns.rs
+++ b/chloro-core/src/formatter/patterns.rs
@@ -87,6 +87,7 @@ impl Rewrite for RangeOperand {

 impl Rewrite for ast::Pat {
     fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
+        let _f = crate::x9b::node_frame(self.syntax(), 1, true);
         match self {
             ast::Pat::OrPat(or) => {
                 let pats: Vec<ast::Pat> = or.pats().collect();
diff --git a/chloro-core/src/formatter/types.rs b/chloro-core/src/formatter/types.rs
index 0838fbd..223309d 100644
--- a/chloro-core/src/formatter/types.rs
+++ b/chloro-core/src/formatter/types.rs
@@ -809,6 +809,7 @@ pub(crate) fn generic_param_span(param: &ast::GenericParam) -> Span {

 impl Rewrite for ast::Type {
     fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
+        let _f = crate::x9b::node_frame(self.syntax(), 2, true);
         match self {
             ast::Type::DynTraitType(dt) => {
                 // A bare trait object (`'a + Trait`, edition 2015 syntax) stays bare.
diff --git a/chloro-core/src/formatter/visitor.rs b/chloro-core/src/formatter/visitor.rs
index 9429c03..174dccf 100644
--- a/chloro-core/src/formatter/visitor.rs
+++ b/chloro-core/src/formatter/visitor.rs
@@ -129,6 +129,7 @@ impl<'a> FmtVisitor<'a> {
     }

     fn visit_stmt(&mut self, stmt: &Stmt, include_empty_semi: bool) {
+        let _f = crate::x9b::enter(crate::x9b::STMT, false);
         if stmt.is_empty() {
             // If the statement is empty, just skip over it. Before that, make sure any comment
             // snippet preceding the semicolon is picked up.
@@ -371,6 +372,7 @@ impl<'a> FmtVisitor<'a> {
     }

     pub(crate) fn visit_item(&mut self, item: &ast::Item) {
+        let _f = crate::x9b::node_frame(item.syntax(), 3, false);
         if self.block_indent.block_indent == 0 {
             // No rewrite of a later top-level item reuses one of this item's nodes.
             self.run.clear_memo();
@@ -496,6 +498,7 @@ impl<'a> FmtVisitor<'a> {

     /// An associated item of a trait or impl.
     pub(crate) fn visit_assoc_item(&mut self, ai: &ast::AssocItem) {
+        let _f = crate::x9b::node_frame(ai.syntax(), 3, false);
         let in_trait = ai
             .syntax()
             .parent()
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..bd2540a 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x9b;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/x9b-port/chloro-core/src/x9b.rs b/chloro-core/src/x9b.rs
new file mode 100644
index 0000000..04c0f1e
--- /dev/null
+++ b/chloro-core/src/x9b.rs
@@ -0,0 +1,89 @@
+//! Scratch instrumentation (X9b): exclusive cost per node kind.
+//!
+//! A stack of frames, one per node being formatted. Cycles (rdtsc) between two stack events
+//! are charged to the frame on top, so a frame's cost excludes its child frames. Each frame
+//! is keyed by a node kind (or a pseudo key) and by whether it runs inside a repeated
+//! rewrite (a rewrite of a node already rewritten in this file). Allocations made while a
+//! frame is on top are charged to it through `alloc_hook`, called from the example's
+//! global allocator. Single-threaded use only.
+use std::cell::{Cell, RefCell};
+use std::collections::HashSet;
+use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering::Relaxed};
+
+pub const PARSE: u16 = 2000;
+pub const ROOT: u16 = 2001;
+pub const STMT: u16 = 2002;
+pub const INSTR: u16 = 2003;
+pub const N: usize = 2048;
+
+static CUR: AtomicU16 = AtomicU16::new(ROOT);
+static REP: AtomicBool = AtomicBool::new(false);
+static CYC: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static CNT: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static ALLOC: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static ABYTES: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+
+thread_local! {
+    static STACK: RefCell<Vec<(u16, bool)>> = const { RefCell::new(Vec::new()) };
+    static LAST: Cell<u64> = const { Cell::new(0) };
+    static SEEN: RefCell<Option<HashSet<(usize, u32, u8)>>> = const { RefCell::new(None) };
+}
+
+fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+fn idx(k: u16, r: bool) -> usize { (k as usize).min(N - 1) * 2 + r as usize }
+fn charge() {
+    let now = tsc();
+    let last = LAST.with(|l| l.replace(now));
+    if last != 0 { CYC[idx(CUR.load(Relaxed), REP.load(Relaxed))].fetch_add(now - last, Relaxed); }
+}
+pub fn alloc_hook(size: usize) {
+    let i = idx(CUR.load(Relaxed), REP.load(Relaxed));
+    ALLOC[i].fetch_add(1, Relaxed);
+    ABYTES[i].fetch_add(size as u64, Relaxed);
+}
+pub struct Frame(());
+impl Drop for Frame {
+    fn drop(&mut self) {
+        charge();
+        STACK.with(|s| {
+            let mut s = s.borrow_mut();
+            s.pop();
+            let (k, r) = s.last().copied().unwrap_or((ROOT, false));
+            CUR.store(k, Relaxed);
+            REP.store(r, Relaxed);
+        });
+    }
+}
+/// Enters a frame for `key`. `repeat` marks a repeated rewrite; frames inside a repeat
+/// inherit it.
+pub fn enter(key: u16, repeat: bool) -> Frame {
+    charge();
+    let r = repeat || REP.load(Relaxed);
+    STACK.with(|s| s.borrow_mut().push((key, r)));
+    CUR.store(key, Relaxed);
+    REP.store(r, Relaxed);
+    CNT[idx(key, r)].fetch_add(1, Relaxed);
+    Frame(())
+}
+/// Whether the node was rewritten before in this file; records it. The bookkeeping runs
+/// under the `INSTR` key so that its cost and allocations are not charged to a node.
+pub fn seen_before(green: usize, offset: u32, cat: u8) -> bool {
+    let _f = enter(INSTR, false);
+    SEEN.with(|s| !s.borrow_mut().get_or_insert_with(HashSet::new).insert((green, offset, cat)))
+}
+/// Called between files: node identities are only unique within one tree.
+pub fn file_done() { SEEN.with(|s| if let Some(s) = s.borrow_mut().as_mut() { s.clear() }); }
+pub fn reset_clock() { LAST.with(|l| l.set(0)); }
+/// (key, repeat, frames, cycles, allocations, allocated bytes) for every non-empty slot.
+pub fn table() -> Vec<(u16, bool, u64, u64, u64, u64)> {
+    (0..2 * N).filter_map(|i| {
+        let (c, y, a, b) = (CNT[i].load(Relaxed), CYC[i].load(Relaxed), ALLOC[i].load(Relaxed), ABYTES[i].load(Relaxed));
+        (c + y + a > 0).then_some(((i / 2) as u16, i % 2 == 1, c, y, a, b))
+    }).collect()
+}
+/// A frame for `node`, keyed by its kind; with `track`, marked as a repeat when the node
+/// (category `cat`) was rewritten before in this file.
+pub fn node_frame(node: &ra_ap_syntax::SyntaxNode, cat: u8, track: bool) -> Frame {
+    let rep = track && seen_before(&*node.green() as *const _ as *const u8 as usize, u32::from(node.text_range().start()), cat);
+    enter(node.kind() as u16, rep)
+}
diff --git a/wt/x9b-port/chloro-core/examples/x9b.rs b/chloro-core/examples/x9b.rs
new file mode 100644
index 0000000..60e85a1
--- /dev/null
+++ b/chloro-core/examples/x9b.rs
@@ -0,0 +1,37 @@
+//! X9b driver: formats every file under ROOT for -n rounds and prints the exclusive cost
+//! table of `x9b` as TSV: key, kind name, repeat, frames, cycles, allocations, bytes.
+use std::alloc::{GlobalAlloc, Layout, System};
+struct Counting;
+unsafe impl GlobalAlloc for Counting {
+    unsafe fn alloc(&self, l: Layout) -> *mut u8 { chloro_core::x9b::alloc_hook(l.size()); unsafe { System.alloc(l) } }
+    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { unsafe { System.dealloc(p, l) } }
+    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { chloro_core::x9b::alloc_hook(n); unsafe { System.realloc(p, l, n) } }
+}
+#[global_allocator]
+static G: Counting = Counting;
+fn main() {
+    let mut a = std::env::args().skip(1);
+    let root = a.next().unwrap();
+    let rounds: usize = a.next().map_or(1, |n| n.parse().unwrap());
+    let files: Vec<String> = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect();
+    for _ in 0..rounds {
+        for s in &files {
+            chloro_core::x9b::reset_clock();
+            let f = chloro_core::x9b::enter(chloro_core::x9b::ROOT, false);
+            std::hint::black_box(chloro_core::format_source(s));
+            drop(f);
+            chloro_core::x9b::file_done();
+        }
+    }
+    for (k, r, c, y, al, b) in chloro_core::x9b::table() {
+        let name = match k {
+            chloro_core::x9b::PARSE => "(parse)".to_string(),
+            chloro_core::x9b::ROOT => "(outside any node frame)".to_string(),
+            chloro_core::x9b::STMT => "(statement)".to_string(),
+            chloro_core::x9b::INSTR => "(instrumentation)".to_string(),
+            k => format!("{:?}", ra_ap_syntax::SyntaxKind::from(k)),
+        };
+        println!("{k}\t{name}\t{r}\t{c}\t{y}\t{al}\t{b}");
+    }
+}
```

</details>

<details>
<summary><code>x9b_poc.diff</code> — X9b, proof of concept</summary>

```diff
diff --git a/chloro-core/src/formatter/node/expr.rs b/chloro-core/src/formatter/node/expr.rs
index 3b8ccb8..4c45904 100644
--- a/chloro-core/src/formatter/node/expr.rs
+++ b/chloro-core/src/formatter/node/expr.rs
@@ -39,6 +39,7 @@ pub fn try_format_expr(node: &SyntaxNode, indent: usize) -> FormatResult {

 /// Inner implementation returning Option for easier chaining.
 pub fn try_format_expr_inner(node: &SyntaxNode, indent: usize) -> Option<String> {
+    let _f = crate::x9b::node_frame(node, 0, false);
     match node.kind() {
         // === Simple / Pass-through ===
         SyntaxKind::PATH_EXPR | SyntaxKind::LITERAL | SyntaxKind::UNDERSCORE_EXPR => {
diff --git a/chloro-core/src/formatter/node.rs b/chloro-core/src/formatter/node.rs
index e62ff11..4a2121d 100644
--- a/chloro-core/src/formatter/node.rs
+++ b/chloro-core/src/formatter/node.rs
@@ -143,6 +143,7 @@ fn sort_use_groups(items: &mut [ItemWithComments]) {

 /// Main node formatting dispatcher
 pub fn format_node(node: &SyntaxNode, buf: &mut String, indent: usize) {
+    let _f = crate::x9b::node_frame(node, 0, false);
     match node.kind() {
         SyntaxKind::SOURCE_FILE => {
             let mut module_inner_docs = Vec::new();
diff --git a/chloro-core/src/formatter.rs b/chloro-core/src/formatter.rs
index 91d92eb..cd63d17 100644
--- a/chloro-core/src/formatter.rs
+++ b/chloro-core/src/formatter.rs
@@ -6,7 +6,10 @@ use ra_ap_syntax::{AstNode, Edition, SourceFile};

 /// Format Rust source code with canonical style.
 pub fn format_source(source: &str) -> String {
-    let parse = SourceFile::parse(source, Edition::CURRENT);
+    let parse = {
+        let _f = crate::x9b::enter(crate::x9b::PARSE, false);
+        SourceFile::parse(source, Edition::CURRENT)
+    };
     let root = parse.tree();

     let mut output = String::with_capacity(source.len());
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 3e4fb34..a226387 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -19,3 +19,5 @@ macro_rules! chloro_debug {

 #[cfg(test)]
 mod tests;
+
+pub mod x9b;
diff --git a/wt/x9b-poc/chloro-core/src/x9b.rs b/chloro-core/src/x9b.rs
new file mode 100644
index 0000000..04c0f1e
--- /dev/null
+++ b/chloro-core/src/x9b.rs
@@ -0,0 +1,89 @@
+//! Scratch instrumentation (X9b): exclusive cost per node kind.
+//!
+//! A stack of frames, one per node being formatted. Cycles (rdtsc) between two stack events
+//! are charged to the frame on top, so a frame's cost excludes its child frames. Each frame
+//! is keyed by a node kind (or a pseudo key) and by whether it runs inside a repeated
+//! rewrite (a rewrite of a node already rewritten in this file). Allocations made while a
+//! frame is on top are charged to it through `alloc_hook`, called from the example's
+//! global allocator. Single-threaded use only.
+use std::cell::{Cell, RefCell};
+use std::collections::HashSet;
+use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering::Relaxed};
+
+pub const PARSE: u16 = 2000;
+pub const ROOT: u16 = 2001;
+pub const STMT: u16 = 2002;
+pub const INSTR: u16 = 2003;
+pub const N: usize = 2048;
+
+static CUR: AtomicU16 = AtomicU16::new(ROOT);
+static REP: AtomicBool = AtomicBool::new(false);
+static CYC: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static CNT: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static ALLOC: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+static ABYTES: [AtomicU64; 2 * N] = [const { AtomicU64::new(0) }; 2 * N];
+
+thread_local! {
+    static STACK: RefCell<Vec<(u16, bool)>> = const { RefCell::new(Vec::new()) };
+    static LAST: Cell<u64> = const { Cell::new(0) };
+    static SEEN: RefCell<Option<HashSet<(usize, u32, u8)>>> = const { RefCell::new(None) };
+}
+
+fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+fn idx(k: u16, r: bool) -> usize { (k as usize).min(N - 1) * 2 + r as usize }
+fn charge() {
+    let now = tsc();
+    let last = LAST.with(|l| l.replace(now));
+    if last != 0 { CYC[idx(CUR.load(Relaxed), REP.load(Relaxed))].fetch_add(now - last, Relaxed); }
+}
+pub fn alloc_hook(size: usize) {
+    let i = idx(CUR.load(Relaxed), REP.load(Relaxed));
+    ALLOC[i].fetch_add(1, Relaxed);
+    ABYTES[i].fetch_add(size as u64, Relaxed);
+}
+pub struct Frame(());
+impl Drop for Frame {
+    fn drop(&mut self) {
+        charge();
+        STACK.with(|s| {
+            let mut s = s.borrow_mut();
+            s.pop();
+            let (k, r) = s.last().copied().unwrap_or((ROOT, false));
+            CUR.store(k, Relaxed);
+            REP.store(r, Relaxed);
+        });
+    }
+}
+/// Enters a frame for `key`. `repeat` marks a repeated rewrite; frames inside a repeat
+/// inherit it.
+pub fn enter(key: u16, repeat: bool) -> Frame {
+    charge();
+    let r = repeat || REP.load(Relaxed);
+    STACK.with(|s| s.borrow_mut().push((key, r)));
+    CUR.store(key, Relaxed);
+    REP.store(r, Relaxed);
+    CNT[idx(key, r)].fetch_add(1, Relaxed);
+    Frame(())
+}
+/// Whether the node was rewritten before in this file; records it. The bookkeeping runs
+/// under the `INSTR` key so that its cost and allocations are not charged to a node.
+pub fn seen_before(green: usize, offset: u32, cat: u8) -> bool {
+    let _f = enter(INSTR, false);
+    SEEN.with(|s| !s.borrow_mut().get_or_insert_with(HashSet::new).insert((green, offset, cat)))
+}
+/// Called between files: node identities are only unique within one tree.
+pub fn file_done() { SEEN.with(|s| if let Some(s) = s.borrow_mut().as_mut() { s.clear() }); }
+pub fn reset_clock() { LAST.with(|l| l.set(0)); }
+/// (key, repeat, frames, cycles, allocations, allocated bytes) for every non-empty slot.
+pub fn table() -> Vec<(u16, bool, u64, u64, u64, u64)> {
+    (0..2 * N).filter_map(|i| {
+        let (c, y, a, b) = (CNT[i].load(Relaxed), CYC[i].load(Relaxed), ALLOC[i].load(Relaxed), ABYTES[i].load(Relaxed));
+        (c + y + a > 0).then_some(((i / 2) as u16, i % 2 == 1, c, y, a, b))
+    }).collect()
+}
+/// A frame for `node`, keyed by its kind; with `track`, marked as a repeat when the node
+/// (category `cat`) was rewritten before in this file.
+pub fn node_frame(node: &ra_ap_syntax::SyntaxNode, cat: u8, track: bool) -> Frame {
+    let rep = track && seen_before(&*node.green() as *const _ as *const u8 as usize, u32::from(node.text_range().start()), cat);
+    enter(node.kind() as u16, rep)
+}
diff --git a/wt/x9b-poc/chloro-core/examples/x9b.rs b/chloro-core/examples/x9b.rs
new file mode 100644
index 0000000..60e85a1
--- /dev/null
+++ b/chloro-core/examples/x9b.rs
@@ -0,0 +1,37 @@
+//! X9b driver: formats every file under ROOT for -n rounds and prints the exclusive cost
+//! table of `x9b` as TSV: key, kind name, repeat, frames, cycles, allocations, bytes.
+use std::alloc::{GlobalAlloc, Layout, System};
+struct Counting;
+unsafe impl GlobalAlloc for Counting {
+    unsafe fn alloc(&self, l: Layout) -> *mut u8 { chloro_core::x9b::alloc_hook(l.size()); unsafe { System.alloc(l) } }
+    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { unsafe { System.dealloc(p, l) } }
+    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { chloro_core::x9b::alloc_hook(n); unsafe { System.realloc(p, l, n) } }
+}
+#[global_allocator]
+static G: Counting = Counting;
+fn main() {
+    let mut a = std::env::args().skip(1);
+    let root = a.next().unwrap();
+    let rounds: usize = a.next().map_or(1, |n| n.parse().unwrap());
+    let files: Vec<String> = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect();
+    for _ in 0..rounds {
+        for s in &files {
+            chloro_core::x9b::reset_clock();
+            let f = chloro_core::x9b::enter(chloro_core::x9b::ROOT, false);
+            std::hint::black_box(chloro_core::format_source(s));
+            drop(f);
+            chloro_core::x9b::file_done();
+        }
+    }
+    for (k, r, c, y, al, b) in chloro_core::x9b::table() {
+        let name = match k {
+            chloro_core::x9b::PARSE => "(parse)".to_string(),
+            chloro_core::x9b::ROOT => "(outside any node frame)".to_string(),
+            chloro_core::x9b::STMT => "(statement)".to_string(),
+            chloro_core::x9b::INSTR => "(instrumentation)".to_string(),
+            k => format!("{:?}", ra_ap_syntax::SyntaxKind::from(k)),
+        };
+        println!("{k}\t{name}\t{r}\t{c}\t{y}\t{al}\t{b}");
+    }
+}
```

</details>

<details>
<summary><code>x9c.diff</code> — X9c, shadow-mode reuse</summary>

```diff
diff --git a/chloro-core/src/formatter/context.rs b/chloro-core/src/formatter/context.rs
index 8a63890..5d2454a 100644
--- a/chloro-core/src/formatter/context.rs
+++ b/chloro-core/src/formatter/context.rs
@@ -250,7 +250,7 @@ impl<'a> RewriteContext<'a> {
         self.is_loop_block.get()
     }

-    fn flags(&self) -> u8 {
+    pub(crate) fn flags(&self) -> u8 {
         u8::from(self.inside_macro.get())
             | u8::from(self.is_if_else_block.get()) << 1
             | u8::from(self.is_loop_block.get()) << 2
diff --git a/chloro-core/src/formatter/expr.rs b/chloro-core/src/formatter/expr.rs
index f3f2db2..6cf45aa 100644
--- a/chloro-core/src/formatter/expr.rs
+++ b/chloro-core/src/formatter/expr.rs
@@ -118,7 +118,30 @@ pub(crate) fn format_expr(
         return format_expr_uncached(expr, expr_type, context, shape);
     }
     context.memoize(expr.syntax(), expr_type as u8, shape, || {
-        format_expr_uncached(expr, expr_type, context, shape)
+        let kind = format!("{:?}", expr.syntax().kind());
+        let key = crate::x9c::key(expr.syntax(), expr_type as u8, context.flags());
+        let cand = crate::x9c::candidate(&kind, key, shape, context.config.max_width(), context.config.hard_tabs(), crate::x9c::has_multiline_literal(expr.syntax()));
+        if crate::x9c::reuse() {
+            if let Some((c, true)) = cand {
+                let r = Some(c);
+                crate::x9c::record(key, shape, &r, false);
+                return r;
+            }
+        }
+        let ps = crate::x9c::prior_shape(key);
+        let skipped = context.run.skipped_range.borrow().len();
+        let (failure, lost) = (context.macro_rewrite_failure.get(), context.run.lost_comment.get());
+        let t0 = crate::x9c::tsc();
+        let fresh = format_expr_uncached(expr, expr_type, context, shape);
+        let t1 = crate::x9c::tsc();
+        let effects = (!failure && context.macro_rewrite_failure.get())
+            || (!lost && context.run.lost_comment.get())
+            || context.run.skipped_range.borrow().len() != skipped;
+        if let Some((c, wider)) = &cand {
+            crate::x9c::compare(&kind, c, *wider, &fresh, t0, t1, ps.unwrap(), shape);
+        }
+        crate::x9c::record(key, shape, &fresh, effects);
+        fresh
     })
 }

diff --git a/chloro-core/src/formatter.rs b/chloro-core/src/formatter.rs
index 3fc8168..2f59b28 100644
--- a/chloro-core/src/formatter.rs
+++ b/chloro-core/src/formatter.rs
@@ -40,12 +40,12 @@ mod patterns;
 pub mod printer;
 mod reorder;
 mod rustc_compat;
-mod shape;
+pub(crate) mod shape;
 mod sort;
 mod span;
 mod stmt;
 mod types;
-mod utils;
+pub(crate) mod utils;
 mod visitor;

 use config::{Config, Settings};
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..190e801 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x9c;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/x9c/chloro-core/src/x9c.rs b/chloro-core/src/x9c.rs
new file mode 100644
index 0000000..ec96989
--- /dev/null
+++ b/chloro-core/src/x9c.rs
@@ -0,0 +1,131 @@
+
+//! Scratch experiment (X9c): reuse of a rewrite result at another indentation.
+//!
+//! For every uncached `format_expr` rewrite of a node that was rewritten before in this file
+//! with the same expression type and context flags, a candidate is built from the latest
+//! earlier result: its lines after the first are shifted by the difference in indentation.
+//! The candidate is *attempted* when the earlier rewrite had no side effects (macro failure,
+//! lost comment, skipped range) and it passes `filtered_str_fits` for the new shape, as
+//! rustfmt's `wrap_str` checks results. Shadow mode compares an attempted candidate with the
+//! fresh rewrite, which is still computed and returned. `X9C_REUSE=1` returns the candidate
+//! instead and skips the fresh rewrite.
+use std::cell::{Cell, RefCell};
+use std::collections::{BTreeMap, HashMap};
+use crate::formatter::shape::Shape;
+
+pub fn prior_shape(key: Key) -> Option<Shape> { PRIOR.with(|p| p.borrow().get(&key).map(|x| x.shape)) }
+pub struct Prior { pub shape: Shape, pub result: Option<String>, pub effects: bool }
+thread_local! {
+    static PRIOR: RefCell<HashMap<(usize, u32, u8, u8), Prior>> = RefCell::new(HashMap::new());
+    static STATS: RefCell<BTreeMap<String, [u64; 12]>> = const { RefCell::new(BTreeMap::new()) };
+    static AGREE: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
+    static AGREE_N: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
+    static SAVED_N: Cell<u64> = const { Cell::new(0) };
+    static DISAGREE: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
+    static SAVED: Cell<u64> = const { Cell::new(0) };
+    static LOST: Cell<u64> = const { Cell::new(0) };
+    static SAMPLES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
+    static REUSE: Cell<Option<bool>> = const { Cell::new(None) };
+}
+pub const NAMES: [&str; 12] = ["first rewrite", "same shape", "prior None", "prior had effects",
+    "multi-line literal or cannot shift", "candidate does not fit", "attempted (new shape dominated)", "agreed (new shape dominated)", "disagreed (new shape dominated)",
+    "attempted (not dominated)", "agreed (not dominated)", "disagreed (not dominated)"];
+pub fn reuse() -> bool {
+    REUSE.with(|c| match c.get() { Some(v) => v, None => { let v = std::env::var("X9C_REUSE").is_ok(); c.set(Some(v)); v } })
+}
+pub fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+fn stat(kind: &str, i: usize) { STATS.with(|s| s.borrow_mut().entry(kind.to_string()).or_insert([0; 12])[i] += 1) }
+/// Shifts every line after the first by `to - from` columns, or `None` when a line has
+/// fewer leading spaces than the shift removes.
+fn shift(text: &str, from: usize, to: usize) -> Option<String> {
+    let mut out = String::with_capacity(text.len() + 64);
+    for (i, line) in text.split('\n').enumerate() {
+        if i > 0 {
+            out.push('\n');
+            if !line.is_empty() {
+                if to >= from {
+                    out.extend(std::iter::repeat_n(' ', to - from));
+                } else {
+                    let cut = from - to;
+                    if line.len() < cut || !line.as_bytes()[..cut].iter().all(|&b| b == b' ') { return None; }
+                    out.push_str(&line[cut..]);
+                    continue;
+                }
+            }
+        }
+        out.push_str(line);
+    }
+    Some(out)
+}
+pub type Key = (usize, u32, u8, u8);
+pub fn key(node: &ra_ap_syntax::SyntaxNode, expr_type: u8, flags: u8) -> Key {
+    (&*node.green() as *const _ as *const u8 as usize, u32::from(node.text_range().start()), expr_type, flags)
+}
+/// The candidate for `key` under `shape`, if one is attempted; records why not otherwise.
+/// Whether `node` contains a string literal token that spans lines: shifting such a result
+/// would change the literal's content.
+pub fn has_multiline_literal(node: &ra_ap_syntax::SyntaxNode) -> bool {
+    node.descendants_with_tokens().any(|e| e.as_token().is_some_and(|t| {
+        use ra_ap_syntax::SyntaxKind::*;
+        matches!(t.kind(), STRING | BYTE_STRING | C_STRING) && t.text().contains('\n')
+    }))
+}
+pub fn candidate(kind: &str, key: Key, shape: Shape, max_width: usize, hard_tabs: bool, literal: bool) -> Option<(String, bool)> {
+    PRIOR.with(|p| {
+        let p = p.borrow();
+        let Some(prior) = p.get(&key) else { stat(kind, 0); return None };
+        if prior.shape == shape { stat(kind, 1); return None }
+        let Some(text) = &prior.result else { stat(kind, 2); return None };
+        if prior.effects { stat(kind, 3); return None }
+        if literal { stat(kind, 4); return None }
+        let Some(c) = (!hard_tabs).then(|| shift(text, prior.shape.indent.width(), shape.indent.width())).flatten() else { stat(kind, 4); return None };
+        if !crate::formatter::utils::filtered_str_fits(&c, max_width, shape) { stat(kind, 5); return None }
+        let wider = prior.shape.width >= shape.width && prior.shape.indent.width() <= shape.indent.width();
+        stat(kind, if wider { 6 } else { 9 });
+        Some((c, wider))
+    })
+}
+pub fn record(key: Key, shape: Shape, result: &Option<String>, effects: bool) {
+    PRIOR.with(|p| { p.borrow_mut().insert(key, Prior { shape, result: result.clone(), effects }); });
+}
+pub fn compare(kind: &str, cand: &str, wider: bool, fresh: &Option<String>, t0: u64, t1: u64, ps: Shape, ns: Shape) {
+    if fresh.as_deref() == Some(cand) {
+        stat(kind, if wider { 7 } else { 10 });
+        if wider { AGREE.with(|a| a.borrow_mut().push((t0, t1))); } else { AGREE_N.with(|a| a.borrow_mut().push((t0, t1))); }
+    } else {
+        stat(kind, if wider { 8 } else { 11 });
+        if !wider { return; }
+        DISAGREE.with(|a| a.borrow_mut().push((t0, t1)));
+        SAMPLES.with(|s| { let mut s = s.borrow_mut(); let want = std::env::var("X9C_KIND").ok(); if want.as_deref().is_none_or(|w| w == kind) && s.len() < 12 { s.push(format!("{kind} prior {ps:?} new {ns:?}\n--- candidate\n{cand}\n--- fresh\n{}\n", fresh.as_deref().unwrap_or("<None>"))); } });
+    }
+}
+fn union(v: &mut Vec<(u64, u64)>) -> u64 {
+    v.sort();
+    let (mut total, mut cur) = (0, None::<(u64, u64)>);
+    for &(a, b) in v.iter() {
+        match cur { Some((s, e)) if a <= e => cur = Some((s, e.max(b))), Some((s, e)) => { total += e - s; cur = Some((a, b)) } None => cur = Some((a, b)) }
+    }
+    if let Some((s, e)) = cur { total += e - s }
+    v.clear();
+    total
+}
+pub fn file_done() {
+    PRIOR.with(|p| p.borrow_mut().clear());
+    SAVED.set(SAVED.get() + AGREE.with(|a| union(&mut a.borrow_mut())));
+    SAVED_N.set(SAVED_N.get() + AGREE_N.with(|a| union(&mut a.borrow_mut())));
+    LOST.set(LOST.get() + DISAGREE.with(|a| union(&mut a.borrow_mut())));
+}
+pub fn dump(total: u64) {
+    STATS.with(|s| {
+        let s = s.borrow();
+        print!("{:22}", "kind"); for n in NAMES { print!(" {:>9}", &n[..n.len().min(9)]); } println!();
+        let mut t = [0u64; 12];
+        for (k, v) in s.iter() { if v[6] == 0 && v[0] < 1000 { for i in 0..12 { t[i] += v[i] } continue } print!("{k:22}"); for i in 0..12 { t[i] += v[i]; print!(" {:>9}", v[i]); } println!(); }
+        print!("{:22}", "TOTAL"); for x in t { print!(" {:>9}", x); } println!();
+        println!("columns: {}", NAMES.join(" | "));
+    });
+    println!("new shape dominated: cycles of fresh rewrites whose candidate agreed (union) {} = {:.1}% of format_source; of disagreeing {} = {:.1}%",
+        SAVED.get(), 100.0 * SAVED.get() as f64 / total as f64, LOST.get(), 100.0 * LOST.get() as f64 / total as f64);
+    println!("not dominated: cycles of agreeing fresh rewrites (union) {} = {:.1}%", SAVED_N.get(), 100.0 * SAVED_N.get() as f64 / total as f64);
+    SAMPLES.with(|s| for x in s.borrow().iter().take(6) { println!("==== disagreement\n{x}"); });
+}
diff --git a/wt/x9c/chloro-core/examples/x9c.rs b/chloro-core/examples/x9c.rs
new file mode 100644
index 0000000..4549838
--- /dev/null
+++ b/chloro-core/examples/x9c.rs
@@ -0,0 +1,14 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let files: Vec<String> = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect();
+    let mut total = 0;
+    for s in &files {
+        let t = chloro_core::x9c::tsc();
+        std::hint::black_box(chloro_core::format_source(s));
+        total += chloro_core::x9c::tsc() - t;
+        chloro_core::x9c::file_done();
+    }
+    println!("{} files, format_source cycles {total}", files.len());
+    chloro_core::x9c::dump(total);
+}
```

</details>
