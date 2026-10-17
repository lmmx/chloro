# Project history

<p class="lede">chloro keeps a development journal: dated records of what was decided, what
was measured, and what was tried and did not work. This page is a guide to it. The website
describes chloro as it is; the journal records how and why it got there.</p>

The journal is the [`docs/journal/`][journal] directory of the repository. Entries are
Markdown files named `YYYY-MM-DD-title.md`, written to the format in
[`docs/JOURNAL.md`][format]: statements of what exists, with the files and figures that
support them, and no recommendations.

## Timeline

| when | what happened | entries |
|---|---|---|
| November–December 2025 | 0.5 to 0.7: a hand-written formatter over rust-analyzer's `rowan` trees, matching rustfmt on 233 of 1220 conformance files | the per-crate changelogs |
| 9–11 October 2026 | The rustfmt port: rustfmt 1.9.0's formatting logic ported module by module onto rust-analyzer's syntax trees, reaching 1219 of 1220. Then an investigation of where the port's time goes, and whether the proof of concept was a better foundation (it was not: most of its speed came from skipping work). Released as 0.8.0. | [rustfmt port][port], [performance checkpoint][checkpoint], [change of direction][direction], [item independence][items], [representations][repr], [phase 2 close][close] |
| 14–17 October 2026 | The performance and conformance programme: 31 experiments run in parallel on separate branches, each kept only if it passed every check twice and changed no output it was not meant to change; six full measurements against rustfmt on a quiet machine. Result: 1220 of 1220, rustfmt 1.8.0 support, and 66% fewer instructions. | [programme][programme], [handoff][handoff] |

## Reading the performance programme

The programme's records use short codes. The [handoff][handoff] is the place to start: it is
written for a reader who did not follow the work, translates every code, and summarises each
experiment, failure and measurement with its source.

| code | meaning | where |
|---|---|---|
| E1 … E31 | experiments, one hypothesis each, with a written record | [ledger][ledger], `experiments/EXP-NNN-*.md` |
| R2, R5 | the conformance fixes, and the fixes for a code review's findings | [conformance ledger][conformance] |
| M1 … M6 | the six full measurements against rustfmt on an idle machine | [measurements][measurements] |
| P1 … P11 | the ranked list of parser-side improvements | [parser theory record][parser] |
| "base", "the start" | `884f9c9`, release 0.8.0 | |
| "best-known-good" | the latest commit whose checks were reproduced on the merged branch | [ledger][ledger] |
| `Ir` | instructions executed, as counted by callgrind | |

Some records worth knowing about on their own:

- The [protocol][protocol]: what counts as a result, the checks a change must pass, and how
  a direction of work is closed.
- The [conformance ledger][conformance]: every mismatch with rustfmt found, its root cause,
  and the six cases still open.
- The [rustfmt versions record][versions]: where rustfmt 1.7.0, 1.8.0 and 1.9.0 differ on real
  code, and why 1.7.0 is not supported.
- The records of directions that did not pay, with the measurement that closed each: a
  faster allocator, code layout, a resident server process, scheduling, an exact width
  model. The handoff's sections 7 and 12 list them.

!!! note "Commit hashes in the journal"

    The commits of the performance branch were re-recorded under the maintainer's name
    before the branch was merged, which gave them new hashes. The journal's references
    read `new` (was `old`): the commit's hash in the repository, with the hash recorded at
    the time beside it. Commands, file names and tool output quoted in the records keep the
    original hashes; [2026-10-17-commit-map.md][map] maps every one of them.

[journal]: https://github.com/lmmx/chloro/tree/master/docs/journal
[format]: https://github.com/lmmx/chloro/blob/master/docs/JOURNAL.md
[port]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-09-rustfmt-port.md
[checkpoint]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-10-performance-checkpoint.md
[direction]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-10-change-of-direction.md
[items]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-10-item-independence.md
[repr]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-10-representations.md
[close]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-11-phase-2-close.md
[programme]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch.md
[handoff]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-17-handoff.md
[ledger]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/experiments.md
[conformance]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/conformance-ledger.md
[measurements]: https://github.com/lmmx/chloro/tree/master/docs/journal/2026-10-14-autoresearch/measurements
[parser]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-15-extensions-theory/parser-and-substrate.md
[protocol]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/protocol.md
[versions]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/rustfmt-versions.md
[map]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-17-commit-map.md
