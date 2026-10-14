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
| E2 | `20261014-cli-build` | build profile (fat LTO, one codegen unit), PGO, a CLI-only allocator, a largest-first work queue in the CLI scheduler, and the process's fixed costs each move the process-level wall time against rustfmt; measured one at a time, then combined | process wall time per workload (hyperfine, pinned and unpinned), peak RSS, `bench -n 5` for the codegen variants; expected 2–10% from each codegen lever, more on skewed crates from the scheduler; refuted for a lever when its interleaved ranges overlap the base | combined (profile + scheduler + PGO) against base: `bench -n 5` pairs −29%, −17%, −20% with non-overlapping ranges; unpinned minima 0.55–0.74x on the five crates, 0.53–1.03x on single big files; peak RSS 0.5–0.8x on crates; mimalloc: no gain outside the spread, 4–10x peak RSS; `panic=abort` and `target-cpu=native` inside the spread, not shippable; details in [experiments/EXP-002-cli-build.md](experiments/EXP-002-cli-build.md) | fmt, clippy, tests pass; `bench --check` 0 of 1238 changed and `conform -i` 1219/1220, 0 not idempotent, on every codegen variant; sorted `--check` reports identical | kept: profile, scheduler, `benchmarks/pgo.sh`; rejected: mimalloc, `panic=abort`, `target-cpu=native` | `0bb8e06`, `6c55740`, `8efd333` | 2026-10-14 |
