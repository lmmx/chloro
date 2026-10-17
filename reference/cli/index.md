# <kbd>chloro</kbd>

Reference for the `chloro` command. The [Command line guide](<https://lmmx.github.io/chloro/guide/cli/index.md>) explains how the options work together.

## Usage

```text
Usage: chloro [OPTIONS] [SOURCES]...

A Rust code formatter reproducing `rustfmt --edition 2024`.

Arguments:
  [SOURCES]...       Path(s) to file(s) or directory to format (default: 'src')

Options:
  -c, --check        Check if files need formatting (exit 1 if so)
  -w, --write        Write formatted output back to files
  -v, --verbose      Show verbose output
      --config OPTS  rustfmt options as comma-separated key=value pairs
      --no-cache     Format every file; do not read or write the caches
      --unit-cache   Also cache each formatted top-level item, so that a changed
                     file formats only the items not seen before (off by default)
      --cache-dir D  Cache directory (default: $CHLORO_CACHE_DIR, else the
                     user cache directory, e.g. ~/.cache/chloro)
      --threads N    Threads to use (default: one per core)
  -V, --version      Print the version and exit
  -h, --help         Show this help message

Examples:
  # Format a single file and print to stdout
  chloro src/lib.rs

  # Format multiple files
  chloro src/*.rs

  # Check if files need formatting
  chloro --check src/

  # Format files in-place
  chloro --write src/

  # Format with rustfmt options
  chloro --config max_width=80,tab_spaces=2 src/lib.rs

Cache:
  chloro records the content hash of every file it finds already formatted,
  keyed by the chloro binary and the --config options, and skips formatting
  files whose content it has seen formatted before. The output and exit code
  are the same with and without the cache.

  With --unit-cache, chloro also keeps the formatted output of each top-level
  item (or group of `use`/`mod`/`extern crate` items) under the same key and
  the item's text, and formats only the items of a changed file it has not
  seen. The output and exit code are the same. It is off by default: a run that
  finds nothing in it (a cold CI run) costs 10-15% more with it on.
```

> [!NOTE]
>
> `[SOURCES]` defaults to the current directory (`.`), not `'src'` as the help text says ([\#91](<https://github.com/lmmx/chloro/issues/91>)).

## Options

### <kbd>--check</kbd> / <kbd>-c</kbd>

Format every file without writing anything. Exit with status 1, and print `Error: N file(s) need formatting`, if any file's formatted text differs from its contents.

### <kbd>--write</kbd> / <kbd>-w</kbd>

Write each file whose formatted text differs from its contents. Without `--check` or `--write`, every formatted file is printed to standard output.

### <kbd>--config OPTS</kbd>

rustfmt options as comma-separated `key=value` pairs, with rustfmt's names and value spellings, plus `rustfmt_version`. An invalid option is an error before any file is read. [Configuration](<https://lmmx.github.io/chloro/guide/configuration/index.md>) lists them.

### <kbd>--threads N</kbd>

The number of worker threads. The default, or `0`, is one per core. Files of 128 KiB and more also have their top-level items formatted on this many threads. [Parallelism](<https://lmmx.github.io/chloro/guide/parallelism/index.md>).

### <kbd>--no-cache</kbd>

Neither read nor write the fixed-point cache or the unit cache. [Caching](<https://lmmx.github.io/chloro/guide/caching/index.md>).

### <kbd>--unit-cache</kbd>

Also cache the formatted text of each top-level item, so that a changed file formats only the items not seen before. Off by default; overridden by `--no-cache`. [Caching](<https://lmmx.github.io/chloro/guide/caching/#the-unit-cache>).

### <kbd>--cache-dir DIR</kbd>

The directory of the caches. Default: `$CHLORO_CACHE_DIR`, else `$XDG_CACHE_HOME/chloro`, else the platform's user cache directory (`~/.cache/chloro`, `~/Library/Caches/chloro`, `%LOCALAPPDATA%\chloro\cache`).

### <kbd>--verbose</kbd> / <kbd>-v</kbd>

Report, on standard error, the sources and mode, the cache file and its size, the number of files and threads, each file as it is processed, and a summary at the end.

### <kbd>--version</kbd> / <kbd>-V</kbd>

Print the version and exit. The plain version, `chloro 0.9.0`, is printed only by a build of the release itself; any other build is marked as a development build, with what is known of its source:

| output | built from |
| --- | --- |
| `chloro 0.9.0` | the release: the commit tagged `chloro-v0.9.0` with no uncommitted changes, or the crate published to crates.io |
| `chloro 0.9.0-dev (2f9b060)` | another commit (`2f9b060`) of a git checkout |
| `chloro 0.9.0-dev (2f9b060, dirty)` | a checkout with uncommitted changes to the files that go into the binary |
| `chloro 0.9.0-dev (unknown source)` | a copy of the source with no git repository or packaging record, which cannot be checked |

The version is the crate's version; `-dev` is added only to this output. The check is made when chloro is compiled (`chloro/build.rs`), from the local repository: the build reruns when HEAD moves, a tag changes, or a source file changes, and `--version` reads nothing at run time.

A binary installed with `just reinstall` also prints its build ID, a hash of the source it was built from, as in `chloro 0.9.0-dev (2f9b060, dirty, build <id>)` ([Benchmarking](<https://lmmx.github.io/chloro/maintainers/benchmarking/#on-any-cargo-project-just-vs-rustfmt>)).

### <kbd>--help</kbd> / <kbd>-h</kbd>

Print the help text and exit.

## Exit status

| status | meaning |
| --- | --- |
| 0 | success; with `--check`, nothing needs formatting |
| 1 | `--check` found files to format, or a file does not parse ([errors](<https://lmmx.github.io/chloro/guide/cli/#exit-status>)), or a file could not be read or written, or a source does not exist, or `--config` or the arguments were invalid |

## Environment

| variable | effect |
| --- | --- |
| `CHLORO_CACHE_DIR` | the cache directory, when `--cache-dir` is not given |
| `XDG_CACHE_HOME` | the parent of the default cache directory, when set |
| `CHLORO_TRACE` | a file to write a per-worker timeline of the run to |
