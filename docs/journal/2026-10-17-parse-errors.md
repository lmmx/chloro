# 2026-10-17: Reporting files that do not parse

A file that rust-analyzer's parser, or chloro's rules for syntax rustc rejects, refused was
left unchanged, as rustfmt leaves it, but silently: exit status 0, `--verbose` counted it as
unchanged, and the cache recorded it as formatted (#87). The command line now reports it as
rustfmt does, and the library has `try_` functions that return the error.

## Current State

- **rustfmt's behaviour, observed with rustfmt 1.9.0.** rustc's first error on standard
  error (`error: …`, ` --> path:line:col`, the line with the error marked, a blank line);
  nothing on standard output for the file under `--emit stdout`; the file not written; the
  other files formatted; exit status 1 in every mode. The path is the canonical one, as
  chloro's. rustfmt printed one error per file in every case tried, and on an unterminated
  string it printed rustc's error and then exited with 101.
- **Library.** `try_format_source_with_config`, `try_format_source_with_config_parallel` and
  `try_format_source_with_unit_cache` return `Result<String, ParseError>`; the functions
  without `try_` are unchanged and share their code (`format_whole`). `ParseError` has
  `message`, `line`, `column` (from 1, characters of the normalised source) and `width`.
  `chloro` re-exports `try_format_source_with_config` and `ParseError`.
- **Cost.** The formatter's parse is untouched. When a file is refused, `parse_error` parses
  it again to find out why: `parse_tree` (the parse without the rustc check, split out of
  `parse`), then `rustc_rejection`, the rustc check returning the element it stopped at
  (`walk` returns the index instead of `true`). A refusal that parses is a formatting failure
  (rustfmt 1.8.0's F6 panic), returned unchanged as before. Bench timings of base and change
  alternate within noise.
- **Position.** rust-analyzer reports a parser error at the trivia before the token it could
  not accept; chloro points at that token, as rustc does for "expected expression, found
  `;`". For an "expected …" error whose next token is on a later line, and at the end of the
  file, it points after the previous token, where rustc puts "expected `;`" and an unclosed
  delimiter. A lexer error covers its token; a rustc-compat rejection covers the rejected node
  or token, to the end of its line.
- **Message.** rust-analyzer's, with `expected SEMICOLON` read as "expected `;`" and ", found
  `x`" (or "keyword `let`", or `<eof>`) appended to "expected …" messages; for syntax only
  rustc rejects, "rustc's parser rejects this syntax". On the cases tried, the position
  matched rustc's for a missing expression, a missing `;` or `,` at a line's end, an
  unmatched `}`, tabs, multi-byte characters, CRLF and a byte order mark; rustc's wording
  differs for delimiters ("unclosed delimiter", "mismatched closing delimiter").
- **Command line.** `ProcessResult::ParseError` carries the rendered error
  (`report::render_parse_error`); the errors are printed after the run in path order, before
  the summary, which counts "Files that do not parse" with `--verbose`; the exit status is 1.
  Such a file is never a fixed point, so the cache keeps reporting it.
- **Checks.** Output hashes 0 of 1238 changed (sequential and four threads), `conform -i`
  1220/1220 at both releases, `config_matrix -n 1220` identical to the base, the workspace
  tests, the site's examples and checks.

## Missing

- **rustc's wording for the rustc-compat rules.** Each rule could name its error ("const
  globals cannot be mutable", "unexpected token: `<-`"), checked against rustc; the general
  message was kept to keep the change small.
- **rustc's notes and suggestions**, its second labels ("unclosed delimiter" at the opening
  one), and colour on a terminal.
- **Which error stops `rustc_syntax`.** A file whose first parser error is one the rebuild
  explains (`Box<(Copy) + Send>`, a const block item) and a later one it does not is reported
  at the first.
- **Known difference 20.** Edition-2015 trait methods with anonymous parameters, which
  rust-analyzer rejects and rustfmt formats, are now reported as parse errors.
