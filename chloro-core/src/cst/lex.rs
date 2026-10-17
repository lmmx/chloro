//! The parser's front end without `LexedStr`: `rustc_lexer`'s tokens converted to
//! `SyntaxKind`s and the parser's [`Input`] built in one pass (EXP-031, lever P2 of the
//! parse-floor bounds).
//!
//! `LexedStr::new` (ra_ap_parser 0.0.307, `lexed_str.rs`) restarts `rustc_lexer::tokenize`
//! at every token, converts each `TokenKind` to a `SyntaxKind` (keywords looked up), checks
//! literal escapes and collects error messages, growing its vectors from empty; then
//! `LexedStr::to_input` (`shortcuts.rs`) walks the tokens again to drop trivia, look up
//! contextual keywords and set joint bits. [`lex`] does both in one loop, into vectors sized
//! from the text, and keeps no messages; the common tokens skip the cursor (P4).
//!
//! Why its tokens are `LexedStr`'s (read from the sources named above; checked on both
//! corpora, random texts and mutated fixtures by `examples/lex_equality.rs`, and against
//! `LexedStr` in debug builds on every text):
//!
//! - The tokens: `LexedStr` starts a cursor (`FrontmatterAllowed::No`) at every token;
//!   [`fast_token`] answers what that cursor answers for whitespace, ASCII identifiers,
//!   one-character punctuation and line comments by a byte scan, and a cursor started at the
//!   token lexes every other token. Where `LexedStr` cuts a token short, a guarded string
//!   prefix (`#"`, `##`) before edition 2024, which becomes a `#` with the rest lexed
//!   again, the next token starts after the `#` the same way.
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
use ra_ap_rustc_lexer::{
    Cursor, DocStyle, FrontmatterAllowed, LiteralKind, Token as LexToken, TokenKind,
};
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
    let bytes = text.as_bytes();
    while let Some(&b) = bytes.get(offset) {
        // The common tokens by a byte scan, the others by a cursor started at the token, as
        // `LexedStr` starts one for every token (see [`fast_token`] for why the scans give
        // the cursor's tokens).
        let class = CLASS[b as usize];
        if class & PUNCT != 0 {
            // One-byte punctuation: `extend_token`'s kind and `to_input`'s joint state.
            let k = PUNCT_KIND[b as usize];
            #[cfg(debug_assertions)]
            check_fast_token(text, offset, fast_token(bytes, offset));
            tokens.push(Token {
                start: offset as u32,
                kind: k,
            });
            if was_joint {
                sink.was_joint();
            }
            sink.push(k);
            was_joint = true;
            offset += 1;
            continue;
        }
        let mut scanned = None;
        if class & WS != 0 {
            let mut end = offset + 1;
            while bytes.get(end).is_some_and(|&b| CLASS[b as usize] & WS != 0) {
                end += 1;
            }
            if bytes.get(end).is_none_or(|&b| b < 0x80) {
                scanned = Some(LexToken {
                    kind: TokenKind::Whitespace,
                    len: (end - offset) as u32,
                });
            }
        } else if class & ID_START != 0 {
            let mut end = offset + 1;
            while bytes
                .get(end)
                .is_some_and(|&b| CLASS[b as usize] & ID_CONTINUE != 0)
            {
                end += 1;
            }
            if bytes
                .get(end)
                .is_none_or(|&b| CLASS[b as usize] & ID_STOP == 0)
            {
                scanned = Some(LexToken {
                    kind: TokenKind::Ident,
                    len: (end - offset) as u32,
                });
            }
        }
        let token = match scanned.or_else(|| fast_token(bytes, offset)) {
            Some(token) => {
                #[cfg(debug_assertions)]
                check_fast_token(text, offset, Some(LexToken { ..token }));
                token
            }
            None => Cursor::new(&text[offset..], FrontmatterAllowed::No).advance_token(),
        };
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

/// Byte classes of the scans in [`lex_tokens`]: Pattern_White_Space's ASCII members, ASCII
/// XID_Start and `_`, ASCII XID_Continue, the bytes after an identifier that make the cursor
/// answer otherwise (`#`, `"`, `'`, and any byte of a non-ASCII character), and the
/// one-byte punctuation that needs no lookahead (all of the cursor's one-symbol tokens but
/// `#` and `/`).
const WS: u8 = 1;
const ID_START: u8 = 2;
const ID_CONTINUE: u8 = 4;
const ID_STOP: u8 = 8;
const PUNCT: u8 = 16;

const PUNCTUATION: [(u8, SyntaxKind); 25] = [
    (b';', T![;]),
    (b',', T![,]),
    (b'.', T![.]),
    (b'(', T!['(']),
    (b')', T![')']),
    (b'{', T!['{']),
    (b'}', T!['}']),
    (b'[', T!['[']),
    (b']', T![']']),
    (b'@', T![@]),
    (b'~', T![~]),
    (b'?', T![?]),
    (b':', T![:]),
    (b'$', T![$]),
    (b'=', T![=]),
    (b'!', T![!]),
    (b'<', T![<]),
    (b'>', T![>]),
    (b'-', T![-]),
    (b'&', T![&]),
    (b'|', T![|]),
    (b'+', T![+]),
    (b'*', T![*]),
    (b'^', T![^]),
    (b'%', T![%]),
];

static CLASS: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut b = 0usize;
    while b < 256 {
        let c = b as u8;
        if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
            t[b] |= WS;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            t[b] |= ID_START | ID_CONTINUE;
        }
        if c.is_ascii_digit() {
            t[b] |= ID_CONTINUE;
        }
        if c >= 0x80 || matches!(c, b'#' | b'"' | b'\'') {
            t[b] |= ID_STOP;
        }
        b += 1;
    }
    let mut i = 0;
    while i < PUNCTUATION.len() {
        t[PUNCTUATION[i].0 as usize] |= PUNCT;
        i += 1;
    }
    t
};

static PUNCT_KIND: [SyntaxKind; 256] = {
    let mut t = [SyntaxKind::EOF; 256];
    let mut i = 0;
    while i < PUNCTUATION.len() {
        t[PUNCTUATION[i].0 as usize] = PUNCTUATION[i].1;
        i += 1;
    }
    t
};

/// Debug builds: a token a scan decided is the cursor's.
#[cfg(debug_assertions)]
fn check_fast_token(text: &str, offset: usize, token: Option<LexToken>) {
    let token = token.expect("the scans decide only tokens `fast_token` decides");
    let reference = Cursor::new(&text[offset..], FrontmatterAllowed::No).advance_token();
    assert!(
        reference.kind == token.kind && reference.len == token.len,
        "{token:?} at {offset} of {text:?}: rustc_lexer {reference:?}"
    );
}

/// The token `Cursor::advance_token` lexes at `offset` (with `FrontmatterAllowed::No`), for
/// the tokens a byte scan decides: whitespace, identifiers and keywords, one-character
/// punctuation and line comments, all ASCII. `None` for every other token, and wherever
/// the byte after the scan could change the answer; the caller then runs the cursor.
///
/// Why each answer is the cursor's (ra-ap-rustc_lexer 0.137.0, `lib.rs`, `advance_token`
/// and the functions it dispatches to; debug builds compare every answer with the
/// cursor's):
///
/// - Whitespace: the cursor eats Pattern_White_Space (`is_whitespace`), whose ASCII members
///   are exactly tab, line feed, vertical tab, form feed, carriage return and space; the
///   scan stops at any other byte, and gives up when that byte starts a non-ASCII
///   character (five of Pattern_White_Space's are).
/// - Identifiers: an ASCII letter or `_` starts one (`is_id_start`); `r`, `b` and `c` start
///   raw identifiers and literals only when a `#`, `"` or `'` follows the letters, and
///   `ident_or_unknown_prefix` answers `UnknownPrefix` for a `#`, `"` or `'` after the
///   identifier and `InvalidIdent` for an emoji: the scan eats ASCII XID_Continue (letters,
///   digits, `_`) and gives up when the next byte is `#`, `"`, `'` or starts a non-ASCII
///   character (which may continue the identifier).
/// - Punctuation: the cursor's one-symbol tokens are one byte each, without lookahead,
///   except `#` before `"` or `#` (a guarded string prefix) and `/` before `/` or `*`.
/// - Line comments: `//` to the next line feed (`line_comment`, `eat_until(b'\n')`), with
///   the doc style of its third and fourth bytes.
#[inline(always)]
fn fast_token(bytes: &[u8], offset: usize) -> Option<LexToken> {
    #[inline(always)]
    fn token(kind: TokenKind, len: usize) -> Option<LexToken> {
        Some(LexToken {
            kind,
            len: len as u32,
        })
    }
    let at = |i: usize| bytes.get(i).copied();
    let b = at(offset)?;
    match b {
        b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C => {
            let mut end = offset + 1;
            while let Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) = at(end) {
                end += 1;
            }
            if at(end).is_some_and(|b| b >= 0x80) {
                return None;
            }
            token(TokenKind::Whitespace, end - offset)
        }
        b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
            let mut end = offset + 1;
            while at(end).is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_') {
                end += 1;
            }
            if at(end).is_some_and(|b| b >= 0x80 || matches!(b, b'#' | b'"' | b'\'')) {
                return None;
            }
            token(TokenKind::Ident, end - offset)
        }
        b';' => token(TokenKind::Semi, 1),
        b',' => token(TokenKind::Comma, 1),
        b'.' => token(TokenKind::Dot, 1),
        b'(' => token(TokenKind::OpenParen, 1),
        b')' => token(TokenKind::CloseParen, 1),
        b'{' => token(TokenKind::OpenBrace, 1),
        b'}' => token(TokenKind::CloseBrace, 1),
        b'[' => token(TokenKind::OpenBracket, 1),
        b']' => token(TokenKind::CloseBracket, 1),
        b'@' => token(TokenKind::At, 1),
        b'~' => token(TokenKind::Tilde, 1),
        b'?' => token(TokenKind::Question, 1),
        b':' => token(TokenKind::Colon, 1),
        b'$' => token(TokenKind::Dollar, 1),
        b'=' => token(TokenKind::Eq, 1),
        b'!' => token(TokenKind::Bang, 1),
        b'<' => token(TokenKind::Lt, 1),
        b'>' => token(TokenKind::Gt, 1),
        b'-' => token(TokenKind::Minus, 1),
        b'&' => token(TokenKind::And, 1),
        b'|' => token(TokenKind::Or, 1),
        b'+' => token(TokenKind::Plus, 1),
        b'*' => token(TokenKind::Star, 1),
        b'^' => token(TokenKind::Caret, 1),
        b'%' => token(TokenKind::Percent, 1),
        b'#' => match at(offset + 1) {
            Some(b'"' | b'#') => None,
            _ => token(TokenKind::Pound, 1),
        },
        b'/' => match at(offset + 1) {
            Some(b'/') => {
                let doc_style = match at(offset + 2) {
                    Some(b'!') => Some(DocStyle::Inner),
                    Some(b'/') if at(offset + 3) != Some(b'/') => Some(DocStyle::Outer),
                    _ => None,
                };
                let rest = &bytes[offset..];
                let len = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
                token(TokenKind::LineComment { doc_style }, len)
            }
            Some(b'*') => None,
            _ => token(TokenKind::Slash, 1),
        },
        _ => None,
    }
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
