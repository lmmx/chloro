# 2026-10-10: Attribute queries on the green tree (G1), and harness corrections

Follows [2026-10-10-representations.md](2026-10-10-representations.md). This entry covers:
- **G1:** the first experiment that moves real formatter code off red nodes, behind
  unchanged function signatures, measured on the whole formatter;
- corrections to the X3/X4 harness;
- what X12b's timings include;
- a reproducibility gap in the conformance check.

No repository code changed. The patches are in the appendix:
- `g1.diff` applies to `e3c2d84`;
- `x34v2.diff` applies to `e3c2d84` and replaces `x34.diff`.

The host is the "Xeon @ 2.10GHz" of the previous entries. Labels as before:
**measured**, **read**, **hypothesis**.

## Choosing the path (measured)

The callgrind profile of `9202b42` on the 49-file sample was used to attribute every red
node (`rowan::cursor::NodeData::new`, 343,219 calls) to the first chloro function above
it. Calls were apportioned through rowan and `ra_ap_syntax` frames in proportion to call
counts.

| chloro function | share of red nodes |
|---|---|
| `format_expr_uncached` | 8.2% |
| `Block::from_block_expr` | 7.9% |
| `rewrite_ident_pat` | 5.6% |
| `rewrite_chain_from` | 5.3% |
| `Vec<Attribute>` collection (`inner_attributes`, called from `visit_fn`, `expr_attrs`, `visit_item`, `visit_assoc_item`) | 5.1% |
| `rewrite_path_segments` | 4.6% |
| `rewrite_closure_fn_decl` | 4.0% |
| attribute path text (`String` collection through `descendants_with_tokens`, called from `Attribute::path_text` and `attr::attr_meta`) | 3.1% |
| `child::<NameRef>` | 2.9% |
| `outer_attributes` | 2.0% |
| `contains_skip` | 1.3% |

No single function creates most red nodes. Attribute queries form one path, about 10–15%
of red nodes, and their red nodes are mostly created and dropped:
- **`inner_attributes`** walks every child of a function body, module or block as a red
  node, to find inner attributes that are almost never there;
- **attribute path text** walks the red descendants of every attribute's path.

`Block::from_block_expr` keeps its statements as red nodes for later use. Its cost comes
from rebuilding them on every block rewrite, which a cache would address, not a green
read. It was not changed.

## G1: what changed (scratch)

Both changes have unchanged signatures and are exact by construction:
1. **`inner_attributes`** first scans the container's green children. It returns an empty
   list, creating no red node, when no child is an attribute with a `!` token and no
   comment starts with `//!` or `/*!`. Otherwise it runs the existing red code
   (`inner_attributes_red`). The check is conservative: it never decides that something
   *is* an inner attribute.
2. **`non_trivia_text`** collects the text of the non-trivia tokens under a node from the
   green tree. `Attribute::path_text` and `attr::attr_meta` use it in place of
   `descendants_with_tokens().filter(..).map(..).collect()`.

Switches: `G1_OFF=1` runs the old code in the same binary. `G1_CHECK=1` computes both and
counts disagreements.

## G1: results (measured)

| check | fixtures | registry |
|---|---|---|
| `inner_attributes` calls, with green check skipping an attribute the red code finds | 87,091 calls; 0 | 337,042 calls; 0 |
| path text calls, differing | 55,777 calls; 0 | 1,202,483 calls; 0 |
| output hashes against `bcef0f9` | 0 of 1238 changed | 0 of 5659 changed |
| `conform` | 1219/1220 | |
| `conform -i` | 0 not idempotent | |

Instructions on the sample (minimum of three, `setarch -R`). Repeated runs of one binary
agree to within 0.0003% (595,313,458 and 595,313,539 for `9202b42`).

| build | instructions | red nodes | allocations |
|---|---|---|---|
| `9202b42` | 595.3 M | 343,219 | 639,573 |
| G1 binary, `G1_OFF=1` | 594.7 M | 343,219 | 639,577 |
| G1 binary, on | 580.7 M (−2.35%) | 305,788 (−10.9%) | 602,035 (−5.9%) |

Wall clock, interleaved runs of the same binary:

| corpus | `G1_OFF=1` | on |
|---|---|---|
| fixtures, best of 5, three pairs | 4.95–5.07 MB/s | 4.89–5.34 MB/s |
| registry, best of 2, two pairs | 6.68–6.84 MB/s | 6.68–6.92 MB/s |

Findings:
- **The saving is real in instructions.** Moving this one path to the green tree removes 11%
  of red nodes and 6% of allocations, with identical output. It saves 2.35% of
  instructions on the sample, with run-to-run variation 0.0003%.
- **It is below wall-clock noise.** On this host, a 2.35% change does not show in wall
  clock.
- **Scaling, hypothesis.** If the rest of the red-node creation could be moved the same way
  and cost the same per node, the total saving would scale with the share of red nodes
  moved, but each path has to keep the formatter's interfaces and its output. Paths that
  keep red nodes for later use (`Block::from_block_expr`, the typed `ast::*` values passed
  between rewrite functions) do not move as cheaply. Not measured.

## Corrections to X3/X4

Three changes to the harness, following review.

1. **The `transmute` is gone.** The red substrate extended token text to `'static` with
   `transmute`. Now every substrate hands the passes token facts — kind, start, length,
   whether it starts like an inner doc comment, whether it is an outer doc comment —
   instead of borrowed text. The green substrate still holds raw pointers into the
   `GreenNode` its struct owns, for the duration of a pass; this is documented in the
   source.
2. **The span pass is now chloro's `rustc_span` rule,** checked against chloro's own
   `rustc_span` over every node in preorder. The previous span pass computed the first and
   last non-trivia token of each subtree, a different and heavier query, and was checked
   only against the other substrates. Results:

   | corpus | files | `compat` decisions differing from chloro's | span checksums differing from chloro's `rustc_span` |
   |---|---|---|---|
   | fixtures | 1042 | 0 | 0 |
   | registry | 5443 | 0 | 0 |

3. **Re-timed:**

   | step | red | green | flat |
   |---|---|---|---|
   | fixtures, build | 0.516–0.585 s (rowan) | (same) | 0.295–0.331 s |
   | fixtures, `compat` | 0.225–0.257 s | 0.050–0.059 s | 0.060–0.066 s |
   | fixtures, `spans` | 0.273–0.314 s | 0.031–0.038 s | 0.041–0.047 s |
   | fixtures, build + both passes | 1.015–1.155 s | 0.598–0.681 s | 0.396–0.444 s |
   | registry, build + both passes | 9.927 s | 5.727 s | 4.010 s |

   Parser events alone take 0.270–0.305 s on the fixtures; best of 5, two runs.

What changes in the X3/X4 entry's findings:
- **Holds:** red traversal costs 4–9x the green walk for the same answers, and flat
  construction is close to the cost of parser events alone.
- **No longer holds:** "flat and green traverse at about the same cost; flat is 2–10%
  faster". In this harness, flat traversal is 10–40% *slower* than green, because it slices
  the source and derives the token facts per token. The ranking of flat against green
  traversal depends on harness details and is **not established**.
- **New totals:** build plus both passes is 1.7x faster on green than on red, and flat is
  1.4–1.5x faster than green. The earlier 3.7x and 1.43x came from the heavier span
  pass.

The X3/X4 entry carries a pointer to these corrections.

## What X12b's timings include

- **In-file unit cycles:** rdtsc around `visit_item` or `walk_reorderable_items` at the top
  level. They include the gap text the visitor formats at the start of the call (comments
  and blank lines before the item), memo clearing, and the timer's own overhead. They
  exclude parsing and file-level checks.
- **Standalone cycles:** rdtsc around a whole `format_source` call on the unit's text.
  That includes normalisation, lexing, parsing, `rustc_compat`, formatting and output
  assembly.
- The two are not the same quantity. Their ratio (1.6x on the fixtures, 3.4x on the
  registry) shows that standalone formatting is not a cheap validation. It is not a precise
  cost model.
- The independence result, output equality, does not depend on these timings.

## Reproducibility gap in the conformance check (read)

- **Untracked snapshots.** `.gitignore` (lines 1–2, present on `master` at `c4d74ee`)
  ignores `chloro-core/tests/conformance/snapshots/ra/parser/` and `.../ra/syntax/`.
  - The working tree holds 3,685 snapshot files; git tracks 1,942.
  - `examples/conform.rs` run on a fresh checkout (`git archive`) finds 639 cases, not 1,220.
- **Consequence.** The 1219/1220 conformance figure of this branch depends on 581 snapshot
  files present in this container but not in the repository.
- **Recreating them** needs rustfmt 1.9.0 and the snapshot generation of commit `fe28548`.
  The output hashes and the option matrix do not depend on these files: the hashes are
  chloro's own outputs, and the matrix runs rustfmt.

## Current State

- `inner_attributes` collects inner attributes by iterating every child of the container as
  a red node (chloro-core/src/formatter/nodes.rs:196-207)
- `Attribute::path_text` and `attr_meta` build an attribute's path text from red descendant
  tokens (chloro-core/src/formatter/nodes.rs:55-68, chloro-core/src/formatter/attr.rs:221-230)
- `.gitignore` ignores the `ra/parser` and `ra/syntax` conformance snapshot directories —
  1,743 snapshot files there, covering 581 conformance cases, are not tracked by git (.gitignore:1-2)
- The formatter code is unchanged from `9202b42`

## Missing

- No green-tree versions of the remaining red-node paths, such as `format_expr_uncached`'s
  casts, `rewrite_ident_pat`, `rewrite_chain_from` and `rewrite_path_segments`
- No tracked copy of the `ra/parser` and `ra/syntax` conformance snapshots

## Appendix: patches

- `g1.diff` applies to `e3c2d84`. Run `G1_CHECK=1 target/release/examples/g1 DIR` for the
  equivalence counts.
- `bench` runs with and without `G1_OFF=1` for the comparison.
- `x34v2.diff` applies to `e3c2d84` and replaces the X3/X4 entry's `x34.diff`.

<details>
<summary><code>g1.diff</code> — G1, attribute queries on the green tree</summary>

```diff
diff --git a/chloro-core/src/formatter/attr.rs b/chloro-core/src/formatter/attr.rs
index 8ca2a05..eda8224 100644
--- a/chloro-core/src/formatter/attr.rs
+++ b/chloro-core/src/formatter/attr.rs
@@ -221,13 +221,7 @@ fn parse_meta_list(tt: &SyntaxNode) -> Option<Vec<MetaItemInner>> {
 pub(crate) fn attr_meta(attr: &ast::Attr) -> Option<MetaItem> {
     let meta = child::<ast::Meta>(attr.syntax())?;
     let path_node = meta.path()?;
-    let path: String = path_node
-        .syntax()
-        .descendants_with_tokens()
-        .filter_map(|e| e.into_token())
-        .filter(|t| !t.kind().is_trivia())
-        .map(|t| t.text().to_string())
-        .collect();
+    let path: String = super::nodes::non_trivia_text(path_node.syntax());
     let kind = if let Some(tt) = meta.token_tree() {
         MetaItemKind::List(parse_meta_list(tt.syntax())?)
     } else if let Some(expr) = meta.expr() {
diff --git a/chloro-core/src/formatter/nodes.rs b/chloro-core/src/formatter/nodes.rs
index 57482e4..643aa39 100644
--- a/chloro-core/src/formatter/nodes.rs
+++ b/chloro-core/src/formatter/nodes.rs
@@ -57,14 +57,7 @@ impl Attribute {
             Attribute::Doc(_) => None,
             Attribute::Normal(a) => {
                 let path = child::<ast::Meta>(a.syntax())?.path()?;
-                Some(
-                    path.syntax()
-                        .descendants_with_tokens()
-                        .filter_map(|e| e.into_token())
-                        .filter(|t| !t.kind().is_trivia())
-                        .map(|t| t.text().to_string())
-                        .collect(),
-                )
+                Some(non_trivia_text(path.syntax()))
             }
         }
     }
@@ -143,6 +136,42 @@ pub(crate) fn split_top_level_commas(tt: &SyntaxNode) -> Vec<Vec<SyntaxToken>> {
     parts
 }

+/// The text of the non-trivia tokens under `node`, in order: what
+/// `descendants_with_tokens().filter(not trivia)` collects, read from the green tree.
+pub(crate) fn non_trivia_text(node: &SyntaxNode) -> String {
+    fn push(green: &rowan::GreenNodeData, out: &mut String) {
+        for child in green.children() {
+            match child {
+                NodeOrToken::Node(n) => push(n, out),
+                NodeOrToken::Token(t) => {
+                    if !RustLanguage::kind_from_raw(t.kind()).is_trivia() {
+                        out.push_str(t.text());
+                    }
+                }
+            }
+        }
+    }
+    let red = || -> String {
+        node.descendants_with_tokens()
+            .filter_map(|e| e.into_token())
+            .filter(|t| !t.kind().is_trivia())
+            .map(|t| t.text().to_string())
+            .collect()
+    };
+    if crate::g1::off() {
+        return red();
+    }
+    let mut out = String::new();
+    push(&node.green(), &mut out);
+    if crate::g1::check() {
+        crate::g1::count(2);
+        if out != red() {
+            crate::g1::count(3);
+        }
+    }
+    out
+}
+
 /// `true` if `node` starts with an attribute or an outer doc comment. Reads the green tree,
 /// which avoids creating cursor nodes for the common case of a node without attributes.
 fn has_leading_attrs(node: &SyntaxNode) -> bool {
@@ -191,9 +220,46 @@ pub(crate) fn outer_attributes(node: &SyntaxNode) -> Vec<Attribute> {
     attrs
 }

+/// `false` when no direct child of `container` can be an inner attribute: no attribute with
+/// a `!` token and no comment starting `//!` or `/*!`. Reads the green tree. A `true` is
+/// confirmed by [`inner_attributes_red`].
+fn may_have_inner_attrs(container: &SyntaxNode) -> bool {
+    container.green().children().any(|child| match child {
+        NodeOrToken::Node(n) => {
+            RustLanguage::kind_from_raw(n.kind()) == SyntaxKind::ATTR
+                && n.children().any(|c| {
+                    matches!(c, NodeOrToken::Token(t) if RustLanguage::kind_from_raw(t.kind()) == T![!])
+                })
+        }
+        NodeOrToken::Token(t) => {
+            RustLanguage::kind_from_raw(t.kind()) == SyntaxKind::COMMENT
+                && (t.text().starts_with("//!") || t.text().starts_with("/*!"))
+        }
+    })
+}
+
 /// Inner attributes (`#![...]`, `//!`) that are direct children of a container node such as
 /// a source file, an item list or a statement list.
 pub(crate) fn inner_attributes(container: &SyntaxNode) -> Vec<Attribute> {
+    if crate::g1::off() {
+        return inner_attributes_red(container);
+    }
+    let fast = !may_have_inner_attrs(container);
+    if crate::g1::check() {
+        crate::g1::count(0);
+        let red = inner_attributes_red(container);
+        if fast && !red.is_empty() {
+            crate::g1::count(1);
+        }
+        return red;
+    }
+    if fast {
+        return Vec::new();
+    }
+    inner_attributes_red(container)
+}
+
+fn inner_attributes_red(container: &SyntaxNode) -> Vec<Attribute> {
     container
         .children_with_tokens()
         .filter_map(|child| match child {
diff --git a/wt/g1/chloro-core/src/g1.rs b/chloro-core/src/g1.rs
new file mode 100644
index 0000000..dab28fd
--- /dev/null
+++ b/chloro-core/src/g1.rs
@@ -0,0 +1,17 @@
+
+//! Scratch experiment (G1) switches and counters: `G1_OFF=1` uses the red-node code only;
+//! `G1_CHECK=1` computes both and counts disagreements.
+use std::cell::Cell;
+thread_local! {
+    static OFF: Cell<Option<bool>> = const { Cell::new(None) };
+    static CHECK: Cell<Option<bool>> = const { Cell::new(None) };
+    static N: Cell<[u64; 4]> = const { Cell::new([0; 4]) };
+}
+fn flag(c: &'static std::thread::LocalKey<Cell<Option<bool>>>, var: &str) -> bool {
+    c.with(|c| match c.get() { Some(v) => v, None => { let v = std::env::var_os(var).is_some(); c.set(Some(v)); v } })
+}
+pub fn off() -> bool { flag(&OFF, "G1_OFF") }
+pub fn check() -> bool { flag(&CHECK, "G1_CHECK") }
+/// `i`: 0 inner_attributes calls, 1 disagreements, 2 path text calls, 3 disagreements.
+pub fn count(i: usize) { N.with(|n| { let mut v = n.get(); v[i] += 1; n.set(v) }) }
+pub fn stats() -> [u64; 4] { N.with(|n| n.get()) }
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..73ce491 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod g1;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/g1/chloro-core/examples/g1.rs b/chloro-core/examples/g1.rs
new file mode 100644
index 0000000..6c9f5dc
--- /dev/null
+++ b/chloro-core/examples/g1.rs
@@ -0,0 +1,11 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let mut n = 0;
+    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
+        if e.path().extension().is_none_or(|x| x != "rs") { continue }
+        let Ok(s) = std::fs::read_to_string(e.path()) else { continue };
+        std::hint::black_box(chloro_core::format_source(&s)); n += 1;
+    }
+    let s = chloro_core::g1::stats();
+    println!("{n} files: inner_attributes {} calls, {} where the green check skipped attributes the red code finds; path text {} calls, {} differing", s[0], s[1], s[2], s[3]);
+}
```

</details>

<details>
<summary><code>x34v2.diff</code> — X3/X4 harness, corrected</summary>

```diff
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..530a2e8 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -59,6 +59,50 @@ fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
     (!rejected_by_rustc(&file, edition)).then_some(file)
 }

+pub(crate) fn x34_spans(source: &str) -> Option<u64> {
+    let edition = Edition::Edition2024;
+    let lexed = LexedStr::new(edition, source);
+    let input = lexed.to_input(edition);
+    let output = TopEntryPoint::SourceFile.parse(&input, edition);
+    let mut builder = SyntaxTreeBuilder::default();
+    lexed.intersperse_trivia(&output, &mut |step| match step {
+        StrStep::Token { kind, text } => builder.token(kind, text),
+        StrStep::Enter { kind } => builder.start_node(kind),
+        StrStep::Exit => builder.finish_node(),
+        StrStep::Error { .. } => {}
+    });
+    let root = builder.finish().syntax_node();
+    let mut acc: u64 = 0;
+    for n in root.descendants() {
+        let sp = super::span::rustc_span(&n);
+        acc = acc.wrapping_mul(31).wrapping_add(u64::from(sp.lo()) << 32 | u64::from(sp.hi()));
+    }
+    Some(acc)
+}
+
+pub(crate) fn x34_parse_and_check(source: &str) -> Option<bool> {
+    let edition = Edition::Edition2024;
+    let lexed = LexedStr::new(edition, source);
+    if lexed.errors().next().is_some() {
+        return None;
+    }
+    let input = lexed.to_input(edition);
+    let output = TopEntryPoint::SourceFile.parse(&input, edition);
+    let mut builder = SyntaxTreeBuilder::default();
+    let mut has_error = false;
+    lexed.intersperse_trivia(&output, &mut |step| match step {
+        StrStep::Token { kind, text } => builder.token(kind, text),
+        StrStep::Enter { kind } => builder.start_node(kind),
+        StrStep::Exit => builder.finish_node(),
+        StrStep::Error { .. } => has_error = true,
+    });
+    if has_error {
+        return None;
+    }
+    let file = SourceFile::cast(builder.finish().syntax_node())?;
+    Some(rejected_by_rustc(&file, edition))
+}
+
 /// Formats a parsed source text. Returns `None` when the text does not parse.
 pub(crate) fn format_text(
     source: &str,
diff --git a/chloro-core/src/formatter.rs b/chloro-core/src/formatter.rs
index 3fc8168..29da99b 100644
--- a/chloro-core/src/formatter.rs
+++ b/chloro-core/src/formatter.rs
@@ -91,3 +91,15 @@ pub(crate) fn write_indent(buf: &mut String, indent: usize) {
         buf.push(' ');
     }
 }
+
+/// Scratch (X3/X4): chloro's own rustc-rejection decision for a source text, after parsing
+/// it the way `format_source` does; `None` when it does not parse.
+pub fn x34_rejected(source: &str) -> Option<bool> {
+    formatting::x34_parse_and_check(source)
+}
+
+/// Scratch (X3/X4): chloro's `rustc_span` of every node in preorder, folded as the `x34`
+/// example folds its own; `None` when the text does not parse.
+pub fn x34_rustc_spans(source: &str) -> Option<u64> {
+    formatting::x34_spans(source)
+}
diff --git a/wt/x34/chloro-core/examples/x34.rs b/chloro-core/examples/x34.rs
new file mode 100644
index 0000000..6b08175
--- /dev/null
+++ b/chloro-core/examples/x34.rs
@@ -0,0 +1,464 @@
+//! X3/X4: the same two passes over three syntax-tree substrates.
+//!
+//! Substrates, all built from the same lexer, parser and `intersperse_trivia`:
+//! - `red`: rowan's `SyntaxNode`/`SyntaxToken` (a red node per child visited);
+//! - `green`: rowan's `&GreenNodeData`, with byte offsets carried down by the walk;
+//! - `flat`: one preorder array of elements (kind, end of subtree, byte range).
+//!
+//! Passes, written once over the `Cst` trait:
+//! - `compat`: chloro's `rustc_compat` rules, rule for rule (`rustc_compat.rs`);
+//! - `spans`: chloro's `rustc_span` rule for every node in preorder, folded into a checksum
+//!   and compared with chloro's own `rustc_span` over the same nodes.
+//!
+//! `x34 check DIR` compares every substrate's `compat` decision with chloro's own
+//! `rejected_by_rustc` and every `spans` checksum with one computed by chloro's own
+//! `rustc_span` over the same nodes. `x34 time DIR N` times
+//! build, passes and both together, best of N rounds.
+
+use ra_ap_parser::{Edition, LexedStr, StrStep, TopEntryPoint};
+use ra_ap_syntax::{NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, SyntaxTreeBuilder, T};
+use rowan::{GreenNodeData, Language};
+use std::time::Instant;
+
+const ED: Edition = Edition::Edition2024;
+
+/// The operations the two passes need.
+trait Cst {
+    type Node: Clone;
+    /// A child: a node, or a token with its kind, text and byte offset.
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<Self::Node>>;
+    /// Byte range of the node.
+    fn range(&self, n: &Self::Node) -> (u32, u32);
+    fn kind(&self, n: &Self::Node) -> SyntaxKind;
+    fn root(&self) -> Self::Node;
+}
+
+#[derive(Clone)]
+enum Elem<N> {
+    Node(N),
+    Token(Tok),
+}
+
+/// What the passes read from a token: no text is borrowed, so no substrate needs to keep
+/// token text alive beyond its own call.
+#[derive(Clone, Copy)]
+#[allow(dead_code)]
+struct Tok {
+    kind: SyntaxKind,
+    start: u32,
+    len: u32,
+    /// The text starts like an inner doc comment (`//!`, `/*!`).
+    inner_doc_prefix: bool,
+    /// The text is an outer doc comment, by chloro's `is_outer_doc_text`.
+    outer_doc: bool,
+}
+
+fn tok(kind: SyntaxKind, text: &str, start: u32) -> Tok {
+    Tok {
+        kind,
+        start,
+        len: text.len() as u32,
+        inner_doc_prefix: text.starts_with("//!") || text.starts_with("/*!"),
+        outer_doc: (text.starts_with("///") && !text.starts_with("////"))
+            || (text.starts_with("/**") && !text.starts_with("/***") && !text.starts_with("/**/")),
+    }
+}
+
+// ---- red ----
+struct Red(SyntaxNode);
+impl Cst for Red {
+    type Node = SyntaxNode;
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<Self::Node>> {
+        n.children_with_tokens().map(|c| match c {
+            NodeOrToken::Node(c) => Elem::Node(c),
+            NodeOrToken::Token(t) => Elem::Token(tok(t.kind(), t.text(), u32::from(t.text_range().start()))),
+        })
+    }
+    fn range(&self, n: &Self::Node) -> (u32, u32) {
+        let r = n.text_range();
+        (r.start().into(), r.end().into())
+    }
+    fn kind(&self, n: &Self::Node) -> SyntaxKind {
+        n.kind()
+    }
+    fn root(&self) -> Self::Node {
+        self.0.clone()
+    }
+}
+
+// ---- green ----
+// Nodes are raw pointers into the `GreenNode` that `Green` owns: they are only used while
+// the `Green` value is alive and never escape a pass.
+struct Green(rowan::GreenNode);
+impl Cst for Green {
+    /// The node and its byte offset.
+    type Node = (*const GreenNodeData, u32);
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<Self::Node>> {
+        let (g, off) = *n;
+        let g = unsafe { &*g };
+        g.children().scan(off, |at, c| {
+            let start = *at;
+            Some(match c {
+                NodeOrToken::Node(c) => {
+                    *at += u32::from(c.text_len());
+                    Elem::Node((c as *const GreenNodeData, start))
+                }
+                NodeOrToken::Token(t) => {
+                    *at += u32::from(t.text_len());
+                    Elem::Token(tok(RustLanguage::kind_from_raw(t.kind()), t.text(), start))
+                }
+            })
+        })
+    }
+    fn kind(&self, n: &Self::Node) -> SyntaxKind {
+        RustLanguage::kind_from_raw(unsafe { &*n.0 }.kind())
+    }
+    fn range(&self, n: &Self::Node) -> (u32, u32) {
+        (n.1, n.1 + u32::from(unsafe { &*n.0 }.text_len()))
+    }
+    fn root(&self) -> Self::Node {
+        (&*self.0 as *const GreenNodeData, 0)
+    }
+}
+
+// ---- flat ----
+#[derive(Clone, Copy)]
+struct FlatElem {
+    kind: SyntaxKind,
+    node: bool,
+    /// Index after the element's subtree (`index + 1` for a token).
+    end: u32,
+    start: u32,
+    len: u32,
+}
+struct Flat<'s> {
+    src: &'s str,
+    elems: Vec<FlatElem>,
+}
+impl Cst for Flat<'_> {
+    type Node = u32;
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<Self::Node>> {
+        let n = *n;
+        let end = self.elems[n as usize].end;
+        let mut i = n + 1;
+        std::iter::from_fn(move || {
+            if i >= end {
+                return None;
+            }
+            let e = self.elems[i as usize];
+            let cur = i;
+            i = e.end;
+            Some(if e.node {
+                Elem::Node(cur)
+            } else {
+                Elem::Token(tok(e.kind, &self.src[e.start as usize..(e.start + e.len) as usize], e.start))
+            })
+        })
+    }
+    fn kind(&self, n: &Self::Node) -> SyntaxKind {
+        self.elems[*n as usize].kind
+    }
+    fn range(&self, n: &Self::Node) -> (u32, u32) {
+        let e = self.elems[*n as usize];
+        (e.start, e.start + e.len)
+    }
+    fn root(&self) -> Self::Node {
+        0
+    }
+}
+
+// ---- builds ----
+fn events(src: &str) -> (LexedStr<'_>, ra_ap_parser::Output) {
+    let lexed = LexedStr::new(ED, src);
+    let output = TopEntryPoint::SourceFile.parse(&lexed.to_input(ED), ED);
+    (lexed, output)
+}
+fn build_rowan(src: &str) -> SyntaxNode {
+    let (lexed, output) = events(src);
+    let mut b = SyntaxTreeBuilder::default();
+    lexed.intersperse_trivia(&output, &mut |step| match step {
+        StrStep::Token { kind, text } => b.token(kind, text),
+        StrStep::Enter { kind } => b.start_node(kind),
+        StrStep::Exit => b.finish_node(),
+        StrStep::Error { .. } => {}
+    });
+    b.finish().syntax_node()
+}
+fn build_flat<'s>(src: &'s str, elems: &mut Vec<FlatElem>, open: &mut Vec<u32>) -> Flat<'s> {
+    elems.clear();
+    open.clear();
+    let (lexed, output) = events(src);
+    let mut at = 0u32;
+    lexed.intersperse_trivia(&output, &mut |step| match step {
+        StrStep::Token { kind, text } => {
+            // Tokens arrive in source order; a split float token's text is not a slice of
+            // the source, so offsets are counted rather than taken from pointers.
+            let start = at;
+            let len = elems.len() as u32;
+            elems.push(FlatElem { kind, node: false, end: len + 1, start, len: text.len() as u32 });
+            at = start + text.len() as u32;
+        }
+        StrStep::Enter { kind } => {
+            open.push(elems.len() as u32);
+            elems.push(FlatElem { kind, node: true, end: 0, start: at, len: 0 });
+        }
+        StrStep::Exit => {
+            let i = open.pop().unwrap() as usize;
+            elems[i].end = elems.len() as u32;
+            elems[i].len = at - elems[i].start;
+        }
+        StrStep::Error { .. } => {}
+    });
+    Flat { src, elems: std::mem::take(elems) }
+}
+
+// ---- pass 1: rustc_compat, rule for rule ----
+fn has_token<C: Cst>(c: &C, n: &C::Node, k: SyntaxKind) -> bool {
+    c.children(n).any(|e| matches!(e, Elem::Token(t) if t.kind == k))
+}
+fn has_node<C: Cst>(c: &C, n: &C::Node, k: SyntaxKind) -> bool {
+    c.children(n).any(|e| matches!(e, Elem::Node(m) if c.kind(&m) == k))
+}
+fn ekind<C: Cst>(c: &C, e: &Elem<C::Node>) -> SyntaxKind {
+    match e {
+        Elem::Node(m) => c.kind(m),
+        Elem::Token(t) => t.kind,
+    }
+}
+fn has_inner_attr<C: Cst>(c: &C, list: &C::Node) -> bool {
+    c.children(list).any(|e| match e {
+        Elem::Node(m) => c.kind(&m) == SyntaxKind::ATTR && has_token(c, &m, T![!]),
+        Elem::Token(t) => t.kind == SyntaxKind::COMMENT && t.inner_doc_prefix,
+    })
+}
+fn condition_index<C: Cst>(c: &C, n: &C::Node) -> Option<usize> {
+    c.children(n).position(|e| matches!(e, Elem::Node(m) if !matches!(c.kind(&m), SyntaxKind::ATTR | SyntaxKind::LABEL)))
+}
+fn let_expr_allowed<C: Cst>(c: &C, path: &[(C::Node, usize)], edition: Edition) -> bool {
+    let mut chained = false;
+    for (parent, index) in path.iter().rev() {
+        let index = *index;
+        match c.kind(parent) {
+            SyntaxKind::BIN_EXPR if has_token(c, parent, T![&&]) => chained = true,
+            SyntaxKind::IF_EXPR | SyntaxKind::WHILE_EXPR => {
+                return condition_index(c, parent) == Some(index) && (!chained || edition.at_least_2024());
+            }
+            SyntaxKind::MATCH_GUARD => return true,
+            _ => return false,
+        }
+    }
+    false
+}
+fn chained_range<C: Cst>(c: &C, range: &C::Node) -> bool {
+    let has_start = |n: &C::Node| {
+        c.children(n)
+            .find(|e| !ekind(c, e).is_trivia())
+            .is_some_and(|e| matches!(e, Elem::Node(_)))
+    };
+    let mut before_op = true;
+    for e in c.children(range) {
+        if matches!(ekind(c, &e), T![..] | T![..=] | T![...]) {
+            before_op = false;
+            continue;
+        }
+        let Elem::Node(n) = e else { continue };
+        if c.kind(&n) != SyntaxKind::RANGE_EXPR {
+            continue;
+        }
+        if before_op || has_start(&n) {
+            return true;
+        }
+    }
+    false
+}
+fn is_comparison<C: Cst>(c: &C, n: &C::Node) -> bool {
+    c.kind(n) == SyntaxKind::BIN_EXPR
+        && c.children(n).any(|e| matches!(ekind(c, &e), T![==] | T![!=] | T![<] | T![>] | T![<=] | T![>=]))
+}
+fn chained_comparison<C: Cst>(c: &C, n: &C::Node) -> bool {
+    is_comparison(c, n) && c.children(n).any(|e| matches!(e, Elem::Node(m) if is_comparison(c, &m)))
+}
+fn node_rejected<C: Cst>(c: &C, n: &C::Node, path: &[(C::Node, usize)], edition: Edition) -> bool {
+    let parent_kind = path.last().map(|(p, _)| c.kind(p));
+    match c.kind(n) {
+        SyntaxKind::CONST => has_token(c, n, T![mut]),
+        SyntaxKind::IMPL => {
+            !has_token(c, n, T![for])
+                && (has_token(c, n, T![unsafe]) || has_token(c, n, T![default]) || has_token(c, n, T![!]))
+        }
+        SyntaxKind::LET_EXPR => !let_expr_allowed(c, path, edition),
+        SyntaxKind::CLOSURE_EXPR => has_token(c, n, T![static]) && has_token(c, n, T![async]),
+        SyntaxKind::REF_EXPR => {
+            has_token(c, n, T![raw]) && !has_token(c, n, T![const]) && !has_token(c, n, T![mut])
+        }
+        SyntaxKind::CONST_BLOCK_PAT => true,
+        SyntaxKind::TYPE_BOUND => has_node(c, n, SyntaxKind::FOR_BINDER) && has_token(c, n, T![?]),
+        SyntaxKind::REST_PAT => {
+            parent_kind == Some(SyntaxKind::RECORD_PAT_FIELD_LIST) && has_node(c, n, SyntaxKind::ATTR)
+        }
+        SyntaxKind::BLOCK_EXPR => {
+            parent_kind == Some(SyntaxKind::IF_EXPR)
+                && c.children(n).any(|e| matches!(e, Elem::Node(m) if c.kind(&m) == SyntaxKind::STMT_LIST && has_inner_attr(c, &m)))
+        }
+        SyntaxKind::RANGE_EXPR => chained_range(c, n),
+        SyntaxKind::BIN_EXPR => chained_comparison(c, n),
+        _ => false,
+    }
+}
+fn walk<C: Cst>(c: &C, n: &C::Node, path: &mut Vec<(C::Node, usize)>, prev_colon: &mut bool) -> bool {
+    if node_rejected(c, n, path, ED) {
+        return true;
+    }
+    for (index, e) in c.children(n).enumerate() {
+        match e {
+            Elem::Token(t) => {
+                if t.kind == T![::] && *prev_colon {
+                    return true;
+                }
+                *prev_colon = t.kind == T![:];
+            }
+            Elem::Node(m) => {
+                path.push((n.clone(), index));
+                let r = walk(c, &m, path, prev_colon);
+                path.pop();
+                if r {
+                    return true;
+                }
+            }
+        }
+    }
+    false
+}
+fn compat<C: Cst>(c: &C) -> bool {
+    walk(c, &c.root(), &mut Vec::new(), &mut false)
+}
+
+// ---- pass 2: rustc_span of every node, as chloro computes it (span.rs) ----
+/// chloro's `rustc_span`: the node's range, with its start moved past leading whitespace and
+/// plain comments among its direct children (an outer doc comment is significant).
+fn rustc_span<C: Cst>(c: &C, n: &C::Node) -> (u32, u32) {
+    let (lo, hi) = c.range(n);
+    let mut at = lo;
+    for e in c.children(n) {
+        match e {
+            Elem::Node(_) => return (at, hi),
+            Elem::Token(t) => {
+                let significant = match t.kind {
+                    SyntaxKind::WHITESPACE => false,
+                    SyntaxKind::COMMENT => t.outer_doc,
+                    _ => true,
+                };
+                if significant {
+                    return (at, hi);
+                }
+                at += t.len;
+            }
+        }
+    }
+    (lo, hi)
+}
+/// The spans of every node in preorder, folded into a checksum.
+fn spans<C: Cst>(c: &C) -> u64 {
+    fn go<C: Cst>(c: &C, n: &C::Node, acc: &mut u64) {
+        let (s, e) = rustc_span(c, n);
+        *acc = acc.wrapping_mul(31).wrapping_add(u64::from(s) << 32 | u64::from(e));
+        for e in c.children(n) {
+            if let Elem::Node(m) = e {
+                go(c, &m, acc);
+            }
+        }
+    }
+    let mut acc = 0;
+    go(c, &c.root(), &mut acc);
+    acc
+}
+
+fn files(root: &str) -> Vec<String> {
+    let mut v: Vec<_> = walkdir::WalkDir::new(root)
+        .sort_by_file_name()
+        .into_iter()
+        .filter_map(|e| e.ok())
+        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
+        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
+        .map(|s| s.strip_prefix('\u{feff}').unwrap_or(&s).replace("\r\n", "\n"))
+        .collect();
+    // Only files chloro formats: they parse without error.
+    v.retain(|s| chloro_core::formatter::x34_rejected(s).is_some());
+    v
+}
+
+fn main() {
+    let args: Vec<String> = std::env::args().skip(1).collect();
+    let srcs = files(&args[1]);
+    let bytes: usize = srcs.iter().map(String::len).sum();
+    let (mut elems, mut open) = (Vec::new(), Vec::new());
+    match args[0].as_str() {
+        "check" => {
+            let (mut wrong, mut span_diff, mut rejected) = (0, 0, 0);
+            for s in &srcs {
+                let truth = chloro_core::formatter::x34_rejected(s).unwrap();
+                rejected += truth as u32;
+                let red = Red(build_rowan(s));
+                let green = Green(red.0.green().into_owned());
+                let flat = build_flat(s, &mut elems, &mut open);
+                let d = [compat(&red), compat(&green), compat(&flat)];
+                wrong += d.iter().filter(|&&x| x != truth).count();
+                let sp = [spans(&red), spans(&green), spans(&flat)];
+                let truth_sp = chloro_core::formatter::x34_rustc_spans(s).unwrap();
+                span_diff += sp.iter().filter(|&&x| x != truth_sp).count();
+                elems = flat.elems;
+            }
+            println!("{} files ({rejected} rejected by chloro): compat decisions differing from chloro's: {wrong}; span checksums differing from chloro's rustc_span over all nodes: {span_diff}", srcs.len());
+        }
+        "time" => {
+            let rounds: usize = args.get(2).map_or(5, |n| n.parse().unwrap());
+            let mut best = [f64::MAX; 9];
+            let names = ["events only", "build rowan", "build flat", "compat red", "compat green", "compat flat", "spans red", "spans green", "spans flat"];
+            for _ in 0..rounds {
+                let mut t = [0f64; 9];
+                for s in &srcs {
+                    let i = Instant::now();
+                    std::hint::black_box(events(s).1.iter().count());
+                    t[0] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    let red = Red(build_rowan(s));
+                    t[1] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    let flat = build_flat(s, &mut elems, &mut open);
+                    t[2] += i.elapsed().as_secs_f64();
+                    let green = Green(red.0.green().into_owned());
+                    let i = Instant::now();
+                    std::hint::black_box(compat(&red));
+                        t[3] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    std::hint::black_box(compat(&green));
+                    t[4] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    std::hint::black_box(compat(&flat));
+                    t[5] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    std::hint::black_box(spans(&red));
+                        t[6] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    std::hint::black_box(spans(&green));
+                    t[7] += i.elapsed().as_secs_f64();
+                    let i = Instant::now();
+                    std::hint::black_box(spans(&flat));
+                    t[8] += i.elapsed().as_secs_f64();
+                    elems = flat.elems;
+                }
+                for k in 0..9 {
+                    best[k] = best[k].min(t[k]);
+                }
+            }
+            let mb = bytes as f64 / 1e6;
+            println!("{} files, {mb:.1} MB, best of {rounds}", srcs.len());
+            for k in 0..9 {
+                println!("{:14} {:8.3} s {:8.2} MB/s", names[k], best[k], mb / best[k]);
+            }
+            println!("build + compat + spans: rowan/red {:.3} s, rowan/green {:.3} s, flat {:.3} s",
+                best[1] + best[3] + best[6], best[1] + best[4] + best[7], best[2] + best[5] + best[8]);
+        }
+        _ => panic!("check|time"),
+    }
+}
```

</details>
