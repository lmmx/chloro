# Unmined tricks: where the next outsized gains hide, with precedents (2026-10-15)

The programme's third brief. After [2026-10-14-autoresearch.md](2026-10-14-autoresearch.md)
(E1–E8, R2–R5; 587.99 M to 312.73 M instructions on the sample) and the overnight run of
[2026-10-15-extensions-theory.md](2026-10-15-extensions-theory.md) (E12–E18; to 248.20 M,
206.92 M profile-guided; [M4](2026-10-14-autoresearch/measurements/m4-quiet-host.md): 0.26–0.44
of rustfmt's wall time on one core, 0.092–0.179 on four), the owner asked for the next phase
to be opened the same way the previous one was: by asking what tricks remain unmined, finding
where the same tricks produced outsized gains elsewhere, and only then choosing experiments.

Three passes are compiled here. Two are the owner's ChatGPT thread (its first answer on data
tapes, hotpath-rs and SIMD; then its answer to "give more detail about the unmined tricks and
search the web for precedents"), summarised in §3 and §4 with their rankings kept. The third
(§5) is this session's own pass over the same question, written against the measured profile
and not against the other two: where it agrees it says so, where it differs it says why, and
each trick names a precedent with its source. §6 is the precedent catalogue. §7 is the plan
that becomes the next experiment wave, under the unchanged
[protocol](2026-10-14-autoresearch/protocol.md): conformance, correctness and engineering
integrity are hard gates, instructions on the sample the primary metric, every claim measured
against the best-known-good (`e640fbf`), every result in the
[ledger](2026-10-14-autoresearch/experiments.md).

## 1. What the floor is made of (measured)

Everything below is from the records; nothing is new measurement.

| quantity | value | where |
|---|---|---|
| instructions, 49-file sample, final head | 248,202,137 (0.4221 of the start); 206,922,054 profile-guided | M4 |
| component shares at `31ce20e` (312.7 M): formatter / parser+lexer / allocator / core+std / flat tree / libc | 39.9% / 28.0% / 15.1% / 7.9% / 5.4% / 3.4% | extensions-theory §"The final head, measured" |
| glibc malloc's share of one thread's time, measured directly | 13–16% | EXP-014 |
| L1 instruction-cache misses | 5,233,543, 1.67% of instructions; `write_list` 266 K, `overflow::Context::rewrite` 372 K, `_int_malloc` 152 K, `rewrite_chain_from` 225 K, `memoize` 95 K | cache simulation, same section |
| conditional branches / mispredicted | 49.6 M / 3.85 M (7.8%) | same |
| indirect branches / mispredicted | 2.63 M / 0.97 M (37.0%), in the parser's output decode and the tree build | same |
| in-process throughput, one core | 11.22–11.79 MB/s (248 M instructions per 650 KB of sample) | M4 |
| parse floor (lexing and parsing that no formatter lever removes) | 33–38 M of the sample's instructions | parser-and-substrate record |
| warm-cache `--check`, 1238 files, four cores | 16.9 ms; a median single-file save about 4 ms of which 1.8 ms is process start-up | M4; incremental-and-process record |

Two derived facts frame this entry:

- **Instructions are not the whole wall clock.** 11.5 MB/s on a 2.8 GHz core at 248 M
  instructions per 650 KB is about 4.4 G instructions per second, an IPC near 1.6. The
  simulator counts 4.8 M mispredicted branches and 5.2 M instruction-cache misses per 312.7 M
  instructions; at typical penalties (15–20 cycles per mispredict, a few to tens of cycles per
  I-cache miss that leaves L1) those two alone are of the order of a third of the cycles. The
  programme's primary metric is blind to both. PGO's record shows the gap: −21.7% instructions
  for 0.78–0.92 of the wall time on M3's head, and on the final head −16.6% instructions for a
  smaller wall gain (M4: 0.906x–0.955x on most pinned crates). This is a derived estimate, not
  a measurement; hardware counters are not available in the container (no `perf`), and the
  first task of the wave below is to make the simulator's branch and I-cache counts a reported
  co-metric so this can be checked and tracked.
- **The parser is now the largest single component and is a dependency.** 28% of
  instructions, 37% of its indirect branches mispredicted, 22.7% of all data-cache read misses
  in `event::process`. The fork-free levers left (P2, P7) are bounded at ≤ 5.5% and ≤ 1.9%; the
  floor of 33–38 M is reachable only by changing `ra_ap_parser` itself.

## 2. The brief

The owner's first message of this phase asked for: profiling the final head for the largest
remaining costs; the shared output sink first (bounded at 8.8–10.5% of instructions); an
optional `hotpath-rs` diagnostic beside the counters; a targeted data-tape experiment (compact,
contiguous output records) on one hot path; a resident-process prototype with the opt-in item
cache measuring real editor-save latency, memory and invalidation cost; `memchr` or other SIMD
only where profiles find a scanning hotspot. Each experiment in an isolated worktree, with
reproducible before-and-after results, through the full gates before integration; speculative
or marginal wins rejected; the best-known-good preserved; the ledger kept current. The second
message asked that the "intensify the breadcrumbs" question be put to this session as well,
and that both answers become this entry before the loop restarts.

## 3. ChatGPT, first pass (the owner's thread), summarised

Verdict: yes to a data-tape investigation, cautiously yes to `hotpath-rs`, no to more SIMD
libraries until a profile names a gap. "Chloro has already moved beyond obvious algorithmic
wins; the next gains come from reducing memory traffic, simplifying hot control flow and
avoiding repeated work."

1. **Data tape.** Since the flat preorder array exists, the question is whether the tape can
   be the primary interface between parsing and formatting instead of richer views being
   rebuilt or walked. Compact records: token kind and span; syntax kind, subtree boundary and
   flags; trivia and comments as spans; formatting events or output fragments consumed by a
   shared sink. Experiment, not rewrite: take the two most expensive formatting paths, convert
   only their intermediate representation, compare against the current flat array on identical
   inputs (instructions, allocations, peak RSS, one- and four-core throughput, output identity),
   keep only an end-to-end gain. The shared sink is the better-grounded first target.
2. **`hotpath-rs`.** As a diagnostic in a separate feature, never on the release path:
   which functions take cumulative time, which allocate most bytes and how often, whether the
   sink is expensive itself or upstream of it, whether hotspots are hot by call count.
   Instrumentation distorts tiny functions; counters and the deterministic instruction count
   remain the authority.
3. **SIMD.** `memchr` worth benchmarking on comment terminators, newlines and delimiters
   against the current paths; hand-written SIMD only where a profile shows a hot scan the
   compiler does not vectorise; a broad SIMD abstraction deferred; no new allocator (E14).
4. **Order:** shared sink (throughput) → resident process plus item cache (latency; the
   0.15 ms figure is a bound, not a measurement) → targeted tape on one hot path → `hotpath-rs`
   and SIMD trials as supporting work.

## 4. ChatGPT, second pass ("the unmined tricks, with precedents"), summarised

Thesis: the next outsized gains come from removing work the formatter performs at all
(intermediate representations, revisited syntax, decisions that could be made earlier,
process-level overhead on tiny edits), not from one more instruction-level trick. Eight
points, ranked by promise:

1. **Change the amount of work** (very high upside, medium–high risk). Precedent: simdjson,
   which rethought parsing around structural indexing rather than tuning a conventional parser
   and reports a quarter or fewer of a reference parser's instructions. Three variants for
   chloro: (A) one compact syntax tape consumed directly by a hot path; (B) a formatting-event
   tape (span, whitespace decision, break, indent change, token) rendered by a specialised sink,
   at the risk of a second IR that costs more than it saves; (C) a hybrid fast path with a
   precise eligibility condition and the general formatter as fallback. Test on `write_list`
   first; count the tape's construction cost against its saving.
2. **Repeated work invisible to a function profile** (very high, low–medium risk). Cache
   decisions, not just data: parent and boundary lookups, fit checks whose inputs are already
   known, indentation and trivia relations recomputed through several helpers, the same source
   region scanned for different questions. Store on the flat record or in a compact side array;
   no general cache. Precedent: rust-analyzer's Salsa early cutoff (reuse when meaningful
   dependencies are unchanged). Two experiments: within one run, and across edits via the unit
   cache, the latter measured as its own latency workload.
3. **Algorithmic cliffs** (high; very high if found). Adversarial-but-valid fixtures (nesting,
   long chains and lists, large comments and strings, many similar items, repeated fit
   decisions) at 1x, 2x, 4x, 8x; plot instructions per byte against size; a superlinear slope
   on one shape is a hidden quadratic path or repeated traversal that a single carried boundary
   may fix. Lesson borrowed from autoresearch reports: the big wins changed the structure of the
   work, not the inner loop.
4. **Cheap common cases, cold rare cases** (high, medium). Dispatch once, early, carry the
   decision; specialise the representation (compact kind, span, flags; cold metadata in a side
   table) and the code path (the general layout machinery as an explicit fallback). Trap: code
   bloat worsening the instruction cache; require fewer executed instructions, not a
   microbenchmark.
5. **Memory traffic as a budget even at low RSS** (medium–high, medium). One reusable scratch
   arena per formatting operation (distinct from the global allocator, which E14 settled);
   inline small collections after measuring length distributions; borrow source text as spans
   until emission; write fragments into a reusable output buffer instead of intermediate
   strings; count bytes copied and capacity growth, not only time.
6. **Machine code after the source wins** (medium, low–medium). Validate PGO against
   representative workload mixes (small, large, mixed); inspect hot/cold layout and move rare
   paths out of the hottest functions; try LLVM BOLT post-link on Linux, expecting a smaller and
   less reliable win than its headline numbers.
7. **SIMD: the scan strategy, not the intrinsic** (medium, low for isolated trials). Bulk-skip
   ordinary bytes with `memchr` where a loop scans to a special character; classify several
   byte classes per iteration; fuse the scan with the next operation so the caller does not
   rescan; keep scalar and vector paths equivalent under test on every target.
8. **Optimise the search process itself.** Hypothesis, mechanism and the measurement that
   should move, stated before each experiment; the maximum plausible gain estimated from the
   component's share; experiments that distinguish explanations preferred; rejected ideas and
   their reasons recorded; strategy changed when a family of local tweaks yields only marginal
   wins.

Order: cliffs and repeated work → compact tape on one hot path → short-lived allocations and
copies → SIMD and machine-code trials → the persistent process measured as its own workload.
Avoid: redesigning the whole representation before one tape path is proven; caches without a
measured recomputation cost above lookup plus invalidation; SIMD for its own sake; instruction
wins taken as wall wins; several changes in one commit; microbenchmark wins that vanish on the
corpus or the configuration matrix.

## 5. This session's pass: the unmined tricks, grounded in the profile

Each trick: the mechanism, where the profile says it bites, the precedent, the bound as far as
it can be stated, the falsifier, the cost. "Bound" is a share of the sample's 248.2 M
instructions unless it says wall or latency.

### T1. The Ir-blind budget: branch mispredicts and the instruction cache

**Mechanism.** §1 estimates that mispredicts and I-cache misses are of the order of a third of
the cycles, and the two levers that move them are code layout and dispatch shape, neither of
which the instruction count rewards. Three concrete sub-levers:

- *Hot/cold splitting of the largest layout functions by hand.* `write_list`,
  `overflow::Context::rewrite`, `rewrite_chain_from` and `rewrite_fn_base` carry the largest
  I-cache miss counts; their error, fallback and rare-option paths sit inline with the hot
  path. `#[cold]` and `#[inline(never)]` on the rare arms (comment-in-list handling, overflow
  retries, `max_width` failure paths) shrink the hot body. This is what PGO does by profile; by
  hand it holds for every workload and composes with PGO.
- *Branchless kind dispatch.* The formatter asks `kind == X || kind == Y || …` and `match kind`
  in many predicates (statement-ness, needs-parens, is-block-like, trailing-comma rules); a
  `[u8; N_KINDS]` flag table indexed by the kind's discriminant answers each in one load with no
  branch. The flat tree's `Elem` already carries the kind as a small integer, so the tables
  cost nothing to key.
- *Post-link layout (BOLT).* Function reordering and hot/cold splitting over the whole binary
  from a run profile, on top of the PGO build.

**Where it bites.** The 5.2 M I1 misses are concentrated in six functions; the 37% indirect
mispredict rate sits in the parser's decode (a dependency) and the tree build (ours: the
`match` over event kinds in `TreeBuilder::steps`, which E17 made one pass but did not make
predictable).

**Precedent.** BOLT on Clang: a Clang built with PGO+LTO and then BOLT compiled a Clang tree
about 12% faster than the PGO+LTO build, with per-file gains above 15%
([LLVM docs, "Optimizing Clang: A Practical Example of Applying BOLT"](https://llvm.googlesource.com/llvm-project/+/refs/heads/main/bolt/docs/OptimizingClang.md));
Android's Rust toolchain pipeline, single codegen unit + LTO + PGO + BOLT, 24.8% faster
compiles ([Android Developers Blog, 2023](https://android-developers.googleblog.com/2023/12/faster-rust-toolchains-for-android.html));
rustc's own `librustc_driver` BOLT PR, −1.8% mean cycles at +48% artifact size
([rust-lang/rust#103595 tracking issue](https://github.com/rust-lang/rust/issues/103595)).
The branch side: the branch-avoiding graph algorithms study found mispredictions correlate with
time more strongly than instructions, branches, loads or stores
([arXiv 1411.1460](https://arxiv.org/pdf/1411.1460)); simdjson's design goal is explicitly
"minimal branching" ([Langdale and Lemire, VLDB J. 2019](https://arxiv.org/abs/1902.08318));
the One Billion Row Challenge's top entries credit branchless inner loops and SWAR as decisive
([InfoQ on Wuerthinger's entry](https://www.infoq.com/news/2024/01/1brc-fast-java-processing)).

**Bound.** Wall only; instructions may rise slightly. If a third of cycles is mispredict and
miss penalty and layout removes a quarter of it, the gain is about 8% wall on the pinned
rows; the Clang precedent (12% on top of PGO+LTO) is the optimistic end. Unknown until the
co-metric exists.

**Falsifier.** Callgrind `--branch-sim=yes --cache-sim=yes` on the sample before and after;
if `Bcm + Bim` and `I1mr` do not fall, or fall without a wall gain in three interleaved
`bench` pairs, the lever is closed.

**Cost.** Small for the attributes and tables (tens of lines); BOLT needs `llvm-bolt` in the
container and a workflow step like E13's; a stop condition if the tool cannot be installed.

### T2. The memo table as a dense side array over the flat tree

**Mechanism.** `context.rs` memoizes rewrites in a `HashMap<MemoKey, MemoEntry, FxBuildHasher>`
and cached blocks in a `HashMap<(u32, u32), CachedBlock>`. Every node is a `u32` index into the
flat array, so a side array indexed by node (with the shape variants of that node in a short
inline list) answers a lookup with one bounds-checked load and no hashing, probing or key
comparison; the array is allocated once per file at the tree's length.

**Where it bites.** `memoize` is one of the eight functions with the largest I-cache miss
counts (94,882); its hashing and probing instructions are its exclusive cost, and every rewrite
pays a lookup. The exclusive share needs reading from a fresh profile before the experiment
(estimate 1–3%).

**Precedent.** rustc's `IndexVec` and the `NodeMap` lineage, whose FNV hasher exists because
hashing node ids was already the bottleneck ([rustc `util::nodemap`](https://www.cs.brandeis.edu/~cs146a/rust/doc-02-21-2015/rustc/util/nodemap/index.html));
Zig's AST rework, nodes as `u32` indices into struct-of-arrays with 15% fewer cache misses and
22% wall ([Kelley's stage2 commit notes](https://git.jakstys.lt/motiejus/zig/src/commit/4dca99d3f6b732c415d270f0c97def144ed6d3b7));
dense interning so later phases index flat arrays directly ([zeli.app write-up](https://zeli.app/es/story/48883467)).

**Bound.** The memo lookups' exclusive instructions, likely 1–3%; plus fewer D1 misses.

**Falsifier.** Instruction count on the sample does not fall by the measured exclusive cost of
the old lookups; or peak RSS rises by more than the array (a few bytes per node).

**Cost.** About 100 lines in `context.rs`; the key's shape component needs a small per-node
list; exact by construction (the same keys, another container).

### T3. A unit queue across files (the four-core wall lever the instruction count cannot see)

**Mechanism.** The CLI hands one file to one worker; large files split their items across
workers (E1) but a crate's tail is one worker finishing one file while three idle. A single
queue of units (whole small files, items of large files) fed by all files, with per-file
stitching when a file's units complete, keeps four cores busy to the end.

**Where it bites.** M4's four-core rows: `hir-ty` CPU 314 ms against 84.5 ms wall (3.7 cores
busy), but `fx-hir` 107 against 32.1 (3.3), `reg-regex-syntax` 248 against 74.8 (3.3),
`reg-tokio` 22.5 against 8.4 (2.7): the gap to 4.0 is the idle tail. The incremental record
bounds it at 6.6–34.6% of four-thread wall on the crates.

**Precedent.** The TypeScript compiler's Go port: about half of its 10x came from
shared-memory parallelism the old runtime could not express
([Hejlsberg, via heise](https://www.heise.de/en/background/Web-development-The-new-TypeScript-compiler-in-Go-10357839.html?seite=2));
Sorbet's design for 100k lines per second per core scaling to 64 cores
([i-programmer on Petrashko's talk](https://i-programmer.info/news/98-languages/11936-sorbet-making-ruby-statically-typed.html)).

**Bound.** 6.6–34.6% of four-core wall on the nine crates; zero on one core (the protocol's
single-core figure must not regress).

**Falsifier.** hyperfine, 15 runs × 2 rounds, four-core wall not below base's range on the
crates with the largest idle tail; or the pinned one-core wall above the current head's.

**Cost.** About 150–250 lines in `chloro/src` (the queue and stitching) and `parallel.rs`
(unit extraction reused from E1/E15); output identity by `tests/parallel_equivalence.rs`
extended to cross-file scheduling.

### T4. The resident process, specified

**Mechanism.** `chloro serve` holds a Unix-domain socket; a client (`chloro fmt-client`, or the
CLI itself when the socket exists) sends `check|write|stdin <path or bytes> <config>` and gets
the result and exit code. The server keeps: the parsed `Settings` per config; the fixed-point
cache in memory; the unit cache in memory (no `.chloro-units` file in the hot path); warm
allocator arenas and page cache. Validation per request: `stat` (size, mtime) against the last
seen, BLAKE3 only when changed. Lifecycle: idle timeout, version check on the binary's
identity, one server per user cache directory.

**Where it bites.** A median-file save today is about 4.3–6.0 ms of which 1.8 ms is process
start-up and the rest cache open, read and whole-file format (incremental record); for a large
file with one edited item the unit cache gives 0.26–0.47 of the format in process (E15) but at
process level the fixed costs dilute it to 1.2–2.0x, which is exactly what a resident process
removes.

**Precedent.** The daemon forks of JavaScript tools exist for this reason and publish their
numbers: `eslint_d` 700 ms → 160 ms per file, `prettier_d_slim` 700 → 130 ms
([eslint_d README](https://github.com/dpinol/eslint_d.js/blob/master/README.md);
[prettier_d_slim via prettier_d](https://github.com/josephfrazier/prettier_d)); Black ships
`blackd` ([man page](https://manpath.be/f29/1/blackd)). rust-analyzer's incremental model is
the structural precedent for keeping parsed state resident.

**Bound.** Latency only: about 0.15 ms per request on a median file (the record's derived
figure) against 4.3–6.0 ms; for a one-item edit in `lints.rs` the in-process 0.26–0.47 of
35 ms rather than 1.2–2.0x of a 30 ms process. Memory: one process holding the cache and the
unit records; must not grow across saves.

**Falsifier.** Measured median and p99 save latency on the five largest fixtures and a median
fixture over 1,000 edits, against the CLI process and rustfmt; RSS after 1,000 saves within
1.2x of after 10; output identical to the CLI's on every request (the identity harness driven
through the socket). If the median is not below 1 ms or RSS grows, the mode is not shipped.

**Cost.** A new CLI mode of roughly the CLI's size (socket, protocol, lifecycle, tests); no
`chloro-core` change.

### T5. A per-file bump arena for rewrite strings, with grow-in-place

**Mechanism.** rustfmt's protocol returns `Option<String>` from every `rewrite`; E14 measured
glibc at 13–16% of one thread's time for these allocations; E16's S10 showed that removing one
large buffer made `realloc` copies worse. A bump arena per file whose last allocation can grow
in place turns the common pattern (push to the newest string) into a pointer bump, and frees
everything at once at the end of the file. This is not a global allocator change (E14's
verdict stands); it is a `String`-like type over an arena for the formatter's intermediates.

**Where it bites.** 15.1% of instructions in the allocator and `RawVec`; `memcpy` from
`_int_realloc` 3.5 M instructions after E16.

**Precedent.** oxc allocates its whole AST in a `bumpalo`-derived arena and attributes much of
its parser's speed to it ([oxc_allocator README](https://docs.rs/crate/oxc_allocator/0.143.0/source/README.md));
bump arenas run 2–5x faster than malloc for many small short-lived allocations and 10x in the
best case, with the gain shrinking for large or long-lived objects
([Hacker News thread on kmregion](https://hn.svelte.dev/item/44284592);
[Medium, arena allocators](https://mnwa.medium.com/speeding-up-algorithms-with-arena-allocators-d72d06f23607)).

**Bound.** Up to the allocator's 15% less the arena's own cost; realistically 5–8% if the
grow-in-place pattern dominates. Peak RSS is the risk: intermediates for a 640 KB file may sum
to tens of MB if never freed; a chunked arena with reset at item boundaries bounds it.

**Falsifier.** Instruction count on the sample not down by at least 3%, or peak RSS on
`stdin-lints` above 1.2x the current head's.

**Cost.** High: the arena string type must satisfy every use of `String` in 26,000 lines of
formatter; start with the list and chain paths only. This overlaps the shared sink (both attack
the same allocations) and must not run concurrently with it on the same files.

### T6. The transactional sink: `Option<String>` as truncate-on-failure

**Mechanism.** The shared sink (decide-once row 1) is blocked by rustfmt's control flow: a
`rewrite` that fails returns `None` and its caller tries another layout, so writing into a
shared buffer needs undo. Undo is one integer: record `buf.len()` before the attempt and
`truncate` on `None`. The sink's API is then `fn rewrite_into(&self, ctx, shape, out: &mut
String) -> bool` with a checkpoint per attempt; the `Option<String>` entry points stay as
wrappers for the modules not yet converted. `write_list` first (its items are already rewritten
strings pushed in a loop), then `rewrite_chain_from`, then `overflow`.

**Where it bites.** The 8.8–10.5% bound of decide-once row 1, plus scanning S6/S9 (the
`contains('\n')` and width passes over produced strings become bookkeeping on the sink: track
"last newline position" and "current column" as bytes are written).

**Precedent.** Biome's and Ruff's formatters separate the layout IR from printing (the
Wadler-style `FormatElement` IR and `Printer`), which is why Ruff is 30x faster than Black at
99.9% compatibility
([Astral, "The Ruff Formatter"](https://astral.sh/blog/the-ruff-formatter);
[biome_formatter docs](https://docs.rs/biome_formatter/)); Oppen's 1980 algorithm prints in
linear time with bounded lookahead and no retries
([Swierstra and Chitil, Kent](https://www.cs.kent.ac.uk/pubs/2009/2847/content.pdf)). chloro
cannot adopt an Oppen printer (it must reproduce rustfmt's retry semantics), but the
truncate-on-failure sink gives the retry protocol a single growing buffer, which is the part of
those designs that transfers.

**Bound.** 8.8–10.5% exact-only (the record); with the width bookkeeping absorbing S6/S9, up
to 12%.

**Falsifier.** `write_list` converted and the sample's instructions not down by its measured
share of string assembly (the record's per-site counts).

**Cost.** Every converted rewrite signature; 300–600 lines for the first three sites; exact by
construction.

### T7. Width by arithmetic, not by rendering (the proof obligations stated)

**Mechanism.** rustfmt decides "fits" by rendering a candidate and measuring; the decide-once
record's rows 2–4 bound the speculation at 15.6–18.1% but rest on an exactness proof that does
not exist. The proof has a finite shape: for a node whose rendering is `source text with
whitespace normalised`, width equals the source span's display width minus removed whitespace
plus inserted whitespace, and both corrections are computable from the tokens without
rendering. The obligation is to enumerate the node kinds for which chloro's rewrite is such a
function (literals, paths, simple types, single-token expressions, idents) and prove, per kind,
that no option or comment changes the rendering. That enumeration is a measured task: the
corpora give the set of kinds and the comparison of arithmetic width with rendered width over
every node in both corpora is the test.

**Precedent.** Yelland's rfmt computes an optimal layout from a cost model without rendering
each candidate ([Google Research](https://research.google/pubs/pub44667/)); zig fmt decides
layout from a syntactic signal (the trailing comma) with no width measurement at all
([Ziggit](https://ziggit.dev/t/steering-zig-fmt/15400)). Neither transfers directly (rustfmt's
decisions are width-based), but both show the decision needs the width, not the string.

**Bound.** 15.6–18.1% at most; exact-only subset (kinds with a proven width function) perhaps
a third of that.

**Falsifier.** Any node in either corpus where arithmetic width differs from rendered width
for a kind the enumeration claims.

**Cost.** Medium: a width function per claimed kind and a harness that compares it against
rendering over both corpora (the E17 tree-equality harness is the model).

### T8. The cliff hunt, designed to be decisive

**Mechanism.** Generated fixtures by shape, at 1x, 2x, 4x, 8x (nesting depth or item count):
method chains, nested closures, nested calls with generic arguments, match arms, long argument
lists, struct literals, long comments and strings, files of many identical items. For each,
instructions on the single file by callgrind, fit a power law `Ir = a·n^k`; `k > 1.1` on any
shape is a cliff, and the call counts (`callgrind_annotate --inclusive=yes`) name the repeated
traversal. rustfmt's own `overflow` retries are a candidate source: each nesting level can
try several tactics and chloro's memo must cap it; the hunt verifies that it does.

**Precedent.** The autoresearch runs whose biggest single wins were structural changes to the
work, not inner-loop tuning ([Karpathy's nanochat run, 2.02 → 1.80 h over about 20 kept
changes](https://datasciencedojo.com/blog/karpathy-autoresearch-explained/); the swarm
write-up that kept negative results for later search
([rekursiv.ai](https://rekursiv.ai/blog/autoautoresearch/))); the sub-millisecond Advent of
Code effort, where every day's solution came from the problem's structure before SIMD
([r/rust, aoc-fastest](https://redlib.hbubli.cc/r/rust/comments/1hrgir5/solving_aoc_2024_in_under_1ms);
[cprimozic, Day 9](https://cprimozic.net/blog/optimizing-advent-of-code-2024/)).

**Bound.** Unknown until found; a single superlinear path dominates the largest files while
being invisible on the median fixture (M4: `fx-hir-ty` 283 ms for 2.9 MB against `fx-parser`
30.8 ms for 235 KB, 1.08x the per-byte cost, so no large cliff on the corpus; the hunt is for
shapes the corpus under-represents).

**Falsifier.** Every shape's exponent within 1.0–1.1 closes the hunt with a negative result
worth recording.

**Cost.** A generator script and a day of reading profiles; fixes are separate gated commits.

### T9. The parser, and when a fork becomes worth it

**Mechanism.** `ra_ap_parser` is 28% of instructions with a 33–38 M floor and the worst
branch behaviour in the binary. Everything fork-free is bounded at about 7% (P2, P7). A fork
(P4/P11 in the record: own lexer, own parser) could use a byte lexer with lookup tables, a
struct-of-arrays event output and a predictable tree build, the design of Zig's and Carbon's
toolchains; the record prices it at 5,195 lines and 15 re-opened ledger cases. The trigger
for reconsidering is quantitative: when the formatter's remaining exact levers (T2, T6, T7) are
landed and the parser is above 40% of the sample.

**Precedent.** Zig's AST layout rework (indices, struct-of-arrays): 15% fewer cache misses,
22% wall; over 30% in later phases ([Kelley commit notes](https://git.jakstys.lt/motiejus/zig/src/commit/4dca99d3f6b732c415d270f0c97def144ed6d3b7);
[TU Delft paper](https://repository.tudelft.nl/file/File_5748302e-41df-4214-b489-32eca09e39bf));
Carbon's toolchain designed around dense token buffers and locality
([CppNow 2023 session](https://schedule.cppnow.org/session/2023/modernizing-compiler-design-for-carbons-toolchain/));
Lezer and tree-sitter for incremental reuse of subtrees
([Lezer guide](https://lezer.codemirror.net/docs/guide)).

**Bound.** From 33–38 M toward perhaps 20 M: 5–7% of the sample, at the highest cost in the
programme.

**Falsifier.** Not an experiment now; the trigger above.

### T10. Diagnostics that change what gets attempted

- The branch and cache co-metric in `profile.sh` (`--branch-sim=yes --cache-sim=yes`, reporting
  `Ir`, `Bcm`, `Bim`, `I1mr`, `D1mr`): without it T1 cannot be judged and the wave will keep
  optimising a number that is a third of the story.
- `hotpath-rs` behind a feature as the owner asked, for the per-function allocation view
  (bytes and counts) that DHAT gives less conveniently; timing from it is not used for
  decisions ([hotpath docs](https://docs.rs/hotpath/latest/hotpath/)).
- The "research process" point of the second pass is already the protocol; what is added is a
  bound-before-build rule: no experiment starts without a stated share of the sample it can at
  most remove, and a falsifier.

### Where this pass differs from the two above

- The simdjson analogy transfers to the parser, not to the formatter: scanning is no longer
  where chloro's instructions go (E3 and E16 took the local scanning levers; `core`/`std`
  searchers are 7.9%), and the parser is a dependency. SIMD is therefore ranked below layout
  and dispatch work, which the profile's mispredict and miss counts single out.
- The "formatting-event tape" (variant B) is, for chloro, the transactional sink with
  checkpoints (T6): the retry protocol needs undo, not a second IR, and the undo is one
  integer. A full event tape would pay for a second representation the sink does not need.
- The arena (T5) is worth one bounded experiment but must not run at the same time as the
  sink; both are allocation levers on the same paths and would be unattributable together.
- `hotpath-rs` is a convenience; the deterministic counts and the simulator already attribute
  instructions and misses per function without instrumentation. It is included for the
  allocation-by-function view only.
- The cliff hunt is agreed and is first in the wave, with a stated form for its negative
  result so it cannot run open-endedly.

## 6. Precedent catalogue

| trick | precedent | reported gain | what transfers | source |
|---|---|---|---|---|
| decide structure once, scan with minimal branching | simdjson | a quarter or fewer instructions than a reference parser | the parser (dependency) and dispatch shape | [arXiv 1902.08318](https://arxiv.org/abs/1902.08318) |
| indices instead of pointers, struct-of-arrays AST | Zig stage2 AST rework | 15% fewer cache misses, 22% wall; >30% in later phases | memo side array (T2); the flat tree already has it | [commit notes](https://git.jakstys.lt/motiejus/zig/src/commit/4dca99d3f6b732c415d270f0c97def144ed6d3b7), [TU Delft](https://repository.tudelft.nl/file/File_5748302e-41df-4214-b489-32eca09e39bf) |
| dense token buffers, locality-first design | Carbon toolchain | (talk; no figure retrieved) | T9's design if a fork is ever justified | [CppNow 2023](https://schedule.cppnow.org/session/2023/modernizing-compiler-design-for-carbons-toolchain/) |
| post-link layout | BOLT on Clang; Android Rust toolchain; rustc | +12% on top of PGO+LTO; 24.8% whole pipeline; −1.8% cycles at +48% size | T1 | [LLVM BOLT docs](https://llvm.googlesource.com/llvm-project/+/refs/heads/main/bolt/docs/OptimizingClang.md), [Android blog](https://android-developers.googleblog.com/2023/12/faster-rust-toolchains-for-android.html), [rust-lang #103595](https://github.com/rust-lang/rust/issues/103595) |
| mispredicts over instructions as the time predictor | branch-avoiding graph algorithms | mispredictions correlate with time more than instructions | T1's co-metric | [arXiv 1411.1460](https://arxiv.org/pdf/1411.1460) |
| branchless inner loops, SWAR, mmap | One Billion Row Challenge | >4 min → <2 s (Java); 480 s → 1.5 s (C) | T1's dispatch tables; nothing for SIMD until a scan is shown hot | [InfoQ](https://www.infoq.com/news/2024/01/1brc-fast-java-processing), [van Kooten](https://www.dannyvankooten.com/blog/2024/1brc/) |
| layout IR separated from printing | Ruff / Biome (Wadler-style IR) | Ruff 30x Black at >99.9% compatibility | T6's single growing buffer | [Astral](https://astral.sh/blog/the-ruff-formatter), [biome_formatter](https://docs.rs/biome_formatter/) |
| linear-time printing with bounded lookahead | Oppen 1980 | linear in input, independent of width | the idea that width, not strings, drives decisions (T7) | [Kent, Swierstra & Chitil](https://www.cs.kent.ac.uk/pubs/2009/2847/content.pdf) |
| layout from a cost model without rendering candidates | Yelland's rfmt | (optimal layout; no speed figure) | T7's width functions | [Google Research](https://research.google/pubs/pub44667/) |
| layout from a syntactic signal, no width | zig fmt trailing comma | (design) | T7's cheap decisions where rustfmt's rule is syntactic | [Ziggit](https://ziggit.dev/t/steering-zig-fmt/15400) |
| resident daemon to remove start-up | eslint_d, prettier_d_slim, blackd | 700 → 160 ms; 700 → 130 ms | T4 | [eslint_d](https://github.com/dpinol/eslint_d.js/blob/master/README.md), [prettier_d](https://github.com/josephfrazier/prettier_d), [blackd](https://manpath.be/f29/1/blackd) |
| shared-memory parallelism | TypeScript Go port | half of a 10x | T3 | [heise](https://www.heise.de/en/background/Web-development-The-new-TypeScript-compiler-in-Go-10357839.html?seite=2) |
| per-core throughput by design | Sorbet | 100k lines/s/core to 64 cores | T3 | [i-programmer](https://i-programmer.info/news/98-languages/11936-sorbet-making-ruby-statically-typed.html) |
| arena allocation for a parser | oxc (bumpalo-derived) | (parser 3x–10x claims, vendor) | T5 | [oxc_allocator](https://docs.rs/crate/oxc_allocator/0.143.0/source/README.md) |
| bump arenas against malloc | general | 2–5x typical, 10x best case for small short-lived objects | T5's bound | [HN](https://hn.svelte.dev/item/44284592) |
| incremental reuse of subtrees | Lezer, tree-sitter; Salsa early cutoff | (design) | T4 and the unit cache | [Lezer guide](https://lezer.codemirror.net/docs/guide) |
| structure first, then SIMD | sub-1 ms Advent of Code; cprimozic | all 49 puzzles under 1 ms | T8 | [r/rust](https://redlib.hbubli.cc/r/rust/comments/1hrgir5/solving_aoc_2024_in_under_1ms), [cprimozic](https://cprimozic.net/blog/optimizing-advent-of-code-2024/) |
| many small kept changes, negative results kept | autoresearch (nanochat) | 2.02 → 1.80 h; swarm 6,164 experiments | the loop's process | [datasciencedojo](https://datasciencedojo.com/blog/karpathy-autoresearch-explained/), [rekursiv.ai](https://rekursiv.ai/blog/autoautoresearch/) |
| `memchr`'s vectorised search | memchr crate | 2x on ripgrep's simple searches (aarch64, 2.6.0) | only where a scan is shown hot | [memchr docs](https://docs.rs/crate/memchr/latest) |
| number parsing redesigned | fast_float | 4x `strtod`, ~1 GB/s | nothing direct; the method | [fast_float README](https://gnu.googlesource.com/gcc/+show/trunk/libstdc++-v3/src/c++17/fast_float/README.md) |

What the search did not find: any report of rustfmt running in superlinear time on nested
chains or closures (the exponential-time reports are rustc's); a published figure for
`blackd`'s latency; Sorbet's or Carbon's technique list in text (both are in talks).

## 7. The plan: wave 3 and the loop

Ranked by supported gain per unit of risk, with the disagreements above applied. Each
experiment: one worktree and branch (`20261015-<slug>`), hypothesis and bound stated in its
record before code, the protocol's gates at every commit, `Ir` plus the branch/cache co-metric,
goldens sequential and at 4 threads, `conform -i` at both versions, the ledger row on landing,
and a negative result recorded in the same form as a positive one.

| id | experiment | record | bound | first falsifier | wave |
|---|---|---|---|---|---|
| E19 | cliff hunt and repeated-work audit (T8, pass-2 §2–3): scaling fixtures by shape, power-law fit, call-count audit of parent/boundary/fit recomputation; fixes as separate gated commits | `EXP-019-cliffs.md` | unknown; negative result closes it | every exponent in 1.0–1.1 and no repeated computation above 0.5% | 3 |
| E20 | transactional shared sink on `write_list`, then chains and overflow (T6; decide-once row 1) | `EXP-020-sink.md` | 8.8–10.5% Ir (12% with width bookkeeping) | `write_list` converted and Ir not down by its string-assembly share | 3 |
| E21 | resident process `chloro serve` with the unit cache in memory; latency, RSS and identity measured (T4) | `EXP-021-resident.md` | median save < 1 ms against 4.3–6.0 | median not below 1 ms, or RSS growth, or any output difference | 3 |
| E22 | the Ir-blind budget (T1, T10): branch/cache co-metric in `profile.sh`; hot/cold attributes on the six functions; kind flag tables; BOLT trial on the PGO binary | `EXP-022-layout.md` | about 8% wall, optimistic 12% | `Bcm+Bim` and `I1mr` not down, or down without a wall gain in 3 pairs | 3 |
| E23 | unit queue across files (T3) | `EXP-023-unit-queue.md` | 6.6–34.6% four-core wall | four-core wall not below base's range; one-core regresses | 4 |
| E24 | memo side array (T2) | `EXP-024-memo-array.md` | 1–3% Ir | Ir not down by the measured lookup cost | 4 |
| E25 | per-file bump arena for rewrite strings (T5), after E20 lands | `EXP-025-arena.md` | 5–8% Ir | Ir not down 3%, or RSS > 1.2x on `stdin-lints` | 4 |
| E26 | width functions for proven kinds (T7) with the arithmetic-versus-rendered harness over both corpora | `EXP-026-width.md` | a third of 15.6–18.1% | any kind with a width mismatch | 4 |
| E27 | PGO training mix (small / large / mixed) and its effect per row; `hotpath-rs` feature for allocation-by-function | `EXP-027-training.md` | diagnostic | — | 4 |
| — | parser fork (T9) | — | 5–7% at the highest cost | trigger: parser > 40% of the sample after E20–E26 | not planned |

Loop rules for this phase, added to the protocol's: a bound and a falsifier before code;
`Ir`, `Bcm+Bim`, `I1mr` and three interleaved `bench` pairs on every kept commit; wave 4
starts only after wave 3's merges are reproduced on the integration host and the ledger
carries them; a quiet-host measurement (M5) closes the phase; the best-known-good moves only
on reproduced gates.

## Current State

- This entry compiles the owner's two ChatGPT passes and this session's pass with 20
  precedents and their sources; wave 3 (E19–E22) is being launched on worktrees from
  `bf05b31` as it is committed, wave 4 (E23–E27) is queued behind it.
- Nothing in this entry is a measurement of chloro; the bounds are the records', the
  precedents are other projects' published figures, and the IPC and penalty estimate in §1 is
  derived and marked so.

## Missing

- Hardware counters: the container has no `perf`; the co-metric is the simulator's.
- The exclusive instruction cost of `memoize`'s lookups and of each `write_list` string
  assembly site on the final head (a fresh annotated profile; E19/E20 take it first).
- Sorbet's and Carbon's technique lists in citable text; both are in recorded talks.
