# ++"chloro"++

Reference for the `chloro` command. The [Command line guide](../guide/cli.md) explains how the
options work together.

## Usage

```text
--8<-- "docs/site/_snippets/cli/help.txt"
```

!!! note

    `[SOURCES]` defaults to the current directory (`.`), not `'src'` as the help text says
    ([#91](https://github.com/lmmx/chloro/issues/91)).

## Options

### ++"--check"++ / ++"-c"++

Format every file without writing anything. Exit with status 1, and print
`Error: N file(s) need formatting`, if any file's formatted text differs from its contents.

### ++"--write"++ / ++"-w"++

Write each file whose formatted text differs from its contents. Without `--check` or
`--write`, every formatted file is printed to standard output.

### ++"--config OPTS"++

rustfmt options as comma-separated `key=value` pairs, with rustfmt's names and value
spellings, plus `rustfmt_version`. An invalid option is an error before any file is read.
[Configuration](../guide/configuration.md) lists them.

### ++"--threads N"++

The number of worker threads. The default, or `0`, is one per core. Files of 128 KiB and
more also have their top-level items formatted on this many threads.
[Parallelism](../guide/parallelism.md).

### ++"--no-cache"++

Neither read nor write the fixed-point cache or the unit cache. [Caching](../guide/caching.md).

### ++"--unit-cache"++

Also cache the formatted text of each top-level item, so that a changed file formats only
the items not seen before. Off by default; overridden by `--no-cache`.
[Caching](../guide/caching.md#the-unit-cache).

### ++"--cache-dir DIR"++

The directory of the caches. Default: `$CHLORO_CACHE_DIR`, else `$XDG_CACHE_HOME/chloro`,
else the platform's user cache directory (`~/.cache/chloro`, `~/Library/Caches/chloro`,
`%LOCALAPPDATA%\chloro\cache`).

### ++"--verbose"++ / ++"-v"++

Report, on standard error, the sources and mode, the cache file and its size, the number of
files and threads, each file as it is processed, and a summary at the end.

### ++"--version"++ / ++"-V"++

Print the version and exit. A binary installed with `just reinstall` from a checkout also
prints its build ID, `chloro 0.8.0 (build <id>)`, a hash of the source it was built from
([Benchmarking](../maintainers/benchmarking.md#on-any-cargo-project-just-vs-rustfmt)).

### ++"--help"++ / ++"-h"++

Print the help text and exit.

## Exit status

| status | meaning |
|---|---|
| 0 | success; with `--check`, nothing needs formatting |
| 1 | `--check` found files to format, or a file could not be read or written, or a source does not exist, or `--config` or the arguments were invalid |

## Environment

| variable | effect |
|---|---|
| `CHLORO_CACHE_DIR` | the cache directory, when `--cache-dir` is not given |
| `XDG_CACHE_HOME` | the parent of the default cache directory, when set |
| `CHLORO_TRACE` | a file to write a per-worker timeline of the run to |
