# Command line

chloro formats the `.rs` files under the paths you give it. Depending on the flags, it prints the formatted files, checks them against what is on disk, or writes them back.

```text
chloro [OPTIONS] [SOURCES]...
```

## Sources

Each source is a file or a directory. A directory is walked recursively for files ending in `.rs`; a file named directly is formatted whatever its extension. With no source, chloro starts at the current directory.

- Every directory under a source is walked, including hidden ones and `target/`. chloro has no ignore rules and does not read `.gitignore`, so in a Cargo project name `src/`, `tests/`, `examples/` and `benches/` rather than `.` ([\#90](<https://github.com/lmmx/chloro/issues/90>)).
- A file reachable twice (named twice, reached through a symlink, or both named and found in a directory) is formatted once.
- A symlinked directory that points back into a directory being walked is not followed again.
- A source path that does not exist is an error, reported before any file is formatted.

## Modes

**<kbd>--check</kbd>**

Format every file and compare. Nothing is written. Exits with status 1 and `Error: N file(s) need formatting` when at least one file would change.

**<kbd>--write</kbd>**

Write each file whose formatted text differs from its contents. Files that are already formatted are not touched.

**(neither)**

Print every formatted file to standard output, one after another. With more than one thread, files are printed in the order they finish, which can vary from run to run; use `--threads 1` for a stable order, or name one file.

With both `--check` and `--write`, files are written and the exit status reports whether any needed formatting.

## Exit status

| status | when |
| --- | --- |
| 0 | every file was processed; with `--check`, none needs formatting |
| 1 | `--check` found files that need formatting; or a file does not parse; or a file could not be read or written; or a source path does not exist; or `--config` was invalid; or the arguments could not be parsed |

A file that does not parse, or that contains syntax rustc's parser rejects, is reported as rustfmt reports it, in every mode: its error is printed to standard error in rustc's layout, nothing is printed or written for it, and the exit status is 1. The other files are formatted as usual.

```text
error: expected expression, found `;`
 --> /home/me/project/src/main.rs:2:13
  |
2 |     let x = ;
  |             ^
```

The position is where rustc reports most syntax errors. The message is rust-analyzer's parser's, worded like rustc's where its tokens allow (as in "expected `;`, found keyword `let`"), or, for syntax that rust-analyzer's parser accepts and rustc's rejects, "rustc's parser rejects this syntax"; rustc's own messages, notes and suggestions are not reproduced. Only the first error of a file is reported. The errors are printed after the run, in path order, `--verbose` counts the files in its summary ("Files that do not parse"), and such a file is never recorded in the cache. A file with an inner `#![rustfmt::skip]` attribute is left unchanged, and is not an error.

## Options

The flags are summarised here; the [CLI reference](<https://lmmx.github.io/chloro/reference/cli/index.md>) has the full help text.

| option | what it does | guide |
| --- | --- | --- |
| `-c`, `--check` | exit 1 if any file would change; write nothing | [above](<#modes>) |
| `-w`, `--write` | write formatted files in place | [above](<#modes>) |
| `-v`, `--verbose` | report each file, the cache in use, the thread count and a summary, on standard error |  |
| `--config OPTS` | rustfmt options as `key=value,key=value` | [Configuration](<https://lmmx.github.io/chloro/guide/configuration/index.md>) |
| `--threads N` | worker threads (default: one per core) | [Parallelism](<https://lmmx.github.io/chloro/guide/parallelism/index.md>) |
| `--no-cache` | neither read nor write the caches | [Caching](<https://lmmx.github.io/chloro/guide/caching/index.md>) |
| `--unit-cache` | also cache each formatted top-level item | [Caching](<https://lmmx.github.io/chloro/guide/caching/#the-unit-cache>) |
| `--cache-dir DIR` | where the caches live | [Caching](<https://lmmx.github.io/chloro/guide/caching/#where-the-cache-lives>) |
| `-V`, `--version` | print the version, marked `-dev` unless built from the release | [Reference](<https://lmmx.github.io/chloro/reference/cli/#-version-v>) |
| `-h`, `--help` | print the help text |  |

> [!NOTE]
>
> **The default source in `--help`**
>
> The help text describes the default source as `'src'`; the default is in fact the current directory (`.`), as described above ([\#91](<https://github.com/lmmx/chloro/issues/91>)).

## Environment variables

| variable | effect |
| --- | --- |
| `CHLORO_CACHE_DIR` | the cache directory when `--cache-dir` is not given |
| `XDG_CACHE_HOME` | when set, the default cache directory is `$XDG_CACHE_HOME/chloro` (on every platform) |
| `CHLORO_TRACE=FILE` | write a per-file, per-worker timeline of the run to `FILE` (a diagnostic for [scheduling](<https://lmmx.github.io/chloro/guide/parallelism/#seeing-the-schedule>)) |

## Byte order marks and line endings

chloro handles both as rustfmt does:

- The file is formatted as rustc reads it: without a byte order mark at the start, and with `\r\n` read as `\n`.
- The output's line endings follow the `newline_style` option. `Unix` and `Windows` force `\n` or `\r\n`, and `Native` uses the platform's. The default, `Auto`, takes the style of the first line ending of the text as rustc reads it, in which `\r\n` is already `\n`: so the output has `\n` line endings (the platform's for a file without a line break).
- Whether a file needs formatting is decided as rustfmt decides it. Under `Auto`, a file that differs from its formatted output only in its `\r\n` line endings or a byte order mark is formatted: `--check` passes and `--write` leaves it as it is. A file with `\r\n` line endings that does need formatting is written with `\n` ones. Under the other styles, any difference in the bytes counts.

A few things rustfmt does that chloro does not (reading `rustfmt.toml`, `--emit`, formatting standard input) are listed under [rustfmt compatibility](<https://lmmx.github.io/chloro/compatibility/#what-is-not-covered>).
