# Parallelism and caches

<p class="lede">Two observations about rustfmt's output make it safe for chloro to format
pieces of a file separately: what a top-level item becomes depends only on the item, and the
text between items can be put back in order afterwards.</p>

## Item independence

rustfmt formats a file by walking its items in order with one visitor. Before building any
parallelism, chloro tested whether each top-level item formats the same on its own as it
does in its file. Splitting the rust-analyzer corpus and a crates.io corpus into units (one
top-level item, or one group of `use`, `mod x;` or `extern crate` items that rustfmt sorts
together) gave 13,650 units in the first and 965,193 in the second. Formatted on its own,
every unit came out as it does in its file, except in the text before it: a comment before
the item, a `#[rustfmt::skip]` module, a `#!` line. No unit differed in any other way.

So a unit's output is a function of its own text, the configuration, and the file's
`rustfmt::skip::macros` and `::attributes` names. That is evidence, not a proof, and chloro
does not rely on it unchecked: both uses below are tested for identical output over both
corpora.

## Formatting items on several threads

`format_source_with_config_parallel` (`formatter/parallel.rs`) parses the file once into one
immutable tree, which the threads share: a node handle is a reference and an index, and the
tree is `Sync`. Workers take units from a shared queue, longest first, and each formats its
unit with a visitor of its own, starting from the state the sequential visitor would have at
that point.

The subtle part is the gap before each unit. The sequential visitor writes the comments and
blank lines before an item from inside the item's own code, and what it writes depends on
the output so far (whether the buffer is empty, how many newlines it ends with). A worker
cannot know that, so it does not write the gap: it records the call it would have made.
The calling thread then walks the units in order, replays each recorded gap call on the real
output buffer, and appends the unit's text.

If a unit's record cannot be stitched where it belongs, the calling thread formats that part
sequentially instead; a panic in a worker is re-raised with its original payload.
`tests/parallel_equivalence.rs` holds the two paths to identical output, and the corpus
check (`examples/bench --threads 4 --check`) compares the parallel output of every corpus
file with hashes recorded from the sequential path.

## The command line's scheduler

The command line runs one worker per core and gives out files largest first from a shared
queue. A file of 128 KiB or more is also split into items, on all threads; the file-level
workers and the item-level threads share the cores, and the operating system schedules
them. Measured with the `CHLORO_TRACE` timeline and rebuilt from per-thread CPU clocks, the
resulting schedule is within 1–1.5% of the shortest the work allows, on every measured
crate.

## The fixed-point cache

The command line's cache (`chloro/src/cli/cache.rs`) records one fact per entry: this exact
file content, formatted by this binary under this configuration, comes out unchanged. It
stores no formatted text, no paths and no timestamps. The binary and the configuration
select the cache file; the content digests (BLAKE3, 256 bits) are its entries. The set is
read once at start-up, shared read-only by the workers, and written once at the end through
a temporary file and a rename. [Caching](../guide/caching.md) describes its behaviour and
limits.

## The unit cache

`format_source_with_unit_cache` is the parallel path with a cache in front of each unit. A
cached unit records its output with every position relative to the unit's start, so a hit
applies wherever the unit has moved in the file. Everything that depends on more than the
unit is recomputed on every call: the parse, the whole-file checks, the text between units,
and the final newline handling. A re-format after an edit to one item therefore costs a
parse, one lookup per unit, and the edited unit.

The command line's `--unit-cache` stores these records in a file next to the fixed-point
cache, keyed by the same binary and configuration. It was checked over both corpora, at one
and four threads, to give output identical to formatting without it.
