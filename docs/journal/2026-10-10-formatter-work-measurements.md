# 2026-10-10: Where the port's formatting work goes (X1, X9, X8)

This entry follows [2026-10-10-change-of-direction.md](2026-10-10-change-of-direction.md). It
records three of the experiments in that entry's matrix. They were run first because they
measure the work the formatter performs before any of it is redesigned:
- **X1:** dynamic counts of syntax and text operations;
- **X9:** the cost of repeated layout attempts;
- **X8:** token-based answers to comment queries.

None of them changes code in the repository:
- each one ran on a scratch copy of the repository at `50d2ca8`, whose formatter code is
  that of `9202b42`;
- the exact patches are in the appendix and apply with `git apply` to `50d2ca8`;
- every patched build reproduced the output hashes recorded at `bcef0f9`: all three on the
  1238 fixtures, and `x8` on the 5659 registry files too.

Labels as before:
- **measured:** produced by a command or patch listed here;
- **read:** taken from source;
- **hypothesis:** not established.

## Summary

- **Repeated layout attempts are bounded** (measured):
  - rewriting an expression, pattern or type node that was already rewritten costs 18.5% of
    `format_source` cycles on the fixtures and 16.9% of instructions on the sample, with
    the counting overhead included;
  - removing every such repeat would make the port about 1.2x faster, about 4.2 MB/s, on
    these figures.
- **A decide-once design can save at most the repeat share**, unless it also changes how
  each first attempt is computed. Most of the port's cost lies in the first attempts:
  allocation, red-tree navigation, building and measuring strings.
- **Repeats split three ways** (measured):

  | class | share of `format_source` cycles |
  |---|---|
  | nodes whose results never differed | 1.5% |
  | results that differ only in indentation | 10.4% |
  | results that differ in layout | 6.6% |

  A cache keyed by node alone could remove at most the 1.5%. Reusing a layout across
  indentations is a candidate for most of the rest. It is untested and needs a rule for
  when reuse is exact.
- **Text rescans of source are spread thin** (measured):
  - CharClasses' 28.8 M instructions on the sample come from six callers of about 1% each;
  - answering `contains_comment` and `find_uncommented` from the lexer's tokens gave
    identical answers on 9.3 M calls, but cost 3% *more* instructions overall. The scans
    it replaced usually end within a few bytes, and the token lookups cost more than that.
- **Most text scanning reads built strings** (measured): `unicode_str_width`,
  `filtered_str_fits` and `filter_normal_code` run over candidate results, not source,
  so no token index can answer them.

## X1: syntax and text operations, by count and volume

Scratch patch `x1.diff` (appendix):
- each operation records calls and bytes;
- each call is classed by whether its text is a slice of a source being formatted (`src`)
  or a string built during formatting (`built`);
- the class is decided by pointer range, against a stack of the sources passed to
  `format_text`.

Measured on the fixtures: 1238 files, 13.9 MB in, 14.2 MB out.

| operation | src calls | src bytes | built calls | built bytes |
|---|---|---|---|---|
| `span_to_snippet` / `snippet` | 4,449,523 | 174.8 M | | |
| `find_uncommented` | 1,265,974 | 82.2 M | 68,257 | 0.6 M |
| `recover_comment_removed` (source snippet; rewritten string) | 764,312 | 64.8 M | 764,312 | 68.4 M |
| `changed_comment_content` (source side) | 66,786 | 25.5 M | | |
| `contains_comment` | 74,072 | 8.6 M | | |
| `contains_comment`, past the `/` fast path | 2,072 | 4.1 M | | |
| `CharClasses::new` (length of the text given) | 1,603,919 | 101.7 M | 82,054 | 16.0 M |
| `CharClasses::next` (characters stepped) | 9,039,181 | 9.0 M | 6,685,852 | 6.7 M |
| `CharClasses::skip_plain` (bytes skipped) | 1,724,567 | 12.8 M | 346,809 | 9.3 M |
| `CommentCodeSlices::new` | 96,892 | 2.0 M | | |
| `UngroupedCommentCodeSlices::new` | 6,156 | 10.7 M | 6,156 | 11.4 M |
| `LineClasses::new` | 685 | 0.4 M | 7,564 | 3.9 M |
| `filtered_str_fits` | | | 165,390 | 14.8 M |
| `filter_normal_code` | | | 165,390 | 14.8 M |
| `filter_normal_code`, past the `/` fast path | | | 7,387 | 3.9 M |
| `unicode_str_width` | 34 | 209 | 3,479,364 | 58.8 M |
| `rewrite_comment` | 5,171 | 0.7 M | 1,274 | 0.1 M |
| `nodes::child` | 2,505,880 | | | |
| `rustc_span` | 2,467,812 | | | |
| `span_without_attrs` | 1,422,747 | | | |
| `has_token` | 938,856 | | | |
| `node_text` | 618,569 | 4.7 M | | |

Findings (measured):
- **Text actually read.** Text handed to `CharClasses` totals 117.7 M bytes. It reads
  21.8 MB of source (9.0 M characters stepped, 12.8 M bytes skipped) and 16.0 MB of built
  strings — the scans stop early.
- **`find_uncommented` stops early.** It receives 82 MB of source in 1.27 M calls, but
  steps through a small part of it. Its needles are mostly single punctuation characters
  near the start of the span (read: `{` is the needle at 11 of the 35 call sites of
  `span_after`/`span_before` that pass a literal).
- **The width of built strings is measured repeatedly.** `unicode_str_width` measures
  58.8 MB of built strings, 4.1x the output.
- **The source-side comment check is mostly string comparison.**
  `recover_comment_removed` compares 64.8 MB of source with 68.4 MB of rewritten text.
  `changed_comment_content` then runs on 66,786 of its 764,312 calls (8.7%).
- **Tree access.** There are about 2.5 M typed child lookups and 2.5 M span computations
  for 0.56 M rewritten nodes (X9).

Inclusive instructions on the 49-file sample (`9202b42`, measured; entries overlap):

| function | inclusive | share |
|---|---|---|
| `CharClasses::next` | 28.8 M | 4.8% |
| `filtered_str_fits` | 19.0 M | 3.2% |
| `recover_comment_removed` | 18.6 M | 3.1% |
| `unicode_str_width` | 17.3 M | 2.9% |
| `find_uncommented` | 10.6 M | 1.8% |
| `filter_normal_code` | 8.6 M | 1.4% |
| `rustc_span` | 4.5 M | 0.75% |
| `span_without_attrs` | 3.1 M | 0.52% |
| `node_text` | 3.1 M | 0.52% |
| `has_token` | 2.3 M | 0.39% |
| `contains_comment` | 2.1 M | 0.35% |

`nodes::child`, across its instantiations, is about 15 M (2.5%).

Callers of `CharClasses::next` on the sample (measured):

| caller | instructions |
|---|---|
| `span_ends_with_comma` | 6.3 M |
| `UngroupedCommentCodeSlices::next` | 6.0 M |
| `LineClasses::next` | 5.3 M |
| `find_uncommented` | 4.7 M |
| `CommentCodeSlices::next` | 4.6 M |
| `contains_comment` | 1.1 M |

## X9: first attempts against repeats

Scratch patch `x9.diff` (appendix). Every uncached rewrite of an expression (inside
`format_expr`'s memo), a pattern (`Rewrite for ast::Pat`) or a type (`Rewrite for
ast::Type`) is recorded per node, as (green node, offset, category). Each record holds a
hash of the shape and a hash of the result.

- **Repeat:** a rewrite is a *repeat* when the node was rewritten before, under any shape.
- **Measuring repeats:** the outermost repeat runs inside `repeat_outer`, a non-inlined
  function, so neither the callgrind inclusive cost nor the rdtsc cycles count nested
  repeats twice. A first version gave `repeat_outer` and `first_outer` identical bodies,
  and LLVM merged them. The patch gives the two functions different bodies.
- **Classes:** each node is classed by its results across all its rewrites:
  - *identical*: all results equal;
  - *indentation*: equal after stripping leading whitespace from every line;
  - *layout*: otherwise.

Measured on the fixtures (rdtsc cycles of `format_source`, parsing included):
- 558,884 nodes; 916,291 rewrites;
- 200,072 nodes rewritten more than once:
  - 171,808 with identical results every time;
  - 18,167 that differ only in indentation;
- repeat work: 18.5% of `format_source` cycles:

  | class | share |
  |---|---|
  | identical | 1.5% |
  | indentation | 10.4% |
  | layout | 6.6% |

Largest repeat costs by node kind and class (share of `format_source` cycles, measured):

| node kind | class | nodes | rewrites | share |
|---|---|---|---|---|
| `MATCH_EXPR` | indentation | 2,067 | 4,999 | 2.80% |
| `METHOD_CALL_EXPR` | layout | 3,421 | 15,359 | 2.78% |
| `METHOD_CALL_EXPR` | indentation | 2,570 | 6,427 | 2.29% |
| `MATCH_EXPR` | layout | 358 | 1,220 | 1.59% |
| `REF_EXPR` | indentation | 199 | 449 | 1.28% |
| `CLOSURE_EXPR` | indentation | 1,485 | 3,886 | 1.07% |
| `IF_EXPR` | indentation | 1,386 | 3,189 | 0.99% |
| `CLOSURE_EXPR` | layout | 1,060 | 5,455 | 0.77% |
| `CALL_EXPR` | indentation | 1,986 | 5,081 | 0.75% |
| `CALL_EXPR` | layout | 1,682 | 7,685 | 0.43% |
| `METHOD_CALL_EXPR` | identical | 42,648 | 65,356 | 0.40% |

Measured on the 49-file sample (callgrind, instrumented build):

| function | inclusive instructions |
|---|---|
| `format_source` | 646.1 M |
| `repeat_outer` | 109.1 M (16.9%) |
| `first_outer` | 341.2 M (includes repeats nested inside first attempts) |

The instrumentation adds about 89 M instructions to the uninstrumented 595.3 M.

Findings (measured):
- Removing all repeated rewrites of these three categories would save about 17–18.5% of
  `format_source` on these corpora. That bounds what avoiding layout speculation *at this
  level* can save.
- The bound leaves out speculation inside one rewrite call: strings built and discarded
  within a single `rewrite_*` function, list tactics, and `rewrite_fn_base`'s parameter
  layouts. That work is counted in first attempts.
- The expensive repeats are on the constructs the layout search exists for: chains,
  `match`, closures, `if`, calls.
- The 171,808 nodes whose results never changed account for only 1.5%: they are cheap
  nodes (paths, short calls).
- More than half of the repeat cost (10.4 of 18.5 points) is on nodes whose results differ
  only in indentation: the same layout produced again at another indent.

Hypotheses (not established):
- **Indentation reuse.** Results that differ only in indentation could be produced by
  re-indenting one result, when the shape's width does not change rustfmt's choice. A rule
  for when that holds is not known. rustfmt's choices depend on the width available, so a
  reused result has to be validated or proven equivalent.
- **First attempts.** The agreement-corpus gap of 2.3–2.4x against the proof of concept
  (previous entry) comes mostly from first attempts, not repeats. The agreement corpus has
  few layout-heavy constructs. This is not measured per rewrite.

## X8: comment queries from tokens

Scratch patch `x8.diff` (appendix):
- while `formatting::parse` builds the tree, it records every token start and every
  comment range;
- `contains_comment` and `find_uncommented` answer from that index when their text is a
  token-aligned slice of a source being formatted;
- otherwise they fall back to `CharClasses`;
- with `X8_CHECK=1`, both answers are computed, disagreements are counted, and the
  `CharClasses` answer is returned.

Equivalence (measured, `X8_CHECK=1`). Two of rustfmt's `CharClasses` behaviours showed up
as disagreements, and the index was adjusted to match them:
- `CharClasses` counts the newline that ends a line comment as part of the comment
  (`chloro-core/src/formatter/comment.rs:816`). 451 `find_uncommented("\n")` answers
  differed until line-comment ranges were extended by their newline.
- Inside a block comment, `CharClasses` reads a `"` as the start of a string, in which
  `*/` does not close the comment (`comment.rs:783-799`), so it can end a block comment
  elsewhere than the lexer does. One registry file differed: globset's `/*! … */`
  crate docs, where a glob pattern contains `*/` before a quote. The index leaves any slice
  containing a block comment to `CharClasses`.

After both adjustments:

| corpus | `contains_comment` from tokens | `find_uncommented` from tokens | disagreements |
|---|---|---|---|
| fixtures | 74,072 | 1,234,081 | 0 |
| registry | 1,564,640 | 6,427,217 | 0 |

The answers agree on these corpora; that is empirical equivalence, not a proof.

Cost (measured):
- output hashes: 0 of 1238 fixtures and 0 of 5659 registry files changed;
- instructions on the sample, three runs: 613.4 M, 618.3 M and 626.4 M, against 595.3 M
  for `9202b42` — **3% more**;
- `formatting::parse` grows by 5.1 M (index building);
- `find_uncommented` costs 25.1 M inclusive, against 10.6 M. Its token-path lookup is two
  binary searches over all token starts and a thread-local access per call, costing
  21.3 M.

Finding (measured): for these two functions, rescanning text is cheaper than the index,
because the scans stop early. The source-side text scanning that a token index could
replace — `span_ends_with_comma`, the comment slicers, `contains_comment`,
`find_uncommented` — is about 4–5% of the sample in all. The index costs about 1% to build.
No gain large enough to steer the architecture is available here.

## What this changes in the previous entry

The previous entry's ordering is corrected as follows:
- **Decide-once formatting.** Its gain from removing speculation is bounded by the repeat
  share, about 17–18.5%. Reusing results across indentation is the largest part of that,
  and is untested.
- **Text rescans.** They are not a large lever on their own. The larger part of text
  scanning runs over built strings, in measuring candidate results.
- **What remains unmeasured.** No single mechanism measured so far explains the 2.3–2.4x
  agreement-corpus gap. What remains is the per-rewrite cost of first attempts:
  allocation, red-tree navigation, string building, string measuring. It is spread across
  the formatter.

## Proposed next measurements (none run)

- **X9b — cost of first attempts per node kind.** Exclusive instructions per first rewrite,
  by kind, on the agreement corpus, against the proof of concept's per-node cost on the
  same items. This locates the 2.3–2.4x gap.
- **X9c — when indentation reuse is exact.** For nodes in the indentation class, record the
  shapes (width, indent) of the rewrites, and test a candidate rule: reuse when the
  result's widest line still fits the new shape and the first attempt was single-line.
  The test passes if the rule never disagrees with a recomputed rewrite on the corpora.
- **X3/X4 — typed accessors on another representation.** As in the previous entry, so that
  representations are compared doing the same work.

The construct for a decide-once or reuse prototype is not chosen. By X9 the candidates are
`match` (mostly indentation repeats) and method chains (layout and indentation repeats).

## Current State

- `CharClasses` classifies the newline that ends a line comment as comment
  (chloro-core/src/formatter/comment.rs:816)
- `CharClasses` treats a `"` inside a block comment as the start of a string, in which `*/`
  does not end the comment — its block comment ends can differ from the lexer's
  (chloro-core/src/formatter/comment.rs:783-799)
- `find_uncommented` accepts characters classified `Normal` or `InString`, and resets a
  partial match on a mismatch without re-checking the mismatched character
  (chloro-core/src/formatter/comment.rs:445-478)
- `span_ends_with_comma` steps `CharClasses` over the whole span to find its last non-comment,
  non-whitespace character (chloro-core/src/formatter/expr.rs:1430-1448)
- `filtered_str_fits` runs `filter_normal_code` on every candidate string before measuring
  its lines (chloro-core/src/formatter/utils.rs:156-182)
- The formatter code is unchanged from `9202b42`

## Missing

- No indentation-relative reuse of rewrite results — `memoize` keys results by exact shape
  (chloro-core/src/formatter/context.rs:271)
- No measurement of speculation inside a single rewrite call — X9 counts repeated rewrites
  of a node only

## Appendix: patches

Each patch applies to `50d2ca8` with `git apply`. Build with
`cargo build --release -p chloro-core --example <x1|x9|x8>`, then run the example on a
directory, e.g. `target/release/examples/x9 chloro-core/tests/conformance/fixtures`.
`x8` takes `X8_CHECK=1` for the equivalence count. Without it, `bench --check` and
callgrind measure the token path. The callgrind sample is every 25th file of the sorted
fixture list.

<details>
<summary><code>x1.diff</code> — X1: operation counters</summary>

```diff
diff --git a/chloro-core/src/formatter/comment.rs b/chloro-core/src/formatter/comment.rs
index 8d1056f..8b0139d 100644
--- a/chloro-core/src/formatter/comment.rs
+++ b/chloro-core/src/formatter/comment.rs
@@ -231,6 +231,7 @@ pub(crate) fn rewrite_comment(
     shape: Shape,
     config: &Settings,
 ) -> Option<String> {
+    crate::x1::rec("rewrite_comment", orig);
     identify_comment(orig, shape, config, false)
 }
 
@@ -443,6 +444,7 @@ pub(crate) trait FindUncommented {
 
 impl FindUncommented for str {
     fn find_uncommented(&self, pat: &str) -> Option<usize> {
+        crate::x1::rec("find_uncommented", self);
         let first = pat.bytes().next().filter(u8::is_ascii);
         let mut needle_iter = pat.chars();
         let mut classes = CharClasses::new(self);
@@ -511,10 +513,12 @@ pub(crate) fn find_comment_end(s: &str) -> Option<usize> {
 
 /// Returns `true` if text contains any comment.
 pub(crate) fn contains_comment(text: &str) -> bool {
+    crate::x1::rec("contains_comment", text);
     // Fast path: every comment starts with `/`.
     if !text.contains('/') {
         return false;
     }
+    crate::x1::rec("contains_comment (scan)", text);
     let mut classes = CharClasses::new(text);
     loop {
         classes.skip_plain(None);
@@ -636,6 +640,7 @@ impl FullCodeCharKind {
 /// Yields `(kind, byte_index, char)`. Lookahead is done directly on the
 /// underlying `&str`, so the iterator never allocates.
 pub(crate) struct CharClasses<'a> {
+    x1_src: bool,
     src: &'a str,
     pos: usize,
     status: CharClassesStatus,
@@ -643,7 +648,9 @@ pub(crate) struct CharClasses<'a> {
 
 impl<'a> CharClasses<'a> {
     pub(crate) fn new(src: &'a str) -> CharClasses<'a> {
+        crate::x1::rec("CharClasses::new", src);
         CharClasses {
+            x1_src: crate::x1::is_src(src),
             src,
             pos: 0,
             status: CharClassesStatus::Normal,
@@ -678,6 +685,7 @@ impl<'a> CharClasses<'a> {
             }
             pos += 1;
         }
+        crate::x1::rec_o("CharClasses::skip_plain (bytes)", self.x1_src, (pos - self.pos) as u64);
         // `pos` is at a char boundary: it is at the end or at an ASCII byte.
         self.pos = pos;
     }
@@ -689,6 +697,7 @@ impl Iterator for CharClasses<'_> {
     fn next(&mut self) -> Option<Self::Item> {
         let idx = self.pos;
         let chr = self.src[idx..].chars().next()?;
+        crate::x1::rec_o("CharClasses::next (chars)", self.x1_src, 1);
         self.pos += chr.len_utf8();
         let mut char_kind = FullCodeCharKind::Normal;
         self.status = match self.status {
@@ -838,6 +847,7 @@ pub(crate) struct LineClasses<'a> {
 
 impl<'a> LineClasses<'a> {
     pub(crate) fn new(s: &'a str) -> Self {
+        crate::x1::rec("LineClasses::new", s);
         LineClasses {
             base: CharClasses::new(s).peekable(),
             src: s,
@@ -899,6 +909,7 @@ struct UngroupedCommentCodeSlices<'a> {
 
 impl<'a> UngroupedCommentCodeSlices<'a> {
     fn new(code: &'a str) -> UngroupedCommentCodeSlices<'a> {
+        crate::x1::rec("UngroupedCommentCodeSlices::new", code);
         UngroupedCommentCodeSlices {
             slice: code,
             iter: CharClasses::new(code),
@@ -974,6 +985,7 @@ pub(crate) struct CommentCodeSlices<'a> {
 
 impl<'a> CommentCodeSlices<'a> {
     pub(crate) fn new(slice: &'a str) -> CommentCodeSlices<'a> {
+        crate::x1::rec("CommentCodeSlices::new", slice);
         CommentCodeSlices {
             slice,
             last_slice_kind: CodeCharKind::Comment,
@@ -1049,6 +1061,8 @@ pub(crate) fn recover_comment_removed(
     context: &RewriteContext<'_>,
 ) -> String {
     let snippet = context.snippet(span);
+    crate::x1::rec("recover_comment_removed (snippet)", snippet);
+    crate::x1::rec("recover_comment_removed (new)", &new);
     if snippet != new && changed_comment_content(snippet, &new) {
         // We missed some comments. Keep the original text. rustfmt reports this as a
         // `LostComment` error, which only matters when formatting a macro definition body.
@@ -1060,9 +1074,11 @@ pub(crate) fn recover_comment_removed(
 }
 
 pub(crate) fn filter_normal_code(code: &str) -> Cow<'_, str> {
+    crate::x1::rec("filter_normal_code", code);
     if !code.contains('/') {
         return Cow::Borrowed(code);
     }
+    crate::x1::rec("filter_normal_code (scan)", code);
     let mut buffer = String::with_capacity(code.len());
     LineClasses::new(code).for_each(|(kind, line)| match kind {
         FullCodeCharKind::Normal
@@ -1087,6 +1103,7 @@ pub(crate) fn filter_normal_code(code: &str) -> Cow<'_, str> {
 /// - whitespace,
 /// - '*' at the beginning of lines in block comments.
 fn changed_comment_content(orig: &str, new: &str) -> bool {
+    crate::x1::rec("changed_comment_content (orig)", orig);
     // Fast path: every comment starts with `/`, and two texts without comments have the
     // same (empty) comment content.
     if !orig.contains('/') && !new.contains('/') {
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..39d908c 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -65,6 +65,10 @@ pub(crate) fn format_text(
     config: &Settings,
     is_macro_def: bool,
 ) -> Option<Formatted> {
+    crate::x1::push_src(source);
+    struct Pop;
+    impl Drop for Pop { fn drop(&mut self) { crate::x1::pop_src() } }
+    let _pop = Pop;
     let file = parse(source, config.edition().to_ra())?;
 
     // `#![rustfmt::skip]` on the file: echo the input.
diff --git a/chloro-core/src/formatter/nodes.rs b/chloro-core/src/formatter/nodes.rs
index 57482e4..2c19403 100644
--- a/chloro-core/src/formatter/nodes.rs
+++ b/chloro-core/src/formatter/nodes.rs
@@ -212,6 +212,7 @@ pub(crate) fn contains_skip(attrs: &[Attribute]) -> bool {
 
 /// The span of `node` without its outer attributes and doc comments.
 pub(crate) fn span_without_attrs(node: &SyntaxNode) -> Span {
+    crate::x1::rec_o("span_without_attrs", true, 0);
     let full = node_range_span(node);
     // The first child that is neither trivia nor an attribute, found on the green tree.
     let mut lo = full.lo();
@@ -236,6 +237,7 @@ pub(crate) fn span_without_attrs(node: &SyntaxNode) -> Span {
 /// Whether `node` has a direct child token of `kind`: a generated `x_token().is_some()`
 /// that reads the green tree instead of creating a red node per child.
 pub(crate) fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
+    crate::x1::rec_o("has_token", true, 0);
     node.green().children().any(|child| {
         matches!(child, NodeOrToken::Token(t) if RustLanguage::kind_from_raw(t.kind()) == kind)
     })
@@ -245,6 +247,7 @@ pub(crate) fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
 /// accessors (`x.expr()`, `x.path()`, ...), but a red node is created only for the child
 /// found, located by a binary search on its range, instead of one per child inspected.
 pub(crate) fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
+    crate::x1::rec_o("nodes::child", true, 0);
     let mut offset = parent.text_range().start();
     for green_child in parent.green().children() {
         let (kind, len) = match green_child {
@@ -272,6 +275,7 @@ pub(crate) fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
 /// The source text of `node`. Reads the green tree, which costs neither a red node per
 /// token nor `fmt` machinery, unlike `node.text().to_string()`.
 pub(crate) fn node_text(node: &SyntaxNode) -> String {
+    crate::x1::rec_o("node_text", true, u32::from(node.text_range().len()) as u64);
     fn push(green: &rowan::GreenNodeData, out: &mut String) {
         for child in green.children() {
             match child {
diff --git a/chloro-core/src/formatter/span.rs b/chloro-core/src/formatter/span.rs
index 7bc36c5..9b8ad7b 100644
--- a/chloro-core/src/formatter/span.rs
+++ b/chloro-core/src/formatter/span.rs
@@ -86,6 +86,7 @@ pub(crate) fn is_outer_doc_text(text: &str) -> bool {
 /// Called for most nodes several times per rewrite, so it reads the green children
 /// directly instead of creating a cursor node per child.
 pub(crate) fn rustc_span(node: &SyntaxNode) -> Span {
+    crate::x1::rec_o("rustc_span", true, 0);
     let full = node_range_span(node);
     let mut lo = full.lo();
     for child in node.green().children() {
@@ -139,7 +140,9 @@ impl<'a> SnippetProvider<'a> {
     }
 
     pub(crate) fn span_to_snippet(&self, span: Span) -> Option<&'a str> {
-        self.src.get(span.lo as usize..span.hi as usize)
+        let r = self.src.get(span.lo as usize..span.hi as usize);
+        if let Some(s) = r { crate::x1::rec("span_to_snippet / snippet", s); }
+        r
     }
 
     pub(crate) fn snippet(&self, span: Span) -> &'a str {
diff --git a/chloro-core/src/formatter/utils.rs b/chloro-core/src/formatter/utils.rs
index b40505a..ba36301 100644
--- a/chloro-core/src/formatter/utils.rs
+++ b/chloro-core/src/formatter/utils.rs
@@ -124,6 +124,7 @@ pub(crate) fn semicolon_for_expr(context: &RewriteContext<'_>, expr: &ast::Expr)
 
 /// Computes the length of the given string, as it would appear in the output.
 pub(crate) fn unicode_str_width(s: &str) -> usize {
+    crate::x1::rec("unicode_str_width", s);
     if !s.is_ascii() {
         return s.width();
     }
@@ -154,6 +155,7 @@ pub(crate) fn wrap_str(s: String, max_width: usize, shape: Shape) -> Option<Stri
 }
 
 pub(crate) fn filtered_str_fits(snippet: &str, max_width: usize, shape: Shape) -> bool {
+    crate::x1::rec("filtered_str_fits", snippet);
     let snippet = &*filter_normal_code(snippet);
     if !snippet.is_empty() {
         // First line must fits with `shape.width`.
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..afcce7d 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x1;
 
 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/chloro-core/src/x1.rs b/chloro-core/src/x1.rs
new file mode 100644
index 0000000..ca1230c
--- /dev/null
+++ b/chloro-core/src/x1.rs
@@ -0,0 +1,30 @@
+
+//! Scratch instrumentation (X1): calls and bytes per operation, split by whether the text
+//! is a slice of a source being formatted or a string built during formatting.
+use std::cell::RefCell;
+use std::collections::BTreeMap;
+#[derive(Default, Clone, Copy)]
+pub struct C { pub src_calls: u64, pub src_bytes: u64, pub built_calls: u64, pub built_bytes: u64 }
+thread_local! {
+    static T: RefCell<BTreeMap<&'static str, C>> = RefCell::new(BTreeMap::new());
+    static SRC: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) };
+}
+pub fn push_src(s: &str) { SRC.with(|v| v.borrow_mut().push((s.as_ptr() as usize, s.as_ptr() as usize + s.len()))) }
+pub fn pop_src() { SRC.with(|v| { v.borrow_mut().pop(); }) }
+pub fn is_src(s: &str) -> bool {
+    let (a, b) = (s.as_ptr() as usize, s.as_ptr() as usize + s.len());
+    SRC.with(|v| v.borrow().iter().any(|&(x, y)| x <= a && b <= y))
+}
+pub fn rec(name: &'static str, s: &str) { let src = is_src(s); rec_o(name, src, s.len() as u64) }
+pub fn rec_o(name: &'static str, src: bool, n: u64) {
+    T.with(|t| { let mut t = t.borrow_mut(); let e = t.entry(name).or_default();
+        if src { e.src_calls += 1; e.src_bytes += n } else { e.built_calls += 1; e.built_bytes += n } })
+}
+pub fn dump() {
+    T.with(|t| {
+        println!("{:42} {:>11} {:>13} {:>11} {:>13}", "operation", "src calls", "src bytes", "built calls", "built bytes");
+        for (k, c) in t.borrow().iter() {
+            println!("{:42} {:>11} {:>13} {:>11} {:>13}", k, c.src_calls, c.src_bytes, c.built_calls, c.built_bytes);
+        }
+    })
+}
diff --git a/chloro-core/examples/x1.rs b/chloro-core/examples/x1.rs
new file mode 100644
index 0000000..7ebb034
--- /dev/null
+++ b/chloro-core/examples/x1.rs
@@ -0,0 +1,9 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let files: Vec<String> = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect();
+    let input: usize = files.iter().map(String::len).sum();
+    let output: usize = files.iter().map(|s| chloro_core::format_source(s).len()).sum();
+    println!("{} files, input {input} bytes, output {output} bytes", files.len());
+    chloro_core::x1::dump();
+}
```

</details>

<details>
<summary><code>x9.diff</code> — X9: first and repeat rewrites</summary>

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
index f3f2db2..63cfa20 100644
--- a/chloro-core/src/formatter/expr.rs
+++ b/chloro-core/src/formatter/expr.rs
@@ -115,10 +115,10 @@ pub(crate) fn format_expr(
     // Paths and literals have no subexpressions: rewriting one again costs less than
     // storing it.
     if matches!(expr, ast::Expr::PathExpr(_) | ast::Expr::Literal(_)) {
-        return format_expr_uncached(expr, expr_type, context, shape);
+        return crate::x9::uncached(expr.syntax(), 0, &(shape, expr_type as u8, context.flags()), || format_expr_uncached(expr, expr_type, context, shape));
     }
     context.memoize(expr.syntax(), expr_type as u8, shape, || {
-        format_expr_uncached(expr, expr_type, context, shape)
+        crate::x9::uncached(expr.syntax(), 0, &(shape, expr_type as u8, context.flags()), || format_expr_uncached(expr, expr_type, context, shape))
     })
 }
 
diff --git a/chloro-core/src/formatter/patterns.rs b/chloro-core/src/formatter/patterns.rs
index bbd6dca..4e9f18f 100644
--- a/chloro-core/src/formatter/patterns.rs
+++ b/chloro-core/src/formatter/patterns.rs
@@ -87,6 +87,12 @@ impl Rewrite for RangeOperand {
 
 impl Rewrite for ast::Pat {
     fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
+        crate::x9::uncached(self.syntax(), 1, &shape, || X9Pat::rewrite_x9(self, context, shape))
+    }
+}
+trait X9Pat { fn rewrite_x9(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String>; }
+impl X9Pat for ast::Pat {
+    fn rewrite_x9(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
         match self {
             ast::Pat::OrPat(or) => {
                 let pats: Vec<ast::Pat> = or.pats().collect();
diff --git a/chloro-core/src/formatter/types.rs b/chloro-core/src/formatter/types.rs
index 0838fbd..d51aea8 100644
--- a/chloro-core/src/formatter/types.rs
+++ b/chloro-core/src/formatter/types.rs
@@ -809,6 +809,12 @@ pub(crate) fn generic_param_span(param: &ast::GenericParam) -> Span {
 
 impl Rewrite for ast::Type {
     fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
+        crate::x9::uncached(self.syntax(), 2, &shape, || X9Type::rewrite_x9(self, context, shape))
+    }
+}
+trait X9Type { fn rewrite_x9(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String>; }
+impl X9Type for ast::Type {
+    fn rewrite_x9(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
         match self {
             ast::Type::DynTraitType(dt) => {
                 // A bare trait object (`'a + Trait`, edition 2015 syntax) stays bare.
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..c32d895 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x9;
 
 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/chloro-core/src/x9.rs b/chloro-core/src/x9.rs
new file mode 100644
index 0000000..015e268
--- /dev/null
+++ b/chloro-core/src/x9.rs
@@ -0,0 +1,83 @@
+
+//! Scratch instrumentation (X9): first against repeat rewrites of a node.
+//! A rewrite is a repeat when the same node (green node, offset) was rewritten before,
+//! under any shape. The outermost repeat runs inside `repeat_outer`, which is never
+//! inlined, so callgrind's inclusive cost of `repeat_outer` is the cost of all repeat work
+//! without double counting nested repeats.
+use std::cell::{Cell, RefCell};
+use std::collections::HashMap;
+use std::hash::{Hash, Hasher, DefaultHasher};
+#[allow(unused_imports)] use std::fmt::Write as _;
+use ra_ap_syntax::{SyntaxKind, SyntaxNode};
+#[derive(Default)]
+pub struct NodeLog { pub kind: Option<SyntaxKind>, pub results: Vec<(u64, u64)>, pub dedented: Vec<u64>, pub repeat_cycles: u64, pub repeat_frames: u64 }
+thread_local! {
+    static IN_REPEAT: Cell<bool> = const { Cell::new(false) };
+    static NODES: RefCell<HashMap<(usize, u32, u8), NodeLog>> = RefCell::new(HashMap::new());
+    static FIRST_OUTER_CYCLES: Cell<u64> = const { Cell::new(0) };
+    static IN_ANY: Cell<bool> = const { Cell::new(false) };
+}
+fn h<T: Hash>(t: &T) -> u64 { let mut s = DefaultHasher::new(); t.hash(&mut s); s.finish() }
+#[inline(never)]
+pub fn repeat_outer<R>(f: impl FnOnce() -> R) -> R { std::hint::black_box(0xA5u8); std::hint::black_box(f()) }
+#[inline(never)]
+pub fn first_outer<R>(f: impl FnOnce() -> R) -> R { std::hint::black_box(0x5Au16); std::hint::black_box(f()) }
+fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+pub fn uncached<S: Hash>(node: &SyntaxNode, cat: u8, shape: &S, f: impl FnOnce() -> Option<String>) -> Option<String> {
+    let id = (&*node.green() as *const _ as *const u8 as usize, u32::from(node.text_range().start()), cat);
+    let seen = NODES.with(|n| n.borrow().get(&id).is_some_and(|l| !l.results.is_empty()));
+    let r;
+    if seen && !IN_REPEAT.get() {
+        IN_REPEAT.set(true);
+        let t = tsc();
+        r = repeat_outer(f);
+        let c = tsc() - t;
+        IN_REPEAT.set(false);
+        NODES.with(|n| { let mut n = n.borrow_mut(); let l = n.entry(id).or_default(); l.repeat_cycles += c; l.repeat_frames += 1; });
+    } else if !IN_ANY.get() {
+        IN_ANY.set(true);
+        let t = tsc();
+        r = first_outer(f);
+        FIRST_OUTER_CYCLES.set(FIRST_OUTER_CYCLES.get() + (tsc() - t));
+        IN_ANY.set(false);
+    } else {
+        r = f();
+    }
+    NODES.with(|n| { let mut n = n.borrow_mut(); let l = n.entry(id).or_default(); l.kind = Some(node.kind()); l.results.push((h(shape), h(&r))); l.dedented.push(h(&r.as_ref().map(|s| s.lines().map(str::trim_start).collect::<Vec<_>>()))); });
+    r
+}
+/// Called between files: node ids are only unique within one tree.
+pub fn file_done() {
+    NODES.with(|n| {
+        let mut n = n.borrow_mut();
+        AGG.with(|a| { let mut a = a.borrow_mut();
+            for (_, l) in n.drain() {
+                let k = format!("{:?}", l.kind.unwrap());
+                let distinct_shapes = { let mut v: Vec<u64> = l.results.iter().map(|r| r.0).collect(); v.sort(); v.dedup(); v.len() };
+                let same = if l.results.iter().all(|r| r.1 == l.results[0].1) { 0u8 } else if l.dedented.iter().all(|d| *d == l.dedented[0]) { 1 } else { 2 };
+                let e = a.entry((k, same)).or_default();
+                e.0 += 1; e.1 += l.results.len() as u64; e.2 += l.repeat_cycles; e.3 += l.repeat_frames; e.4 += distinct_shapes as u64;
+                if l.results.len() > 1 { e.5 += 1 }
+            }
+        });
+    });
+}
+thread_local! { static AGG: RefCell<HashMap<(String, u8), (u64, u64, u64, u64, u64, u64)>> = RefCell::new(HashMap::new()); }
+pub fn first_outer_cycles() -> u64 { FIRST_OUTER_CYCLES.get() }
+pub fn dump(total_cycles: u64) {
+    AGG.with(|a| {
+        let a = a.borrow();
+        let mut v: Vec<_> = a.iter().collect();
+        v.sort_by_key(|(_, e)| std::cmp::Reverse(e.2));
+        let (mut tn, mut tr, mut tc, mut tc_same, mut multi, mut multi_same, mut tc_indent, mut multi_indent) = (0, 0, 0, 0, 0, 0, 0u64, 0u64);
+        println!("{:22} {:>5} {:>8} {:>9} {:>9} {:>14} {:>7}", "kind", "same", "nodes", ">1 rw", "rewrites", "repeat cycles", "% total");
+        for ((k, same), e) in &v {
+            tn += e.0; tr += e.1; tc += e.2; multi += e.5; if *same == 0 { tc_same += e.2; multi_same += e.5 } if *same == 1 { tc_indent += e.2; multi_indent += e.5 }
+            if e.2 * 1000 > total_cycles { println!("{:22} {:>5} {:>8} {:>9} {:>9} {:>14} {:>6.2}%", k, ["same","indent","differ"][*same as usize], e.0, e.5, e.1, e.2, 100.0 * e.2 as f64 / total_cycles as f64); }
+        }
+        println!("nodes {tn}, rewrites {tr}, nodes rewritten more than once {multi} (identical every time: {multi_same}; identical apart from indentation: {multi_indent})");
+        println!("repeat cycles on nodes whose results differ only in indentation: {tc_indent} = {:.1}%", 100.0 * tc_indent as f64 / total_cycles as f64);
+        println!("repeat cycles {tc} = {:.1}% of format_source cycles; on nodes whose every result was identical: {tc_same} = {:.1}%",
+            100.0 * tc as f64 / total_cycles as f64, 100.0 * tc_same as f64 / total_cycles as f64);
+    })
+}
diff --git a/chloro-core/examples/x9.rs b/chloro-core/examples/x9.rs
new file mode 100644
index 0000000..d9ac3e4
--- /dev/null
+++ b/chloro-core/examples/x9.rs
@@ -0,0 +1,16 @@
+fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let files: Vec<String> = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).collect();
+    let mut total = 0;
+    for s in &files {
+        let t = tsc();
+        std::hint::black_box(chloro_core::format_source(s));
+        total += tsc() - t;
+        chloro_core::x9::file_done();
+    }
+    println!("{} files; format_source cycles {total}; outermost first-rewrite cycles {} ({:.1}%)", files.len(),
+        chloro_core::x9::first_outer_cycles(), 100.0 * chloro_core::x9::first_outer_cycles() as f64 / total as f64);
+    chloro_core::x9::dump(total);
+}
```

</details>

<details>
<summary><code>x8.diff</code> — X8: token index for comment queries</summary>

```diff
diff --git a/chloro-core/src/formatter/comment.rs b/chloro-core/src/formatter/comment.rs
index 8d1056f..20a30b5 100644
--- a/chloro-core/src/formatter/comment.rs
+++ b/chloro-core/src/formatter/comment.rs
@@ -443,9 +443,34 @@ pub(crate) trait FindUncommented {
 
 impl FindUncommented for str {
     fn find_uncommented(&self, pat: &str) -> Option<usize> {
+        if crate::x8::check() {
+            let old = find_uncommented_old(self, pat);
+            if let Some(new) = crate::x8::find_uncommented(self, pat) { crate::x8::stat(2); if new != old { crate::x8::stat(3); if crate::x8::stats()[3] <= 5 { eprintln!("mismatch {pat:?} old {old:?} new {new:?} in {:?}", &self[..self.len().min(200)]); } } }
+            return old;
+        }
+        if let Some(new) = crate::x8::find_uncommented(self, pat) { return new; }
+        find_uncommented_old(self, pat)
+    }
+    fn find_last_uncommented(&self, pat: &str) -> Option<usize> {
+        find_last_uncommented_old(self, pat)
+    }
+}
+fn find_last_uncommented_old(this: &str, pat: &str) -> Option<usize> {
+        if let Some(left) = this.find_uncommented(pat) {
+            let mut result = left;
+            while let Some(next) = this[(result + 1)..].find_last_uncommented(pat) {
+                result += next + 1;
+            }
+            Some(result)
+        } else {
+            None
+        }
+}
+fn find_uncommented_old(this: &str, pat: &str) -> Option<usize> {
+    {
         let first = pat.bytes().next().filter(u8::is_ascii);
         let mut needle_iter = pat.chars();
-        let mut classes = CharClasses::new(self);
+        let mut classes = CharClasses::new(this);
         loop {
             // A skipped character is `Normal` and differs from the first character of the
             // pattern, so it would only reset a needle that is already reset.
@@ -473,22 +498,10 @@ impl FindUncommented for str {
         // Handle case where the pattern is a suffix of the search string
         match needle_iter.next() {
             Some(_) => None,
-            None => Some(self.len() - pat.len()),
+            None => Some(this.len() - pat.len()),
         }
     }
 
-    fn find_last_uncommented(&self, pat: &str) -> Option<usize> {
-        if let Some(left) = self.find_uncommented(pat) {
-            let mut result = left;
-            // add 1 to use find_last_uncommented for &str after pat
-            while let Some(next) = self[(result + 1)..].find_last_uncommented(pat) {
-                result += next + 1;
-            }
-            Some(result)
-        } else {
-            None
-        }
-    }
 }
 
 /// Returns the first byte position after the first comment. The given string
@@ -511,6 +524,15 @@ pub(crate) fn find_comment_end(s: &str) -> Option<usize> {
 
 /// Returns `true` if text contains any comment.
 pub(crate) fn contains_comment(text: &str) -> bool {
+    if crate::x8::check() {
+        let old = contains_comment_old(text);
+        if let Some(new) = crate::x8::contains_comment(text) { crate::x8::stat(0); if new != old { crate::x8::stat(1) } }
+        return old;
+    }
+    if let Some(new) = crate::x8::contains_comment(text) { return new; }
+    contains_comment_old(text)
+}
+fn contains_comment_old(text: &str) -> bool {
     // Fast path: every comment starts with `/`.
     if !text.contains('/') {
         return false;
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..b915cc5 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -37,6 +37,7 @@ pub(crate) struct Formatted {
 /// Only lexer and parser errors count. `SourceFile::parse(..).errors()` would also run
 /// rust-analyzer's semantic validation (e.g. `crate` in the middle of a path), which
 /// rustc's parser does not do, and would cost a second traversal of the tree.
+thread_local! { static X8_PENDING: std::cell::RefCell<Option<crate::x8::Index>> = const { std::cell::RefCell::new(None) }; }
 fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
     let lexed = LexedStr::new(edition, source);
     if lexed.errors().next().is_some() {
@@ -46,8 +47,21 @@ fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
     let output = TopEntryPoint::SourceFile.parse(&input, edition);
     let mut builder = SyntaxTreeBuilder::default();
     let mut has_error = false;
+    let base = source.as_ptr() as usize;
+    let mut starts = Vec::with_capacity(lexed.len());
+    let mut comments = Vec::new();
     lexed.intersperse_trivia(&output, &mut |step| match step {
-        StrStep::Token { kind, text } => builder.token(kind, text),
+        StrStep::Token { kind, text } => {
+            let off = (text.as_ptr() as usize - base) as u32;
+            starts.push(off);
+            if kind == ra_ap_syntax::SyntaxKind::COMMENT {
+                // `CharClasses` counts the newline that ends a line comment as comment.
+                let end = off as usize + text.len();
+                let nl = text.starts_with("//") && source.as_bytes().get(end) == Some(&b'\n');
+                comments.push((off, end as u32 + u32::from(nl)));
+            }
+            builder.token(kind, text)
+        }
         StrStep::Enter { kind } => builder.start_node(kind),
         StrStep::Exit => builder.finish_node(),
         StrStep::Error { .. } => has_error = true,
@@ -56,6 +70,7 @@ fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
         return None;
     }
     let file = SourceFile::cast(builder.finish().syntax_node())?;
+    X8_PENDING.with(|p| *p.borrow_mut() = Some(crate::x8::Index { base, len: source.len(), starts, comments }));
     (!rejected_by_rustc(&file, edition)).then_some(file)
 }
 
@@ -66,6 +81,11 @@ pub(crate) fn format_text(
     is_macro_def: bool,
 ) -> Option<Formatted> {
     let file = parse(source, config.edition().to_ra())?;
+    let ix = X8_PENDING.with(|p| p.borrow_mut().take()).expect("index");
+    crate::x8::STACK.with(|s| s.borrow_mut().push(ix));
+    struct Pop;
+    impl Drop for Pop { fn drop(&mut self) { crate::x8::STACK.with(|s| { s.borrow_mut().pop(); }) } }
+    let _pop = Pop;
 
     // `#![rustfmt::skip]` on the file: echo the input.
     if contains_skip(&inner_attributes(file.syntax())) {
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..76380f0 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x8;
 
 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/chloro-core/src/x8.rs b/chloro-core/src/x8.rs
new file mode 100644
index 0000000..168575a
--- /dev/null
+++ b/chloro-core/src/x8.rs
@@ -0,0 +1,76 @@
+
+//! Scratch experiment (X8): answer comment queries on source slices from the lexer's
+//! tokens instead of rescanning text with `CharClasses`.
+//! `X8_CHECK=1` computes both answers, counts disagreements and returns the `CharClasses`
+//! answer; otherwise the token answer is returned whenever the slice is token-aligned.
+use std::cell::{Cell, RefCell};
+pub struct Index { pub base: usize, pub len: usize, pub starts: Vec<u32>, pub comments: Vec<(u32, u32)> }
+thread_local! {
+    pub static STACK: RefCell<Vec<Index>> = const { RefCell::new(Vec::new()) };
+    pub static CHECK: Cell<Option<bool>> = const { Cell::new(None) };
+    pub static STATS: RefCell<[u64; 8]> = const { RefCell::new([0; 8]) };
+}
+pub fn check() -> bool {
+    CHECK.with(|c| match c.get() { Some(v) => v, None => { let v = std::env::var("X8_CHECK").is_ok(); c.set(Some(v)); v } })
+}
+pub fn stat(i: usize) { STATS.with(|s| s.borrow_mut()[i] += 1) }
+pub fn stats() -> [u64; 8] { STATS.with(|s| *s.borrow()) }
+/// The token-aligned range of `s` in the innermost source that contains it.
+fn locate<R>(s: &str, f: impl FnOnce(&Index, u32, u32) -> R) -> Option<R> {
+    let a = s.as_ptr() as usize;
+    STACK.with(|st| {
+        let st = st.borrow();
+        let ix = st.iter().rev().find(|ix| ix.base <= a && a + s.len() <= ix.base + ix.len)?;
+        let lo = (a - ix.base) as u32;
+        let hi = lo + s.len() as u32;
+        let aligned = |p: u32| p as usize == ix.len || ix.starts.binary_search(&p).is_ok();
+        (aligned(lo) && aligned(hi)).then(|| f(ix, lo, hi))
+    })
+}
+pub fn contains_comment(s: &str) -> Option<bool> {
+    locate(s, |ix, lo, hi| {
+        let i = ix.comments.partition_point(|c| c.0 < lo);
+        i < ix.comments.len() && ix.comments[i].0 < hi
+    })
+}
+/// `find_uncommented` with the same naive matcher: comment bytes reset the needle, every
+/// other byte is matched; a mismatch resets the needle without re-checking the byte.
+pub fn find_uncommented(s: &str, pat: &str) -> Option<Option<usize>> {
+    if !pat.is_ascii() || pat.is_empty() { return None; }
+    locate(s, |ix, lo, hi| -> Option<Option<usize>> {
+        // `CharClasses` reads quotes inside block comments as strings, which can end the
+        // comment elsewhere than the lexer does: leave block comments to it.
+        let first = ix.comments.partition_point(|c| c.0 < lo);
+        if ix.comments[first..].iter().take_while(|c| c.0 < hi).any(|c| s.as_bytes()[(c.0 - lo) as usize + 1] == b'*') {
+            return None;
+        }
+        let p = pat.as_bytes();
+        let b = s.as_bytes();
+        let mut k = 0usize;
+        let mut ci = ix.comments.partition_point(|c| c.0 < lo);
+        let mut i = 0usize;
+        let n = b.len();
+        while i < n {
+            // Next comment start within the slice, relative to the slice.
+            let cstart = if ci < ix.comments.len() && ix.comments[ci].0 < hi { (ix.comments[ci].0 - lo) as usize } else { n };
+            if i == cstart {
+                if k == p.len() { return Some(Some(i - p.len())); }
+                k = 0;
+                i = (ix.comments[ci].1 - lo) as usize;
+                ci += 1;
+                continue;
+            }
+            if k == 0 {
+                // Jump to the next occurrence of the first byte before the next comment.
+                match b[i..cstart].iter().position(|&c| c == p[0]) {
+                    Some(off) => i += off,
+                    None => { i = cstart; continue; }
+                }
+            }
+            if k == p.len() { return Some(Some(i - p.len())); }
+            if b[i] == p[k] { k += 1 } else { k = 0 }
+            i += 1;
+        }
+        Some((k == p.len()).then(|| n - p.len()))
+    }).flatten()
+}
diff --git a/chloro-core/examples/x8.rs b/chloro-core/examples/x8.rs
new file mode 100644
index 0000000..42209f0
--- /dev/null
+++ b/chloro-core/examples/x8.rs
@@ -0,0 +1,11 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let mut n = 0;
+    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
+        if e.path().extension().is_none_or(|x| x != "rs") { continue }
+        let Ok(s) = std::fs::read_to_string(e.path()) else { continue };
+        std::hint::black_box(chloro_core::format_source(&s)); n += 1;
+    }
+    let st = chloro_core::x8::stats();
+    println!("{n} files: contains_comment token-answered {} mismatches {}; find_uncommented token-answered {} mismatches {}", st[0], st[1], st[2], st[3]);
+}
```

</details>
