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
| E2 | `20261014-cli-build` | build profile (fat LTO, one codegen unit), PGO, a CLI-only allocator, a largest-first work queue in the CLI scheduler, and the process's fixed costs each move the process-level wall time against rustfmt; measured one at a time, then combined | process wall time per workload (hyperfine, pinned and unpinned), peak RSS, `bench -n 5` and callgrind `Ir` for the codegen variants; expected 2–10% from each codegen lever, more on skewed crates from the scheduler; refuted for a lever when its interleaved ranges overlap the base | profile + PGO against base: `Ir` 588.0 M → 448.5 M (−23.7%) on the 49-file sample, `bench -n 5` pairs −20%, −28%, −21% with non-overlapping ranges, unpinned means 0.69–0.94x on the five crates and 0.66–0.98x on single big files; profile alone `Ir` −5.7%, PGO alone −16.2%; scheduler alone 0.67–0.83x unpinned minima on the crates (cross-check for E1, code dropped); mimalloc `Ir` −12.4% but wall time inside the spread and 2–10x peak RSS; `panic=abort` and `target-cpu=native` inside the spread, not shippable; details in [experiments/EXP-002-cli-build.md](experiments/EXP-002-cli-build.md) | fmt, clippy, tests pass; `bench --check` 0 of 1238 changed and `conform -i` 1219/1220, 0 not idempotent, on every codegen variant; sorted `--check` reports identical | kept: profile, `benchmarks/pgo.sh`; dropped: scheduler (E1 carries it); rejected: mimalloc, `panic=abort`, `target-cpu=native` | `0bb8e06`, `d10c1e3` | 2026-10-14 |
