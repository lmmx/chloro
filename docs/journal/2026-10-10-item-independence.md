# 2026-10-10: Independence of top-level items (X12b)

Follows [2026-10-10-literals-and-canonical-text.md](2026-10-10-literals-and-canonical-text.md).

**Question.** Does each top-level item format the same on its own as it does in its file?
If it does, a cache from an item's text to its formatted text could be correct across runs.

**Scope.** No cache was built and no repository code changed. The measurement ran on a
scratch copy; the patch (`x12b.diff`) is in the appendix and applies to `598e024`. The
instrumented build reproduced the output hashes recorded at `bcef0f9` on the 1238
fixtures. The host is the one of the previous two entries ("Xeon @ 2.10GHz").

## Method

- **Units.** Every top-level item formatted by `FmtVisitor::visit_item` is a unit.
  `use`, `extern crate` and `mod x;` items are formatted in groups, by
  `walk_reorderable_items`, which sorts them within a group. Each such group is one unit,
  with the range from its first item to its last.
- **Only the outermost file is recorded.** `macro_rules!` bodies are formatted as nested
  snippets with metavariables renamed, and their ranges index the snippet. An earlier run
  recorded them too; its "other difference" and most of its "standalone left as written"
  results came from that error, and they disappeared once nested `format_text` calls were
  excluded.
- **In-file output.** For each unit: the text the unit appended to the visitor's buffer,
  with rdtsc cycles, inclusive.
- **Standalone output.** The unit's source range — the source text after the same
  normalisation `format_source` applies (BOM removed, `\r\n` read as `\n`) — formatted on
  its own by `format_source`, with the same default configuration and edition 2024, also
  timed.
- **Normalisation:** only whitespace at both ends of the whole unit's text is trimmed before
  comparing. Nothing inside a unit is normalised.
- **Classes,** tested in this order:
  1. equal;
  2. in-file output carries a comment before the unit — standalone text equals the in-file
     text after that prefix;
  3. in-file output carries other text before the unit;
  4. the same two cases after the unit;
  5. the two differ only in blank lines;
  6. standalone output is the unit's source unchanged;
  7. other difference.

## Results (measured)

| class | fixtures (1238 files) | perturbed fixtures | registry (5659 files) |
|---|---|---|---|
| equal | 13,074 units; 67.1% of `format_source` cycles | 13,074; 67.6% | 958,886; 61.0% |
| in-file carries a comment before the unit | 566; 6.4% | 566; 6.4% | 6,283; 2.0% |
| in-file carries other text before the unit | 6 | 6 | 13 |
| standalone left as written | 4 | 4 | 11 |
| any other class | 0 | 0 | 0 |

Equal units on the registry, by kind:

| kind | units | in-file Mcycles |
|---|---|---|
| `CONST` | 713,896 | 5,508 |
| `IMPL` | 35,964 | 4,964 |
| `MACRO_CALL` | 65,191 | 3,899 |
| `FN` | 13,223 | 3,766 |
| `STRUCT` | 58,134 | 2,222 |
| `MODULE` | 940 | 2,201 |
| `TYPE_ALIAS` | 48,716 | 776 |
| `ENUM` | 8,774 | 493 |
| `MACRO_RULES` | 923 | 317 |
| `EXTERN_BLOCK` | 239 | 211 |
| `TRAIT` | 612 | 190 |
| `STATIC` | 248 | 87 |
| `MODULE` group | 946 | 51 |
| `EXTERN_CRATE` group | 203 | 7 |

The residual classes, read from samples:
- **Comment before the unit.** Comments between items are formatted by the visitor's
  missed-span code, before `visit_item` writes the item. The unit's own text equals its
  standalone text by the definition of the class.
- **Other text before the unit,** and **standalone left as written.**
  - A `#[rustfmt::skip] mod x;` item writes nothing when it is visited. Its text is
    emitted with the gap before the next unit, so the skipped unit is empty in-file and
    the next unit carries it.
  - Cargo-script front matter (`---` blocks) and a `#!` line are file preamble, emitted
    before the first unit.

Findings:
- **Every unit formats as it does alone, gap text aside.** On these corpora, once the text
  between units is set aside (comments, blank lines, skipped items, file preamble), every
  top-level unit's in-file output equals its standalone output:
  - fixtures: 13,650 units;
  - registry: 965,193 units.

  This is evidence that a top-level unit's output depends on its own text and the
  configuration only. It is not a proof.
- **Perturbing the input changes nothing here.** The perturbed fixtures give the same
  classes as the formatted fixtures.
- **Standalone formatting is not a cheap check.** It costs 1.6x the in-file cycles on the
  fixtures (6,810 against 4,317 Mcycles for the equal units) and 3.4x on the registry
  (86,858 against 25,181). Each standalone run parses its unit as a file, and the
  registry's 714k small `CONST` units are dominated by that fixed cost per tree. A cache
  would not format standalone; it would look up the unit's text. Hashing the text was not
  measured; it is a pass over the bytes, against 300–500 cycles per input byte for
  formatting.

## What a cross-run cache would need in its key

From the classes above (read, and measured where stated):

- **In the key:**
  - the unit's text after source normalisation;
  - the whole configuration, including `edition` and `style_edition`;
  - the formatter's version;
  - for reorderable groups, the whole group's text, since sorting depends on every item
    in it.
- **Checked per file, before any lookup.** A parse error, a construct rustc rejects
  (`rustc_compat.rs`) or `#![rustfmt::skip]` anywhere in the file returns the whole file
  unchanged (`chloro-core/src/formatter/formatting.rs:40-75`). A lookup therefore still
  parses the file — about a quarter to a third of `format_source`
  (change-of-direction entry, Evidence 4) — unless the whole file's text is also cached.
- **Outside units:** comments and blank lines between units, skipped items and file
  preamble, which the visitor formats outside units.
- **Not shown to be needed,** on these corpora: neighbouring items, the unit's position
  in the file.
- **Not covered:** items below the top level (inside `mod { }`, `impl` or function bodies),
  which X12b did not test as separate units, and non-default configurations.

## Bound for a warm cache (single file, every unit a hit)

Equal units plus the item part of comment-before units are 73.5% of `format_source` cycles
on the fixtures and 63.0% on the registry. A run on unchanged files where every unit hits
would still parse each file, format the gaps and assemble the output: at best about 26–37%
of the cycles now spent. That is a bound for repeated runs on unchanged code, not for a
first run, and the hit rate on real edits is not measured.

## Current State

- Top-level `use`, `extern crate` and `mod x;` items are formatted in groups and sorted
  within each group by `walk_reorderable_items` (chloro-core/src/formatter/reorder.rs:242,
  chloro-core/src/formatter/reorder.rs:272)
- A `#[rustfmt::skip]` top-level item writes nothing when visited — its text is emitted
  with the gap before the next item (chloro-core/src/formatter/visitor.rs:373-400)
- A parse error, a rustc-rejected construct or `#![rustfmt::skip]` returns the whole file
  unchanged (chloro-core/src/formatter/formatting.rs:40-75)
- The formatter code is unchanged from `9202b42`

## Missing

- No cache of formatted items, within a run or across runs
- No measurement of item independence below the top level, or under non-default options

## Appendix: patch

`x12b.diff` applies to `598e024`. Run it as `target/release/examples/x12b DIR`.
`X12B_SAMPLES="<class>"` prints up to four examples of one class.

<details>
<summary><code>x12b.diff</code> — X12b, in-file against standalone units</summary>

```diff
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..9e9fb33 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -65,6 +65,10 @@ pub(crate) fn format_text(
     config: &Settings,
     is_macro_def: bool,
 ) -> Option<Formatted> {
+    crate::x12b::DEPTH.with(|d| d.set(d.get() + 1));
+    struct Depth;
+    impl Drop for Depth { fn drop(&mut self) { crate::x12b::DEPTH.with(|d| d.set(d.get() - 1)) } }
+    let _depth = Depth;
     let file = parse(source, config.edition().to_ra())?;

     // `#![rustfmt::skip]` on the file: echo the input.
diff --git a/chloro-core/src/formatter/reorder.rs b/chloro-core/src/formatter/reorder.rs
index e692544..e278273 100644
--- a/chloro-core/src/formatter/reorder.rs
+++ b/chloro-core/src/formatter/reorder.rs
@@ -243,6 +243,23 @@ impl FmtVisitor<'_> {
         &mut self,
         items: &[ast::Item],
         item_kind: ReorderableItemKind,
+    ) -> usize {
+        if self.block_indent.block_indent != 0 || !crate::x12b::on() {
+            return self.walk_reorderable_items_inner(items, item_kind);
+        }
+        let before = self.buffer.len();
+        let t0 = crate::x12b::tsc();
+        let n = self.walk_reorderable_items_inner(items, item_kind);
+        let cycles = crate::x12b::tsc() - t0;
+        let (lo, hi) = (items[0].span().lo() as usize, items[n - 1].span().hi() as usize);
+        crate::x12b::push(format!("{:?} group", items[0].syntax().kind()), lo, hi, &self.buffer[before..], cycles);
+        n
+    }
+
+    fn walk_reorderable_items_inner(
+        &mut self,
+        items: &[ast::Item],
+        item_kind: ReorderableItemKind,
     ) -> usize {
         let mut last = self.line_range(items[0].span());
         let item_length = items
diff --git a/chloro-core/src/formatter/visitor.rs b/chloro-core/src/formatter/visitor.rs
index 9429c03..29cf977 100644
--- a/chloro-core/src/formatter/visitor.rs
+++ b/chloro-core/src/formatter/visitor.rs
@@ -371,6 +371,18 @@ impl<'a> FmtVisitor<'a> {
     }

     pub(crate) fn visit_item(&mut self, item: &ast::Item) {
+        if self.block_indent.block_indent != 0 || !crate::x12b::on() {
+            return self.visit_item_inner(item);
+        }
+        let before = self.buffer.len();
+        let t0 = crate::x12b::tsc();
+        self.visit_item_inner(item);
+        let cycles = crate::x12b::tsc() - t0;
+        let span = item.span();
+        crate::x12b::push(format!("{:?}", item.syntax().kind()), span.lo() as usize, span.hi() as usize, &self.buffer[before..], cycles);
+    }
+
+    fn visit_item_inner(&mut self, item: &ast::Item) {
         if self.block_indent.block_indent == 0 {
             // No rewrite of a later top-level item reuses one of this item's nodes.
             self.run.clear_memo();
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..f1fd9e5 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x12b;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/x12b/chloro-core/src/x12b.rs b/chloro-core/src/x12b.rs
new file mode 100644
index 0000000..4c2e0d6
--- /dev/null
+++ b/chloro-core/src/x12b.rs
@@ -0,0 +1,20 @@
+
+//! Scratch measurement (X12b): the output of every top-level item, and of every group of
+//! reorderable top-level items (`use`, `extern crate`, `mod x;`), as formatted in its file,
+//! with the item's source range and the cycles spent on it.
+use std::cell::{Cell, RefCell};
+pub struct Unit { pub kind: String, pub lo: usize, pub hi: usize, pub out: String, pub cycles: u64 }
+thread_local! {
+    pub static UNITS: RefCell<Vec<Unit>> = const { RefCell::new(Vec::new()) };
+    pub static ON: Cell<bool> = const { Cell::new(false) };
+    /// Nesting of `format_text`: macro definition bodies are formatted as nested snippets,
+    /// whose spans index the snippet, not the file. Only depth 1 is recorded.
+    pub static DEPTH: Cell<u32> = const { Cell::new(0) };
+}
+pub fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+pub fn on() -> bool { ON.with(|o| o.get()) && DEPTH.with(|d| d.get()) == 1 }
+pub fn set_on(v: bool) { ON.with(|o| o.set(v)) }
+pub fn push(kind: String, lo: usize, hi: usize, out: &str, cycles: u64) {
+    if on() { UNITS.with(|u| u.borrow_mut().push(Unit { kind, lo, hi, out: out.to_owned(), cycles })) }
+}
+pub fn take() -> Vec<Unit> { UNITS.with(|u| std::mem::take(&mut *u.borrow_mut())) }
diff --git a/wt/x12b/chloro-core/examples/x12b.rs b/chloro-core/examples/x12b.rs
new file mode 100644
index 0000000..e6f9409
--- /dev/null
+++ b/chloro-core/examples/x12b.rs
@@ -0,0 +1,67 @@
+//! X12b driver: in-file against standalone output of every top-level unit.
+use std::collections::BTreeMap;
+use chloro_core::x12b;
+
+/// Why a unit's standalone output differs from its in-file output (or "equal").
+fn classify(src: &str, inf: &str, st: &str) -> String {
+    let (i, s) = (inf.trim(), st.trim());
+    if i == s { return "equal".into() }
+    if let Some(prefix) = i.strip_suffix(s) {
+        let p = prefix.trim();
+        return if p.starts_with("//") || p.starts_with("/*") { "in-file carries a comment before the item".into() }
+               else { "in-file carries other text before the item".into() };
+    }
+    if let Some(suffix) = i.strip_prefix(s) {
+        let p = suffix.trim();
+        return if p.starts_with("//") || p.starts_with("/*") { "in-file carries a comment after the item".into() }
+               else { "in-file carries other text after the item".into() };
+    }
+    if i.lines().filter(|l| !l.trim().is_empty()).eq(s.lines().filter(|l| !l.trim().is_empty())) {
+        return "differs only in blank lines".into();
+    }
+    if st == src || s == src.trim() {
+        return "standalone left as written".into();
+    }
+    "other difference".into()
+}
+
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let samples_of = std::env::var("X12B_SAMPLES").ok();
+    let mut agg: BTreeMap<(String, String), [u64; 3]> = BTreeMap::new();
+    let (mut total, mut files, mut shown) = (0u64, 0, 0);
+    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
+        if e.path().extension().is_none_or(|x| x != "rs") { continue }
+        let Ok(raw) = std::fs::read_to_string(e.path()) else { continue };
+        // The normalisation `format_source` applies before formatting (formatting.rs).
+        let src = raw.strip_prefix('\u{feff}').unwrap_or(&raw).replace("\r\n", "\n");
+        x12b::set_on(true);
+        let t = x12b::tsc();
+        std::hint::black_box(chloro_core::format_source(&src));
+        total += x12b::tsc() - t;
+        x12b::set_on(false);
+        files += 1;
+        for u in x12b::take() {
+            let item = &src[u.lo..u.hi];
+            let t = x12b::tsc();
+            let st = chloro_core::format_source(item);
+            let st_cycles = x12b::tsc() - t;
+            let class = classify(item, &u.out, &st);
+            if samples_of.as_deref() == Some(class.as_str()) && shown < 4 {
+                shown += 1;
+                println!("==== {} {}\n--- in-file\n{}\n--- standalone\n{}", u.kind, e.path().display(), u.out.trim(), st.trim());
+            }
+            let a = agg.entry((u.kind.clone(), class)).or_default();
+            a[0] += 1; a[1] += u.cycles; a[2] += st_cycles;
+        }
+    }
+    println!("{files} files; format_source {:.0} Mcycles", total as f64 / 1e6);
+    let mut by_class: BTreeMap<String, [u64; 3]> = BTreeMap::new();
+    for ((_, c), a) in &agg { let b = by_class.entry(c.clone()).or_default(); for i in 0..3 { b[i] += a[i] } }
+    println!("{:44} {:>9} {:>14} {:>11} {:>16}", "class", "units", "in-file Mcyc", "% of total", "standalone Mcyc");
+    for (c, a) in &by_class { println!("{:44} {:>9} {:>14.1} {:>10.1}% {:>16.1}", c, a[0], a[1] as f64 / 1e6, 100.0 * a[1] as f64 / total as f64, a[2] as f64 / 1e6); }
+    println!("-- by kind (units, in-file Mcycles) for classes other than equal:");
+    for ((k, c), a) in &agg { if c != "equal" && a[0] >= 3 { println!("  {:24} {:44} {:>7} {:>9.1}", k, c, a[0], a[1] as f64 / 1e6); } }
+    println!("-- equal, by kind:");
+    for ((k, c), a) in &agg { if c == "equal" && a[0] >= 20 { println!("  {:24} {:>7} {:>9.1}", k, a[0], a[1] as f64 / 1e6); } }
+}
```

</details>
