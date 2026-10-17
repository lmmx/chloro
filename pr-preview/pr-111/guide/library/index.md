# Library

Each of chloro's library functions takes source text and returns formatted text. None of them reads or writes files, and none starts a thread unless you ask for threads.

## Which crate

| crate | what it gives you |
| --- | --- |
| [`chloro`](<https://docs.rs/chloro>) | `format_source`, `format_source_with_config`, `try_format_source_with_config`, `Config`, `ParseError`; and the command line behind the `cli` feature |
| [`chloro-core`](<https://docs.rs/chloro-core>) | all of the above, plus the parallel and cached entry points, `parse_only`, and the syntax tree |

`chloro` re-exports these items from `chloro-core`, so both crates use the same `Config` type. The examples on this page that use only `format_source*` and `Config` work with either.

## Format a string

```rust
let formatted = chloro::format_source("fn main(){let x=1;}\n");
assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");
```

`format_source` formats with rustfmt's defaults at edition 2024, the same as `format_source_with_config(source, &Config::default())`.

The formatting functions return text, not errors. Source that does not parse, or that rustc's parser would reject, comes back unchanged, as rustfmt leaves such a file alone; so does a file with an inner `#![rustfmt::skip]` attribute, or any file when `disable_all_formatting` is set ([parse errors](<#parse-errors>) tells the first case from the others). To find out whether anything changed, compare the output with the input, or, to decide as rustfmt does, use `is_unchanged`:

```rust
let source = "fn main() {\n    let x = 1;\n}\n";
let needs_formatting = chloro::format_source(source) != source;
assert!(!needs_formatting);

// Not valid Rust: returned as written.
let broken = "fn main( {";
assert_eq!(chloro::format_source(broken), broken);

// `\r\n` line endings: the output has `\n` ones, and rustfmt counts the file as formatted.
let crlf = "fn main() {\r\n    let x = 1;\r\n}\r\n";
let config = chloro::Config::default();
let formatted = chloro::format_source_with_config(crlf, &config);
assert_eq!(formatted, source);
assert!(chloro::is_unchanged(crlf, &formatted, &config));
```

As in rustfmt, a byte order mark is dropped and `\r\n` in the input reads as `\n`; the output's line endings are set by `newline_style`, by default `\n` (the platform's for input with no line break). `is_unchanged` compares as rustfmt's `--check` does: under the default `newline_style = "Auto"`, a source that differs from its output only in `\r\n` line endings or a byte order mark is unchanged; under the other styles, the bytes must be equal.

## Parse errors

`try_format_source_with_config` returns the same text as `format_source_with_config`, or the syntax error for which that returns the source unchanged: the error rustfmt reports for a file it leaves alone, which the command line prints ([exit status](<https://lmmx.github.io/chloro/pr-preview/pr-111/guide/cli/#exit-status>)).

```rust
let config = chloro::Config::default();
let source = "fn main() {\n    let x = ;\n}\n";
let error = chloro::try_format_source_with_config(source, &config).unwrap_err();
assert_eq!((error.line(), error.column()), (2, 13));
assert_eq!(error.message(), "expected expression, found `;`");
assert_eq!(error.to_string(), "2:13: expected expression, found `;`");

// Left unchanged, but not an error.
let skipped = "#![rustfmt::skip]\nfn main(){}\n";
assert_eq!(chloro::try_format_source_with_config(skipped, &config).as_deref(), Ok(skipped));
```

The position is where rustc reports most syntax errors: the line, and the column in characters, both from 1, on the source with any byte order mark dropped and `\r\n` read as `\n`. `width()` is the number of characters the error covers on its line. The message is rust-analyzer's parser's, worded like rustc's where its tokens allow, or for syntax that rust-analyzer's parser accepts and rustc's rejects, "rustc's parser rejects this syntax". The error is found by parsing the source again once it has been refused, so the `try_` functions cost no more than the others on a source that parses. `chloro-core` has `try_format_source_with_config_parallel` and `try_format_source_with_unit_cache` for the entry points below.

## Options

```rust
let mut config = chloro::Config::default();
config.set("max_width", "80").unwrap();
config.set("edition", "2021").unwrap();
config.set("rustfmt_version", "1.8.0").unwrap();

let formatted = chloro::format_source_with_config("fn main(){}", &config);
assert_eq!(formatted, "fn main() {}\n");

// Unknown and unstable options are errors, with a message to show the user.
assert!(config.set("max_widht", "80").is_err());
assert!(config.set("wrap_comments", "true").is_err());
```

`Config::set` takes rustfmt's option names and value spellings; the fields of `Config` can also be set directly. [Configuration](<https://lmmx.github.io/chloro/pr-preview/pr-111/guide/configuration/index.md>) lists every option.

## One large file on several threads

```rust
use chloro_core::{Config, format_source_with_config, format_source_with_config_parallel};

let source = "use std::fmt;\nfn a(){}\nfn b(){}\nstruct S{x:u8}\n";
let config = Config::default();

let parallel = format_source_with_config_parallel(source, &config, 4);
assert_eq!(parallel, format_source_with_config(source, &config));
```

`format_source_with_config_parallel` formats the file's top-level items on up to `threads` scoped threads, started for the call and joined before it returns. The output is identical to `format_source_with_config`'s. With `threads <= 1`, or a file with fewer than two top-level items, no thread is started.

Starting threads costs more than it saves on a small file, and a program formatting many files usually does better to format several files at once with the sequential function. The command line splits only files of 128 KiB and more; see [Parallelism](<https://lmmx.github.io/chloro/pr-preview/pr-111/guide/parallelism/index.md>).

## A cache of formatted items

`format_source_with_unit_cache` is the parallel function with a cache of formatted top-level units, which you provide by implementing `UnitCache`. Each unit the cache does not hold is formatted and offered to `insert`; each one it does hold is reused. The output is identical to `format_source_with_config`'s.

```rust
use std::cell::RefCell;
use std::collections::HashMap;

use chloro_core::{CachedUnit, Config, UnitCache, UnitKey, format_source_with_unit_cache};

/// Units in memory, keyed by everything a unit's output depends on.
#[derive(Default)]
struct MemoryUnits(RefCell<HashMap<(Vec<u8>, String), CachedUnit>>);

impl UnitCache for MemoryUnits {
    fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit> {
        let key = (key.context().to_vec(), key.text().to_owned());
        self.0.borrow().get(&key).cloned()
    }

    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
        let key = (key.context().to_vec(), key.text().to_owned());
        self.0.borrow_mut().insert(key, unit);
    }
}

let units = MemoryUnits::default();
let config = Config::default();

let before = "fn a(){}\nfn b(){}\n";
let after = "fn a(){}\nfn b(){1}\n"; // only `b` changed

assert_eq!(format_source_with_unit_cache(before, &config, 1, &units), "fn a() {}\nfn b() {}\n");
// The second call formats `b` and reuses `a`.
assert_eq!(format_source_with_unit_cache(after, &config, 1, &units), "fn a() {}\nfn b() {\n    1\n}\n");
```

The rules for an implementation:

- `get` may return a unit only if `insert` was given it for a key with equal `context()` and `text()`. Keying by both, or by a collision-resistant hash of both, satisfies this.
- The key does not identify the build of `chloro-core`. A cache kept across processes must also be keyed by the build (the command line names its cache file by the digest of its executable). `CachedUnit::to_bytes` and `CachedUnit::from_bytes` serialise a unit for such a store.
- The formatter calls the cache from the calling thread only, so the implementation needs no synchronisation of its own.

## Parse without formatting

```rust
use chloro_core::{Config, parse_only};

let elements = parse_only("fn main() {}", &Config::default());
assert!(elements.is_some());
assert_eq!(parse_only("fn main( {", &Config::default()), None);
```

`parse_only` runs the steps formatting starts with (normalisation, lexing, parsing, building the syntax tree, and the checks for syntax rustc's parser handles differently) and returns the number of nodes and tokens in the file's tree, or `None` where formatting would return the source unchanged without formatting it. It exists to measure the parser's share of the formatter's cost, and to check quickly whether chloro would format a file at all.

## Debug output

`chloro_core::chloro_debug!` prints to standard error when debug output is enabled, either with the `CHLORO_DEBUG` environment variable at start-up or with `chloro_core::debug::set_debug(true)`. It is for developing chloro itself.

## The syntax tree

`chloro_core::cst` is the flat syntax tree the formatter reads, with a typed layer that keeps rust-analyzer's `ast` names. It is public so that the tools in `chloro-core/examples` can use it, and is not re-exported by `chloro`. [Parsing and the syntax tree](<https://lmmx.github.io/chloro/pr-preview/pr-111/architecture/parsing/index.md>) describes it.
