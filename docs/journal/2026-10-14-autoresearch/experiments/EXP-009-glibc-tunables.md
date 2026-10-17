# EXP-009: glibc malloc tunables through GLIBC_TUNABLES

Date: 2026-10-14. Code changes: none. Binary: `target/release/chloro` (flat substrate, dynamically linked against the system libc).

## Question

Whether glibc malloc tunables set through the `GLIBC_TUNABLES` environment variable change the wall time of the chloro CLI. glibc malloc/free accounts for about 12% of the instructions (`docs/journal/2026-10-14-autoresearch/profile-flat.md`, lever 1). mimalloc is rejected for 2 to 10 times the peak memory (`experiments/EXP-002-cli-build.md`, E2c).

## Environment

- CPU: Intel Xeon 2.80GHz, 4 CPUs; all measured commands run on CPU 3 through `taskset -c 3`.
- Kernel 6.18.44; glibc 2.39 (Ubuntu 2.39-0ubuntu8.9); hyperfine 2.0.0; valgrind callgrind.
- Load average (1, 5, 15 min, from `/proc/loadavg`):

| Moment | Load average |
|---|---|
| before pass A, hir-ty | 0.06 0.41 0.76 |
| after pass A, hir-ty | 0.97 0.60 0.81 |
| before pass A, lints.rs | 0.97 0.60 0.81 |
| after pass A, lints.rs | 1.06 0.63 0.82 |
| before pass B, hir-ty | 1.04 0.65 0.82 |
| after pass B, hir-ty | 1.14 0.72 0.84 |
| before pass B, lints.rs | 1.14 0.72 0.84 |
| after pass B, lints.rs | 1.13 0.72 0.84 |
| before callgrind runs | 0.70 0.66 0.81 |
| after callgrind runs | 0.87 0.70 0.82 |

## Variants

| Name | GLIBC_TUNABLES |
|---|---|
| default | unset (`env -u GLIBC_TUNABLES`) |
| 1 | `glibc.malloc.tcache_count=0` |
| 2 | `glibc.malloc.tcache_max=4096:glibc.malloc.tcache_count=64` |
| 3 | `glibc.malloc.mmap_threshold=1048576:glibc.malloc.trim_threshold=268435456:glibc.malloc.top_pad=67108864` |
| 4 | `glibc.malloc.arena_max=1` |

## Commands

Workload A, hir-ty (122 files):

```
hyperfine -N -i -w 2 -r 12 --export-json hirty.json \
  -n default 'taskset -c 3 env -u GLIBC_TUNABLES target/release/chloro --no-cache --check <fixtures>/rust-analyzer/hir-ty/src' \
  -n v1 'taskset -c 3 env GLIBC_TUNABLES=<variant 1> target/release/chloro --no-cache --check <fixtures>/rust-analyzer/hir-ty/src' \
  ... (v2, v3, v4 as v1) ... \
  -n default_repeat 'taskset -c 3 env -u GLIBC_TUNABLES target/release/chloro --no-cache --check <fixtures>/rust-analyzer/hir-ty/src'
```

Workload B, single file: the same hyperfine invocation with the arguments `--no-cache --check --threads 1 <fixtures>/rust-analyzer/ide-db/src/generated/lints.rs`.

`<fixtures>` is `/home/user/chloro/chloro-core/tests/conformance/fixtures`. `-i` is set because `--check` exits 1 when files need formatting (95 of 122 files in hir-ty). Pass A runs the commands in the order default, 1, 2, 3, 4, default (repeat). Pass B runs them in the order default, 4, 3, 2, 1, default (repeat). The repeated default row measures the drift between two default runs in one pass. Peak RSS is hyperfine's `memory_peak_resident` mean over the 12 runs. Ratios are relative to the `default` row of the same pass.

Callgrind, per variant, on the 49-file sample (`/tmp/chloro-bench/sample49`):

```
GLIBC_TUNABLES=<variant> setarch $(uname -m) -R taskset -c 3 valgrind --tool=callgrind target/release/examples/bench -n 1 --root /tmp/chloro-bench/sample49 2>&1 | grep refs
```

Syscall check that the tunables reach the process: `GLIBC_TUNABLES=<variant> taskset -c 0 strace -f -c -e trace=brk,mmap,munmap,madvise,mremap target/release/chloro --no-cache --check <hir-ty>`.

## Workload A: hir-ty, 122 files, default thread setting

| Pass | Variant | min (ms) | mean ± σ (ms) | Peak RSS (MiB) | Mean ratio | Min ratio | RSS ratio |
|---|---|---|---|---|---|---|---|
| A | default | 343.1 | 382.0 ± 33.7 | 8.12 | 1.000 | 1.000 | 1.000 |
| A | 1: tcache_count=0 | 403.2 | 444.4 ± 33.7 | 7.29 | 1.163 | 1.175 | 0.897 |
| A | 2: tcache_max=4096, tcache_count=64 | 330.0 | 355.8 ± 25.4 | 8.38 | 0.931 | 0.962 | 1.032 |
| A | 3: mmap_threshold, trim_threshold, top_pad | 333.6 | 358.6 ± 24.6 | 8.07 | 0.939 | 0.972 | 0.993 |
| A | 4: arena_max=1 | 331.2 | 360.6 ± 28.1 | 7.95 | 0.944 | 0.965 | 0.979 |
| A | default (repeat) | 358.7 | 407.0 ± 29.1 | 8.13 | 1.065 | 1.045 | 1.001 |
| B | default | 320.3 | 331.8 ± 13.9 | 8.12 | 1.000 | 1.000 | 1.000 |
| B | 1: tcache_count=0 | 399.5 | 418.9 ± 14.6 | 7.28 | 1.263 | 1.247 | 0.897 |
| B | 2: tcache_max=4096, tcache_count=64 | 339.8 | 364.9 ± 24.0 | 8.39 | 1.100 | 1.061 | 1.033 |
| B | 3: mmap_threshold, trim_threshold, top_pad | 338.1 | 349.0 ± 15.1 | 8.10 | 1.052 | 1.055 | 0.997 |
| B | 4: arena_max=1 | 335.6 | 355.3 ± 20.7 | 7.92 | 1.071 | 1.048 | 0.976 |
| B | default (repeat) | 331.6 | 365.2 ± 25.7 | 8.12 | 1.101 | 1.035 | 1.001 |

## Workload B: lints.rs, 1 file, `--threads 1`

| Pass | Variant | min (ms) | mean ± σ (ms) | Peak RSS (MiB) | Mean ratio | Min ratio | RSS ratio |
|---|---|---|---|---|---|---|---|
| A | default | 40.2 | 44.2 ± 4.8 | 9.83 | 1.000 | 1.000 | 1.000 |
| A | 1: tcache_count=0 | 43.1 | 49.7 ± 5.0 | 9.66 | 1.124 | 1.070 | 0.982 |
| A | 2: tcache_max=4096, tcache_count=64 | 43.7 | 46.0 ± 2.1 | 10.59 | 1.040 | 1.086 | 1.076 |
| A | 3: mmap_threshold, trim_threshold, top_pad | 37.9 | 45.0 ± 7.4 | 11.67 | 1.017 | 0.942 | 1.187 |
| A | 4: arena_max=1 | 41.1 | 45.3 ± 3.8 | 9.77 | 1.025 | 1.022 | 0.993 |
| A | default (repeat) | 41.9 | 45.6 ± 3.9 | 9.83 | 1.032 | 1.041 | 1.000 |
| B | default | 38.7 | 42.3 ± 4.5 | 9.84 | 1.000 | 1.000 | 1.000 |
| B | 1: tcache_count=0 | 42.7 | 47.1 ± 7.8 | 9.63 | 1.113 | 1.103 | 0.978 |
| B | 2: tcache_max=4096, tcache_count=64 | 38.8 | 43.3 ± 7.7 | 10.58 | 1.024 | 1.003 | 1.075 |
| B | 3: mmap_threshold, trim_threshold, top_pad | 37.3 | 43.9 ± 9.8 | 11.71 | 1.037 | 0.964 | 1.190 |
| B | 4: arena_max=1 | 39.2 | 47.6 ± 8.4 | 9.78 | 1.124 | 1.013 | 0.994 |
| B | default (repeat) | 38.7 | 40.5 ± 1.8 | 9.84 | 0.956 | 1.000 | 1.000 |

## Tunables reach the process

Syscall counts from `strace -f -c` on workload A (one run per row):

| Variant | mmap | munmap | mremap | brk | madvise |
|---|---|---|---|---|---|
| default | 19 | 8 | 6 | 5 | 2 |
| 2 | 19 | 8 | 6 | 5 | 2 |
| 3 | 16 | 5 | 0 | 3 | 1 |

Variant 3 changes the syscall counts. Variant 2 leaves them unchanged.

## Callgrind, 49-file sample

| Variant | Ir (callgrind refs) | Ratio to default |
|---|---|---|
| default | 312,839,176 | 1.0000 |
| default (second run) | 312,839,170 | 1.0000 |
| default (third run) | 312,839,207 | 1.0000 |
| 1: tcache_count=0 | 348,840,969 | 1.1151 |
| 2: tcache_max=4096, tcache_count=64 | 308,623,731 | 0.9865 |
| 3: mmap_threshold, trim_threshold, top_pad | 309,489,915 | 0.9893 |
| 4: arena_max=1 | 312,842,568 | 1.0000 |
| tcache_count=64 alone | 308,623,249 | 0.9865 |

The six `malloc.c` functions listed by `callgrind_annotate` (malloc, _int_malloc, free, _int_free, realloc, _int_realloc) sum to 38,058,244 Ir in the default run (12.17% of 312,839,176), 37,886,257 Ir with variant 3 and 36,204,714 Ir with variant 2. With variant 2 the six-function sum is lower by 1,853,530 Ir of a 4,215,445 Ir total decrease; `_int_malloc` goes from 6,384,585 to 4,809,441 Ir. The `tcache_count=64`-alone count differs from the variant 2 count (308,623,731) by 482 Ir, so the instruction-count effect of `tcache_max=4096` is that difference. The three default runs differ by at most 37 Ir. An earlier parallel run of default, 2 and 3 on CPUs 3, 2 and 1 gave 312,839,176, 308,622,062 and 309,489,971 Ir.

## Observations

- Repeated default against default in the same pass: mean ratio 1.065 (hir-ty, pass A), 1.101 (hir-ty, pass B), 1.032 (lints.rs, pass A), 0.956 (lints.rs, pass B).
- Variants 2, 3 and 4 on hir-ty: mean ratios 0.93 to 0.94 in pass A and 1.05 to 1.10 in pass B. On lints.rs: 1.02 to 1.04 in pass A and 1.02 to 1.12 in pass B.
- Variant 1 mean ratios: 1.163 and 1.263 (hir-ty), 1.124 and 1.113 (lints.rs); instruction ratio 1.115.
- Peak RSS for variant 3 on lints.rs is 11.7 MiB against 9.8 MiB for default; on hir-ty it is 8.1 MiB against 8.1 MiB.
- Instruction-count ratios for variants 2 and 3 are 0.9865 and 0.9893; the wall-time σ is 4% to 9% of the mean on hir-ty and 4% to 22% on lints.rs.

## Conclusion

No variant lowers the wall time outside the spread: variants 2, 3 and 4 have mean ratios of 0.93 to 1.12 across two passes and two workloads, with the sign changing between passes on hir-ty; the mean ratio of a repeated default run spans 0.96 to 1.10, and the instruction counts of variants 2 and 3 are 1.35% and 1.07% below default.
Variant 1 (`tcache_count=0`) is the one variant outside the spread: hir-ty 1.16 and 1.26, lints.rs 1.12 and 1.11 (σ of 4.5 to 7.8 ms there), and 11.5% more instructions.
