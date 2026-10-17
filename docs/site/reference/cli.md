---
title: "Command-line reference"
description: "Reference for the chloro command: usage, every option, environment variables and exit statuses."
---

# ++"chloro"++

Reference for the `chloro` command. The [Command line guide](../guide/cli.md) explains how the
options work together.

## Usage

```text
--8<-- "docs/site/_snippets/cli/help.txt"
```

Which files a directory contributes is described in the guide, under
[Sources](../guide/cli.md#sources).

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

Print the version and exit. The plain version, `chloro 0.9.0`, is printed only by a build
of the release itself; any other build is marked as a development build, with what is known
of its source:

| output | built from |
|---|---|
| `chloro 0.9.0` | the release: the commit tagged `chloro-v0.9.0` with no uncommitted changes, or the crate published to crates.io |
| `chloro 0.9.0-dev (2f9b060)` | another commit (`2f9b060`) of a git checkout |
| `chloro 0.9.0-dev (2f9b060, dirty)` | a checkout with uncommitted changes to the files that go into the binary |
| `chloro 0.9.0-dev (unknown source)` | a copy of the source with no git repository or packaging record, which cannot be checked |

The version is the crate's version; `-dev` is added only to this output. The check is made
when chloro is compiled (`chloro/build.rs`), from the local repository: the build reruns when
HEAD moves, a tag changes, or a source file changes, and `--version` reads nothing at run
time.

A binary installed with `just reinstall` also prints its build ID, a hash of the source it
was built from, as in `chloro 0.9.0-dev (2f9b060, dirty, build <id>)`
([Benchmarking](../maintainers/benchmarking.md#on-any-cargo-project-just-vs-rustfmt)).

### ++"--help"++ / ++"-h"++

Print the help text and exit.

## Exit status

| status | meaning |
|---|---|
| 0 | success; with `--check`, nothing needs formatting |
| 1 | `--check` found files to format, or a file does not parse ([errors](../guide/cli.md#exit-status)), or a file could not be read or written, or a source does not exist, or `--config` or the arguments were invalid |

## Environment

| variable | effect |
|---|---|
| `CHLORO_CACHE_DIR` | the cache directory, when `--cache-dir` is not given |
| `XDG_CACHE_HOME` | the parent of the default cache directory, when set |
| `CHLORO_TRACE` | a file to write a per-worker timeline of the run to |
