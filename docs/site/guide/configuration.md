---
description: "rustfmt's stable options in chloro, with the same names, values and defaults, and the rustfmt_version option."
---

# Configuration

<p class="lede">chloro accepts rustfmt's stable options, with the same names, value spellings
and defaults. It adds one option, <code>rustfmt_version</code>, which selects the rustfmt
release to reproduce.</p>

The command line reads each file's `rustfmt.toml` and its crate's edition as `cargo fmt`
does ([below](#configuration-files)), and `--config` over both. With none of them, and in
the library, chloro formats as `rustfmt --edition 2024` does with no `rustfmt.toml`.

## Setting options

=== "CLI"

    ```sh
    chloro --config max_width=80,use_small_heuristics=Max --write src/
    ```

    `--config` takes comma-separated `key=value` pairs, as rustfmt's `--config` does. Values
    may be quoted (`edition="2021"`), and a choice may be written in any case
    (`newline_style=unix`), as rustfmt reads it.

=== "By name"

    ```rust
    let mut config = chloro::Config::default();
    config.set("max_width", "80").unwrap();
    config.set("use_small_heuristics", "Max").unwrap();
    let formatted = chloro::format_source_with_config("fn main() {}\n", &config);
    ```

    `Config::set` takes the same names and spellings as `--config` and returns an error
    message for an unknown option or an invalid value.

=== "By field"

    ```rust
    let config = chloro::Config {
        max_width: 80,
        tab_spaces: 2,
        ..chloro::Config::default()
    };
    let formatted = chloro::format_source_with_config("fn main() {}\n", &config);
    ```

    Every stable option is a public field of `Config`. The enum-valued fields
    (`newline_style`, `edition`, …) take types from `chloro_core::formatter::config`.

## Configuration files

The command line finds the configuration of each file as `cargo fmt` does:

1. **`rustfmt.toml`.** The nearest `.rustfmt.toml` or `rustfmt.toml` (in that order in each
   directory), in the file's directory or one of its parents, as rustfmt reads it. Without
   one, the home directory's, then the one in `rustfmt/` under the user configuration
   directory (`$XDG_CONFIG_HOME` or `~/.config` on Linux, `~/Library/Application Support` on
   macOS, `%APPDATA%` on Windows).
2. **The edition.** `cargo fmt` passes each crate's edition to rustfmt, so chloro takes it
   from the nearest `Cargo.toml` that has a `[package]`: its `edition`, the workspace's
   `[workspace.package] edition` for `edition.workspace = true`, or 2015 when it sets none, as
   Cargo does. It overrides an `edition` in `rustfmt.toml`; `style_edition` follows it unless
   `rustfmt.toml` sets one. A file with no package above it keeps the configuration file's
   edition, or 2024.
3. **`--config`**, which overrides both.

```toml title="rustfmt.toml"
max_width = 120
use_field_init_shorthand = true
```

The values have rustfmt's TOML types: an integer for a width or a count, `true` or `false`
for a flag, a string for a choice or an edition (`edition = "2021"`). As stable rustfmt
does, chloro warns about an unstable or unknown option and ignores it, and a value an option
does not take is an error that stops the run before any file is formatted:

```console
$ chloro --check
Warning: can't set `imports_granularity = "Crate"`, unstable features are only available in nightly channel.
$ chloro --check
The file `/home/me/project/rustfmt.toml` failed to parse.
Error details: invalid type: string "120", expected usize in `max_width`
```

`--config-path PATH` names the configuration file for every file, or a directory that holds
one. `--isolated` reads no configuration file and no `Cargo.toml`: every file gets the
defaults and `--config`. `--verbose` names each configuration file and edition it uses. The
fixed-point cache is kept per configuration, so after an edit to a `rustfmt.toml` its files
are formatted again.

The library reads no files: `Config::set_from_file(key, FileValue)` applies one option of a
configuration file with these rules, for a tool that reads its own.

## The options

| option | default | values |
|---|---|---|
| `max_width` | `100` | unsigned integer |
| `hard_tabs` | `false` | `true`, `false` |
| `tab_spaces` | `4` | unsigned integer |
| `newline_style` | `Auto` | `Auto`, `Native`, `Unix`, `Windows` |
| `use_small_heuristics` | `Default` | `Default`, `Max`, `Off` |
| `fn_call_width` | from `use_small_heuristics` (60) | unsigned integer |
| `attr_fn_like_width` | from `use_small_heuristics` (70) | unsigned integer |
| `struct_lit_width` | from `use_small_heuristics` (18) | unsigned integer |
| `struct_variant_width` | from `use_small_heuristics` (35) | unsigned integer |
| `array_width` | from `use_small_heuristics` (60) | unsigned integer |
| `chain_width` | from `use_small_heuristics` (60) | unsigned integer |
| `single_line_if_else_max_width` | from `use_small_heuristics` (50) | unsigned integer; `0` always breaks |
| `single_line_let_else_max_width` | from `use_small_heuristics` (50) | unsigned integer; `0` always breaks |
| `reorder_imports` | `true` | `true`, `false` |
| `reorder_modules` | `true` | `true`, `false` |
| `remove_nested_parens` | `true` | `true`, `false` |
| `short_array_element_width_threshold` | `10` | unsigned integer |
| `match_arm_leading_pipes` | `Never` | `Always`, `Never`, `Preserve` |
| `fn_params_layout` (or `fn_args_layout`) | `Tall` | `Compressed`, `Tall`, `Vertical` |
| `match_block_trailing_comma` | `false` | `true`, `false` |
| `edition` | `2024` | `2015`, `2018`, `2021`, `2024` |
| `style_edition` | follows `edition` | `2015`, `2018`, `2021`, `2024` |
| `merge_derives` | `true` | `true`, `false` |
| `use_try_shorthand` | `false` | `true`, `false` |
| `use_field_init_shorthand` | `false` | `true`, `false` |
| `force_explicit_abi` | `true` | `true`, `false` |
| `disable_all_formatting` | `false` | `true`, `false` |
| `rustfmt_version` | `1.9.0` | `1.8.0`, `1.9.0` (chloro only) |

Each option means what it means in rustfmt; the
[rustfmt configuration reference](https://rust-lang.github.io/rustfmt/) describes them.
[rustfmt compatibility](../compatibility.md#options) lists the option values on which
chloro's output has been compared with rustfmt's.

### Width heuristics

The eight width options, `fn_call_width` to `single_line_let_else_max_width`, are derived
from `max_width` unless set, as in rustfmt:

- `use_small_heuristics = Default` uses the values in brackets above at `max_width = 100`,
  scaled by `max_width / 100` (rounded to the nearest tenth) when `max_width` is larger.
- `Max` sets each to `max_width`.
- `Off` sets each to 0.

An explicitly set width wins over the heuristic and is capped at `max_width`.

### Editions

`edition` is the Rust edition the source is parsed as: it decides what is a keyword and
what syntax is accepted. `style_edition` is the edition of the Rust Style Guide the output
follows. As in rustfmt, an unset `style_edition` follows `edition`, so `edition=2021` alone
formats with the 2021 style. Style editions 2015, 2018 and 2021 are the same style; 2024
differs from them.

### `rustfmt_version`

This option is specific to chloro. It selects the rustfmt release whose output is
reproduced:

- `1.9.0`, the default: `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the release chloro is
  ported from.
- `1.8.0`: `rustfmt 1.8.0-stable (6b00bc3880 2025-06-23)`, as shipped with Rust 1.88.0.

The two releases differ in a handful of formatting decisions and in which syntax their
parsers accept; [rustfmt compatibility](../compatibility.md#rustfmt-releases) lists the
differences. `1.7.0` and earlier are rejected with an explanation: rustfmt 1.7.0 has no
2024 Style Guide rules, and its output differs from 1.9.0's too widely to reproduce with a
few version checks in the formatter.

### Unstable rustfmt options

rustfmt's unstable options (those that need nightly rustfmt) are not configurable. chloro
always behaves as rustfmt does with each at its default. Naming one with its default value
is accepted, so a configuration written for stable rustfmt still works; any other value is
an error:

```console
$ chloro --config wrap_comments=true src/
Error: invalid --config: `wrap_comments` is an unstable rustfmt option; only its default `false` is supported
```

??? info "The unstable options and the defaults chloro applies"

    `indent_style = Block`, `wrap_comments = false`, `format_code_in_doc_comments = false`,
    `doc_comment_code_block_width = 100`, `comment_width = 80`, `normalize_comments = false`,
    `normalize_doc_attributes = false`, `format_strings = false`,
    `format_macro_matchers = false`, `format_macro_bodies = true`,
    `skip_macro_invocations = []`, `hex_literal_case = Preserve`,
    `float_literal_trailing_zero = Preserve`, `empty_item_single_line = true`,
    `struct_lit_single_line = true`, `fn_single_line = false`, `where_single_line = false`,
    `imports_indent = Block`, `imports_layout = Mixed`, `imports_granularity = Preserve`,
    `group_imports = Preserve`, `reorder_impl_items = false`,
    `type_punctuation_density = Wide`, `space_before_colon = false`,
    `space_after_colon = true`, `spaces_around_ranges = false`, `binop_separator = Front`,
    `combine_control_expr = true`, `overflow_delimited_expr = false`,
    `struct_field_align_threshold = 0`, `enum_discrim_align_threshold = 0`,
    `match_arm_blocks = true`, `match_arm_indent = true`, `force_multiline_blocks = false`,
    `brace_style = SameLineWhere`, `control_brace_style = AlwaysSameLine`,
    `trailing_semicolon = true`, `trailing_comma = Vertical`, `blank_lines_upper_bound = 1`,
    `blank_lines_lower_bound = 0`, `inline_attribute_width = 0`,
    `format_generated_files = true`, `condense_wildcard_suffixes = false`.

## Attributes in the source

chloro honours rustfmt's attributes:

- `#![rustfmt::skip]` as an inner attribute of a file leaves the file as written.
- `#[rustfmt::skip]` on an item, statement or expression leaves that node as written.
- `#[rustfmt::skip::macros(name, …)]` and `#[rustfmt::skip::attributes(name, …)]` leave
  calls of those macros, or those attributes, as written inside the item they are on; as
  inner attributes (`#![…]`) they apply to the whole file or module.
