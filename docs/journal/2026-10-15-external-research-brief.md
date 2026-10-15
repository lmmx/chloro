# An external research brief and report on the next gains, and what they add (2026-10-15)

The owner asked ChatGPT to turn the sources of
[2026-10-15-unmined-tricks.md](2026-10-15-unmined-tricks.md) into a research brief for a
separate conversation, ran that brief, and asked that the whole thread be journalled and then
compared with the work already identified here. The four documents are recorded verbatim under
[2026-10-15-external-research-brief/](2026-10-15-external-research-brief/):

| document | what it is |
|---|---|
| [01-brief.md](2026-10-15-external-research-brief/01-brief.md) | the research brief as ChatGPT generated it (the prompt the owner gave the research conversation), with its bibliography of twenty sources and its own rationale |
| [02-first-exchange.md](2026-10-15-external-research-brief/02-first-exchange.md) | the research conversation's first reply (an approach, not the research) and the owner's retort |
| [03-research-report.md](2026-10-15-external-research-brief/03-research-report.md) | the research report: executive synthesis, seven deep dives (A–G), comparison matrix, roadmap, "what not to do yet", source audit |
| [04-what-it-adds.md](2026-10-15-external-research-brief/04-what-it-adds.md) | ChatGPT's own summary of what the report adds, and its suggested backlog order |

**Status of these documents.** They are external research synthesis. Their claims and
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

## Current State

- The four documents are recorded verbatim; this assessment is this session's.
- Seven additions adopted into briefs and the protocol (§1, §4); E19 sent the additive-term
  note; E21 sent the client-latency criterion; E23/E24/E27 briefs carry the rest when wave 4
  launches.
- E20 merged next (gates reproduced); E22 under reproduction; E19 and E21 running.

## Missing

- The registry sample and the parse-only phase do not exist yet (E27).
- The report's claims about Biome's snapshot API and Lezer's reuse limits are taken from the
  report; neither has been read against the sources by this session.
