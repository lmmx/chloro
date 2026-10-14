# Review: CLI cache, parallel item formatting, flat CST

Reviewer pass over `claude/chloro-perf-research-swarm-nrscj8` (integration head), read-only.
Scope: `chloro/src/cli/cache.rs` with `worker.rs` and `cli.rs`;
`chloro-core/src/formatter/parallel.rs`; `chloro-core/src/cst/mod.rs`.
Inputs were run through `target/release/chloro` and `target/release/examples/fmt_stdin`.

## Findings (most severe first)

1. **latent: temporary cache file name can carry one configuration's entries into another
   configuration's cache file.** `chloro/src/cli/cache.rs:326-327`. The temporary name is
   `.{pid}.{subsec_nanos}.tmp`; it does not name the target cache file and is opened without
   `create_new`. Trigger: two chloro processes that share a cache directory, have the same PID
   (separate PID namespaces, for example two CI containers that mount one cache directory)
   and save at the same `subsec_nanos` value, with different `--config`. Sequence: A writes
   the temporary file, B overwrites it with entries for config Y, A renames it to config X's
   cache file, B's rename fails. Config X's file then holds fixed points of config Y, and a
   later `--check` under X reports a file that X would reformat as formatted. The collision
   is unlikely at nanosecond resolution and more likely on clocks with coarse resolution.
   Smallest fix: put the cache file name in the temporary name
   (`.{file_name}.{pid}.{nanos}.tmp`) and open it with `OpenOptions::create_new(true)`.
2. **latent (macOS, Windows): the binary key can name a binary other than the running one.**
   `chloro/src/cli/cache.rs:192-194`. `tool_key` hashes the file at `current_exe()` when the
   cache is opened, after file discovery. On Linux `current_exe()` reads `/proc/self/exe`; once
   the binary is replaced by a rename the link reads `<path> (deleted)`, `fs::read` fails and
   the cache is disabled, so Linux is safe. On macOS and Windows `current_exe()` returns the
   original path, so a binary replaced between process start and `Cache::open` (a `cargo
   install` or `cargo build` during a long discovery) is hashed instead of the running code,
   and the old code's fixed points are filed under the new binary's key. Smallest fix: on
   Linux hash `/proc/self/exe` directly; elsewhere record the executable's metadata (size,
   mtime) at the start of `main` and again before `save`, and skip `save` when it differs or
   is newer than the process start.
3. **latent: the cache key does not record whether the parallel or the sequential path
   produced the observation.** `chloro/src/cli/worker.rs:82-86` and `:108`. A file of at least
   128 KiB is checked with `format_source_with_config_parallel` when `--threads` > 1, the
   `--write` second pass always uses the sequential path, and a later run may use either. The
   cache is correct only while the two paths are byte-identical; a divergence in
   `parallel.rs` becomes a stale hit under the other thread count. The 28 adversarial inputs
   below give identical output on both paths, so no hit is wrong today. Smallest fix: none
   needed while the equivalence holds; a test that runs both paths over the corpus guards it.
4. **bug (reporting) / latent (write race): one file is processed twice when it is reachable
   twice.** `chloro/src/cli.rs:99-110`, `chloro/src/cli/orchestrate.rs:24`. Discovered files
   are canonicalised, files named on the command line are not, and the list is not
   de-duplicated. Confirmed: a directory holding `b.rs` and a symlink `a.rs -> b.rs` gives
   `Found 2 Rust file(s)`, two `Processing: .../b.rs` lines, and under `--check`
   `Error: 2 file(s) need formatting` for one file. Under `--write` two workers write the
   same path; if one reads while the other is between the truncate and the write of
   `fs::write`, it formats a truncated file and writes that back. The cache records only
   digests it verified, so no wrong hit follows. Smallest fix: canonicalise every entry of
   `all_files`, then `sort` and `dedup` before `format_all`.
5. **latent: in release builds a unit with no recorded gap but with text loses the text.**
   `chloro-core/src/formatter/parallel.rs:116-122`. When `out.gap` is `None` the loop
   `continue`s; only `debug_assert!`s check that the text is empty and that `last_pos` did
   not move. Any future item path that pushes text before its first `format_missing*` call
   drops that item from the output in release builds. The same holds for
   `chloro-core/src/formatter/visitor.rs:124` (`defer_gap`): if text precedes the first gap
   call, release builds replay the gap before the text instead of after it. Neither is
   reachable with the inputs tried. Smallest fix: turn both checks into `assert!`s, or have
   `format_file` fall back to `visit_items_with_reordering` when either condition fails.
6. **latent: `>4 GiB` input wraps offsets silently.** `chloro-core/src/cst/mod.rs:235`, `:243`,
   `:248`, `:262-263`, `:281`. Element indices, token lengths and the running offset are
   `u32`, built with `as u32` and `+=`; the release profile does not enable overflow checks,
   and no entry point rejects a source longer than `u32::MAX` bytes. On such an input
   `start`/`len` wrap, `Node::text`/`Token::text` (`:485-488`, `:618-621`) slice the wrong
   bytes or panic on a char boundary, and the output is garbage instead of a refusal. The
   lexer (`ra_ap_parser::LexedStr`) and `BytePos` are `u32` too. Smallest fix: in
   `format_text` and `format_text_parallel`, return `None` (input echoed) when
   `source.len() > u32::MAX as usize`.
7. **nit: a worker panic loses its payload.** `chloro-core/src/formatter/parallel.rs:216`.
   `h.join().expect(..)` re-panics with a fixed message; the original message is printed by
   the panic hook, and the outcome matches the sequential path (the process panics, the
   cache is not saved). The other workers drain the queue before the scope unwinds. Smallest
   fix: `std::panic::resume_unwind(payload)`.
8. **nit: `line_number -= 1` can underflow on a worker.**
   `chloro-core/src/formatter/imports.rs:45-48`. A worker's buffer starts as an uncounted
   `"\n"` with `line_number = 0`; a second, non-deferred removed-import gap before any text
   pops that newline and decrements 0. A unit is one item or one reorderable group written by
   one `push_rewrite`, so no input reaches this today. Smallest fix: `saturating_sub`, or
   decrement only when `line_number > 0`.
9. **nit: the cache file does not record its full key.** `chloro/src/cli/cache.rs:209-220`,
   `:338-357`. The file name keeps 128 bits of `BLAKE3(tool || config)` and the header holds
   no key, so a file name collision (probability about 2^-64 for the number of files in one
   directory) or a cache file copied under another name is read as valid. Smallest fix:
   write the 32-byte key into the header and compare it in `parse`.
10. **nit: temporary files left by a crash are never removed.**
    `chloro/src/cli/cache.rs:361-385`. `remove_stale_files` considers only names ending in
    `.chloro-cache`; `.{pid}.{nanos}.tmp` files survive. Smallest fix: also remove stale
    `.tmp` files older than `STALE_FILE_AGE`.

## Checked and sound

### Cache (`cache.rs`, `worker.rs`, `cli.rs`)

- The content key is BLAKE3 over the exact bytes handed to the formatter
  (`read_to_string` output), so a path, a symlink, an mtime or a BOM never enters it; a
  file that is not UTF-8 fails `read_to_string` and is neither formatted nor cached.
- `config_key_bytes` destructures `Config` without `..` and matches every enum without a
  wildcard; every field is serialised with a fixed width or a tag, so two distinct
  configurations cannot serialise to the same bytes. `Config` is the only input to the
  formatter besides the content: `Args::formatting_config` reads only `--config`, there is
  no `rustfmt.toml` discovery, and the one environment variable read in `chloro-core`
  (`CHLORO_DEBUG`) only switches debug logging. `Native` newline style is covered by the OS
  in the tool key.
- A hit returns `changed: false` and, in print mode, echoes the input; this equals the
  formatter's output exactly when the recorded fact `format(C) == C` holds. Every recorded
  digest is either an unchanged input or a `--write` output whose second pass returned it
  unchanged, so every entry is an observed fixed point. Files the formatter refuses
  (syntax error, `format_failure`) come back unchanged and are recorded; a hit reproduces
  the same unchanged result.
- Two processes with different PIDs on one cache directory: each reads once and writes
  once through a rename, so the last writer wins and entries are lost, never mixed; every
  entry of one file belongs to that file's (tool, config) pair.
- An unwritable or invalid cache directory: confirmed with `--cache-dir <file>/sub`; open
  yields an empty cache, save fails with `Not a directory`, which is logged under `-v`,
  and the run's result is unaffected. A malformed, truncated or foreign cache file fails
  `parse` and is replaced.
- Generation arithmetic: `wrapping_add` and `wrapping_sub` order entries correctly across a
  `u32` wrap; eviction keeps the `MAX_ENTRIES` most recent.

### Parallel items (`parallel.rs`)

- Visitor state at a unit's start matches the sequential visitor at a top-level item:
  `block_indent` empty, `is_if_else_block`/`is_loop_block` false, `is_macro_def` false (the
  parallel path is only reached for files, never for macro definition bodies), skip context
  from the file's inner attributes (`visit_item` saves and restores it per item, so reuse
  of a worker visitor across units does not leak names), `last_pos` at the unit's start,
  and a buffer ending in a newline. The memo and the block cache are cleared at every
  top-level item on both paths, so a per-worker `RunState` sees the same memo state.
- The first gap call is recorded and replayed on the main buffer, so the only read of
  preceding output (empty buffer, trailing newlines) happens on the real buffer.
- `skipped_range`: `push_skipped_with_span` stores `hi = line_number + 1`; the worker counts
  newlines after its uncounted leading `"\n"`, and the main thread adds `line_number` after
  the replayed gap, which equals the sequential count. Ranges pushed by nested
  `from_context` visitors (`unit: None`) start their own count at 0 on both paths and are
  correctly left unshifted.
- Thread counts: 0 and 1 and files with fewer than two units take the sequential path;
  more threads than units are capped by `threads.min(units.len())`; `outputs` is rebuilt in
  unit order.
- Sequential versus `--threads 8` on 14 files of about 140 KiB (padding plus: two items on
  one line; `#[rustfmt::skip]` first and last; `use {};` first, between items and last;
  `#![rustfmt::skip::macros]` with a leading comment; skipped `mod x;`; trailing and
  same-line comments with blank-line runs; `macro_rules!` and macro calls; inner doc and
  trailing comment; `use`/`mod`/`extern crate` groups; skipped items nested in `mod` and
  `impl`; `#![rustfmt::skip::attributes]`; CRLF), each under the default configuration and
  under `reorder_imports=false,reorder_modules=false`: all 28 outputs and exit codes are
  byte-identical.

### Flat CST (`cst/mod.rs`)

- No `unsafe`; handles are `&'t Tree<'t>` plus an index, and fragment trees live in a
  `typed_arena::Arena` borrowed for the run's `'a`, so the borrow checker ties every
  handle in the memo and the macro-argument cache to the arena. Each worker owns its own
  arena and `RunState`; `Tree` holds only `&str`, `Vec<Elem>` and `u32`, so sharing the file
  tree across threads is `Sync`.
- Tree identity in the run caches is `(tree id, index)`; ids come from a per-run counter
  starting at 1 (the file is 0) and every `format_text` builds its own `RunState`, so ids do
  not repeat within one cache.
- Index arithmetic: `end` is set to the element count at `finish_node`, so a subtree is
  `idx..end`; `Children` steps by `end`; `parent_of` returns the nearest earlier node whose
  `end` exceeds the index, which in preorder is the parent; `first_token` and `last_token`
  follow rowan's first-child and last-child chains, including `None` for an empty node.
- Split float tokens: the builder advances the offset by each piece's length and the pieces
  are contiguous source slices, so `text()` slices them exactly; `x.0.1`, `x.0 .1` and
  `1.0.max(2.)` format correctly through `fmt_stdin`.
- Empty and trivia-only inputs (`""`, `"\n"`, `"   \n\n"`, `// c`, `/* c */`, a BOM alone,
  a shebang alone, `#![a]`) format without a panic to the expected text: the parser always
  emits the `SOURCE_FILE` node, so `root()` exists.
