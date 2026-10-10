# 2026-10-10: G1 in production, a second path off red nodes (G2), conformance reproducibility

Follows [2026-10-10-green-attribute-queries.md](2026-10-10-green-attribute-queries.md).
This entry covers:
- **G1 in production:** G1 applied on its own merits, as one small commit (`eadb4a5`) that
  can be reverted independently, with regression tests;
- **G2:** a second path moved off red nodes, chosen from red-node creation and the
  instruction cost around it. Measured in scratch, not applied;
- **instruction counts under load:** a measurement artefact found while doing this, with
  corrected figures;
- **conformance reproducibility:** whether rustfmt 1.9.0 regenerates the untracked
  snapshots, and the two conformance figures.

The cache and representation decisions stay deferred. The architecture decision stays open.

The host is the "Xeon @ 2.10GHz" of the previous entries. Labels as before:
**measured**, **read**, **hypothesis**.

## G1 in production (measured)

`eadb4a5` is the G1 patch of the previous entry, without the scratch switches (`G1_OFF`,
`G1_CHECK`) and the `g1` module. Two parts:
- **`inner_attributes`** returns early when `may_have_inner_attrs` finds no candidate in
  the container's green children. Otherwise it runs the unchanged cursor code.
- **`non_trivia_text`** builds attribute path text from the green tree. It is used by
  `Attribute::path_text` and `attr::attr_meta`.

Signatures are unchanged. Exactness argument:
- The green check accepts exactly what the cursor code would accept: an `ATTR` node with a
  direct `!` token, or a comment starting `//!` or `/*!`.
- When the check finds a candidate, the cursor code decides; when it finds none, the
  cursor code would have returned nothing.

Regression tests (`formatter::nodes::tests`):

| test | what it checks |
|---|---|
| `inner_attributes_match_a_cursor_walk_on_every_node` | `inner_attributes` equals the replaced cursor computation, kept as a test helper, on *every* node of nine sources |
| `non_trivia_text_matches_a_cursor_walk_on_every_node` | the same for `non_trivia_text` against `descendants_with_tokens` |
| `inner_attributes_are_found` | the three inner attributes of source 0 (`#![..]`, `//!`, `/*! */`) are returned |
| `attribute_paths_drop_whitespace_and_comments` | `#[rustfmt /* c */ :: skip]`, `#[ derive ( Debug ) ]` and `#[cfg_attr(..)]` give `rustfmt::skip`, `derive` and `cfg_attr` |

The nine sources cover:
- inner and outer attributes in files, function bodies, `if` blocks, modules, impls, traits
  and extern blocks;
- `//!` and `/*! */` doc comments, and the comments that resemble doc comments:
  `////`, `/***` and `/**/`;
- outer doc comments on statements;
- attribute paths with comments and whitespace inside, raw identifiers (`r#raw::path`) and
  generic arguments (`a::b::<c>`).

Validation of `eadb4a5`:

| check | result |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets` | clean |
| workspace tests | pass (140 library tests, 4 new) |
| output hashes against `bcef0f9` | 0 of 1238 fixtures, 0 of 5659 registry files changed |
| `conform`, full local corpus | 1219/1220 |
| `conform`, fresh checkout (`git archive HEAD`) | 639/639 |
| `conform -i` | 0 not idempotent, both corpora |
| option matrix (`config_matrix`, 31 options against `rustfmt --config`) | identical, line for line, to the phase-1 result |

The equivalence counts of the previous entry still apply: the shadow run compared old and
new on every call, over 424,133 `inner_attributes` calls and 1,258,260 path-text calls,
with 0 disagreements.

## Instruction counts under load (measured)

Callgrind counts for `9202b42` on the 49-file sample (minimum of three, `setarch -R`):

| when | conditions | instructions |
|---|---|---|
| 06:18–08:48 | "@ 2.80GHz" host, idle | 595,313,4xx |
| 17:36–20:13 | "@ 2.10GHz" host, idle; spans a container restart | 595,313,392–595,313,485 |
| 20:28–20:34 | while `config_matrix` ran rustfmt on all 4 vCPUs (load average about 5.5) | 611,611,003–611,781,296 |
| 20:50 | idle again | 595,313,398 |

- The binary was the same throughout: built at 07:24, before the first of these
  measurements.
- The `bench` example is single-threaded. Its only time-dependent calls are the
  `Instant` reads around each file, which do not change control flow.
- Yet under concurrent load its guest instruction count rose by 2.7% (about 16M).
  Repeated runs under load also spread more widely: 611.6M to 612.4M.
- Idle, the minimum reproduces to within 100 instructions across hours, a host change and
  a container restart.
- The third run of a triple is often a high outlier, even idle (for example 603.9M and
  605.2M against 595.3M). The minimum of three handles this.
- **Mechanism: unknown.**
- **Practice from here:** instruction counts are taken only when nothing else runs. All
  builds being compared are measured in one window. Counts taken under load are discarded.

The G1 and G2 figures first recorded for this entry were taken under load, at 20:34. They
gave G1 −2.56% (about 420 instructions per red node removed) and G2 −0.89% (about 250 per
red node). The idle figures below supersede them. The per-node gap between the two shrinks
when measured idle.

## Choosing the second path (measured)

Red nodes (`rowan::cursor::NodeData::new` calls) after G1 were attributed to the first
chloro frame above them, as in the previous entry. The attribution is apportioned through
rowan and `ra_ap_syntax` frames.

"Inclusive" is the callgrind inclusive instruction count of that frame. Recursion makes it
exceed the total for `format_expr_uncached`. These inclusive counts come from a single run
taken under load, so they are inflated by about 2.7%. The red-node counts are call counts
and are exact.

| chloro frame | red nodes | share | inclusive |
|---|---|---|---|
| `format_expr_uncached` | 28,206 | 9.2% | 748.0 M (recursive) |
| `Block::from_block_expr` | 27,316 | 8.9% | 11.7 M |
| `rewrite_ident_pat` | 19,238 | 6.3% | 7.4 M |
| `rewrite_chain_from` | 18,278 | 6.0% | 143.3 M |
| `rewrite_path_segments` | 15,945 | 5.2% | 14.5 M |
| `rewrite_closure_fn_decl` | 13,851 | 4.5% | 16.5 M |
| `child::<NameRef>` | 9,913 | 3.2% | 3.9 M |
| `Stmt::rewrite` | 7,298 | 2.4% | 302.0 M |
| `outer_attributes` | 6,890 | 2.3% | 6.4 M |
| `child::<Expr>` | 6,852 | 2.2% | 2.7 M |
| `FnSig::from_fn` | 6,407 | 2.1% | 6.0 M |
| `child::<PathSegment>` | 5,974 | 2.0% | 2.3 M |
| `MacroTokens::splice` | 5,969 | 2.0% | 5.7 M |
| `child::<Meta>` | 5,893 | 1.9% | 2.3 M |

Total: 305,788 red nodes.

Candidates considered:
- **`Block::from_block_expr`** creates many red nodes for little inclusive cost: 11.7M, 2%
  of the total. Its nodes are statements kept for later rewriting.
  - Over the sample it is called 2,571 times on 1,486 distinct blocks: 1.73 calls per
    block. On the fixtures, 73,567 calls on 37,941 blocks: 1.94.
  - Most of its red nodes come from rebuilding the same block. That is a cache decision,
    which stays deferred.
- **`format_expr_uncached`, `rewrite_chain_from` and `Stmt::rewrite`** have large
  inclusive costs, but most of it is layout work in their callees. Their red nodes are
  scattered over typed-AST accessors used across the rewrite code.
- **`rewrite_ident_pat`** has a high red-node rate for its cost: 19,238 red nodes in 7.4M
  inclusive. Its red nodes come from the generated `ref_token()` and `mut_token()`
  accessors.
  - Each accessor is `support::token`, which iterates `children_with_tokens()` and creates
    a cursor element for every child it passes, nodes included. Read in
    `ra_ap_syntax-0.0.307`.
  - The same accessors decide a single keyword in three more places:
    - `RefExpr` (`raw_token`, `const_token`, `mut_token`) in `format_expr_uncached`;
    - closures (`async_token`, `gen_token`) in `rewrite_closure_fn_decl`;
    - `let` statements (`super_token`) in `LetStmt::rewrite`.

G2 takes the last group. It is the path with the most red nodes that the green tree can
answer behind the same interfaces, with no cache.

## G2: token presence from the green tree (scratch, measured)

The four call sites use `has_token`, which already existed in `nodes.rs` and reads the
green children. Only presence was used at each site. The `RefExpr` match never read
`const_token`, so it is dropped. The patch (`g2.diff`, appendix) applies to `eadb4a5`. It
needs no new imports and changes no signature.

| check | result |
|---|---|
| output hashes against `bcef0f9` | 0 of 1238, 0 of 5659 changed |
| `conform` | 1219/1220 |
| `conform -i` | 0 not idempotent |

Instructions on the sample, idle, one window, minimum of three:

| build | instructions | red nodes | allocations |
|---|---|---|---|
| `9202b42` | 595,313,398 | 343,219 | 639,576 |
| `eadb4a5` (G1) | 580,651,737 (−2.46%) | 305,788 (−37,431) | 602,035 (−37,541) |
| G2 on `eadb4a5` | 574,205,288 (−1.11% against G1; −3.55% against `9202b42`) | 284,789 (−20,999) | 581,036 (−20,999) |

Instructions saved per red node removed:
- G1: 14.66M over 37,431 red nodes, about 392.
- G2: 6.45M over 20,999, about 307.

Findings:
- **G2 removes 6.9% of the remaining red nodes and 1.1% of instructions, with identical
  output.**
- **Each red node G2 removes is exactly one allocation.** The allocation count falls by
  the red-node count, 20,999. G1 removed 110 more allocations than red nodes: the strings
  of the replaced path-text collection.
- **Savings are not proportional to red nodes removed.** A G1 red node is worth about 390
  instructions; a G2 red node about 310.
  - Hypothesis: G1 also removed the preorder iterator state of `descendants_with_tokens`
    and the `Attr` casts. G2 removes only element creation and drop inside
    `support::token`.
  - Not measured.
  - The scaling estimate of the previous entry (saving proportional to the share of red
    nodes moved) is therefore an upper bound, not a prediction.
- **Wall clock: not measured for G2.** G1's 2.5% did not show above wall-clock noise on
  this host, and G2 is smaller.

G2 is a candidate for the same kind of small production change as G1. It is not applied.

## Conformance reproducibility (measured)

Regeneration: the snapshot regeneration of `fe28548` was repeated for the 581 untracked
`ra/parser` and `ra/syntax` cases. It runs `rustfmt --edition 2024` on stdin and falls back
to the input when rustfmt fails (`tests/conformance/helpers.rs:27-30`). The script
(`regen.py`, appendix) ran with rustfmt `1.9.0-stable (2d8144b788 2026-07-07)` and compared
the result byte for byte:

| outcome | cases |
|---|---|
| rustfmt succeeds; output identical to the snapshot | 365 |
| rustfmt fails (parse errors in `err/` fixtures); snapshot equals the input, as the fallback writes | 216 |
| differing | 0 |
| snapshot missing | 0 |

The untracked snapshots are what rustfmt 1.9.0 produces today. The 1219/1220 figure does
not rest on stale or hand-edited files.

Two figures, both for `eadb4a5`:

| corpus | cases | identical | not idempotent |
|---|---|---|---|
| full local corpus: tracked snapshots plus the 581 regenerated ones | 1,220 | 1,219 | 0 |
| fresh checkout (`git archive HEAD`): tracked snapshots only | 639 | 639 | 0 |

The one difference, `parser/err/0024_many_type_parens.rs`, is among the untracked cases.

Size, if the snapshots were tracked (read):
- the two ignored directories hold 8.9 MB;
- `examples/conform.rs` reads only the `.rustfmt.rs` files (`conform.rs:65`): 581 files,
  3.2 MB;
- the 582 parser and syntax fixtures themselves are tracked.

Whether to track them, track only the `.rustfmt.rs` files, or document regeneration is
open. `.gitignore` is unchanged.

## Current State

- `inner_attributes` scans the container's green children and returns early when none
  can be an inner attribute (chloro-core/src/formatter/nodes.rs:207-224, 228-231)
- Attribute path text is read from the green tree by `non_trivia_text`
  (chloro-core/src/formatter/nodes.rs:139-157, 60, chloro-core/src/formatter/attr.rs:224)
- Regression tests compare both against a cursor walk on every node
  (chloro-core/src/formatter/nodes.rs:612-695)
- Token presence in `rewrite_ident_pat`, `RefExpr`, closures and `let` still uses the
  generated `*_token()` accessors (chloro-core/src/formatter/patterns.rs:212,
  chloro-core/src/formatter/expr.rs:272, chloro-core/src/formatter/closures.rs:238,
  chloro-core/src/formatter/items.rs:136)
- `.gitignore` ignores the `ra/parser` and `ra/syntax` snapshot directories (.gitignore:1-2)

## Missing

- G2 is not applied
- No cache for `Block::from_block_expr` (1.73–1.94 calls per distinct block) — deferred
  with the other cache decisions
- No tracked copy, and no documented regeneration step, for the 581 untracked conformance
  snapshots
- No explanation for instruction counts rising under concurrent load

## Appendix

<details>
<summary><code>g2.diff</code>: token presence from the green tree; applies to <code>eadb4a5</code></summary>

```diff
--- a/chloro-core/src/formatter/patterns.rs
+++ b/chloro-core/src/formatter/patterns.rs
@@ -209,9 +209,9 @@
         ""
     };
     // `ref mut x`: rustc's `ByRef::Yes(Mut)`.
-    let (ref_kw, mut_infix) = match (p.ref_token(), p.mut_token()) {
-        (Some(_), Some(_)) => ("ref", "mut"),
-        (Some(_), None) => ("ref", ""),
+    let (ref_kw, mut_infix) = match (has_token(p.syntax(), T![ref]), has_token(p.syntax(), T![mut])) {
+        (true, true) => ("ref", "mut"),
+        (true, false) => ("ref", ""),
         _ => ("", ""),
     };
     let name = p.name()?;
--- a/chloro-core/src/formatter/expr.rs
+++ b/chloro-core/src/formatter/expr.rs
@@ -269,11 +269,11 @@
             Some(e) => rewrite_unary_prefix(context, "do yeet ", &e, shape),
         },
         ast::Expr::RefExpr(r) => {
-            let operator_str = match (r.raw_token(), r.const_token(), r.mut_token()) {
-                (Some(_), _, Some(_)) => "&raw mut ",
-                (Some(_), _, None) => "&raw const ",
-                (None, _, Some(_)) => "&mut ",
-                (None, _, None) => "&",
+            let operator_str = match (has_token(r.syntax(), T![raw]), has_token(r.syntax(), T![mut])) {
+                (true, true) => "&raw mut ",
+                (true, false) => "&raw const ",
+                (false, true) => "&mut ",
+                (false, false) => "&",
             };
             rewrite_unary_prefix(context, operator_str, &r.expr()?, shape)
         }
--- a/chloro-core/src/formatter/closures.rs
+++ b/chloro-core/src/formatter/closures.rs
@@ -235,11 +235,11 @@
     } else {
         ""
     };
-    let coro = match (closure.async_token(), closure.gen_token()) {
-        (Some(_), Some(_)) => "async gen ",
-        (Some(_), None) => "async ",
-        (None, Some(_)) => "gen ",
-        (None, None) => "",
+    let coro = match (has_token(closure.syntax(), T![async]), has_token(closure.syntax(), T![gen])) {
+        (true, true) => "async gen ",
+        (true, false) => "async ",
+        (false, true) => "gen ",
+        (false, false) => "",
     };
     let capture_str = if has_token(closure.syntax(), T![move]) {
         "move "
--- a/chloro-core/src/formatter/items.rs
+++ b/chloro-core/src/formatter/items.rs
@@ -133,7 +133,7 @@
             return None;
         }
         // rustfmt does not format `super let` yet.
-        if self.super_token().is_some() {
+        if has_token(self.syntax(), T![super]) {
             return None;
         }
         let span = item_span(self.syntax());
```

The long `match` lines would be reflowed by `cargo fmt` in a production commit.

</details>

<details>
<summary><code>regen.py</code>: regenerate and compare the untracked snapshots</summary>

Run as `python3 -I regen.py chloro-core/tests/conformance` with rustfmt 1.9.0 on `PATH`.

```python
# Regenerates the .rustfmt.rs snapshots of the untracked ra/parser and ra/syntax cases with
# the installed rustfmt and compares them byte for byte with the files present.
import os, sys, subprocess, concurrent.futures as cf
root = sys.argv[1]  # chloro-core/tests/conformance
fx = os.path.join(root, 'fixtures', 'rust-analyzer'); snaps = os.path.join(root, 'snapshots', 'ra')
jobs = []
for crate in ('parser', 'syntax'):
    base = os.path.join(fx, crate)
    for d, _, fs in os.walk(base):
        for f in fs:
            if not f.endswith('.rs'): continue
            rel = os.path.relpath(os.path.join(d, f), base)
            key = f"{crate.replace('-', '_')}/{rel[:-3].replace('/', '_')}"
            jobs.append((os.path.join(d, f), os.path.join(snaps, key + '.rustfmt.rs')))
def run(job):
    src, snap = job
    code = open(src, encoding='utf-8').read()
    p = subprocess.run(['rustfmt', '--edition', '2024'], input=code, capture_output=True, text=True)
    out = p.stdout if p.returncode == 0 else None
    have = open(snap, encoding='utf-8').read() if os.path.exists(snap) else None
    return src, p.returncode, out, have, code
same = differ = missing = failed_same_as_input = failed_other = 0
samples = []
with cf.ThreadPoolExecutor(4) as ex:
    for src, rc, out, have, code in ex.map(run, jobs):
        if have is None: missing += 1; continue
        if out is None:
            if have == code: failed_same_as_input += 1
            else: failed_other += 1; samples.append(src)
            continue
        if out == have: same += 1
        else: differ += 1; samples.append(src)
print(f'{len(jobs)} cases: regenerated identical {same}, differing {differ}, snapshot missing {missing}, '
      f'rustfmt failed and snapshot equals input {failed_same_as_input}, rustfmt failed otherwise {failed_other}')
for s in samples[:5]: print('  ', s)
```

</details>
