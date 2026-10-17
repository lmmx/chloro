---
description: "chloro's public Rust API on docs.rs, and where each entry point is explained on this site."
---

# Rust API

The Rust API is documented on docs.rs. This page lists the public entry points and where
each is explained on this site.

<div class="chl-cards">
<a href="https://docs.rs/chloro"><strong>docs.rs/chloro</strong><span>The library facade and the command-line crate.</span></a>
<a href="https://docs.rs/chloro-core"><strong>docs.rs/chloro-core</strong><span>The formatter, its entry points, the configuration and the syntax tree.</span></a>
</div>

## Formatting

| item | in `chloro` | in `chloro-core` | |
|---|---|---|---|
| `format_source(source: &str) -> String` | yes | yes | rustfmt's defaults, edition 2024 |
| `format_source_with_config(source: &str, config: &Config) -> String` | yes | yes | any options |
| `format_source_with_config_parallel(source, config, threads) -> String` | | yes | top-level items on up to `threads` threads; [guide](../guide/library.md#one-large-file-on-several-threads) |
| `format_source_with_unit_cache(source, config, threads, cache: &dyn UnitCache) -> String` | | yes | reuses formatted items; [guide](../guide/library.md#a-cache-of-formatted-items) |
| `parse_only(source, config) -> Option<usize>` | | yes | the parse alone; [guide](../guide/library.md#parse-without-formatting) |

Every formatting function returns the source unchanged when it does not parse, when rustc's
parser would reject it, when it has an inner `#![rustfmt::skip]`, or when
`disable_all_formatting` is set. All of them return the same text for the same source and
configuration.

## Configuration

| item | |
|---|---|
| `Config` | one public field per stable rustfmt option, plus `rustfmt_version`; `Default` is rustfmt's defaults |
| `Config::set(&mut self, key: &str, value: &str) -> Result<(), String>` | set an option by rustfmt's name and spelling |
| `chloro_core::formatter::config::{NewlineStyle, Heuristics, Edition, StyleEdition, MatchArmLeadingPipe, Density, RustfmtVersion}` | the enum-valued options |

[Configuration](../guide/configuration.md) lists every option and its default.

## The unit cache

| item | |
|---|---|
| `trait UnitCache { fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit>; fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit); }` | the store `format_source_with_unit_cache` reads and fills |
| `UnitKey` | what a unit's output depends on: `context()` (configuration, skip names, crate version) and `text()` |
| `CachedUnit` | one formatted unit; `to_bytes` and `from_bytes` serialise it |

## Lower level

| item | |
|---|---|
| `chloro_core::cst` | the flat syntax tree and its typed layer ([Parsing](../architecture/parsing.md)) |
| `chloro_core::chloro_debug!`, `chloro_core::debug` | debug output to standard error, for developing chloro |
| `chloro_core::formatter::printer` | a printer trait from chloro's earlier formatter, unused by the current one |
