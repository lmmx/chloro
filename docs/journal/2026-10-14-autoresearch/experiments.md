# Experiment ledger

Append-only. One row per experiment; the procedure and gates are in
[protocol.md](protocol.md). Rows with more than a page of evidence link to a file under
`experiments/`.

## Best-known-good

| field | value |
|---|---|
| commit | `884f9c9` (master at the start of the programme) |
| `conform -i` | 1219/1220 identical, 0 not idempotent |
| `bench -n 5`, 1238 fixtures, 13.9 MB | 3.041 s, 4.57 MB/s (host: Xeon @ 2.80GHz, 4 vCPU, 2026-10-14) |
| golden hashes | `scratchpad/golden/fixtures-884f9c9.tsv` (outside the repository; re-record with `bench --record` at this commit) |
| `Ir` on the 49-file sample | pending ([profile-baseline.md](profile-baseline.md)) |

## Ledger

| id | branch | hypothesis | metric, expected | result | gates | decision | commit | date |
|---|---|---|---|---|---|---|---|---|
| E1 | `20261014-parallel-items` | formatting a file's top-level items on several threads, with the gaps, reorder groups and skips kept sequential, cuts a large file's wall time by close to the core count with identical output ([EXP-001](experiments/EXP-001-parallel-items.md)) | process wall time on the five largest fixtures, ≥ 1.5x on 4 cores; identical output | 1.50x–1.88x (minima of 15 runs) on the five files; crates 1.24x–1.82x, of which 1.24x–1.49x from a largest-first file queue alone; single core within 1% on files, 1–6% on crates; CPU +4–21% per split file; 0 of 1238 fixtures and 0 of 21,466 registry files differ at 1–4 threads; `conform -i` 1219/1220, 0 not idempotent (host: Xeon @ 2.10GHz, not the BKG host) | 1–3 passed; 4 (`Ir`) not applicable (threads); 5 as `bench -n 5` per file; 6–7 not run (no output change) | kept; hypothesis revised: the gain is bounded by the parse and the longest item, about 2x on this corpus | `d60c1fa`, `f847a87` | 2026-10-14 |
