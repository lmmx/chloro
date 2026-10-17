# Command line

<p class="lede">chloro formats the <code>.rs</code> files under the paths you give it.
Depending on the flags, it prints the formatted files, checks them against what is on disk,
or writes them back.</p>

```text
chloro [OPTIONS] [SOURCES]...
```

## Sources

Each source is a file or a directory. A directory is walked recursively for files ending in
`.rs`; a file named directly is formatted whatever its extension. With no source, chloro
starts at the current directory.

- Every directory under a source is walked, including hidden ones and `target/`. chloro has
  no ignore rules and does not read `.gitignore`, so in a Cargo project name `src/`,
  `tests/`, `examples/` and `benches/` rather than `.`
  ([#90](https://github.com/lmmx/chloro/issues/90)).
- A file reachable twice (named twice, reached through a symlink, or both named and found in
  a directory) is formatted once.
- A symlinked directory that points back into a directory being walked is not followed again.
- A source path that does not exist is an error, reported before any file is formatted.

## Modes

++"--check"++
:   Format every file and compare. Nothing is written. Exits with status 1 and
    `Error: N file(s) need formatting` when at least one file would change.

++"--write"++
:   Write each file whose formatted text differs from its contents. Files that are already
    formatted are not touched.

(neither)
:   Print every formatted file to standard output, one after another. With more than one
    thread, files are printed in the order they finish, which can vary from run to run; use
    `--threads 1` for a stable order, or name one file.

With both `--check` and `--write`, files are written and the exit status reports whether
any needed formatting.

## Exit status

| status | when |
|---|---|
| 0 | every file was processed; with `--check`, none needs formatting |
| 1 | `--check` found files that need formatting; or a file could not be read or written; or a source path does not exist; or `--config` was invalid; or the arguments could not be parsed |

A file that does not parse, or that contains syntax rustc's parser rejects, is left
unchanged, as rustfmt leaves it. Unlike rustfmt, which prints the parse error and exits with
status 1, chloro does not report such a file, even with `--verbose`, and counts it as
formatted; that is a known issue ([#87](https://github.com/lmmx/chloro/issues/87)). A file
with an inner `#![rustfmt::skip]` attribute is also left unchanged.

## Options

The flags are summarised here; the [CLI reference](../reference/cli.md) has the full help
text.

| option | what it does | guide |
|---|---|---|
| `-c`, `--check` | exit 1 if any file would change; write nothing | [above](#modes) |
| `-w`, `--write` | write formatted files in place | [above](#modes) |
| `-v`, `--verbose` | report each file, the cache in use, the thread count and a summary, on standard error | |
| `--config OPTS` | rustfmt options as `key=value,key=value` | [Configuration](configuration.md) |
| `--threads N` | worker threads (default: one per core) | [Parallelism](parallelism.md) |
| `--no-cache` | neither read nor write the caches | [Caching](caching.md) |
| `--unit-cache` | also cache each formatted top-level item | [Caching](caching.md#the-unit-cache) |
| `--cache-dir DIR` | where the caches live | [Caching](caching.md#where-the-cache-lives) |
| `-V`, `--version` | print the version, marked `-dev` unless built from the release | [Reference](../reference/cli.md#-version-v) |
| `-h`, `--help` | print the help text | |

!!! note "The default source in `--help`"

    The help text describes the default source as `'src'`; the default is in fact the
    current directory (`.`), as described above
    ([#91](https://github.com/lmmx/chloro/issues/91)).

## Environment variables

| variable | effect |
|---|---|
| `CHLORO_CACHE_DIR` | the cache directory when `--cache-dir` is not given |
| `XDG_CACHE_HOME` | when set, the default cache directory is `$XDG_CACHE_HOME/chloro` (on every platform) |
| `CHLORO_TRACE=FILE` | write a per-file, per-worker timeline of the run to `FILE` (a diagnostic for [scheduling](parallelism.md#seeing-the-schedule)) |

## Byte order marks and line endings

chloro handles both as rustfmt does:

- A byte order mark at the start of a file is dropped.
- Line endings follow the `newline_style` option. The default, `Auto`, uses the style of the
  file's first line ending (`\r\n` or `\n`), or the platform's for a file without one, as
  rustfmt does; `Unix` and `Windows` force one and `Native` uses the platform's.

A few things rustfmt does that chloro does not (reading `rustfmt.toml`, `--emit`, formatting
standard input) are listed under
[rustfmt compatibility](../compatibility.md#what-is-not-covered).
