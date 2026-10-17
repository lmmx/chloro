//! The parser's front end without `LexedStr`: `rustc_lexer`'s tokens converted to
//! `SyntaxKind`s and the parser's [`Input`] built in one pass (EXP-031, lever P2 of the
//! parse-floor bounds).
//!
//! `LexedStr::new` (ra_ap_parser 0.0.307, `lexed_str.rs`) restarts `rustc_lexer::tokenize`
//! at every token, converts each `TokenKind` to a `SyntaxKind` (keywords looked up), checks
//! literal escapes and collects error messages, growing its vectors from empty; then
//! `LexedStr::to_input` (`shortcuts.rs`) walks the tokens again to drop trivia, look up
//! contextual keywords and set joint bits. [`lex`] does both in one loop over one cursor,
//! into vectors sized from the text, and keeps no messages.
//!
//! Why its tokens are `LexedStr`'s (read from the sources named above; checked on both
//! corpora, random texts and mutated fixtures by `examples/lex_equality.rs`, and against
//! `LexedStr` in debug builds on every text):
//!
//! - One cursor: with `FrontmatterAllowed::No`, `Cursor::advance_token` reads nothing but
//!   the remaining text (its `prev` character exists only in debug builds, for debug
//!   assertions), so one cursor over the text yields the tokens a new cursor per token
//!   yields. The one place `LexedStr` cuts a token short, a guarded string prefix (`#"`,
//!   `##`) before edition 2024, which becomes a `#` with the rest lexed again, restarts the
//!   cursor after the `#` the same way.
//! - The start of the text: `LexedStr::new` first runs the frontmatter parser, which emits
//!   a shebang or frontmatter token only when the text starts with `#!` not followed by `[`
//!   (after whitespace), or when its first character that is not Pattern_White_Space is `-`;
//!   [`lex`] answers `None` for those texts.
//! - Errors: [`lex`] answers `None` at the first token `LexedStr` attaches an error message
//!   to (the conditions of `Converter::extend_token` and `extend_literal`; an escape error
//!   counts when `err_to_msg` gives it a message, [`reported`]). A `Some` is therefore a
//!   text `LexedStr` lexes without errors, and the caller takes `LexedStr` for every other
//!   text, so error reporting and `rustc_syntax::reparse` see what they saw before.
//! - The input: `to_input`'s loop, run on each token as it is pushed.
//!
//! The cost of a text [`lex`] gives up on is the part lexed before it gave up.

#[cfg(debug_assertions)]
use ra_ap_parser::Output;
use ra_ap_parser::{Edition, Input, LexedStr, SyntaxKind, T};
use ra_ap_rustc_lexer::{Cursor, FrontmatterAllowed, LiteralKind, TokenKind};
use rustc_literal_escaper::{
    EscapeError, unescape_byte, unescape_byte_str, unescape_c_str, unescape_char, unescape_str,
};

#[cfg(debug_assertions)]
use super::TreeBuilder;

/// The tokens of a text as `LexedStr` holds them: kinds, start offsets, and an `EOF` entry
/// at the end whose start is the text's length.
pub struct Tokens<'a> {
    text: &'a str,
    /// For the debug builds' reference build.
    #[cfg(debug_assertions)]
    edition: Edition,
    tokens: Vec<Token>,
}

/// A token's kind and start offset.
#[derive(Clone, Copy)]
struct Token {
    start: u32,
    kind: SyntaxKind,
}

impl<'a> Tokens<'a> {
    /// The lexed text.
    pub fn as_str(&self) -> &'a str {
        self.text
    }
}

/// What the tree build reads of a lexed text: `LexedStr` or [`Tokens`].
pub trait Lexed {
    /// The number of tokens, `EOF` not counted.
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn kind(&self, i: usize) -> SyntaxKind;
    /// The start of token `i`; `i == len()` gives the text's length.
    fn text_start(&self, i: usize) -> usize;
    fn text(&self, i: usize) -> &str;
    /// `TreeBuilder::steps_interspersed` over this text, the debug builds' reference.
    #[cfg(debug_assertions)]
    fn interspersed(&self, builder: &mut TreeBuilder<'_>, output: &Output) -> Option<()>;
}

impl Lexed for LexedStr<'_> {
    #[inline]
    fn len(&self) -> usize {
        LexedStr::len(self)
    }
    #[inline]
    fn kind(&self, i: usize) -> SyntaxKind {
        LexedStr::kind(self, i)
    }
    #[inline]
    fn text_start(&self, i: usize) -> usize {
        LexedStr::text_start(self, i)
    }
    #[inline]
    fn text(&self, i: usize) -> &str {
        LexedStr::text(self, i)
    }
    #[cfg(debug_assertions)]
    fn interspersed(&self, builder: &mut TreeBuilder<'_>, output: &Output) -> Option<()> {
        builder.steps_interspersed(self, output)
    }
}

impl Lexed for Tokens<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.tokens.len() - 1
    }
    #[inline]
    fn kind(&self, i: usize) -> SyntaxKind {
        self.tokens[i].kind
    }
    #[inline]
    fn text_start(&self, i: usize) -> usize {
        self.tokens[i].start as usize
    }
    #[inline]
    fn text(&self, i: usize) -> &str {
        &self.text[self.tokens[i].start as usize..self.tokens[i + 1].start as usize]
    }
    #[cfg(debug_assertions)]
    fn interspersed(&self, builder: &mut TreeBuilder<'_>, output: &Output) -> Option<()> {
        builder.steps_interspersed(&LexedStr::new(self.edition, self.text), output)
    }
}

/// Where [`lex_into`] writes the parser's input: `Input`, or a recording for the equality
/// harness.
pub trait InputSink {
    fn push(&mut self, kind: SyntaxKind);
    fn push_ident(&mut self, contextual_kind: SyntaxKind);
    fn was_joint(&mut self);
}

impl InputSink for Input {
    #[inline(always)]
    fn push(&mut self, kind: SyntaxKind) {
        Input::push(self, kind);
    }
    #[inline(always)]
    fn push_ident(&mut self, contextual_kind: SyntaxKind) {
        Input::push_ident(self, contextual_kind);
    }
    #[inline(always)]
    fn was_joint(&mut self) {
        Input::was_joint(self);
    }
}

/// The tokens of `text` and the parser's input for them, as `LexedStr::new(edition, text)`
/// and its `to_input(edition)` give them; `None` when `LexedStr` reports a lexer error or
/// lexes a shebang or frontmatter at the start (see the module documentation).
pub fn lex(edition: Edition, text: &str) -> Option<(Tokens<'_>, Input)> {
    // Room for a token per four bytes: the samples average 3.9 (registry) and 6.9
    // (fixtures) bytes per token; a denser text grows the vectors as `LexedStr` does.
    let capacity = text.len() / 4 + 16;
    let mut input = Input::with_capacity(capacity);
    let tokens = lex_into(edition, text, &mut input, capacity)?;
    Some((tokens, input))
}

/// [`lex`], writing the input to `sink`.
pub fn lex_into<'a, S: InputSink>(
    edition: Edition,
    text: &'a str,
    sink: &mut S,
    capacity: usize,
) -> Option<Tokens<'a>> {
    let tokens = lex_tokens(edition, text, sink, capacity);
    #[cfg(debug_assertions)]
    check_against_lexed_str(edition, text, tokens.as_ref());
    tokens
}

#[inline]
fn lex_tokens<'a, S: InputSink>(
    edition: Edition,
    text: &'a str,
    sink: &mut S,
    capacity: usize,
) -> Option<Tokens<'a>> {
    if starts_with_script_prefix(text) {
        return None;
    }
    let mut tokens = Vec::with_capacity(capacity);
    let mut offset = 0usize;
    // `to_input`'s state: the previous token was punctuation that may join the next one.
    let mut was_joint = false;
    let mut cursor = Cursor::new(text, FrontmatterAllowed::No);
    loop {
        let token = cursor.advance_token();
        let mut len = token.len as usize;
        // The token's text, sliced where it is read.
        let token_text = || &text[offset..offset + token.len as usize];
        // `Converter::extend_token`, with `None` where it records an error.
        let k = match token.kind {
            TokenKind::Eof => break,
            TokenKind::Whitespace => SyntaxKind::WHITESPACE,
            TokenKind::LineComment { .. } => SyntaxKind::COMMENT,
            TokenKind::BlockComment { terminated, .. } => {
                if !terminated {
                    return None;
                }
                SyntaxKind::COMMENT
            }
            TokenKind::Ident => {
                let t = token_text();
                if t == "_" {
                    SyntaxKind::UNDERSCORE
                } else {
                    let k = keyword(t, edition);
                    if k == SyntaxKind::IDENT {
                        // `to_input`'s contextual keyword.
                        sink.push_ident(contextual_keyword(t, edition));
                        tokens.push(Token {
                            start: offset as u32,
                            kind: k,
                        });
                        offset += len;
                        continue;
                    }
                    k
                }
            }
            TokenKind::RawIdent => SyntaxKind::IDENT,
            TokenKind::Literal { kind, .. } => literal(kind, token_text())?,
            TokenKind::Lifetime { starts_with_number } => {
                if starts_with_number {
                    return None;
                }
                SyntaxKind::LIFETIME_IDENT
            }
            TokenKind::RawLifetime => SyntaxKind::LIFETIME_IDENT,
            TokenKind::Semi => T![;],
            TokenKind::Comma => T![,],
            TokenKind::Dot => T![.],
            TokenKind::OpenParen => T!['('],
            TokenKind::CloseParen => T![')'],
            TokenKind::OpenBrace => T!['{'],
            TokenKind::CloseBrace => T!['}'],
            TokenKind::OpenBracket => T!['['],
            TokenKind::CloseBracket => T![']'],
            TokenKind::At => T![@],
            TokenKind::Pound => T![#],
            TokenKind::Tilde => T![~],
            TokenKind::Question => T![?],
            TokenKind::Colon => T![:],
            TokenKind::Dollar => T![$],
            TokenKind::Eq => T![=],
            TokenKind::Bang => T![!],
            TokenKind::Lt => T![<],
            TokenKind::Gt => T![>],
            TokenKind::Minus => T![-],
            TokenKind::And => T![&],
            TokenKind::Or => T![|],
            TokenKind::Plus => T![+],
            TokenKind::Star => T![*],
            TokenKind::Slash => T![/],
            TokenKind::Caret => T![^],
            TokenKind::Percent => T![%],
            // No message: the parser reports the `ERROR` token.
            TokenKind::Unknown => SyntaxKind::ERROR,
            TokenKind::UnknownPrefix if token_text() == "builtin" => SyntaxKind::IDENT,
            TokenKind::GuardedStrPrefix if !edition.at_least_2024() => {
                // `#"` or `##`: a `#`, and the rest lexed again.
                len -= 1;
                cursor = Cursor::new(&text[offset + len..], FrontmatterAllowed::No);
                T![#]
            }
            // Each with a message: an invalid identifier, a reserved guarded string prefix,
            // an unknown literal or lifetime prefix, and frontmatter (which a cursor
            // without `FrontmatterAllowed::Yes` does not produce).
            TokenKind::InvalidIdent
            | TokenKind::GuardedStrPrefix
            | TokenKind::UnknownPrefix
            | TokenKind::UnknownPrefixLifetime
            | TokenKind::Frontmatter { .. } => return None,
        };
        tokens.push(Token {
            start: offset as u32,
            kind: k,
        });
        // `LexedStr::to_input` (identifiers that are not keywords took their turn above).
        if k.is_trivia() {
            was_joint = false;
        } else if k == SyntaxKind::IDENT {
            sink.push_ident(contextual_keyword(&text[offset..offset + len], edition));
        } else {
            if was_joint {
                sink.was_joint();
            }
            sink.push(k);
            // A float with a fractional part is joint, which the parser reads when it splits
            // the float in a field access.
            if k == SyntaxKind::FLOAT_NUMBER {
                if !text[offset..offset + len].ends_with('.') {
                    sink.was_joint();
                } else {
                    was_joint = false;
                }
            } else {
                was_joint = true;
            }
        }
        offset += len;
    }
    tokens.push(Token {
        start: offset as u32,
        kind: SyntaxKind::EOF,
    });
    Some(Tokens {
        text,
        #[cfg(debug_assertions)]
        edition,
        tokens,
    })
}

/// `SyntaxKind::from_keyword(ident, edition)`, or `IDENT`. Every keyword it knows (and
/// every contextual keyword) is 2 to 15 bytes of lowercase letters and `_`, except `Self`,
/// so other identifiers (types, most of all) skip the lookup.
#[inline]
fn keyword(ident: &str, edition: Edition) -> SyntaxKind {
    if may_be_keyword(ident) {
        SyntaxKind::from_keyword(ident, edition).unwrap_or(SyntaxKind::IDENT)
    } else {
        debug_assert!(
            SyntaxKind::from_keyword(ident, edition).is_none(),
            "{ident}"
        );
        SyntaxKind::IDENT
    }
}

/// `SyntaxKind::from_contextual_keyword(ident, edition)`, or `IDENT`, as `to_input` pushes it.
#[inline]
fn contextual_keyword(ident: &str, edition: Edition) -> SyntaxKind {
    if may_be_keyword(ident) {
        SyntaxKind::from_contextual_keyword(ident, edition).unwrap_or(SyntaxKind::IDENT)
    } else {
        debug_assert!(
            SyntaxKind::from_contextual_keyword(ident, edition).is_none(),
            "{ident}"
        );
        SyntaxKind::IDENT
    }
}

#[inline]
fn may_be_keyword(ident: &str) -> bool {
    let b = ident.as_bytes();
    (2..=15).contains(&b.len()) && (b[0].is_ascii_lowercase() || ident == "Self")
}

/// `Converter::extend_literal`'s kind for a literal token, `None` where it records an error.
#[inline]
fn literal(kind: LiteralKind, token_text: &str) -> Option<SyntaxKind> {
    // The literal's contents: after the opening quote (and the one-letter prefix), up to
    // the last quote (a suffix follows it).
    let contents = |prefix: usize, quote: char| {
        let text = &token_text[prefix..];
        &text[..text
            .rfind(quote)
            .expect("a terminated literal has its quote")]
    };
    Some(match kind {
        LiteralKind::Int { empty_int, .. } => {
            if empty_int {
                return None;
            }
            SyntaxKind::INT_NUMBER
        }
        LiteralKind::Float { empty_exponent, .. } => {
            if empty_exponent {
                return None;
            }
            SyntaxKind::FLOAT_NUMBER
        }
        LiteralKind::Char { terminated } => {
            if !terminated || unescape_char(contents(1, '\'')).is_err_and(reported) {
                return None;
            }
            SyntaxKind::CHAR
        }
        LiteralKind::Byte { terminated } => {
            if !terminated || unescape_byte(contents(2, '\'')).is_err_and(reported) {
                return None;
            }
            SyntaxKind::BYTE
        }
        LiteralKind::Str { terminated } => {
            if !terminated {
                return None;
            }
            let text = contents(1, '"');
            // Without a backslash, the only errors are a `"` (the lexer ends the string at
            // the first one) and a bare carriage return (no message).
            if text.as_bytes().contains(&b'\\') {
                let mut error = false;
                unescape_str(text, |_, res| error |= res.is_err_and(reported));
                if error {
                    return None;
                }
            }
            SyntaxKind::STRING
        }
        LiteralKind::ByteStr { terminated } => {
            if !terminated {
                return None;
            }
            let mut error = false;
            unescape_byte_str(contents(2, '"'), |_, res| error |= res.is_err_and(reported));
            if error {
                return None;
            }
            SyntaxKind::BYTE_STRING
        }
        LiteralKind::CStr { terminated } => {
            if !terminated {
                return None;
            }
            let mut error = false;
            unescape_c_str(contents(2, '"'), |_, res| error |= res.is_err_and(reported));
            if error {
                return None;
            }
            SyntaxKind::C_STRING
        }
        LiteralKind::RawStr { n_hashes } => {
            n_hashes?;
            SyntaxKind::STRING
        }
        LiteralKind::RawByteStr { n_hashes } => {
            n_hashes?;
            SyntaxKind::BYTE_STRING
        }
        LiteralKind::RawCStr { n_hashes } => {
            n_hashes?;
            SyntaxKind::C_STRING
        }
    })
}

/// Whether `LexedStr` reports an escape error: `err_to_msg` (ra_ap_parser 0.0.307,
/// `lexed_str.rs`) gives these five an empty message, which `Converter::push` drops.
#[inline]
fn reported(e: EscapeError) -> bool {
    !matches!(
        e,
        EscapeError::LoneSlash
            | EscapeError::BareCarriageReturn
            | EscapeError::BareCarriageReturnInRawString
            | EscapeError::UnskippedWhitespaceWarning
            | EscapeError::MultipleSkippedLinesWarning
    )
}

/// Whether `LexedStr::new` may lex a shebang or frontmatter token at the start of `text`:
/// `ScriptSource::parse` (ra_ap_parser 0.0.307, `frontmatter.rs`) finds a shebang only after
/// `#!` not followed by `[` (after `trim_start`), and a fence only at a `-` after
/// whitespace lines; when it errs, `rustc_lexer::strip_shebang` looks for a shebang, which
/// also needs `#!`. Other texts get no token before the first one `rustc_lexer` lexes.
pub fn starts_with_script_prefix(text: &str) -> bool {
    if let Some(rest) = text.strip_prefix("#!") {
        return !rest.trim_start().starts_with('[');
    }
    text.chars().find(|&c| !is_pattern_white_space(c)) == Some('-')
}

/// `frontmatter.rs`'s `is_whitespace` (Pattern_White_Space).
fn is_pattern_white_space(c: char) -> bool {
    matches!(
        c,
        '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{0009}'
            | '\u{0020}'
    )
}

/// Debug builds: a `Some` is `LexedStr`'s tokens without errors; a `None` is a text with a
/// lexer error or a script prefix.
#[cfg(debug_assertions)]
fn check_against_lexed_str(edition: Edition, text: &str, tokens: Option<&Tokens<'_>>) {
    let lexed = LexedStr::new(edition, text);
    match tokens {
        Some(t) => {
            assert!(lexed.errors().next().is_none(), "{text:?} has lexer errors");
            assert_eq!(Lexed::len(t), lexed.len(), "token count of {text:?}");
            for i in 0..lexed.len() {
                assert_eq!(
                    t.tokens[i].kind,
                    lexed.kind(i),
                    "kind of token {i} of {text:?}"
                );
                assert_eq!(t.text_start(i), lexed.text_start(i), "start of token {i}");
            }
            assert_eq!(t.text_start(lexed.len()), lexed.text_start(lexed.len()));
        }
        None => assert!(
            lexed.errors().next().is_some() || starts_with_script_prefix(text),
            "{text:?} lexes without errors"
        ),
    }
}
