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
| E1 | `20261014-parallel-items` | top-level items of one file formatted on several threads, stitched by the sequential visitor's gap handling, give identical output | wall time of the five largest fixtures and of `hir`/`hir-ty` crate-level runs; expected up to the core count on one large file, no `Ir` change | running (sibling session) | | | | 2026-10-14 |
| E2 | `20261014-cli-build` | LTO + 1 codegen unit, PGO, a CLI-only allocator and largest-first dynamic scheduling each move process-level time; measured one at a time then combined | `bench` throughput and hyperfine crate-level/single-file/tiny-file wall time; expected 5–25% from codegen, scheduling gain on skewed crates | running (sibling session) | | | | 2026-10-14 |
| E6 | `20261014-cli-cache` | a CLI cache keyed by content digest, effective config and formatter identity makes `--check`/`--write` on an unchanged tree I/O-bound with no output change | warm vs cold wall time on the 1238 fixtures; expected ≥10x on the hit path | running (sibling session) | | | | 2026-10-14 |
