# Caching

Most formatter runs, from a pre-commit hook or an editor on save, are over files that are already formatted. chloro records which file contents it has seen come out unchanged and skips them on later runs.

## The fixed-point cache

When chloro finds a file already formatted, it records the file's BLAKE3 digest. On a later run, a file whose digest is recorded is reported as formatted without being parsed. The digest is recorded only for content observed to be a fixed point: a file left unchanged by the run, or a file just written whose formatted text formats to itself.

The cache is on by default for `--check`, `--write` and printing. It never changes what chloro does: the output and the exit status of a run are the same with and without it.

On the 1238 already-formatted files of the conformance corpus, on four cores:

| run | wall time |
| --- | --- |
| `--check`, warm cache | 16.2 ms |
| `--check`, cache directory removed before the run | 364 ms |
| `--check --no-cache` | 356 ms |
| `--check` with chloro 0.8.0, which has no cache | 1014 ms |

These figures are from measurement [M6](<https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/measurements/m6-quiet-host.md>); the [Performance](<https://lmmx.github.io/chloro/pr-preview/pr-104/performance/index.md>) page describes the conditions.

### What the key covers

Each cache file holds the digests for one chloro binary and one configuration:

- **The binary.** The BLAKE3 digest of the running executable, with the crate version, the cache format version and the target. Rebuilding or upgrading chloro starts a new cache file, so a digest recorded by one build is never trusted by another.
- **The configuration.** Every field of the `Config` built from `--config`, serialised field by field. Changing an option starts a new cache file.
- **The content.** The digest of the file's exact bytes. Paths and timestamps of source files are not recorded, so renaming or touching a file does not invalidate it.

Hashing the 2 MB executable on every run would cost more than the rest of a small run, so its digest is recorded under the executable's identity (path, size and modification time, and on Unix also device, inode and status-change time) and read back while that identity is unchanged.

> [!NOTE]
>
> **Limitation**
>
> An executable rewritten in place with every one of those identity fields unchanged would be taken for the binary recorded earlier. On Unix that requires a write within the file system's timestamp granularity of the previous change, a clock set back, or a write that bypasses the file system; on Windows, where the identity is path, size and modification time, a same-size rewrite with the modification time restored. Use `--no-cache`, or remove the cache directory, if you replace chloro's binary by such means.

### Where the cache lives

chloro uses the first of these that is set:

1. `--cache-dir DIR`
2. `$CHLORO_CACHE_DIR`
3. `$XDG_CACHE_HOME/chloro`
4. `%LOCALAPPDATA%\chloro\cache` on Windows
5. `~/Library/Caches/chloro` on macOS
6. `~/.cache/chloro`

If none can be resolved, the run proceeds without a cache. `--verbose` prints the cache file in use and how many entries it holds.

The directory holds `*.chloro-cache` files (one per binary and configuration), `*.chloro-exe` records of executable digests and, with `--unit-cache`, `*.chloro-units` files. A cache file keeps at most 131,072 digests, dropping the least recently seen; files not written for 30 days are removed by later runs. Writes go to a temporary file that is renamed into place, so concurrent runs do not corrupt a cache; deleting the directory at any time is safe.

### Turning it off

```sh
chloro --no-cache --check src/
```

`--no-cache` neither reads nor writes any cache, the unit cache included.

## The unit cache

`--unit-cache` adds a second, opt-in cache, for files that did change. It keeps the formatted text of each top-level item (or each group of `use`, `mod` and `extern crate` items that rustfmt sorts together), keyed by the item's text, the whole configuration and the file's `rustfmt::skip` names. When a file changes, only the items not seen before are formatted; the rest of the file (the parse, the checks that apply to the whole file, the text between items) is recomputed every time, so the result is exactly what formatting the whole file gives. This was verified over the conformance corpus and a registry corpus of 25,077 files, at one and four threads.

```sh
chloro --unit-cache --write src/
```

It is off by default because it costs more than it saves when it starts empty: a cold `--check` of the conformance corpus took 10–15% longer with it, and its file grew to 8.6 MB for that corpus. It helps when the same large files are reformatted after small edits. In process, reformatting a large file after a one-item edit took 0.26–0.47 of the time of formatting it from scratch; at the command line, process start-up and reading the cache take back part of that saving. The records are capped at 64 MiB per cache file.

The library exposes the same mechanism, with a store you provide; see [the library guide](<https://lmmx.github.io/chloro/pr-preview/pr-104/guide/library/#a-cache-of-formatted-items>).
