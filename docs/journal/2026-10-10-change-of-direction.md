# 2026-10-10: Change of direction — why the proof of concept was faster

This entry starts a new phase on the same branch (`claude/gifted-wright-d4z4rp`) and the
same PR ([lmmx/chloro#85](https://github.com/lmmx/chloro/pull/85), not merged). Phase 1 is
recorded in [2026-10-10-performance-checkpoint.md](2026-10-10-performance-checkpoint.md).

The phase starts with an investigation, not a code change. No formatter code changed in
this entry. The only code added is measurement tooling under `chloro-core/examples/`, so that
every number below can be reproduced (see Reproduction).

Every statement below is labelled as one of:
- **measured:** produced by a command listed here;
- **probed:** checked by running a formatter on a hand-written input;
- **read:** taken from the source at a cited commit;
- **hypothesis:** not established.

## The change of direction, as asked

- Stop optimising the phase 1 architecture (the rustfmt port) for now.
- Treat the proof of concept (`origin/master`) as a possibly valuable foundation, not a
  disposable prototype. The working assumption to test is that its per-node specialisation
  made it fast (about 8.2 MB/s against the port's 3.5 MB/s), and that giving it up was not
  shown to be necessary for rustfmt conformance.
- Investigate before changing code:
  - the exact baseline commits;
  - why the proof of concept was faster: dispatch, traversal, speculative formatting,
    allocations, intermediate strings, parsing, layout decisions;
  - which of the port's costs rustfmt conformance requires, and which the architecture adds;
  - how the conformance fixes map onto the proof of concept;
  - the smallest experiment that tests the direction;
  - the next performance ceiling, and routes beyond it.
- Then, with evidence, recover the fast architecture as the foundation:
  - a recovery branch from the last fast per-node commit;
  - fixes ported one by one, each measured for both speed and conformance;
  - aim for performance substantially beyond 8.2 MB/s, keeping 1219/1220 conformance.
- Constraints:
  - no rowan fork;
  - parallel formatting of top-level items is deferred;
  - a child cache only after measuring what it would save;
  - avoiding string construction for failed layouts is to be investigated early, but not
    rewritten yet;
  - the 6,861-file output-hash check, the differential rustfmt checks and the benchmark
    corpus all stay;
  - commits are made as Louis Maddox.

## Baseline commits

| role | commit | content |
|---|---|---|
| proof of concept, last fast per-node implementation | `c4d74ee` (`origin/master`) | `chore(pre-commit): autoupdate hooks (#84)`; `master` is untouched and no branch has been created from it |
| port before performance work | `bcef0f9` | `dev: in-process benchmark and output-equivalence check` |
| current formatter code | `9202b42` | last commit that changes formatter code; `ca56dfc` and this entry change only docs and examples |

All three commits use the same toolchain and the same dependency versions (read from each
`Cargo.lock`), so the parser cost is the same in each:
- `rustc 1.97.0 (2d8144b78 2026-07-07)`;
- `ra_ap_syntax`/`ra_ap_parser` 0.0.307, `rowan` 0.15.15, `unicode-width` 0.1.14;
- no `[profile]` overrides.

The proof of concept builds its tree with `SourceFile::parse(source, Edition::CURRENT)`
(`c4d74ee:chloro-core/src/formatter.rs:9`; `Edition::CURRENT` is `Edition2024` in
`ra_ap_edition` 0.0.307). The port lexes and parses itself and skips
rust-analyzer's validation pass (`chloro-core/src/formatter/formatting.rs:40`). Both run
the same lexer, parser and `intersperse_trivia`.

Host: 4 vCPU Intel Xeon @ 2.80GHz, kernel `6.18.44-fc-v114`, 15 GB RAM — the host on which
phase 1 ended.

## Method

- **Builds:**
  - each commit is extracted with `git archive` into a scratch directory, never checked
    out over the working tree;
  - `bench.rs`, `fmt_dir.rs` and `alloc_count.rs` are copied into each copy;
  - all three use only `chloro_core::format_source`, which the proof of concept also
    exports.
- **Corpora:**
  - **fixtures:** 1238 files, 13.9 MB (`chloro-core/tests/conformance/fixtures`): the
    1220 rust-analyzer files plus 18 others. The same 1238 files at all three commits
    (checked by checksums). rust-analyzer's sources are already formatted with rustfmt, so
    copying code verbatim gives rustfmt's output for most of it — this matters for the proof
    of concept, see Evidence 3.
  - **registry:** 5659 files, 122.8 MB (`~/.cargo/registry/src`). This corpus depends on
    what the container has downloaded and is not fixed.
  - **sample:** 49 fixtures (every 25th of the sorted list), used for callgrind.
- **Wall clock:** `examples/bench.rs`, best of `-n` rounds, in process, one thread. Phase 1
  measured run-to-run noise of about ±4%.
- **Instructions:** callgrind and cachegrind (valgrind 3.22), under `setarch -R`. Phase 1
  measured about ±2% variance between runs of one binary, located in glibc `malloc`.
- **Output checks:** the golden output hashes recorded in phase 1 at `bcef0f9`
  (`/tmp/claude-0/scratch/golden`, still present in this container) are used through
  `bench.rs --check`.
- **Instrumentation:** some measurements need counters inside the formatter. These were made
  in scratch copies of `9202b42`, never in the repository; each one is described where used.

## Evidence

### 1. Throughput

`examples/bench.rs`, measured.

| corpus | `c4d74ee` (proof of concept) | `bcef0f9` (port, before perf) | `9202b42` (port, now) |
|---|---|---|---|
| fixtures, best of 5, two passes | 8.24, 8.96 MB/s | 2.75, 2.81 MB/s | 3.64, 3.59 MB/s |
| registry, best of 3 | 8.61 MB/s | 1.73 MB/s | 4.65 MB/s |

On the registry the memo (`2d8a579`) matters more than on the fixtures: `bcef0f9` is at
1.73 MB/s.

### 2. The parse floor

`examples/parse_floor.rs`, measured. Each mode runs the same lexer and parser; the modes
differ in what is built from the parser's events.

| mode | fixtures | registry |
|---|---|---|
| `lex` (lexer only) | 125 MB/s | 98 MB/s |
| `events` (lexer + parser, no tree) | 41–43 MB/s | 42 MB/s |
| `flat` (events + trivia into two flat arrays) | 31–33 MB/s | 27 MB/s |
| `rowan` (events + trivia into a rowan tree, as `formatting::parse`) | 13.0–14.2 MB/s | 13.1 MB/s |
| `rowan-walk` (as `rowan`, then a walk of every red node and token) | 12.8 MB/s | 11.0 MB/s |

Instructions for the whole fixture corpus, one round (callgrind, measured):

| mode | instructions |
|---|---|
| `events` | 2.26 G |
| `flat` | 3.13 G |
| `rowan` with a shared `NodeCache` | 4.39 G |
| `rowan` | 4.67 G |

Findings (measured):
- Building the rowan tree takes about two thirds of the time of `rowan` mode: 0.65–0.7 s of
  about 1.0 s on the fixtures.
- A formatter that reads a rowan tree built this way cannot exceed about 13–14 MB/s on
  these corpora, even if formatting cost nothing.
- The proof of concept's 8.2–9.0 MB/s is about 60–65% of that ceiling.
- The same parser feeding a flat array tree reaches 27–33 MB/s.
- Reusing one `NodeCache` across files saves 6% of instructions. Wall-clock results were
  11.85–15.08 MB/s with the shared cache against 11.40–12.70 MB/s without, over three
  single-round runs: not separable from noise.

### 3. How much of its input each version lays out

`examples/spacing.rs`, measured. The fixtures are copied with every single-space whitespace
token doubled (`perturb`). Comments, literals and newlines are left alone. Both versions
then format the original and the perturbed copy, and the tool counts the doubled spaces
still in each output.

| version | doubled spaces left as written |
|---|---|
| `c4d74ee` | 62.4% |
| `9202b42` | 3.7% |

The counts cover the 1025 fixtures the port formats; files returned unchanged are left out.

- The proof of concept leaves about three fifths of inter-token spacing as written: it
  copies that code instead of laying it out.
- The port's 3.7% consists mostly of comments, macro bodies and lines left as written
  (read from a sample of the surviving lines). rustfmt also leaves those as written. The
  3.7% was not classified line by line.

Why the proof of concept copies so much (read, and probed):
- **Statements.** `format_stmt_list` passes each statement node to `try_format_expr`
  (`c4d74ee:chloro-core/src/formatter/node/block.rs:102`). `LET_STMT` and `EXPR_STMT` are
  not expression kinds, so `try_format_expr_inner` returns `None`
  (`c4d74ee:chloro-core/src/formatter/node/expr.rs:94`) and the statement text is copied
  (`block.rs:117`). Every `let` and expression statement in a function body is therefore
  copied as written. Only the tail expression is formatted.
- **Probe:**
  - input: `fn f() {\n    let   x=foo( a,b ) ;\n    bar (  x ,1);\n    x.map(|y|y+1).collect::<Vec<_>>()\n}`;
  - output: all three lines as written;
  - in the same input, `fn g() -> u32 { a+b }` became `a + b` on its own line.
- **Method chains.** Every method call whose receiver is a method call, field access or
  `.await` returns `None` and is copied
  (`c4d74ee:chloro-core/src/formatter/node/expr/collections.rs:143`).
- **Copied unchanged in all cases:**
  - macro calls and macro definitions (`node.rs:319`, `node/expr.rs:88`);
  - attributes (`node.rs:326`);
  - the type of a cast (`node/expr/operators.rs:54`).
- **Any unsupported subexpression** makes its whole statement or tail expression verbatim,
  because `None` propagates through `?`.

### 4. Where the instructions go

callgrind on the 49-file sample, exclusive cost grouped by component, measured (minimum
run; three runs of `c4d74ee` within 0.001%, three of `9202b42` within 1.4%).

| component | `c4d74ee` | `9202b42` |
|---|---|---|
| total | 299 M | 595 M |
| parser (`ra_ap_parser`, lexer) | 87 M (29%) | 96 M (16%) |
| rowan green tree building | 42 M (14%) | 55 M (9%) |
| rowan red tree (`cursor`) | 69 M (23%) | 61 M (10%) |
| `ra_ap_syntax` typed accessors | 12 M (4%) | 17 M (3%) |
| `malloc`/`free`/`realloc`/`RawVec` | 70 M (23%) | 130 M (22%) |
| `libc` `mem*`, `core`/`std` string and iterator code | 12 M (4%) | 50 M (8%) |
| chloro's own code | 4.8 M (1.6%) | about 174 M (29%) |

The parse-only run of the same sample (`parse_floor rowan`) executes 163 M instructions.

Of chloro's own 174 M in `9202b42`, by module (measured):

| module | instructions |
|---|---|
| `comment` (`CharClasses`, `LineClasses`, comment slicing) | 55 M |
| `utils` (`unicode_str_width`, `filtered_str_fits`) | 19 M |
| `lists` | 14 M |
| `nodes` | 12 M |
| `overflow` | 10 M |
| `context` (memo) | 10 M |
| `rustc_compat` | 9 M |
| `visitor` | 9 M |
| `expr` | 8.5 M |
| `chains` | 7.3 M |
| the rest | 3 M or less each |

Findings (measured):
- The proof of concept's own formatting code executes 1.6% of its instructions. The rest
  is parsing, rowan and the allocator.
- Of the proof of concept's 144 M formatting instructions (`format_node`, inclusive),
  73 M are `SyntaxText`'s `Display`: the copying of verbatim text through a red-tree walk.
- In both versions most of the time goes to parsing, the rowan tree and allocation, not
  to the per-node formatting functions.

### 5. Dispatch and traversal (read)

Both versions dispatch on the node kind in a `match` and recurse into children:
- the proof of concept: `format_node` (`c4d74ee:.../node.rs`) and `try_format_expr_inner`
  (`.../node/expr.rs:39`);
- the port: `format_expr_uncached` (`chloro-core/src/formatter/expr.rs:125`), the `Rewrite`
  impls per node type, and the visitor's item and statement match.

| | proof of concept | port |
|---|---|---|
| dispatch | per-node `match` | per-node `match` |
| node access | `ast::*::cast` and typed accessors, one red node per access | the same, plus the green-tree lookups of phase 1 |
| result of an expression | `Option<String>`, built with `format!` | `Option<String>` |
| width | a fixed `MAX_WIDTH` compared with the indent plus the single-line string (`collections.rs:109`); no width passed to children | a `Shape` (indent, width, offset) passed to every child |
| layouts per node | at most two (single line, else multi-line), with children formatted once and reused | rustfmt's trial and fallback: children re-formatted for each shape tried |
| comments | taken from the tree's trivia tokens where handled | rustfmt's text scans of source snippets (`comment.rs:638`, `CharClasses`) |

- Per-node dispatch does not distinguish the two versions.
- What distinguishes them:
  - how much of the input is laid out (Evidence 3);
  - whether a child's layout depends on the width left for it, and is retried;
  - how comments are found.

Red nodes created on the sample (callgrind call counts, measured): 300,678 in `c4d74ee`
(verbatim copying walks every copied subtree) and 343,219 in `9202b42`. Red-node churn is
of the same size in both versions, so it does not explain the gap.

### 6. Allocations

`examples/alloc_count.rs` on the fixtures, measured:

| | allocations | per input byte | bytes requested |
|---|---|---|---|
| parse only (`SourceFile::parse`, scratch probe) | 1.41 M | 0.10 | 54x input |
| `c4d74ee` | 10.28 M | 0.74 | 99x input |
| `bcef0f9` | 36.64 M | 2.64 | 547x input |
| `9202b42` | 20.27 M | 1.46 | 451x input |

The 64–127-byte bucket holds 11.6 M of `9202b42`'s 20.3 M allocations. That size matches
red `NodeData` and short strings.

DHAT on the sample, `9202b42` (measured):
- 45.8% of all bytes requested came from 2290 calls of `FmtVisitor::from_context`
  (`chloro-core/src/formatter/visitor.rs:105`);
- `from_context` calls `FmtVisitor::new`, which reserves twice the file size for the
  buffer (`visitor.rs:91`), then replaces the buffer with a 256-byte one (`visitor.rs:107`);
- the block visitor of every block rewrite makes this reservation.

**Tried and refuted:** building the visitor without that reservation (scratch copy) changed
no output (`bench --check`: 0 of 1238 changed). It did not change wall time either:
3.49–3.64 MB/s against 3.49–3.60 MB/s for `9202b42`, three interleaved pairs. The memory
is reserved and never touched, so the bytes-requested figures above overstate cost. The
count of allocations is the figure that matters.

### 7. Speculative formatting in the port

Measured by instrumenting a scratch copy of `9202b42`: counters in `format_expr` keyed by
node kind, a set of distinct (green node, offset) pairs, and the length of every `Some`
result. Over all fixtures:
- 696,064 `format_expr` calls; 682,171 run uncached (paths and literals are never cached);
- 394,855 distinct expression nodes, so 1.73 uncached rewrites per expression node;
- by kind:
  - closures: 2.68 rewrites per node;
  - parenthesised expressions: 2.04;
  - casts: 1.93;
  - tuples: 1.87;
  - method calls: 1.79;
  - matches: 1.79;
  - `for`: 1.12;
  - `while`: 1.08;
- 5,128 rewrites (0.75%) return `None`;
- the `Some` results total 50.2 MB, against 14.2 MB of formatted output.

Findings (measured):
- With the memo, failed layouts that return `None` are rare.
- The repeated work comes from successful rewrites of the same node under different shapes
  (1.73 per node), and from string results that a parent then does not use.
- Calls are not instructions, so the share of instructions spent on repeated rewrites is
  **not measured**.

### 8. Same input, same output: the agreement corpus

Comparing whole files mixes up skipped work and overhead, so this comparison runs both
versions on code where they produce the same, rustfmt-identical output. Steps
(`examples/spacing.rs`, measured):
1. `split` writes each top-level item of each fixture that rust-analyzer parses without
   error to its own file: 16,078 items from 1029 files.
2. `perturb` doubles their spaces, so that a version that copies an item as written gives
   a different output from one that formats it.
3. `agree --rustfmt` keeps items where both versions give the same output, the output
   differs from the input, and the output equals `rustfmt --edition 2024`. 7262 items
   (1.63 MB) qualify.

| item kind | items | bytes |
|---|---|---|
| `fn` | 2051 | 1.23 MB |
| `use` | 3463 | 0.22 MB |
| `mod` | 504 | 47 KB |
| `impl` | 523 | 43 KB |
| `struct` | 417 | 49 KB |
| `enum` | 109 | 24 KB |
| other | 195 | 16 KB |

Results on the `fn` items: both versions give rustfmt's output for every file. The fn items
that qualify are those with no `let` or expression statements (see Evidence 3) — mostly
test functions whose body is one call with a raw string literal.

| measure | `c4d74ee` | `9202b42` | parse only (`rowan`) |
|---|---|---|---|
| wall (best of 10) | 0.132 s, 9.29 MB/s | 0.189 s, 6.49 MB/s | 0.090 s |
| wall, minus parse | 0.042 s | 0.099 s (2.4x) | |
| instructions | 387 M | 615 M | 210 M |
| instructions, minus parse | 177 M | 405 M (2.3x) | |

On all 7262 items: 0.318–0.325 s (`c4d74ee`) against 0.401–0.412 s (`9202b42`). Parsing
the 7262 small files into rowan trees takes 0.226 s, and the flat-array build takes
0.038 s. The difference shows a large fixed cost per rowan tree.

Where `9202b42` spends the extra instructions on the `fn` items (callgrind, measured,
exclusive):
- `CharClasses::next`: 46 M (7.5%);
- `unicode_str_width`: 21 M (3.5%);
- `LineClasses::next`: 6.6 M.

These come from `rewrite_literal` → `filtered_str_fits` (`chloro-core/src/formatter/expr.rs:1251`,
`chloro-core/src/formatter/utils.rs:156`), 7.4% inclusive. That code scans the text of
every multi-line string literal character by character, as rustfmt does. The lexer has
already classified that text as one string token.

Finding (measured): on code where both versions give rustfmt's output, the port spends
about 2.3–2.4x the proof of concept's formatting work. That gap is overhead of the port's
implementation, not work that conformance needs.

Limits:
- the agreement corpus contains only constructs the proof of concept formats correctly;
- the agreement corpus is biased towards simple items;
- the gap on code that needs rustfmt's layout search (`let`, chains, closures, `match`) is
  **not measured**, because the proof of concept has no correct output to compare there.

### 9. Instruction cache (simulated)

cachegrind with its default cache model, whole fixture corpus, measured:

| | instructions | I1 misses | D1 misses |
|---|---|---|---|
| `c4d74ee` | 8.76 G | 6.7 M | 34.3 M |
| `9202b42` | 17.91 G | 216.4 M | 67.5 M |
| parse only | 4.86 G | 2.5 M | 29.7 M |

The port executes 2.04x the instructions of the proof of concept, and takes 2.3–2.5x the
wall time. Its simulated L1 instruction misses are 32x the proof of concept's.

**Hypothesis:** the port's code size (21,063 lines under `chloro-core/src/formatter/`
against 3,954 in the proof of concept) causes front-end stalls. These would explain part
of the gap between the instruction ratio and the time ratio. No hardware counters are
available in this container (`perf` is not installed), so this is unconfirmed.

### 10. Build settings

`lto = "fat"` with `codegen-units = 1`, set through `CARGO_PROFILE_RELEASE_*` in a separate
target directory, measured over two interleaved passes:

| version | default | LTO + 1 codegen unit |
|---|---|---|
| `c4d74ee` | 8.32, 8.47 MB/s | 9.07, 9.02 MB/s |
| `9202b42` | 3.65, 3.62 MB/s | 3.94, 3.71 MB/s |

The gain is 2–8%, at the edge of noise, and not pursued.

## What the evidence establishes about the regression

Established (measured):
1. **Most of the proof of concept's lead comes from work the proof of concept does not
   do:**
   - it lays out about two fifths of inter-token positions and copies the rest (Evidence 3);
   - its own formatting code is 1.6% of its instructions (Evidence 4);
   - it scored 233/1220 conformance (phase 1, `examples/conform.rs`).
2. **Per-node dispatch does not distinguish the two versions:** both dispatch per node kind
   and both build an `Option<String>` per expression (Evidence 5).
3. **Parsing into rowan bounds both versions:**
   - it is 51.6% of the proof of concept's instructions (`SourceFile::parse`, inclusive) and
     about 60–65% of its wall time;
   - the ceiling for any formatter on this rowan tree is about 13–14 MB/s (Evidence 2).
4. **On code where both give rustfmt's output, the port does about 2.3–2.4x the
   formatting work** (Evidence 8). That gap is overhead of the port's implementation.
5. **The port makes about 2x the proof of concept's allocations** (20.3 M against 10.3 M on
   the fixtures). The bulk of the increase comes from strings and lists, not red nodes
   (Evidence 5, 6).

Not established (hypotheses):
- **Text rescans.** Answering text-scanning questions (comment presence, string-literal
  line widths, comment positions) from the token stream would remove most of the `comment`
  and `utils` cost (74 M of 595 M on the sample), with no change in output. This is
  untested. rustfmt rescans text because rustc's AST has no trivia, while rowan's tree
  holds every comment as a token.
- **Shape-independent results.** A large share of the repeated rewrites (1.73 per node)
  produces results that do not depend on the width given. Untested.
- **Instruction cache.** Instruction-cache pressure explains part of the gap between the
  2.0x instruction ratio and the 2.3–2.5x time ratio. Untested on hardware.

### The three ideas raised at the end of phase 1

- **Strings for layouts that fail.** The proof of concept does not avoid this work (read):
  - `format_call_expr` builds the whole single-line string with `format!` and `join`, then
    compares its length with `MAX_WIDTH` and builds the multi-line string instead when it
    does not fit (`c4d74ee:chloro-core/src/formatter/node/expr/collections.rs:100-125`);
  - the method call path and `format_delimited_list` (arrays, tuples) do the same
    (`collections.rs:204`, `collections.rs:302`).

  The proof of concept avoids *repeated* work only because it formats each child once, at
  one indent, whatever width is left. The port's `None` results are 0.75% of rewrites
  (Evidence 7), so "failed layouts" in the narrow sense are rare. What the port repeats
  are successful rewrites under other shapes.
- **Top-level items in parallel:** deferred, as asked. Single-file latency is not measured
  in this entry.
- **Tree allocations:**
  - red nodes are created at a similar rate in both versions (Evidence 5);
  - the proof of concept does not avoid red-node allocation;
  - the larger lever is tree *construction* (Evidence 2), which `rowan` decides and a child
    cache would not change.

## Conformance work mapped onto the proof of concept

The proof of concept did not lack a list of fixes. It had no implementation of most of
rustfmt's behaviour: the port (`2c1c00e`) replaced it wholesale. Mapped by subsystem
(read):

| subsystem (port module) | class | in the proof of concept |
|---|---|---|
| source normalisation: BOM, CRLF (`97b412f`, `formatting.rs`) | essential correctness, independent of architecture | absent |
| rustc acceptance rules: let chains before 2024, chained comparisons and ranges, parse errors (`ee38eb1`, `rustc_compat.rs`) | essential correctness; a whole-tree pre-pass in any design | absent; the proof of concept formats files with parse errors |
| `style_edition` from `edition` (`17c45d4`, `config.rs`) | essential correctness, configuration | absent; no configuration (`MAX_WIDTH` constant) |
| rustc block expression kinds (`fd479ae`, `nodes.rs`) | algorithmic; could be reimplemented per node | absent |
| `use` visibility as `Option<Option<_>>`, `use` tree normalisation and merging (`2cf80a3`, `imports.rs`) | algorithmic; could be reimplemented per node | partial: grouping and sorting only (`node/useitem/`) |
| `Shape` width budgets and trial-and-fallback layouts (`shape.rs`, `expr.rs`, `overflow.rs`, `chains.rs`, `closures.rs`, `matches.rs`) | essential for conformance — output depends on which layout fits first (phase 1, "Why the port is slower") | one fixed `MAX_WIDTH` check, two layouts at most |
| statements, `let`, `let … else` (`stmt.rs`, `expr.rs`) | essential | copied as written |
| lists with comments between items (`lists.rs`) | essential | absent |
| comments in gaps between nodes (`missed_spans.rs`, `comment.rs`) | essential behaviour; the text-rescan implementation is a design choice (hypothesis above) | leading and trailing comments of items and statements only |
| macro argument re-parsing and splicing (`macro_args.rs`, `macros.rs`) | essential | macros copied as written |
| attributes and doc comments (`attr.rs`) | essential | copied as written |
| types, patterns, items (`types.rs`, `patterns.rs`, `items.rs`) | essential | partial: types and many items copied |

Phase 1 performance commits, by whether they would carry over to a per-node design (read;
the transfers are predictions, **not measured**):

| commit | change | carries over to the proof of concept's design? |
|---|---|---|
| `2d8a579` memo | caches rewrites per (node, shape, flags) | only once layouts are retried per shape; the proof of concept formats children once |
| `e4df9e4` `node_text` from the green tree | text without a red-tree walk | yes, directly — `SyntaxText`'s `Display` is 24.5% of the proof of concept's instructions (Evidence 4) |
| `74fb4a7`, `7e2be6c`, `86b77a8` green-tree lookups | typed child lookups without red nodes | yes — the proof of concept uses the same typed accessors |
| `a156c83`, `4edd403`, `8d06135` text scanning | faster rescans | not applicable — the proof of concept does not rescan text |
| `bd3bad1`, `6b07131` macro splicing | cheaper re-parsed macro arguments | not applicable until macro arguments are formatted |
| `03e063f`, `9202b42` | `format!` removal, Fx hashing | yes, in kind — the proof of concept builds with `format!` throughout |

## The next ceiling

Budget arithmetic on the fixtures (13.9 MB), from Evidence 2:

| target | total time | tree build, rowan (1.0 s) | tree build, flat arrays (0.43 s) |
|---|---|---|---|
| 8.2 MB/s, the proof of concept | 1.70 s | 0.70 s left for formatting | 1.27 s left |
| 16 MB/s, about 2x the proof of concept | 0.87 s | impossible | 0.44 s left |
| 25 MB/s | 0.56 s | impossible | 0.13 s left |

- **Above about 13–14 MB/s,** no amount of work on formatting helps while the tree is a
  rowan tree built per file. That holds for the proof of concept as much as the port.
- **Recovering the proof of concept** therefore cannot by itself go "substantially beyond"
  8.2 MB/s. Its ceiling is the same rowan ceiling.
- **At 16 MB/s with a flat tree,** formatting gets 0.44 s for 13.9 MB. That is about 6x less
  than the port's 2.8 s now, and less than the proof of concept's 0.6 s, which covers only
  two fifths of the layout work.
- **The flat build** stores kind, parent, sibling and token ranges, but not token text
  interning or a typed API. A typed layer over it would cost something. **Not measured.**

## Recovery branch: decision deferred to the evidence

The request was to create a recovery branch from `c4d74ee` and port the conformance fixes
onto it. The evidence above does not support that as a route to speed:
- the proof of concept's speed comes mostly from not doing the work (Evidence 3, 4);
- the work it skips is rustfmt's layout search, which conformance needs (phase 1);
- its ceiling is the same rowan ceiling (Evidence 2).

Rebuilding conformance on it would re-implement the port.

The parts of the proof of concept the evidence supports keeping:
- formatting a child once where its result does not depend on the width given;
- reading comments from the tree instead of from text.

Both are ideas to test inside whichever base carries the conformance work, not reasons to
change the base.

No recovery branch has been created. `origin/master` (`c4d74ee`) stays untouched as the
baseline. Whether to create the recovery branch anyway is the user's decision.

## Proposed experiments (none run)

Ordered by cost. Each one has a pass criterion fixed before it runs.

1. **E1 — flat tree under a typed shim, on one whole-tree pass.** Smallest test of the
   direction "keep the port's algorithm, replace its substrate".
   - Build the flat tree of `examples/parse_floor.rs` with the trivia rules of
     `intersperse_trivia`.
   - Add typed accessors with the method names that `rustc_compat.rs` uses (`kind`,
     `children`, `parent`, `text_range`, the `ast::*` accessors it calls).
   - Run `rejected_by_rustc` on both trees.
   - Pass: identical accept/reject decisions on all 1238 fixtures and all registry files;
     flat build + pass at least 2x faster than rowan build + pass;
     `chloro-core/src/formatter/rustc_compat.rs` changes only in imports.
   - The shim's fidelity and cost are the unknowns that decide whether the whole formatter
     can move.
2. **E2 — token-based text queries.** Answer `contains_comment` and `filtered_str_fits` for
   string literals from the token stream instead of `CharClasses`, behind the same
   signatures.
   - Pass: 0 changed output hashes on 1238 fixtures and the registry; option matrix
     unchanged; at least 8% fewer instructions on the sample (the `comment` + `utils`
     share is 12.4%).
   - This is the first measurement of how much of the port's overhead is the rustfmt
     text-rescan design.
3. **E3 — share of repeated rewrites.** In a scratch copy, attribute the inclusive
   instructions of `format_expr_uncached` to first and repeat rewrites of each (node,
   shape). This measures the most that shape-independence or a measure-first pass could
   save, before any design work.
4. **E4 — flat tree per top-level item** (suggested by the user's reading; Sampson,
   "Flattening ASTs"). This is the variant of E1 that builds the flat tree per item and
   discards it after formatting. The rowan cost per tree on 7262 small files (0.226 s
   against 0.038 s flat, Evidence 8) suggests a fixed cost per tree that this variant
   avoids. Peak memory is **not measured**.

Deferred, as asked: parallel formatting of top-level items; a rowan fork (excluded); a
child cache, which E1 makes moot if it passes.

## Unresolved questions

- How much of the port's formatting cost on layout-heavy code (`let`, chains, closures,
  `match`) is overhead? Evidence 8 covers only code the proof of concept formats correctly.
- How much does a typed shim over a flat tree cost? E1 measures this.
- Does instruction-cache pressure explain part of the time-to-instruction ratio? This needs
  hardware counters.
- The fixtures are rust-analyzer's own rustfmt-formatted sources. Throughput on
  unformatted input is **not measured** for either version: Evidence 3 used perturbed
  inputs only for counting, not for timing.
- The registry corpus is container-dependent (5659 files now, 5623 when phase 1 recorded
  hashes).

## Reproduction

Commands, from the repository root, with `W` an empty scratch directory:

```sh
for c in c4d74ee bcef0f9 9202b42; do
  mkdir -p $W/$c && git archive $c | tar -x -C $W/$c
  cp chloro-core/examples/{bench,fmt_dir,alloc_count}.rs $W/$c/chloro-core/examples/
  (cd $W/$c && cargo build --release -p chloro-core --example bench --example fmt_dir --example alloc_count)
done
F=chloro-core/tests/conformance/fixtures
cargo build --release -p chloro-core --example parse_floor --example spacing
$W/c4d74ee/target/release/examples/bench -n 5 --root $F           # Evidence 1, per commit
target/release/examples/parse_floor rowan $F -n 5                  # Evidence 2, per mode
$W/9202b42/target/release/examples/alloc_count $F                  # Evidence 6, per commit
target/release/examples/spacing perturb $F $W/pert                 # Evidence 3
$W/9202b42/target/release/examples/fmt_dir $W/pert $W/out/9202b42/pert   # and $F → .../orig
target/release/examples/spacing survival $F $W/pert $W/out/9202b42/orig $W/out/9202b42/pert
target/release/examples/spacing split $F $W/items                  # Evidence 8
target/release/examples/spacing perturb $W/items $W/items_pert
# fmt_dir both versions over $W/items_pert, then:
target/release/examples/spacing agree $W/items_pert $W/items_out/c4d74ee $W/items_out/9202b42 $W/agree --rustfmt
```

callgrind and cachegrind runs use
`setarch $(uname -m) -R valgrind --tool=callgrind ... bench -n 1 --root SAMPLE`, with
SAMPLE holding every 25th file of the sorted fixture list.

Exclusive costs were grouped by component by matching function names:
- `ra_ap_parser` and the lexer → parser;
- `rowan` green and `NodeCache` → green tree building;
- `rowan::cursor` → red tree;
- `malloc`, `free`, `__rust_alloc` and `RawVec` → allocator;
- `chloro_core::formatter::<module>` → chloro's own code, by module.

Instrumentation counts (Evidence 7) and the DHAT and visitor-buffer runs (Evidence 6)
used scratch patches to `9202b42` and are described where used.

## Current State

- `examples/parse_floor.rs` times lexing, parsing, rowan tree building (per-file or shared
  `NodeCache`), a red-tree walk, and a flat two-array tree built from the same parser
  events, over any directory of Rust files (chloro-core/examples/parse_floor.rs)
- `examples/spacing.rs` splits files into one file per top-level item, doubles
  single-space whitespace tokens, counts doubled spaces left in formatter outputs, and
  selects files on which two outputs agree, optionally checked against
  `rustfmt --edition 2024` (chloro-core/examples/spacing.rs)
- `examples/fmt_dir.rs` formats a directory tree into another with
  `chloro_core::format_source` only, and builds unchanged in a `c4d74ee` checkout
  (chloro-core/examples/fmt_dir.rs)
- `examples/alloc_count.rs` counts heap allocations and request sizes of `format_source`
  through a counting global allocator, and builds unchanged in a `c4d74ee` checkout
  (chloro-core/examples/alloc_count.rs)
- `FmtVisitor::from_context` reserves twice the file size through `FmtVisitor::new` and
  then replaces the buffer with a 256-byte one — 45.8% of bytes requested on the 49-file
  sample, with no measurable wall-time cost
  (chloro-core/src/formatter/visitor.rs:84-110)
- `filtered_str_fits` scans every line of a multi-line string literal with `LineClasses`
  and `CharClasses` (chloro-core/src/formatter/utils.rs:156,
  chloro-core/src/formatter/comment.rs:638-700)
- The formatter code is unchanged from `9202b42`: 1219 of 1220 conformance fixtures match
  rustfmt, and the output hashes recorded at `bcef0f9` match

## Missing

- No recovery branch from `c4d74ee` — `origin/master` stays the untouched baseline
- No flat-tree implementation beyond the measurement in `examples/parse_floor.rs`, which
  stores no token text and has no typed accessors (chloro-core/examples/parse_floor.rs)
- No hardware performance counters in this container (`perf` not installed) — instruction
  cache effects are known only from cachegrind's simulation
- No fixed, checked-in second corpus — the registry corpus depends on the container's cargo
  cache
