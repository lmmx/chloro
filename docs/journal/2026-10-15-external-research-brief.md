# An external research brief and report on the next gains, and what they add (2026-10-15)

The owner asked ChatGPT to turn the sources of
[2026-10-15-unmined-tricks.md](2026-10-15-unmined-tricks.md) into a research brief for a
separate conversation, ran that brief, and asked that the whole thread be journalled and then
compared with the work already identified here. The five documents are recorded verbatim under
[2026-10-15-external-research-brief/](2026-10-15-external-research-brief/):

| document | what it is |
|---|---|
| [01-brief.md](2026-10-15-external-research-brief/01-brief.md) | the research brief as ChatGPT generated it (the prompt the owner gave the research conversation), with its bibliography of twenty sources and its own rationale |
| [02-first-exchange.md](2026-10-15-external-research-brief/02-first-exchange.md) | the research conversation's first reply (an approach, not the research) and the owner's retort |
| [03-research-report.md](2026-10-15-external-research-brief/03-research-report.md) | the research report: executive synthesis, seven deep dives (A–G), comparison matrix, roadmap, "what not to do yet", source audit |
| [04-what-it-adds.md](2026-10-15-external-research-brief/04-what-it-adds.md) | ChatGPT's own summary of what the report adds, and its suggested backlog order |
| [05-next-step-message.md](2026-10-15-external-research-brief/05-next-step-message.md) | ChatGPT's "next step" message of 2026-10-17 with the reconciliation instructions the session carried out (§6 below); the same paste repeated documents 1 and 4, which are not stored twice |

**Status of these documents.** They are external research synthesis (document 5 is an instruction, not research). Their claims and
recommendations are pending verification against this repository's records; the report says
itself that it did not inspect chloro's source, profiles or harness, and that it could not
verify the Zig commit's figures. Where a figure in the report comes from this repository it is
quoted from the brief, which took it from the records; the 405-file directory timings
(218.5 ms against 275.4 and 554.8 for two releases) are the owner's own measurements from
elsewhere and are not in this repository. Nothing in the four documents changes a measured
number here.

The rest of this entry is this session's assessment, written on 2026-10-15 with the first two
wave-3 reports (E20, E22) in hand: §1 what the report adds that the unmined-tricks entry did not
have; §2 what it sharpens; §3 what the wave has already answered, including where the report's
expectations were wrong for chloro; §4 the changes made to the plan and the protocol as a
result; §5 what is not adopted and why.

## 1. New, and adopted

1. **A second instruction-count sample as a validation set.** The report's point G and the
   summary's rule 6: an optimisation selected on one sample can fit that sample's syntax
   distribution. The programme's primary metric has been the instruction count on one 49-file
   sample drawn from the fixtures, and every lever since E3 was selected on it. The registry
   corpus validates conformance but not performance. Adopted: a second 49-file sample drawn
   from the registry corpus (`profile.sh --sample`, which already exists), reported in every
   record beside the fixture sample; a kept lever must not regress it. This also addresses the
   heap-layout term E20 found (below): two samples make a layout-driven swing on one of them
   visible.
2. **Rollback is state, not just bytes.** Report C5 and summary rule 3, from Biome's state
   snapshots: a transactional sink that truncates the buffer but leaves width, indentation,
   pending whitespace or memo entries mutated would be wrong. E20's kept `write_list_into`
   carries no such state, so truncation is complete there; the next sink steps (the S6/S9
   bookkeeping of the last-newline position and the column, E26's width functions) would carry
   it. Adopted as a design rule for those steps: a `Checkpoint` struct holding every mutable
   field the attempt can change, restored as a unit, with a mutation test that fails if any
   field is left out.
3. **Dense tables cost at initialisation, and the fix is a generation counter.** Report A2:
   clearing an N-slot side array per file can cost more than the hash lookups it replaces on
   small files; a generation stamp per slot avoids the clear; per-node slot against node×shape
   table depends on the shape cardinality per node, which must be measured first. Adopted into
   E24's brief: instrument hit rate, occupancy and shapes-per-node on both samples before
   choosing the layout; measure small and large files separately; lazily-initialised or
   generation-stamped storage.
4. **Fit the fixed term.** Report F: `T(n) = a + b·n^p`, with the additive constant, rather
   than the unmined-tricks entry's `Ir = a·n^k`. For in-process instruction counts on single
   files the constant is the per-file overhead (parse set-up, settings, the file's tree) and is
   not negligible at the 1x scale, so a fit without it overstates the exponent of small shapes.
   Adopted into E19 (sent to the running agent for its fit; recorded here in case the record
   predates the message).
5. **Measure imbalance before scheduling work.** Report E1 and summary rule 5: per-file and
   per-phase durations, worker utilisation and the completion-time tail distinguish uneven
   file sizes from serial phases, queue starvation and scheduling overhead; a unit queue is the
   remedy for only the first. Adopted as E23's first step, with a stop rule: if the tail is
   not the idle cores' cause, the queue is not built.
6. **A parse-only phase in the harness.** Report A3: the parser's wall share and scaling should
   be measured independently before any parser decision. `bench` times `format_source`
   whole; a `--phase parse` option (parse and build the flat tree, no formatting) gives the
   parser's own MB/s and instruction count per file, and with the cliff generator its own
   exponents. Adopted into E27 (harness and diagnostics).
7. **The daemon's client is part of the latency.** Report E2, from `eslint_d`'s caveat: a daemon
   helps only where the client would otherwise repeat work, and an editor that keeps its own
   long-lived process gains nothing. rust-analyzer runs the configured formatter as a new
   process per request, so for chloro the forwarding client's own start-up is on the critical
   path and is what E21 must measure (median and p99 of the client round trip, not the
   server's handling time alone). Added to E21's acceptance criteria.

## 2. Already identified, sharpened by the report

| topic | in this repository | what the report adds |
|---|---|---|
| dense memoisation | T2, E24 | the layout alternatives (per-node slot, node×shape, mask plus values), initialisation and clearing cost, the instrumentation to run first |
| transactional sink | T6, E20 | the state-snapshot framing (adopted, §1.2); "instrument retries and discarded bytes first" (E20 did this: `write_list`'s cost was scanning, not assembly) |
| Ir-blind budget, BOLT, branchless predicates | T1, E22 | "frequent is not mispredicted"; inspect the assembly and the per-branch miss rate before rewriting; four comparable binaries; relocation and size costs counted (E22 measured all of these, §3) |
| cliff hunt | T8, E19 | the additive term (adopted, §1.4); exponent drift as a cache or allocation phase change rather than an algorithm |
| arena against sink | T5, E25 queued after E20 | the same separation rule, plus "retained capacity between files" and one-thread against multi-thread memory as reported quantities |
| resident process | T4, E21 | the two scoreboards (already E21's design) with cold against warm requests made explicit; file renames and configuration changes as invalidation tests |
| parser fork | T9 | the parser-only profile as the prerequisite (adopted as a harness phase, §1.6) |
| research process | protocol, T10 | selection bias named as the mechanism behind the confirmation-run and validation-corpus rules (adopted, §1.1) |

## 3. What the wave has already answered

Two of the report's five ranked directions were under test when the report arrived, and their
results bear on it:

- **Transactional sink (report rank 2, "medium confidence in payoff").** E20
  ([EXP-020-sink.md](2026-10-14-autoresearch/experiments/EXP-020-sink.md), reproduced by the
  integrator): the sink steps removed 1.86% of instructions against the 8.8–10.5% bound, and
  `write_list`'s cost was two per-item scans (4.86 M), not string assembly; the chain step was
  rejected because growing one shared buffer cost more in `realloc` copies than it saved and
  moved the parser's own vector growth by +3.5–4.7 M through heap layout. The report's "promising
  only if the existing retries really do rebuild substantial output" was the right caution; the
  answer for chloro is that they do not, under glibc, beyond `write_list`'s call site. The kept
  total is −4.10% (248.20 M → 238.03 M), most of it from computing the two scans where they are
  read.
- **Binary layout and BOLT (report rank 3, "medium-low confidence").** E22
  ([EXP-022-layout.md](2026-10-14-autoresearch/experiments/EXP-022-layout.md), reproduction in
  progress at the time of writing): the co-metric now exists (`profile.sh --sim`: Bcm 7.85%, Bim
  37.1%, I1mr 1.96% of instructions at the base; mispredicts plus L1 misses about 28% of
  estimated cycles, consistent with the entry's "about a third"). But the misses come from a hot
  footprint of 49 KB (80% of instructions) against a 32 KB L1 instruction cache, not from cold
  code: hand splitting is bounded at about 2% of misses and was rejected; kind predicates carry
  1.8% of mispredicts and the flag table raised instructions; function order alone moves I1mr
  ±1% and Bcm ±0.5% between random relinks, so any layout candidate needs four or more shuffled
  relinks to measure. BOLT on the PGO binary: identity held, I1mr −9.1%, Bcm −3.4%, wall within
  ±4% in both directions under load, binary 2.47x larger (2.67 → 6.59 MB). Not proposed for the
  release workflow; the layout lever is closed under the plateau rule. The report's "do not
  assume BOLT will deliver the Clang result" was right, and the mechanism is now known: the
  hot path is simply larger than L1i, which layout cannot fix and only less code can. One
  change was kept: `steps_fused` finishes nodes at their own `Exit` (−0.105% instructions,
  −2 to −3% of mispredicted branches).

The report's rank 1 (dense memoisation) is E24, queued; rank 4 (scaling) is E19, running; rank
5 (persistent process) is E21, running.

## 4. Changes to the plan and the protocol

- Protocol §3 gains a second sample: `benchmarks/profile.sh --sample` on a 49-file registry
  sample, recorded with the fixture sample in every record from E23 on; a kept lever regresses
  neither. The sample list is committed under `benchmarks/samples/` so it is reproducible.
- Protocol §4 gains a confirmation run: a kept lever's instruction counts are re-run by the
  integrator from a clean build before merge (already practice, now written), and any change
  under about 2 M instructions is reported with and without the parser's vector-growth term
  (E20's finding) until that term is made layout-independent.
- Wave 4's order follows the report's and the summary's information-value argument where it
  differs from the entry's: E24 (memo side array, cheap and bounded) before E23 (unit queue),
  and E23 opens with the imbalance diagnostic; E25 (arena) stays behind E20 and now also behind
  E24, since both change allocation patterns on the same paths; E27 (harness: parse-only phase,
  the registry sample, the growth-term figure) moves up to run alongside E24.
- The layout lever (T1's splitting and BOLT parts) is closed; T1's remaining content is the
  co-metric itself and "less hot code", which is what the sink, the memo array and the width
  functions are.

## 5. Not adopted

- **A full document IR.** The report and the summary both decline it; so does the entry. E20
  confirms the sink is the part that transfers and that even it is bounded under glibc.
- **Incremental parsing.** Deferred by the report and by T9; E21's unit cache gives the editor
  path reuse at item granularity without parser changes, and the parse-only phase will say
  what a re-parse costs before anything larger is considered.
- **A general arena before phase-attributed allocation costs.** The report's rule; E25 already
  waits on E20 and now E24. E20's attribution builds (allocator arcs by caller: formatter 27.9 M,
  parser 7.7 M at the head) are the phase costs the report asks for.
- **The report's chloro figures that this repository cannot check.** The 405-file directory
  timings are kept as the owner's and marked so; they are not carried into README or records.

## 6. Retrospective (2026-10-17): the report's ideas against the ledger after waves 3–5

Written on 2026-10-17 at the owner's request (document 5), after waves 3–5 (E19–E31) and M6.
Every status below is read from the committed ledger ([2026-10-14-autoresearch/experiments.md](2026-10-14-autoresearch/experiments.md))
and the experiment records, not from the report or from memory; "covered" means an experiment
with its gates reproduced on the integration host measured the idea on chloro, whatever the
outcome.

### 6.1 The seven candidates named in document 5

| candidate (document 5's wording) | status | the evidence in this repository |
|---|---|---|
| LLVM BOLT on top of the existing PGO build | **covered, closed** (E22) | BOLT 20 applied to the PGO build ([EXP-022](2026-10-14-autoresearch/experiments/EXP-022-layout.md) § BOLT trial): output identical, `I1mr` −9.1%, `Bcm` −3.4%, `Ir` −0.08%, the simulator's cycle estimate −1.6%, wall inside ±4% on CLI minima; not proposed for the release workflow; the layout lever closed under protocol §7. The report's Clang figures (202.72 → 180.11 s) remain the source's, not reproduced here. What is not done: a quiet-host wall pair of PGO against PGO + BOLT (E22's wall rows were taken under load). |
| controlled scaling experiments for algorithmic cliffs | **covered** (E19, E26, E30; M5, M6) | `benchmarks/cliffs/` generates 20 input shapes at geometric scales and fits `a + b·n^p` (the additive term from this report, §1.4); E19 found the nested-closure cliff (exponent 3.6), E26 removed it (depth 16: 105.4 M → 5.5 M; exponent 0.03 over depths 8–16), E30 re-ran five shapes (exponents 0.94–1.31, none moved by more than 0.001 from its rule); M5 and M6 carry the shape on base, head and head-pgo with identical output. |
| more complete rollback state for transactional output sinks | **covered in part; the rest closed** (E20) | the kept sink (`write_list_into`) restores the buffer length only, which is complete there because the sites it covers mutate nothing else ([EXP-020](2026-10-14-autoresearch/experiments/EXP-020-sink.md)); the chain's growing-buffer sink, which would have carried more state, was rejected on cost (+2.0–2.5 M) before any state question arose; the `Checkpoint` design rule of §1.2 stands for any future sink step and has had no occasion to apply. |
| measuring per-file parallel load imbalance before finer-grained scheduling | **covered, twice** (E23, E29) | E23 built the `CHLORO_TRACE` timeline and the reconstruction from thread CPU clocks and stopped the unit queue by its rule (tail ≤ 12.9%); E29 measured the split file's own schedule on patched traces and showed M5's `fx-hir` tail to be the release trace's blind spot (idle 0.7–1.6 core-ms; the schedule 1.1–1.5% above the work-conserving bound), built two scheduling levers, reverted both ([EXP-029](2026-10-14-autoresearch/experiments/EXP-029-split-tail.md)). |
| independent confirmation runs and a separate validation corpus | **covered, in the protocol** (E27; §3 and §4.6) | the registry validation sample (`benchmarks/samples/registry49.txt`, 49 files of four crates) is reported beside the fixture sample in every record from E23 on and a kept lever regresses neither (E30's rule C was reverted on exactly this: +10 K on the registry sample); every merge's gates are re-run by the integrator from a clean build before merging, and the per-file figure separates a lever's effect from the heap-layout term. |
| memoisation representation and reset-cost experiments | **covered, closed** (E24, E28) | E24 measured hit rate (3.2% fixtures, 0.6% registry), occupancy and shapes per node before building, then four side-array forms including an in-place clear and chunked storage: best −0.16% per file, rejected; the clear cost was measured (0.25 M of a 2.34 M container cost) ([EXP-024](2026-10-14-autoresearch/experiments/EXP-024-memo-array.md)); E28 counted the store cost per lookup (692 `Ir`) against the hits and kept a depth gate (−2.49%) ([EXP-028](2026-10-14-autoresearch/experiments/EXP-028-memo-policy.md)). |
| incremental parsing distinguished from compact parsing, evidence before a redesign | **compact parsing covered; incremental parsing not pursued, with the evidence recorded** | compact: E7b (flat tree), E17 (fused build), E31 (own lexer front end, −8.06% / −14.76%); the parse-only phase is measured per record (33.7% / 41.7% at M6) and the fork's bound is derived ([EXP-031](2026-10-14-autoresearch/experiments/EXP-031-parse-floor-2.md) § Decision). Incremental: no experiment; the evidence the brief asks for first is in E21 (a save through a fresh client costs 3.1–3.3 ms, of which the process alone 2.1–2.3) and E29 (the parse is 88–93% of a split file's sequential time, 10–13 ms for 236–640 KB files), so re-parsing dominates only the largest files on the editor path; deferred, not refuted. |

### 6.2 What became of the seven additions adopted on 2026-10-15 (§1)

1. The registry validation sample: built by E27, in every record since, decisive once (E30's
   rule C).
2. Rollback as state: no sink step after E20 carried state, so the `Checkpoint` rule has not
   been exercised.
3. Generation-stamped dense tables: E24 measured before building and found the lookup volume
   too small for any layout to matter (0.16% at best); the stamp was not needed.
4. The additive term in cliff fits: in `gen.py fit` from E19 on; M5 and M6 report `p` with `a`.
5. Imbalance measured before scheduling: E23's stop rule held; E29 extended the diagnostic to
   the split file and reversed M5's reading of its tail.
6. The parse-only phase: in `bench --phase parse` and `profile.sh --phase parse` from E27 on;
   it put the parser's share at 37.3% / 43% (E27), 39.1% / 46.6% (M5) and 33.7% / 41.7% (M6),
   which is what sent E31 at the lexer and bounds the fork.
7. The daemon client's latency: E21 measured it (3.1–3.3 ms median per save through a fresh
   client) and the lever was refuted for the CLI on exactly this criterion.

### 6.3 The summary's suggested order (document 4 § 4) against what ran

Document 4 proposed: scaling diagnostics, memoisation representation, output/retry
instrumentation, PGO + BOLT, parallel scheduling diagnostics, the persistent process, then
incremental parsing or a redesign. The waves ran, in order of launch: E19 (scaling), E20
(sink, with the retry instrumentation first), E21 (persistent process), E22 (BOLT and layout),
E23 (scheduling diagnostic), E24 (memo representation), E25 (arena), E26 (width bound), E27
(harness), E28 (memo policy), then E29, E30, E31. Every item of the suggested order has an
experiment; the order differed where a lever's bound or an agent's availability put it earlier.

### 6.4 What remains new, and what is unverified

- New and untested: nothing in documents 1–5 names an idea without an experiment in the
  ledger, except the parser fork (P3/P6/P11), which the brief and the report both defer and
  which is the owner's open decision (the decision record in
  [2026-10-14-autoresearch.md](2026-10-14-autoresearch.md) § What is left), and a quiet-host
  wall pair for PGO against PGO + BOLT, which E22 did not take.
- Refinements the report sharpened and the waves used: the additive term in fits, the
  two-sample rule, the imbalance-first rule, the client-latency criterion (§1).
- Still unverified against their sources by any session here: the report's descriptions of
  Biome's state-snapshot API, Lezer's reuse limits, the Zig AST rework's figures and the
  Clang BOLT figures; the report's own chloro figures that came from outside this repository
  (the 405-file directory timings). None of these has been carried into a record or the
  README.

## Current State

- The five documents are recorded verbatim; §1–§5 are the 2026-10-15 assessment and §6 the
  2026-10-17 retrospective.
- The seven additions of §1 are in the protocol (§3 two samples, §4.6 confirmation run), the
  harness (`profile.sh --per-file`, `--sample-list`, `--phase parse`; `cliffs/gen.py fit` with
  the additive term) and the records of E19, E21, E23, E24, E27, E29 and E30.
- Every experiment the report and the summary proposed has a ledger row with its gates and a
  decision (E19–E31); the parser fork is the one proposal without an experiment.

## Missing

- The report's claims about Biome's snapshot API, Lezer's reuse limits, the Zig AST rework and
  the Clang BOLT run are the report's; none has been read against its source by a session here.
- A quiet-host wall pair of PGO against PGO + BOLT (E22 measured instructions, the simulator's
  cache and branch events, and wall under load).
