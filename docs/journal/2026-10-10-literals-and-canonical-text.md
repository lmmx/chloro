# 2026-10-10: Literal widths, what the proof of concept copies, and canonical items (X8b, X11a, X12a)

Follows [2026-10-10-first-attempts-and-reuse.md](2026-10-10-first-attempts-and-reuse.md).
This entry covers three experiments:
- **X8b:** literal width;
- **X11a:** classifying why the proof of concept copies text as written;
- **X12a:** bounding what a recogniser of already-canonical text could save.

None changes repository code. All ran on scratch copies whose patches, or a generating
script, are in the appendix. On the host of the previous entry ("Xeon @ 2.10GHz"), every
instrumented build reproduced its uninstrumented outputs on the 1238 fixtures. X8b also did
so on the 5659 registry files.

Labels as before: **measured**, **read**, **hypothesis**.

## Qualifications to the previous entry

Recorded after review:
- **X9b's per-kind figures are diagnostic, not an attribution of the whole gap.** Frame
  boundaries differ between the versions. The agreement corpus lets the proof of concept
  skip work. The per-frame overhead of rdtsc and the frame stack is not calibrated, and
  allocation counts are not allocation time. The ratios hold under that instrumentation
  only.
- **X9c rejects the tested reuse rules, not every reuse strategy.** A rule restricted to
  subtrees without chains stays untested. Given the measured ceiling (8.7% on the fixtures,
  2.2% on the registry, before checking costs), it is not pursued now.
- **The proof of concept's copying is not evidence of a recogniser.** X11a below classifies
  why it copies, and finds none.

## X8b: literal width

### Version 1, refuted: cache the width facts of long literals

`rewrite_literal`'s result for a string literal is the source text or `None`. Only the fit
decision depends on the shape. Version 1 cached the shape-independent facts that
`filtered_str_fits` reads (first-line width, widest later line, last-line width,
continuation flag), computed once per literal of 64 bytes or more by the existing
functions, so the decision is exact by construction.

| | result (measured) |
|---|---|
| shadow check: cached decision against the old path | 13,975 rewrites on the fixtures and 3,207 on the registry; 0 differences |
| instructions, minimum of three, sample | 597.5 M against 595.3 M (+0.4%) |
| instructions, minimum of three, `fn` items | 625.5 M against 615.1 M (+1.7%) |

The cache never pays: literals are almost never rewritten twice. X9b counted 24 repeat
frames against 4,974 first frames on the `fn` items. The 8x per-literal cost of X9b is the
cost of one pass, not of repetition.

### Version 2: skip `filter_normal_code` for string literal tokens

Inside `rewrite_literal` on the `fn` items (callgrind, baseline, measured):
- `filtered_str_fits` is 44.7 M of `rewrite_literal`'s 45.7 M inclusive;
- `LineClasses` is about 22.6 M of that: these test strings contain `//`, which sends
  `filter_normal_code` down its `CharClasses` path;
- `unicode_str_width` is about 21 M.

Version 2 tests the hypothesis that `filter_normal_code` returns a string literal token
unchanged. It computes the same checks as `filtered_str_fits` (`first_line_width`,
`is_single_line`, `lines().skip(1)` widths, `last_line_width`) on the literal text directly.
It does so only for `LitKind::Str` and `LitKind::StrRaw`, and checks the edition-2024
continuation rule in one pass. A shadow mode (`X8B_CHECK=1`) computes the old result on
every call and also checks the identity.

| | result (measured) |
|---|---|
| shadow check, fixtures | 43,734 string literal rewrites; 0 differ; `filter_normal_code` changed the literal 0 times |
| shadow check, registry | 191,698 rewrites; 0 differ; 0 identity failures |
| output hashes | 0 of 1238 fixtures, 0 of 5659 registry files changed |
| instructions, sample (minimum of three) | 590.1 M against 595.3 M (−0.9%, inside the 2% noise) |
| instructions, `fn` items | 589.5 M against 615.1 M (−4.2%) |
| wall, `fn` items, three interleaved pairs | 8.72–8.96 MB/s against 8.22–8.45 MB/s |
| wall, fixtures, three interleaved pairs | 4.94–5.10 MB/s against 4.82–5.08 MB/s (noise) |

Findings:
- **Exactness rests on evidence, not proof.** That `filter_normal_code` leaves a string
  literal token unchanged held on all 235,432 calls; that is evidence, not a proof. A literal
  that `CharClasses` misreads (a raw string containing `"#`-like sequences, or a byte
  string) could break the identity. The shadow check is the guard.
- **The gain is real and small**: about 4% on literal-heavy code, nothing measurable
  overall. Version 2 is a candidate production change. It is not applied, pending a
  decision.

## X11a: why the proof of concept copies text

Method (read, then measured):
- The proof of concept's verbatim copies all go through `.text().to_string()` on a node or
  token, or format a node's `syntax().text()`; 88 call sites.
- A generating script (appendix) makes every such call record its bytes against its call
  site (`#[track_caller]`).
- Each site is then classed by reading its code.

A search of the proof of concept's sources found no comparison of input text with output
text, and no check for whether text is already formatted (read).

Bytes copied as written, by reason (measured; fixtures 13.9 MB, perturbed copy 14.3 MB):

| reason | fixtures | perturbed |
|---|---|---|
| **fallback**: a statement or expression whose formatting returned `None` is copied whole (`node/block.rs:120`, `node/expr/controlflow.rs:137`) | 55.8% | 55.6% |
| **policy**: constructs always copied as written | 20.2% | 20.2% |
| leaf tokens: paths, literals, `_` (rustfmt copies these too) | 4.2% | 4.1% |
| comment text | 2.1% | 2.0% |
| sort keys of `use` items (not output) | 1.7% | 1.7% |

The policy sites, largest first:
- `const` and `static` initialisers (`node/const_static.rs:41`);
- function parameters and return types (`node/function.rs:81`, `:188`);
- `use` trees (`node/useitem.rs:55`);
- closure block bodies (`node/expr/controlflow.rs:574`);
- `match` arm patterns (`controlflow.rs:320`);
- macro expressions (`node/expr.rs:94`);
- attributes (`printer.rs:80`), item macro calls (`node/macrocall.rs:18`) and
  `macro_rules!` (`node.rs:324`).

Classing is by pattern over each site's source, so a site can be misclassed. The two
fallback sites and the largest policy sites were read.

Findings:
- **What the copying is.** The proof of concept copies about three quarters of its input as
  written: 55.8% by fallback (dispatch gaps and unsupported constructs propagating `None`)
  and 20.2% by construct-specific policy.
- **It does not depend on whether the input is canonical.** The shares are the same on
  formatted and perturbed input. The proof of concept has no canonical-text recogniser.
  Its speed on already-formatted code is the speed of not formatting most of it, which
  happens to give the right output when the input is already formatted.

## X12a: what a recogniser of canonical items could save at most

Method (`x12.diff`):
- Every top-level `visit_item` call is timed (rdtsc, inclusive).
- The output it appends to the buffer is compared with the item's source text, both
  trimmed of surrounding whitespace.
- Comments and blank lines between items go into the output slice, so an item next to
  them counts as not canonical. That makes the comparison conservative.

| corpus and options | items canonical | cycles on canonical items (share of `format_source`) | cycles on all top-level items |
|---|---|---|---|
| fixtures, defaults | 7997 of 11,917 (67.1%) | 21.9% | 72.0% |
| rust-analyzer fixtures (1220), defaults | 66.9% | 20.5% | 71.8% |
| rust-analyzer fixtures, `use_small_heuristics=Max` | 84.0% | 63.6% | 71.1% |
| rust-analyzer fixtures, `use_small_heuristics=Max`, `reorder_modules=true` | 84.0% | 63.3% | 71.1% |
| perturbed fixtures, defaults | 133 of 11,917 (1.1%) | 0.2% | 72.6% |
| registry, defaults | 870,246 of 960,275 (90.6%) | 42.9% | 63.8% |

Findings:
- **Bound.** A recogniser that cost nothing and accepted exactly the canonical items would
  save at most 42.9% of `format_source` on the registry (about 1.75x) and 21.9% on the
  fixtures. On perturbed input it would save nothing. That is the largest bound measured
  in this phase. It is a bound under a conservative comparison, not a design.
- **The fixtures are not formatted under rustfmt's defaults.** Under
  `use_small_heuristics=Max`, 84.0% of rust-analyzer's items are canonical, against 66.9%
  under the defaults. rust-analyzer formats its sources with its own `rustfmt.toml`.
  Every fixtures benchmark in this phase has therefore timed a corpus that is about two
  thirds canonical under the configuration being formatted. The registry is closer to
  formatted-default code.
- **Parse and glue.** Top-level items take 64–73% of `format_source`; the rest is parsing
  and top-level glue.

### What a sound recogniser would need

A recogniser saves time only if accepting an item costs less than formatting it, and is
correct only if it never accepts an item whose formatted output differs from its source.
Three classes:
1. **Exact proof for a restricted class.** No class has been identified where proving
   canonicality is cheaper than formatting, beyond leaf tokens, which are already cheap.
   rustfmt's output for even a single-line expression depends on width heuristics
   (`fn_call_width`, `struct_lit_width`, `chain_width`, …) and on normalisations such as
   trailing commas, `dyn` and import sorting. A checker for one construct would
   re-implement that construct's decisions, without building strings. Whether that is
   cheaper is not measured. **Hypothesis.**
2. **Heuristic detection.** Useful only for measuring coverage. It must not skip
   formatting.
3. **Speculative acceptance with validation.** Validating requires formatting, so within
   one run it saves nothing.

A different guarantee is available across runs: a cache from an item's text, the
configuration and the item's context to its formatted text. It is correct if an item's
formatted output depends only on those inputs. The visitor's comments, blank lines between
items, `use` reordering across items, nested indentation and file-level attributes all
carry context, so that condition has to be tested, not assumed. Such a cache speeds up
repeated runs on unchanged code, not a single run.

## Proposed next experiments (none run)

- **X12b — independence of items.** Format every top-level item of the corpora on its own
  (same configuration, indentation 0), and compare with its text from formatting the
  whole file. The share that agrees, and the reasons for disagreement, decide whether a
  cross-run item cache can be correct, and for which items.
- **X3/X4 — representations doing the same work.** As proposed in the change-of-direction
  entry.
- **X8b — production decision.** Apply version 2 to `expr.rs` as a production change, with
  a unit test of the literal identity, or leave it out on the grounds that the overall
  gain is not measurable.

The architecture decision stays open. The measurements so far give these bounds on single
runs:

| what is removed | share of `format_source` |
|---|---|
| repeated rewrites | 17–18.5% |
| indentation reuse, under the tested rules | at most 2.2–8.7% |
| literal scanning | up to about 4% on literal-heavy code |
| work on canonical items, if a sound free recogniser existed | up to 43% |

They also bound the representation: rowan construction caps any formatter at about
13–14 MB/s on the old host (change-of-direction entry).

## Current State

- `rewrite_string_lit` passes every string literal through `wrap_str`, which runs
  `filter_normal_code` on the literal text before measuring its lines
  (chloro-core/src/formatter/expr.rs:1270-1283, chloro-core/src/formatter/utils.rs:156-182)
- `visit_item` formats every top-level item, whether or not its source text is already in
  output form (chloro-core/src/formatter/visitor.rs:373)
- The formatter code is unchanged from `9202b42`

## Missing

- No recogniser of already-canonical source text — every item is formatted
- No cache of formatted items across runs

## Appendix: patches and script

- `x8b2.diff` and `x12.diff` apply to `d136f89`. X8b's shadow check runs with
  `X8B_CHECK=1 target/release/examples/x8b DIR`.
- `X8B_OFF=1` switches version 2 off, for instruction counts against the same binary.
- X12a runs as `target/release/examples/x12 DIR [key=value ...]`; the options are passed
  to `Config::set`.
- `make_x11.py` applies X11a to a checkout of `c4d74ee`. Run it from the checkout root,
  copy this branch's `chloro-core/examples/bench.rs` into the checkout, then run
  `target/release/examples/x11 DIR`. The script reproduces the measured scratch tree
  exactly (checked with `diff -r`).

<details>
<summary><code>x8b2.diff</code> — X8b version 2, with shadow check</summary>

```diff
diff --git a/chloro-core/src/formatter/expr.rs b/chloro-core/src/formatter/expr.rs
index f3f2db2..c849917 100644
--- a/chloro-core/src/formatter/expr.rs
+++ b/chloro-core/src/formatter/expr.rs
@@ -1248,11 +1248,55 @@ pub(crate) fn stmt_is_expr(stmt: &Stmt) -> bool {
     matches!(stmt.kind, StmtKind::Expr(..))
 }

+/// `filtered_str_fits` for the text of a string literal token, without
+/// `filter_normal_code`: the hypothesis under test is that `filter_normal_code` returns a
+/// string literal token unchanged.
+fn literal_fits(text: &str, max_width: usize, shape: Shape) -> bool {
+    use super::utils::{first_line_width, is_single_line};
+    first_line_width(text) <= shape.width
+        && (is_single_line(text)
+            || (!text.lines().skip(1).any(|line| unicode_str_width(line) > max_width)
+                && last_line_width(text) <= shape.used_width() + shape.width))
+}
+
 pub(crate) fn rewrite_literal(
     context: &RewriteContext<'_>,
     kind: LitKind,
     span: Span,
     shape: Shape,
+) -> Option<String> {
+    if !matches!(kind, LitKind::Str | LitKind::StrRaw) || crate::x8b::off() {
+        return rewrite_literal_old(context, kind, span, shape);
+    }
+    let text = context.snippet(span);
+    let keep = (kind == LitKind::Str
+        && context.config.style_edition() >= StyleEdition::Edition2024
+        && {
+            let mut lines = text.lines().peekable();
+            let mut continued = true;
+            while let Some(line) = lines.next() {
+                if lines.peek().is_some() && !line.ends_with('\\') {
+                    continued = false;
+                    break;
+                }
+            }
+            continued
+        })
+        || literal_fits(text, context.config.max_width(), shape);
+    let result = keep.then(|| text.to_owned());
+    if crate::x8b::check() {
+        let old = rewrite_literal_old(context, kind, span, shape);
+        let identity = super::comment::filter_normal_code(text) == text;
+        crate::x8b::count(old != result, !identity);
+    }
+    result
+}
+
+fn rewrite_literal_old(
+    context: &RewriteContext<'_>,
+    kind: LitKind,
+    span: Span,
+    shape: Shape,
 ) -> Option<String> {
     debug_assert!(!context.config.format_strings());
     match kind {
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..f89bbbe 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x8b;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/x8b2/chloro-core/src/x8b.rs b/chloro-core/src/x8b.rs
new file mode 100644
index 0000000..4707b50
--- /dev/null
+++ b/chloro-core/src/x8b.rs
@@ -0,0 +1,17 @@
+
+//! Scratch experiment (X8b v2) switches and counters.
+use std::cell::Cell;
+thread_local! {
+    static OFF: Cell<Option<bool>> = const { Cell::new(None) };
+    static CHECK: Cell<Option<bool>> = const { Cell::new(None) };
+    static N: Cell<(u64, u64, u64)> = const { Cell::new((0, 0, 0)) };
+}
+fn flag(c: &'static std::thread::LocalKey<Cell<Option<bool>>>, var: &str) -> bool {
+    c.with(|c| match c.get() { Some(v) => v, None => { let v = std::env::var_os(var).is_some(); c.set(Some(v)); v } })
+}
+pub fn off() -> bool { flag(&OFF, "X8B_OFF") }
+pub fn check() -> bool { flag(&CHECK, "X8B_CHECK") }
+pub fn count(differs: bool, not_identity: bool) {
+    N.with(|n| { let (a, b, c) = n.get(); n.set((a + 1, b + differs as u64, c + not_identity as u64)) })
+}
+pub fn stats() -> (u64, u64, u64) { N.with(|n| n.get()) }
diff --git a/wt/x8b2/chloro-core/examples/x8b.rs b/chloro-core/examples/x8b.rs
new file mode 100644
index 0000000..8bbcd94
--- /dev/null
+++ b/chloro-core/examples/x8b.rs
@@ -0,0 +1,11 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let mut n = 0;
+    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
+        if e.path().extension().is_none_or(|x| x != "rs") { continue }
+        let Ok(s) = std::fs::read_to_string(e.path()) else { continue };
+        std::hint::black_box(chloro_core::format_source(&s)); n += 1;
+    }
+    let (calls, differ, not_identity) = chloro_core::x8b::stats();
+    println!("{n} files: {calls} string literal rewrites checked; {differ} differ from the old path; filter_normal_code changed the literal {not_identity} times");
+}
```

</details>

<details>
<summary><code>x12.diff</code> — X12a, canonical items</summary>

```diff
diff --git a/chloro-core/src/formatter/visitor.rs b/chloro-core/src/formatter/visitor.rs
index 9429c03..ce11814 100644
--- a/chloro-core/src/formatter/visitor.rs
+++ b/chloro-core/src/formatter/visitor.rs
@@ -371,6 +371,19 @@ impl<'a> FmtVisitor<'a> {
     }

     pub(crate) fn visit_item(&mut self, item: &ast::Item) {
+        if self.block_indent.block_indent != 0 {
+            return self.visit_item_inner(item);
+        }
+        let before = self.buffer.len();
+        let t0 = crate::x12::tsc();
+        self.visit_item_inner(item);
+        let cycles = crate::x12::tsc() - t0;
+        let source = self.snippet(item.span());
+        let same = self.buffer[before..].trim() == source.trim();
+        crate::x12::item(cycles, same);
+    }
+
+    fn visit_item_inner(&mut self, item: &ast::Item) {
         if self.block_indent.block_indent == 0 {
             // No rewrite of a later top-level item reuses one of this item's nodes.
             self.run.clear_memo();
diff --git a/chloro-core/src/lib.rs b/chloro-core/src/lib.rs
index 64baf68..ea060de 100644
--- a/chloro-core/src/lib.rs
+++ b/chloro-core/src/lib.rs
@@ -1,6 +1,7 @@
 //! chloro-core: the formatter behind chloro, a reproduction of `rustfmt --edition 2024`.
 pub mod debug;
 pub mod formatter;
+pub mod x12;

 pub use formatter::config::Config;
 pub use formatter::{format_source, format_source_with_config};
diff --git a/wt/x12/chloro-core/src/x12.rs b/chloro-core/src/x12.rs
new file mode 100644
index 0000000..2a5d217
--- /dev/null
+++ b/chloro-core/src/x12.rs
@@ -0,0 +1,11 @@
+
+//! Scratch measurement (X12a): time spent on top-level items whose formatted text equals
+//! their source text (whitespace trimmed at both ends), against all top-level items.
+use std::cell::Cell;
+thread_local! { pub static T: Cell<[u64; 4]> = const { Cell::new([0; 4]) }; }
+pub fn tsc() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
+/// Records one top-level item: its cycles, and whether its output equals its source.
+pub fn item(cycles: u64, same: bool) {
+    T.with(|t| { let mut v = t.get(); v[0] += 1; v[1] += cycles; if same { v[2] += 1; v[3] += cycles; } t.set(v); })
+}
+pub fn get() -> [u64; 4] { T.with(|t| t.get()) }
diff --git a/wt/x12/chloro-core/examples/x12.rs b/chloro-core/examples/x12.rs
new file mode 100644
index 0000000..22679bb
--- /dev/null
+++ b/chloro-core/examples/x12.rs
@@ -0,0 +1,16 @@
+fn main() {
+    let root = std::env::args().nth(1).unwrap();
+    let mut config = chloro_core::Config::default();
+    for kv in std::env::args().skip(2) { let (k, v) = kv.split_once('=').unwrap(); config.set(k, v).unwrap(); }
+    let (mut total, mut n) = (0u64, 0);
+    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
+        if e.path().extension().is_none_or(|x| x != "rs") { continue }
+        let Ok(s) = std::fs::read_to_string(e.path()) else { continue };
+        let t = chloro_core::x12::tsc();
+        std::hint::black_box(chloro_core::format_source_with_config(&s, &config));
+        total += chloro_core::x12::tsc() - t; n += 1;
+    }
+    let [items, cyc, same, same_cyc] = chloro_core::x12::get();
+    println!("{n} files; format_source {:.0} Mcycles; top-level items {items}, {:.0} Mcycles ({:.1}% of format_source); items whose output equals their source {same} ({:.1}%), {:.0} Mcycles ({:.1}% of format_source)",
+        total as f64 / 1e6, cyc as f64 / 1e6, 100.0 * cyc as f64 / total as f64, 100.0 * same as f64 / items as f64, same_cyc as f64 / 1e6, 100.0 * same_cyc as f64 / total as f64);
+}
```

</details>

<details>
<summary><code>make_x11.py</code> — X11a, applied to c4d74ee</summary>

```python
# Applies the X11a instrumentation to a checkout of c4d74ee (run from the checkout root).
import os
hdr = '#[allow(unused_imports)]\nuse crate::x11::X11;'
targets = {
    'formatter/node/useitem.rs': ('format!("{} ", vis.syntax().text())', 'format!("{} ", vis.syntax().text().to_string().x11())'),
    'formatter/node/function.rs': ('&format!("{},", self_param.syntax().text())', '&format!("{},", self_param.syntax().text().to_string().x11())'),
    'formatter/node/expr/operators.rs': ('cast.ty()?.syntax().text()\n', 'cast.ty()?.syntax().text().to_string().x11()\n'),
    'formatter/node/expr/jumps.rs': ('let_expr.pat()?.syntax().text(),', 'let_expr.pat()?.syntax().text().to_string().x11(),'),
}
os.chdir('chloro-core/src')
for root, _, fs in os.walk('formatter'):
    for f in fs:
        p = os.path.join(root, f)
        if not f.endswith('.rs') or 'tests' in root or f == 'tests.rs':
            continue
        s = o = open(p).read()
        s = s.replace('.text().to_string()', '.text().to_string().x11()')
        if p in targets:
            a, b = targets[p]; assert a in s, (p, a); s = s.replace(a, b)
        if s == o:
            continue
        # The import goes after any leading inner doc comments.
        lines = s.split('\n'); i = 0
        while i < len(lines) and (lines[i].startswith('//!') or (lines[i].strip() == '' and i + 1 < len(lines) and lines[i + 1].startswith('//!'))):
            i += 1
        open(p, 'w').write('\n'.join(lines[:i] + hdr.split('\n') + lines[i:]))
open('lib.rs', 'a').write('''
pub mod x11 {
    //! Scratch instrumentation (X11a): bytes copied as written, per call site.
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::panic::Location;
    thread_local! { static T: RefCell<BTreeMap<String, (u64, u64)>> = const { RefCell::new(BTreeMap::new()) }; }
    pub trait X11 { fn x11(self) -> Self; }
    impl X11 for String {
        #[track_caller]
        fn x11(self) -> String {
            let l = Location::caller();
            T.with(|t| { let mut t = t.borrow_mut(); let e = t.entry(format!("{}:{}", l.file(), l.line())).or_default(); e.0 += 1; e.1 += self.len() as u64; });
            self
        }
    }
    pub fn dump() { T.with(|t| { let mut v: Vec<_> = t.borrow().iter().map(|(k, v)| (k.clone(), *v)).collect(); v.sort_by_key(|x| std::cmp::Reverse(x.1.1)); for (k, (c, b)) in v { println!("{b:>10} bytes {c:>8} calls  {k}"); } }) }
}
''')
os.makedirs('../examples', exist_ok=True)
open('../examples/x11.rs', 'w').write('''fn main() {
    let root = std::env::args().nth(1).unwrap();
    let (mut n, mut inp, mut out) = (0, 0, 0);
    for e in walkdir::WalkDir::new(&root).sort_by_file_name().into_iter().filter_map(|e| e.ok()) {
        if e.path().extension().is_none_or(|x| x != "rs") { continue }
        let Ok(s) = std::fs::read_to_string(e.path()) else { continue };
        inp += s.len(); out += chloro_core::format_source(&s).len(); n += 1;
    }
    println!("{n} files, input {inp} bytes, output {out} bytes");
    chloro_core::x11::dump();
}
''')
```

</details>
