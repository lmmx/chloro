# 2026-10-17: Which files a directory contributes

With no source, chloro walks the current directory, and the walk went into every directory,
so `chloro --check` at the root of a Cargo project also checked `target/` (a build script's
`OUT_DIR` sources) and `.git/` (#90). `--help` gave the default source as `src` (#91).

## Current State

- **The rule.** Entries found in a walk are skipped when their name starts with `.` (files
  and directories), and a directory is skipped when it holds a `CACHEDIR.TAG` file starting
  with the signature of the Cache Directory Tagging Specification
  (https://bford.info/cachedir/). Cargo writes that tag in every target directory, nested
  ones included (`target/<triple>/`), so a build directory is recognised whatever its name or
  location, and a source directory named `target` is not mistaken for one. A source named on
  the command line is always used, hidden or tagged.
- **Cost.** The tag is found in the directory listing the walk already reads; a directory
  that has one costs one open and a 43-byte read. Every other directory costs nothing more.
- **Not chosen.** Skipping directories named `target` (a module can be named `target`), and
  reading `.gitignore` (a parser or the `ignore` crate's dependency tree, for a rule most
  projects state through `target/` and hidden directories anyway).
- **`--help`** gives the default as `'.'`, the current directory, and says what the walk
  skips; the site's help snippet is regenerated.

## Missing

- `.gitignore` and `.ignore` files, and a flag to walk hidden entries.
