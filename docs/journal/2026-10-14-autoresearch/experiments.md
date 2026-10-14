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
| E6 | `20261014-cli-cache` | a persistent cache of content digests known to be fixed points makes `--check`/`--write` on an unchanged tree I/O-bound | CLI only: hash file, look up `(binary, config, content)` key, skip the formatter on a hit | process wall time on a formatted tree, ≥10x | formatted fixtures (1238 files, 14.2 MB): warm 32.7 ± 2.2 ms vs 819.4 ± 65.3 ms no-cache (25.1x, 4 threads), 28.9 ms vs 2904.6 ms on one core (100.7x); raw fixtures unchanged within noise; +188 KB binary; [EXP-006](experiments/EXP-006-cli-cache.md) | 1 ✓ 2 ✓ (library untouched: 1219/1220, 0 not idempotent) 3 ✓ 4 ✓ 5 ✓ (`syn` absent) | kept | `6e893f4` | 2026-10-14 |
