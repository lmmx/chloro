# Parallelism

By default chloro uses every core. Files are spread across worker threads, and a large file has its top-level items formatted on several threads at once. The output is identical whatever the number of threads.

## Files in parallel

The command line starts one worker per core (`std::thread::available_parallelism`), or `--threads N` workers. The files are sorted by size, largest first, and handed out from a shared queue: each worker takes the next file as soon as it finishes one. Starting with the large files keeps one late, large file from leaving the other cores idle at the end of a run.

```sh
chloro --check src/                 # one worker per core
chloro --threads 2 --check src/     # two workers
chloro --threads 1 --check src/     # sequential
```

rustfmt formats the files of a crate on one thread. On the nine crates of the [performance measurements](<https://lmmx.github.io/chloro/pr-preview/pr-101/performance/index.md>), chloro on four cores took 0.08–0.17 of rustfmt's wall time, against 0.21–0.35 on one core.

## Large files

A file of 128 KiB or more is also split: its top-level items are formatted on up to `--threads` threads, while the text between the items (comments, blank lines, items left as written) is written in order on one thread and the formatted items are put back between it. The threshold is a constant of the command line (`PARALLEL_ITEMS_MIN_BYTES`); below it, starting threads for one file costs more than it saves when the other cores are already busy with other files.

On the five largest files of the conformance corpus, formatting each file alone with four threads was 1.1–1.7 times as fast as with one, for 1.09–1.32 times the CPU time.

Splitting a file does not change its output, because in rustfmt the output of a top-level item depends only on the item's own text and the configuration. The exceptions are the text between items, and groups of `use`, `mod` and `extern crate` items, which rustfmt sorts together. chloro keeps both on the calling thread and treats a sortable group as a single unit. [Parallelism and caches](<https://lmmx.github.io/chloro/pr-preview/pr-101/architecture/concurrency/index.md>) explains how the pieces are put back together and how that is tested.

The library exposes the same split as [`format_source_with_config_parallel`](<https://lmmx.github.io/chloro/pr-preview/pr-101/guide/library/#one-large-file-on-several-threads>); the library never starts threads unless asked.

## Output order

With `--check` or `--write` the order in which files finish does not matter. When printing to standard output, each file is written as soon as it is formatted, so with several threads the order of the files can vary between runs (the bytes of each file do not). Use `--threads 1` when the order matters.

## Seeing the schedule

`CHLORO_TRACE=FILE` makes the command line write a timeline of the run: when each worker took each file, and when it read, hashed and formatted it. The scripts under `benchmarks/schedule/` turn a trace into per-worker idle time and the length of the run's tail. On the measured crates the schedule came within 1–1.5% of the shortest possible for the work, so there is little to gain from changing it.
