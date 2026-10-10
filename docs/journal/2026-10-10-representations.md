# 2026-10-10: Representations doing the same work (X3/X4)

Follows [2026-10-10-item-independence.md](2026-10-10-item-independence.md).

*Corrected in [2026-10-10-green-attribute-queries.md](2026-10-10-green-attribute-queries.md):
the harness no longer uses `transmute`. The span pass now computes chloro's `rustc_span`
rule and is checked against chloro's own `rustc_span`. The flat-against-green traversal
ranking below does not hold under the corrected harness; red-against-green and the
construction results do.*

**Question.** What do three syntax-tree substrates cost when they do identical work? This
compares representations; it does not choose one.

**Scope.** No repository code changed. The experiment ran on a scratch copy; the patch
(`x34.diff`) is in the appendix and applies to `dee6892`. It adds:
- a public wrapper around chloro's own `rejected_by_rustc`, used as ground truth;
- an example, `x34`.

The patched build reproduced the output hashes recorded at `bcef0f9` on the 1238
fixtures. The host is the "Xeon @ 2.10GHz" of the previous entries; wall-clock numbers
are not comparable with the change-of-direction entry's host.

## Method

**Substrates**, all built from the same `LexedStr`, `TopEntryPoint::SourceFile` parse and
`intersperse_trivia`:
- **red:** rowan's `SyntaxNode` and `SyntaxToken`, as `formatting::parse` builds and the
  formatter's typed accessors read them. A red node is created for every child visited.
  Nodes are reference counted and cloned where a pass keeps them.
- **green:** `&GreenNodeData` from the same rowan tree, with byte offsets carried down by
  the walk. No allocation while walking. This is how `rustc_compat.rs` and the phase-1
  helpers in `nodes.rs` already read the tree.
- **flat:** one preorder array of elements: kind, node or token, index after the
  subtree, byte start, length. Children are found by skipping subtrees.
  - Token offsets are counted, not taken from text pointers: a split float token's text
    is not a slice of the source, which an earlier version got wrong.
  - The flat build stores no token text, only ranges into the source, which the
    formatter keeps anyway.

**Passes**, each written once over a small trait (`children`, `kind`, `root`):
1. **`compat`:** chloro's `rustc_compat` rules, rule for rule, with the same ancestor path
   and the same `:::` token check. Like chloro's, it stops at the first rejection.
2. **`spans`:** for every node, the first and last non-trivia token, found by descending
   the leftmost and rightmost paths, as `rustc_span` does. The results are folded into a
   checksum. The formatter asks for a span about 2.5 million times on the fixtures (X1).

**Equivalence check** (`x34 check DIR`):
- each substrate's `compat` decision is compared with chloro's `rejected_by_rustc` on the
  same text;
- the three `spans` checksums are compared with each other;
- only files that parse without error are used: those are the files chloro formats.

**Timing** (`x34 time DIR N`): every step separately, best of N rounds, one thread.

## Results (measured)

Equivalence:

| corpus | files | rejected by chloro | `compat` decisions differing from chloro's | files whose `spans` checksums differ between substrates |
|---|---|---|---|---|
| fixtures | 1042 | 11 | 0 | 0 |
| registry | 5443 | 12 | 0 | 0 |

Timing on the fixtures (13.9 MB, best of 5; two runs within 1%):

| step | red | green | flat |
|---|---|---|---|
| build | 0.555–0.560 s (rowan tree) | (same tree) | 0.319–0.321 s |
| `compat` | 0.230–0.234 s | 0.040 s | 0.036 s |
| `spans` | 2.309–2.329 s | 0.248–0.249 s | 0.236–0.238 s |
| build + `compat` + `spans` | 3.095–3.123 s | 0.844–0.849 s | 0.591–0.594 s |

Parser events alone (lexing and parsing, no tree) take 0.300–0.303 s.

Timing on the registry (122.3 MB, best of 3):

| step | red | green | flat |
|---|---|---|---|
| build | 4.836 s | (same tree) | 2.905 s |
| `compat` | 2.335 s | 0.343 s | 0.336 s |
| `spans` | 13.495 s | 1.426 s | 1.393 s |
| build + `compat` + `spans` | 20.666 s | 6.605 s | 4.634 s |

Parser events alone take 2.569 s.

Findings:
- **Red nodes are the expensive part of traversal.** For the same pass and the same
  answers, walking red nodes costs:
  - `compat`: 5.8x (fixtures) and 6.8x (registry) the green walk;
  - `spans`: 9.3x and 9.5x.
- **Flat and green traverse at about the same cost.** Flat is 2–10% faster across the four
  pass measurements.
- **Flat's advantage is construction.** On this host, building the flat array costs
  0.02 s (fixtures) and 0.34 s (registry) more than parser events alone. Building the
  rowan tree costs 0.25 s and 2.27 s more.
- **End to end,** for build plus both passes:
  - green is 3.7x (fixtures) and 3.1x (registry) faster than red;
  - flat is 1.43x faster than green on both corpora.

Limits:
- **Pass-level, not formatter-level.** The formatter reads most of the tree through
  `ra_ap_syntax`'s typed accessors, which are red. How much of the formatter's time a
  green or flat substrate would remove is **not measured**. The change-of-direction entry's
  profile gives a rough scale, not a bound: red cursor 10.3%, typed accessors 2.9%, and
  part of the allocator's 22% (red nodes are about half of all allocations).
- **No typed layer.** Neither non-red substrate has a typed accessor layer
  (`ast::BinExpr::lhs()` and the rest). Its cost is not measured. The `Cst` trait here
  has three methods; the formatter uses several hundred typed accessors.
- **One pass shape.** The passes walk whole trees once. The formatter's access pattern —
  repeated, local queries during layout attempts — is not reproduced.

## What this changes

- **Rowan without red nodes is a measured option, without a fork.** Most of the traversal
  difference comes from not creating red nodes. A green cursor over rowan's existing tree
  gets it, using rowan's public API: `rustc_compat.rs` already reads the green tree.
  Phase 1 already moved some helpers there (`has_token`, `child::<N>`, `node_text`,
  `span_without_attrs`).
- **A flat tree adds two things over that:** cheaper construction (0.24 s of 0.56 s on the
  fixtures here) and a small traversal gain. The cost is replacing every typed accessor.
- **Neither result decides the architecture.** The formatter-level effect depends on how
  much of the formatter's red access can move to another substrate. That is the next
  measurement this result points to, for example porting one hot formatter path to
  green-only access behind the same functions and timing the whole formatter.

## Current State

- `rustc_compat.rs` walks the green tree with an explicit ancestor path and no red nodes
  (chloro-core/src/formatter/rustc_compat.rs:12-216)
- `has_token`, `child::<N>`, `node_text` and `span_without_attrs` read the green tree and
  create a red node only for a node they return (chloro-core/src/formatter/nodes.rs:213-286)
- The formatter's other syntax access goes through `ra_ap_syntax`'s typed accessors on
  red nodes
- The formatter code is unchanged from `9202b42`

## Missing

- No typed accessor layer over the green tree or a flat array
- No formatter-level measurement of a non-red substrate

## Appendix: patch

`x34.diff` applies to `dee6892`. Run it as `target/release/examples/x34 check DIR` or
`target/release/examples/x34 time DIR ROUNDS`.

<details>
<summary><code>x34.diff</code> — X3/X4, three substrates, two passes</summary>

```diff
diff --git a/chloro-core/src/formatter/formatting.rs b/chloro-core/src/formatter/formatting.rs
index 0ab57aa..e05fb38 100644
--- a/chloro-core/src/formatter/formatting.rs
+++ b/chloro-core/src/formatter/formatting.rs
@@ -59,6 +59,29 @@ fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
     (!rejected_by_rustc(&file, edition)).then_some(file)
 }

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
index 3fc8168..504d27c 100644
--- a/chloro-core/src/formatter.rs
+++ b/chloro-core/src/formatter.rs
@@ -91,3 +91,9 @@ pub(crate) fn write_indent(buf: &mut String, indent: usize) {
         buf.push(' ');
     }
 }
+
+/// Scratch (X3/X4): chloro's own rustc-rejection decision for a source text, after parsing
+/// it the way `format_source` does; `None` when it does not parse.
+pub fn x34_rejected(source: &str) -> Option<bool> {
+    formatting::x34_parse_and_check(source)
+}
diff --git a/wt/x34/chloro-core/examples/x34.rs b/chloro-core/examples/x34.rs
new file mode 100644
index 0000000..01c29dd
--- /dev/null
+++ b/chloro-core/examples/x34.rs
@@ -0,0 +1,461 @@
+//! X3/X4: the same two passes over three syntax-tree substrates.
+//!
+//! Substrates, all built from the same lexer, parser and `intersperse_trivia`:
+//! - `red`: rowan's `SyntaxNode`/`SyntaxToken` (a red node per child visited);
+//! - `green`: rowan's `&GreenNodeData`, with byte offsets carried down by the walk;
+//! - `flat`: one preorder array of elements (kind, end of subtree, byte range).
+//!
+//! Passes, written once over the `Cst` trait:
+//! - `compat`: chloro's `rustc_compat` rules, rule for rule (`rustc_compat.rs`);
+//! - `spans`: for every node, the range from its first to its last non-trivia token, as
+//!   `rustc_span` computes it (leading/trailing whitespace and comments excluded), summed
+//!   into a checksum.
+//!
+//! `x34 check DIR` compares every substrate's `compat` decision with chloro's own
+//! `rejected_by_rustc` and the `spans` checksums with each other. `x34 time DIR N` times
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
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<'_, Self::Node>>;
+    fn kind(&self, n: &Self::Node) -> SyntaxKind;
+    fn root(&self) -> Self::Node;
+}
+
+#[derive(Clone)]
+enum Elem<'a, N> {
+    Node(N),
+    Token(SyntaxKind, &'a str, u32),
+}
+
+// ---- red ----
+struct Red(SyntaxNode);
+impl Cst for Red {
+    type Node = SyntaxNode;
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<'_, Self::Node>> {
+        n.children_with_tokens().map(|c| match c {
+            NodeOrToken::Node(c) => Elem::Node(c),
+            NodeOrToken::Token(t) => {
+                // The token's text lives in the green tree, which the root keeps alive.
+                let text: &'static str = unsafe { std::mem::transmute::<&str, &'static str>(t.text()) };
+                Elem::Token(t.kind(), text, u32::from(t.text_range().start()))
+            }
+        })
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
+struct Green(rowan::GreenNode);
+impl Cst for Green {
+    /// The node and its byte offset.
+    type Node = (*const GreenNodeData, u32);
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<'_, Self::Node>> {
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
+                    Elem::Token(RustLanguage::kind_from_raw(t.kind()), t.text(), start)
+                }
+            })
+        })
+    }
+    fn kind(&self, n: &Self::Node) -> SyntaxKind {
+        RustLanguage::kind_from_raw(unsafe { &*n.0 }.kind())
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
+    fn children(&self, n: &Self::Node) -> impl Iterator<Item = Elem<'_, Self::Node>> {
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
+                Elem::Token(e.kind, &self.src[e.start as usize..(e.start + e.len) as usize], e.start)
+            })
+        })
+    }
+    fn kind(&self, n: &Self::Node) -> SyntaxKind {
+        self.elems[*n as usize].kind
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
+    c.children(n).any(|e| matches!(e, Elem::Token(t, ..) if t == k))
+}
+fn has_node<C: Cst>(c: &C, n: &C::Node, k: SyntaxKind) -> bool {
+    c.children(n).any(|e| matches!(e, Elem::Node(m) if c.kind(&m) == k))
+}
+fn ekind<C: Cst>(c: &C, e: &Elem<'_, C::Node>) -> SyntaxKind {
+    match e {
+        Elem::Node(m) => c.kind(m),
+        Elem::Token(k, ..) => *k,
+    }
+}
+fn has_inner_attr<C: Cst>(c: &C, list: &C::Node) -> bool {
+    c.children(list).any(|e| match e {
+        Elem::Node(m) => c.kind(&m) == SyntaxKind::ATTR && has_token(c, &m, T![!]),
+        Elem::Token(k, text, _) => {
+            k == SyntaxKind::COMMENT && (text.starts_with("//!") || text.starts_with("/*!"))
+        }
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
+            Elem::Token(k, ..) => {
+                if k == T![::] && *prev_colon {
+                    return true;
+                }
+                *prev_colon = k == T![:];
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
+// ---- pass 2: rustc-style spans of every node ----
+/// The (start, end) of the first and last non-trivia token under `n`.
+fn trimmed<C: Cst>(c: &C, n: &C::Node) -> Option<(u32, u32)> {
+    let mut first = None;
+    let mut last = None;
+    for e in c.children(n) {
+        let r = match e {
+            Elem::Token(k, text, start) if !k.is_trivia() => Some((start, start + text.len() as u32)),
+            Elem::Token(..) => None,
+            Elem::Node(m) => trimmed(c, &m),
+        };
+        if let Some((s, e)) = r {
+            first.get_or_insert(s);
+            last = Some(e);
+        }
+    }
+    Some((first?, last?))
+}
+fn spans<C: Cst>(c: &C) -> u64 {
+    // Every node's span, as the formatter asks for it: once per node, from the top.
+    fn go<C: Cst>(c: &C, n: &C::Node, acc: &mut u64) {
+        if let Some((s, e)) = trimmed_shallow(c, n) {
+            *acc = acc.wrapping_mul(31).wrapping_add(u64::from(s) << 32 | u64::from(e));
+        }
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
+/// First and last non-trivia token of `n`, descending only along the leftmost and rightmost
+/// paths (as `rustc_span` does), not through the whole subtree.
+fn trimmed_shallow<C: Cst>(c: &C, n: &C::Node) -> Option<(u32, u32)> {
+    fn first<C: Cst>(c: &C, n: &C::Node) -> Option<u32> {
+        for e in c.children(n) {
+            match e {
+                Elem::Token(k, _, s) if !k.is_trivia() => return Some(s),
+                Elem::Token(..) => {}
+                Elem::Node(m) => {
+                    if let Some(s) = first(c, &m) {
+                        return Some(s);
+                    }
+                }
+            }
+        }
+        None
+    }
+    fn last<C: Cst>(c: &C, n: &C::Node) -> Option<u32> {
+        let mut found = None;
+        for e in c.children(n) {
+            match e {
+                Elem::Token(k, text, s) if !k.is_trivia() => found = Some(s + text.len() as u32),
+                Elem::Token(..) => {}
+                Elem::Node(m) => {
+                    if let Some(e) = last(c, &m) {
+                        found = Some(e);
+                    }
+                }
+            }
+        }
+        found
+    }
+    let _ = trimmed::<C>;
+    Some((first(c, n)?, last(c, n)?))
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
+                if sp[0] != sp[1] || sp[1] != sp[2] {
+                    span_diff += 1;
+                }
+                elems = flat.elems;
+            }
+            println!("{} files ({rejected} rejected by chloro): compat decisions differing from chloro's: {wrong}; files whose span checksums differ between substrates: {span_diff}", srcs.len());
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
