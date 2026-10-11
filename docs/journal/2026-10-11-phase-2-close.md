# 2026-10-11: Closing phase 2

Follows [2026-10-11-parse-drop-thread.md](2026-10-11-parse-drop-thread.md). This entry
closes the phase opened by
[2026-10-10-change-of-direction.md](2026-10-10-change-of-direction.md). It records:
- the final same-window comparison with the proof of concept;
- what phase 2 changed;
- what it established;
- what is left for a later phase.

No code changed in this entry.

## Stopping rule

Phase 2 stops here, by decision, not because the gap to the proof of concept is closed:
- the validated changes are landed;
- the measurement baseline is corrected;
- conformance is reproducible from a fresh checkout;
- the remaining questions are written down below.

Further optimisation is a new phase with its own justification, not a condition for
finishing this one.

## Final comparison (measured)

One window, on the "Xeon @ 2.80GHz" host. This is the host of the phase-1 figures; the
container moved back to it. `bench` was run with each version's own binary, all builds
alternating:
- **fixtures:** the 1220 rust-analyzer fixtures, `--root .../fixtures/rust-analyzer`,
  best of 5, three rounds;
- **registry:** the 5659-file cargo registry corpus, best of 2, one round.

| build | fixtures | registry |
|---|---|---|
| `c4d74ee` (proof of concept) | 8.90–9.42 MB/s | 8.29 MB/s |
| `bcef0f9` (port, before performance work) | 2.72–2.87 MB/s | not measured |
| `9202b42` (port at the start of phase 2) | 3.44–3.63 MB/s | 4.79 MB/s |
| `f6c5be9` (G1 + D1 + G2; HEAD's code) | 4.04–4.14 MB/s | 5.70 MB/s |

Findings:
- **Phase 2 made the port faster.** It gained 12–20% on the fixtures and 19% on the
  registry over `9202b42`, with identical output. Most of this is D1. G1 and G2 are below
  wall-clock resolution on their own.
- **The proof of concept is still faster.**
  - It is 2.2–2.3x faster on the fixtures, down from 2.5–2.7x for `9202b42` in this window.
  - It is 1.45x faster on the registry, down from 1.73x.
- **The overhead regression is partly undone; the rest of the gap is mostly work the port
  does.**
  - The change-of-direction entry established that the proof of concept copies about 76%
    of its input as written, 55.8% through fallback. The port lays that code out, which is
    how it reaches 1219/1220 against the 233/1220 that `fe28548` records for the code
    before the port.
  - Matching the proof of concept's throughput was never a precondition for correctness,
    and phase 2 does not claim it.
- **The proof of concept has the same background-thread cost, in smaller measure (read).**
  It uses the same `ra_ap_syntax` 0.0.307 and calls `SourceFile::parse` once per file
  (`formatter.rs:9` at `c4d74ee`). So it also hands one tree per file to the
  `ParseNodeDropper` thread, but never re-parses macro fragments. Its figures above include
  that cost; D1-style building would make it somewhat faster too. Not measured.

## What phase 2 changed

Each change is a separate commit and can be reverted on its own:

| commit | change | evidence |
|---|---|---|
| `eadb4a5` | G1: inner attributes and attribute paths read from the green tree | −2.46% instructions (pre-D1 basis); 4 regression tests |
| `fc90e53` | D1: syntax trees built without a `Parse`, so formatting is single-threaded | 27,059 → 0 futex calls per fixture pass; deterministic instruction counts; wall clock above |
| `14e45a1` | the 581 parser and syntax `.rustfmt.rs` snapshots tracked; regeneration documented | a fresh checkout reports 1219/1220 |
| `f6c5be9` | G2: keyword presence read from the green tree | −1.15% instructions, −20,999 red nodes and allocations; 1 regression test |

For every code change:
- output hashes unchanged on the fixtures (1238) and the registry (5659);
- `conform` 1219/1220, every output idempotent;
- the 31-option matrix identical to phase 1.

This is broad regression coverage over two corpora and 31 option values, not proof of
equivalence for all Rust programs.

Measurement tools added (`chloro-core/examples/`):
- `parse_floor.rs`;
- `spacing.rs`;
- `fmt_dir.rs`;
- `alloc_count.rs`.

## What phase 2 established

- Most of the proof of concept's speed came from skipped work, not from a cheaper way of
  doing the same work.
- Building rowan trees caps throughput.
- Traversing red nodes costs 4–9x the green walk for the same answers.
- Repeated rewrites are 17–18.5% of `format_source`.
- Top-level items format the same standalone as in-file, once gap text is set aside
  (X12b).
- Instruction counts before `fc90e53` exclude freeing syntax trees and depended on thread
  scheduling. From `fc90e53` on they are reproducible to about 0.0006%.

## Deferred to a later phase

- **Re-ranking red-node paths on the D1 baseline.**
  - The remaining `child::<T>` and `support::child` accessors split into two groups:
    nodes discarded at once (green-tree candidates) and nodes kept for later (cache
    candidates).
  - Rank by instructions and allocations, not node counts.
- **Caching:** `Block::from_block_expr` is called 1.73–1.94 times per block. Caching it is
  one option; the item-level cache that X12b makes possible is another.
- **Representation:** a flat tree or another substrate in place of rowan. The X3/X4 results
  do not settle the ranking.
- **Attribution of D1's gain** between futex syscalls and cross-thread frees.
- **Parallel formatting of items.**

## Current State

- Single-threaded formatting on green-tree reads where G1 and G2 apply
  (chloro-core/src/formatter/formatting.rs:35-59, chloro-core/src/formatter/nodes.rs)
- Conformance 1219/1220 reproducible from a fresh checkout
  (chloro-core/tests/conformance/snapshots/ra/)
- README states 4.1 MB/s on the fixtures and 5.7 MB/s on the registry (README.md)

## Missing

- The proof of concept's throughput is not matched (2.2–2.3x on the fixtures, 1.45x on the
  registry)
- The README's process-time comparison with rustfmt (0.35x–0.71x) predates phase 2; chloro
  has become faster since, so it now overstates chloro's time and has not been re-measured
