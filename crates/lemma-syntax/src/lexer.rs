//! The lexer of `docs/grammar.md` section 2: every token with its byte range and the lexical
//! faults of section 7 attached to it. It takes no feedback from the parser.

mod comment;
mod number;

use lemma_diagnostics::Kind;

use crate::token::{
    C1Symbol, C1Variant, Comment, Fault, Keyword, Lexed, ScalarClass, ScalarType, TextRange, Token,
    TokenKind, VectorType, byte_name, code_point_name, is_printable_ascii, is_word_byte,
};

const SIGMA: char = '\u{03A3}';
const ELEMENT_OF: char = '\u{2208}';
const FOR_ALL: char = '\u{2200}';
const SMALL_ELEMENT_OF: char = '\u{220A}';
const TURNED_A: char = '\u{2C6F}';
const ELLIPSIS: char = '\u{2026}';
const ALMOST_EQUAL: char = '\u{2248}';
const MEMBERSHIP_LOOKALIKES: [char; 2] = ['\u{03F5}', '\u{03B5}'];
const SUM_LOOKALIKES: [char; 8] = [
    '\u{2211}',
    '\u{01A9}',
    '\u{2140}',
    '\u{1D6BA}',
    '\u{1D6F4}',
    '\u{1D72E}',
    '\u{1D768}',
    '\u{1D7A2}',
];
const UNREPAIRED_LOOKALIKES: [char; 6] = [
    '\u{2209}', '\u{220B}', '\u{220C}', '\u{220D}', '\u{2203}', '\u{2204}',
];
const SPACE_LOOKALIKES: [char; 4] = ['\u{00A0}', '\u{2007}', '\u{2009}', '\u{202F}'];
const ZERO_WIDTH: [char; 2] = ['\u{200B}', '\u{FEFF}'];

static OPERATOR_LOOKALIKES: [(char, TokenKind, &str); 11] = [
    ('\u{2264}', TokenKind::LessEqual, "<="),
    ('\u{2265}', TokenKind::GreaterEqual, ">="),
    ('\u{2260}', TokenKind::BangEqual, "!="),
    ('\u{00D7}', TokenKind::Star, "*"),
    ('\u{22C5}', TokenKind::Star, "*"),
    ('\u{00B7}', TokenKind::Star, "*"),
    ('\u{2212}', TokenKind::Minus, "-"),
    ('\u{2192}', TokenKind::Arrow, "->"),
    ('\u{2227}', TokenKind::AndAnd, "&&"),
    ('\u{2228}', TokenKind::OrOr, "||"),
    ('\u{00AC}', TokenKind::Bang, "!"),
];

/// Lexes `text` into tokens and comments under the given C1 variant.
///
/// Lexical faults are attached to tokens, never reported on their own: a repaired fault sits on
/// the token that carries its repair (or on the next token, for trivia), an unrepaired one on an
/// [`TokenKind::Error`] token.
pub fn lex(text: &[u8], variant: C1Variant) -> Lexed {
    Lexer {
        bytes: text,
        variant,
        position: 0,
        pending: Vec::new(),
        tokens: Vec::new(),
        comments: Vec::new(),
    }
    .run()
}

enum Decoded {
    Char(char, usize),
    Invalid(usize),
}

/// Decodes the code point at `position`, or the invalid byte sequence there; `None` at the end.
fn decode(bytes: &[u8], position: usize) -> Option<Decoded> {
    let rest = bytes.get(position..).filter(|rest| !rest.is_empty())?;
    let window = rest.get(..rest.len().min(4))?;
    let valid_length = match std::str::from_utf8(window) {
        Ok(_) => window.len(),
        Err(error) => error.valid_up_to(),
    };
    let first = window
        .get(..valid_length)
        .and_then(|valid| std::str::from_utf8(valid).ok())
        .and_then(|valid| valid.chars().next());
    let Some(character) = first else {
        let invalid_length = std::str::from_utf8(window)
            .err()
            .and_then(|error| error.error_len())
            .unwrap_or(window.len());
        return Some(Decoded::Invalid(invalid_length));
    };
    Some(Decoded::Char(character, character.len_utf8()))
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange { start, end }
}

fn fault(kind: Kind, message: String, repair: Vec<(TextRange, String)>) -> Fault {
    Fault {
        kind,
        message,
        repair,
        span: None,
    }
}

fn is_blank_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r')
}

fn word_kind(word: &str) -> TokenKind {
    match word {
        "sum" => TokenKind::Sum(C1Variant::Ascii),
        "in" => TokenKind::In(C1Variant::Ascii),
        "forall" => TokenKind::Forall(C1Variant::Ascii),
        _ => Keyword::from_word(word)
            .map(TokenKind::Keyword)
            .or_else(|| scalar_type(word).map(TokenKind::ScalarType))
            .or_else(|| vector_type(word).map(TokenKind::VectorType))
            .unwrap_or_else(|| TokenKind::Name(word.to_string())),
    }
}

fn scalar_type(word: &str) -> Option<ScalarType> {
    let (class, digits) = match word.split_at_checked(1)? {
        ("u", digits) => (ScalarClass::Unsigned, digits),
        ("i", digits) => (ScalarClass::Signed, digits),
        ("f", digits) => (ScalarClass::Float, digits),
        _ => return None,
    };
    let integer = class != ScalarClass::Float;
    let bits = match digits {
        "8" if integer => 8,
        "16" => 16,
        "32" => 32,
        "64" => 64,
        "128" if integer => 128,
        _ => return None,
    };
    Some(ScalarType { class, bits })
}

fn vector_type(word: &str) -> Option<VectorType> {
    let (element, lanes) = word.split_once('x')?;
    let lanes = match lanes {
        "2" => 2,
        "4" => 4,
        "8" => 8,
        "16" => 16,
        "32" => 32,
        "64" => 64,
        _ => return None,
    };
    Some(VectorType {
        element: scalar_type(element)?,
        lanes,
    })
}

enum Operator {
    Token(TokenKind),
    Rejected(Kind),
    PathSeparator,
}

/// The longest operator or punctuation spelling at the start of `rest`, with its length.
fn operator(rest: &[u8]) -> Option<(usize, Operator)> {
    let unsupported = Operator::Rejected(Kind::UnsupportedOperator);
    let range_operator = Operator::Rejected(Kind::RangeOperator);
    let found = match rest {
        [b'.', b'.', b'<', ..] => (3, Operator::Token(TokenKind::DotDotLess)),
        [b'.', b'.', b'.' | b'=', ..] => (3, range_operator),
        [b'.', b'.', ..] => (2, range_operator),
        [b':', b':', ..] => (2, Operator::PathSeparator),
        [b'-', b'>', ..] => (2, Operator::Token(TokenKind::Arrow)),
        [b'=', b'=', ..] => (2, Operator::Token(TokenKind::EqualEqual)),
        [b'!', b'=', ..] => (2, Operator::Token(TokenKind::BangEqual)),
        [b'<', b'=', ..] => (2, Operator::Token(TokenKind::LessEqual)),
        [b'>', b'=', ..] => (2, Operator::Token(TokenKind::GreaterEqual)),
        [b'&', b'&', ..] => (2, Operator::Token(TokenKind::AndAnd)),
        [b'|', b'|', ..] => (2, Operator::Token(TokenKind::OrOr)),
        [b'+', b'+' | b'=', ..]
        | [b'-', b'-' | b'=', ..]
        | [b'*', b'*' | b'=', ..]
        | [b'/' | b'%', b'=', ..]
        | [b'<', b'<', ..]
        | [b'>', b'>', ..] => (2, unsupported),
        [b'|' | b'^' | b'~', ..] => (1, unsupported),
        [b'(', ..] => (1, Operator::Token(TokenKind::LeftParen)),
        [b')', ..] => (1, Operator::Token(TokenKind::RightParen)),
        [b'[', ..] => (1, Operator::Token(TokenKind::LeftBracket)),
        [b']', ..] => (1, Operator::Token(TokenKind::RightBracket)),
        [b'{', ..] => (1, Operator::Token(TokenKind::LeftBrace)),
        [b'}', ..] => (1, Operator::Token(TokenKind::RightBrace)),
        [b',', ..] => (1, Operator::Token(TokenKind::Comma)),
        [b':', ..] => (1, Operator::Token(TokenKind::Colon)),
        [b'.', ..] => (1, Operator::Token(TokenKind::Dot)),
        [b'=', ..] => (1, Operator::Token(TokenKind::Equals)),
        [b'&', ..] => (1, Operator::Token(TokenKind::Ampersand)),
        [b'+', ..] => (1, Operator::Token(TokenKind::Plus)),
        [b'-', ..] => (1, Operator::Token(TokenKind::Minus)),
        [b'*', ..] => (1, Operator::Token(TokenKind::Star)),
        [b'/', ..] => (1, Operator::Token(TokenKind::Slash)),
        [b'%', ..] => (1, Operator::Token(TokenKind::Percent)),
        [b'<', ..] => (1, Operator::Token(TokenKind::Less)),
        [b'>', ..] => (1, Operator::Token(TokenKind::Greater)),
        [b'!', ..] => (1, Operator::Token(TokenKind::Bang)),
        _ => return None,
    };
    Some(found)
}

/// One scanned token: what it is, where it ends, and the faults of its own bytes.
struct Scanned {
    kind: TokenKind,
    end: usize,
    faults: Vec<Fault>,
}

impl Scanned {
    fn plain(kind: TokenKind, end: usize) -> Self {
        Self {
            kind,
            end,
            faults: Vec::new(),
        }
    }

    fn rejected(kind: Kind, message: String, end: usize) -> Self {
        Self {
            kind: TokenKind::Error,
            end,
            faults: vec![fault(kind, message, Vec::new())],
        }
    }
}

struct Lexer<'a> {
    bytes: &'a [u8],
    variant: C1Variant,
    position: usize,
    pending: Vec<Fault>,
    tokens: Vec<Token>,
    comments: Vec<Comment>,
}

impl Lexer<'_> {
    fn run(mut self) -> Lexed {
        loop {
            self.skip_trivia();
            let start = self.position;
            let scanned = self.scan(start);
            let at_end = scanned.kind == TokenKind::Eof;
            let mut faults = std::mem::take(&mut self.pending);
            faults.extend(scanned.faults);
            self.tokens.push(Token {
                kind: scanned.kind,
                range: range(start, scanned.end),
                faults,
            });
            self.position = scanned.end;
            if at_end {
                return Lexed {
                    tokens: self.tokens,
                    comments: self.comments,
                };
            }
        }
    }

    fn byte(&self, position: usize) -> Option<u8> {
        self.bytes.get(position).copied()
    }

    fn text(&self, from: usize, to: usize) -> String {
        let slice = self.bytes.get(from..to).unwrap_or_default();
        String::from_utf8_lossy(slice).into_owned()
    }

    /// The end of the line at `from`: before its line feed, and before the CR of a CRLF.
    fn line_end(&self, from: usize) -> usize {
        let rest = self.bytes.get(from..).unwrap_or_default();
        let length = rest.iter().position(|&byte| byte == b'\n');
        let line_feed = from + length.unwrap_or(rest.len());
        let Some(before) = line_feed.checked_sub(1).filter(|&before| before >= from) else {
            return line_feed;
        };
        let ends_with_crlf =
            (self.byte(before), self.byte(line_feed)) == (Some(b'\r'), Some(b'\n'));
        if ends_with_crlf { before } else { line_feed }
    }

    fn follows_word_byte(&self, position: usize) -> bool {
        position
            .checked_sub(1)
            .and_then(|before| self.byte(before))
            .is_some_and(is_word_byte)
    }

    fn precedes_word_byte(&self, position: usize) -> bool {
        self.byte(position).is_some_and(is_word_byte)
    }

    fn starts_line(&self, position: usize) -> bool {
        let before = self.bytes.get(..position).unwrap_or_default();
        before
            .iter()
            .rev()
            .find(|&&byte| !is_blank_byte(byte))
            .is_none_or(|&byte| byte == b'\n')
    }

    fn rest_of_line_is_blank(&self, from: usize) -> bool {
        let line = self
            .bytes
            .get(from..self.line_end(from))
            .unwrap_or_default();
        line.iter().all(|&byte| is_blank_byte(byte))
    }

    fn skip_trivia(&mut self) {
        while self.consume_trivia() {}
    }

    fn consume_trivia(&mut self) -> bool {
        let Some(byte) = self.byte(self.position) else {
            return false;
        };
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => {
                self.position += 1;
                true
            }
            b'/' => self.consume_slash_trivia(),
            b';' => self.consume_semicolon(),
            b'#' => self.consume_hash(),
            0x80..=0xFF => self.consume_unicode_trivia(),
            _ => false,
        }
    }

    fn consume_slash_trivia(&mut self) -> bool {
        let start = self.position;
        match self.byte(start + 1) {
            Some(b'/') => {
                let end = self.line_end(start);
                self.add_comment(range(start, end), "", start, None);
                true
            }
            Some(b'*') => self.consume_block_comment(),
            _ => false,
        }
    }

    /// Records a comment whose text is `prefix` followed by the bytes from `body_start`, and its
    /// one fault: the `marker` fault, or the comment-character fault, or both merged.
    fn add_comment(
        &mut self,
        comment_range: TextRange,
        prefix: &str,
        body_start: usize,
        marker: Option<Fault>,
    ) {
        let end = comment_range.end;
        let rejected = comment::find(self.bytes, body_start, end);
        let body = comment::text_without(self.bytes, body_start, end, rejected.as_ref());
        let fault = match (marker, rejected) {
            (Some(marker), Some(rejected)) => Some(rejected.merged_into(marker)),
            (Some(marker), None) => Some(marker),
            (None, Some(rejected)) => Some(rejected.fault()),
            (None, None) => None,
        };
        self.pending.extend(fault);
        self.comments.push(Comment {
            range: comment_range,
            text: format!("{prefix}{body}"),
        });
        self.position = end;
    }

    fn invalid_run_end(&self, start: usize) -> usize {
        let mut end = start;
        while let Some(Decoded::Invalid(length)) = decode(self.bytes, end) {
            end += length;
        }
        end
    }

    fn block_comment_close(&self, start: usize) -> Option<usize> {
        let body = self.bytes.get(start + 2..)?;
        let offset = body.windows(2).position(|pair| pair == b"*/")?;
        Some(start + 2 + offset)
    }

    fn block_comment_end(&self, start: usize) -> usize {
        self.block_comment_close(start)
            .map_or(self.bytes.len(), |close| close + 2)
    }

    fn consume_block_comment(&mut self) -> bool {
        let start = self.position;
        let Some(close) = self.block_comment_close(start) else {
            return false;
        };
        let end = close + 2;
        let body = self.bytes.get(start + 2..close).unwrap_or_default();
        let single_line = !body.contains(&b'\n') && self.rest_of_line_is_blank(end);
        if !single_line {
            return false;
        }
        let rejected = comment::find(self.bytes, start + 2, close);
        let body_text = comment::text_without(self.bytes, start + 2, close, rejected.as_ref());
        let replacement = format!("//{body_text}");
        let comment_range = range(start, end);
        let base = "a block comment is not part of the language; use `//`";
        let message = rejected.map_or_else(|| base.to_string(), |found| found.note_to(base));
        let edit = (comment_range, replacement.clone());
        self.pending
            .push(fault(Kind::BlockComment, message, vec![edit]));
        self.comments.push(Comment {
            range: comment_range,
            text: replacement,
        });
        self.position = end;
        true
    }

    fn consume_semicolon(&mut self) -> bool {
        let start = self.position;
        if self.starts_line(start) {
            let mut after = start + 1;
            while self.byte(after) == Some(b';') {
                after += 1;
            }
            let edit = (range(start, after), "//".to_string());
            let message = "`;` is not part of the language; comments start with `//`".to_string();
            let marker = fault(Kind::Semicolon, message, vec![edit]);
            let end = self.line_end(after);
            self.add_comment(range(start, end), "//", after, Some(marker));
            return true;
        }
        if !self.rest_allows_deleting_semicolon(start + 1) {
            return false;
        }
        let edit = (range(start, start + 1), String::new());
        let message = "`;` ends no statement; lines end at the line break".to_string();
        self.pending
            .push(fault(Kind::Semicolon, message, vec![edit]));
        self.position = start + 1;
        true
    }

    fn rest_allows_deleting_semicolon(&self, from: usize) -> bool {
        let mut index = from;
        while self.byte(index).is_some_and(is_blank_byte) {
            index += 1;
        }
        match self.byte(index) {
            None | Some(b'\n' | b'}') => true,
            Some(b'/') => self.byte(index + 1) == Some(b'/'),
            Some(byte) if byte.is_ascii_lowercase() => {
                let word: String = self
                    .bytes
                    .get(index..)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|&&byte| is_word_byte(byte))
                    .map(|&byte| char::from(byte))
                    .collect();
                !matches!(
                    word_kind(&word),
                    TokenKind::Name(_) | TokenKind::ScalarType(_) | TokenKind::VectorType(_)
                )
            }
            Some(_) => false,
        }
    }

    fn consume_hash(&mut self) -> bool {
        let start = self.position;
        let at_line_start = start == 0 || self.byte(start - 1) == Some(b'\n');
        if !at_line_start || self.byte(start + 1) == Some(b'[') {
            return false;
        }
        let edit = (range(start, start + 1), "//".to_string());
        let message = "`#` is not part of the language; comments start with `//`".to_string();
        let marker = fault(Kind::HashComment, message, vec![edit]);
        let end = self.line_end(start);
        self.add_comment(range(start, end), "//", start + 1, Some(marker));
        true
    }

    fn consume_unicode_trivia(&mut self) -> bool {
        let start = self.position;
        let Some(Decoded::Char(character, length)) = decode(self.bytes, start) else {
            return false;
        };
        let replacement = match character {
            lookalike if SPACE_LOOKALIKES.contains(&lookalike) => " ",
            lookalike if ZERO_WIDTH.contains(&lookalike) => "",
            _ => return false,
        };
        self.pending
            .push(space_fault(character, start, length, replacement));
        self.position = start + length;
        true
    }

    fn scan(&self, start: usize) -> Scanned {
        let Some(decoded) = decode(self.bytes, start) else {
            return Scanned::plain(TokenKind::Eof, start);
        };
        match decoded {
            Decoded::Invalid(_) => self.scan_invalid_encoding(start),
            Decoded::Char(character, _) if character.is_ascii() => {
                self.scan_ascii(start, character)
            }
            Decoded::Char(character, length) => self.scan_unicode(start, character, length),
        }
    }

    fn scan_invalid_encoding(&self, start: usize) -> Scanned {
        let end = self.invalid_run_end(start);
        let first = self.byte(start).map_or_else(String::new, byte_name);
        let message = match (end - start).saturating_sub(1) {
            0 => format!("{first} is not UTF-8"),
            more => format!("{first} and {more} more are not UTF-8"),
        };
        Scanned::rejected(Kind::InvalidEncoding, message, end)
    }

    fn scan_ascii(&self, start: usize, character: char) -> Scanned {
        let next_is_digit = self
            .byte(start + 1)
            .is_some_and(|byte| byte.is_ascii_digit());
        match character {
            'a'..='z' | 'A'..='Z' | '_' => self.scan_word(start),
            '0'..='9' => self.scan_number(start),
            '.' if next_is_digit => self.scan_number(start),
            '@' => self.scan_alignment(start),
            '/' if self.byte(start + 1) == Some(b'*') => self.scan_rejected_block_comment(start),
            '#' => self.scan_rejected_hash_comment(start),
            ';' => {
                let message = "`;` is not part of the language".to_string();
                Scanned::rejected(Kind::Semicolon, message, start + 1)
            }
            _ => self.scan_operator(start, character),
        }
    }

    /// A block comment without a repair; its text is not quoted, which may span lines.
    fn scan_rejected_block_comment(&self, start: usize) -> Scanned {
        let end = self.block_comment_end(start);
        let base = "`/*` is a block comment; use `//`";
        let message = comment::find(self.bytes, start + 2, end)
            .map_or_else(|| base.to_string(), |rejected| rejected.note_to(base));
        Scanned::rejected(Kind::BlockComment, message, end)
    }

    /// A `#` line without a repair; its text is not quoted, which may hold a tab.
    fn scan_rejected_hash_comment(&self, start: usize) -> Scanned {
        let end = self.line_end(start);
        let base = "`#` is not a comment; comments start with `//`";
        let message = comment::find(self.bytes, start + 1, end)
            .map_or_else(|| base.to_string(), |rejected| rejected.note_to(base));
        Scanned::rejected(Kind::HashComment, message, end)
    }

    fn scan_operator(&self, start: usize, character: char) -> Scanned {
        let rest = self.bytes.get(start..).unwrap_or_default();
        let Some((length, found)) = operator(rest) else {
            let named = if is_printable_ascii(character) {
                format!("`{character}`")
            } else {
                code_point_name(character)
            };
            let message = format!("{named} begins no token");
            return Scanned::rejected(Kind::InvalidCharacter, message, start + 1);
        };
        let end = start + length;
        let spelling = self.text(start, end);
        match found {
            Operator::Token(kind) => Scanned::plain(kind, end),
            Operator::Rejected(Kind::RangeOperator) => {
                let message = format!("`{spelling}` is not a range; write `lo..<hi`");
                Scanned::rejected(Kind::RangeOperator, message, end)
            }
            Operator::Rejected(kind) => {
                let message = format!("`{spelling}` is not an operator of the language");
                Scanned::rejected(kind, message, end)
            }
            Operator::PathSeparator => {
                let message = "`::` is not a path separator; write `.`".to_string();
                let edit = (range(start, end), ".".to_string());
                Scanned {
                    kind: TokenKind::Dot,
                    end,
                    faults: vec![fault(Kind::PathSeparator, message, vec![edit])],
                }
            }
        }
    }

    fn scan_alignment(&self, start: usize) -> Scanned {
        let digits_start = start + 2;
        let digits_end = (digits_start..self.bytes.len())
            .find(|&index| !self.byte(index).is_some_and(|byte| byte.is_ascii_digit()))
            .unwrap_or(self.bytes.len());
        if self.byte(start + 1) == Some(b'a') && digits_end > digits_start {
            let digits = self.text(digits_start, digits_end);
            return Scanned::plain(TokenKind::Align(digits), digits_end);
        }
        let message = "`@` begins no token unless an alignment `@aN` follows".to_string();
        Scanned::rejected(Kind::InvalidCharacter, message, start + 1)
    }

    fn scan_word(&self, start: usize) -> Scanned {
        let mut end = start;
        let mut word = String::new();
        let mut faults = Vec::new();
        loop {
            if let Some(byte) = self.byte(end).filter(|&byte| is_word_byte(byte)) {
                word.push(char::from(byte));
                end += 1;
                continue;
            }
            let Some((run_end, run_faults)) = self.zero_width_run_inside_word(end) else {
                break;
            };
            faults.extend(run_faults);
            end = run_end;
        }
        let is_identifier_shaped = word.starts_with(|first: char| first.is_ascii_lowercase())
            && word
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !is_identifier_shaped {
            let message = format!(
                "`{word}` is not an identifier: use lowercase letters, digits and `_`, starting with a letter"
            );
            return Scanned::rejected(Kind::InvalidIdentifier, message, end);
        }
        Scanned {
            kind: word_kind(&word),
            end,
            faults,
        }
    }

    /// The zero-width code points at `from` and the end of their run, when a word byte follows.
    fn zero_width_run_inside_word(&self, from: usize) -> Option<(usize, Vec<Fault>)> {
        let mut end = from;
        let mut faults = Vec::new();
        while let Some(Decoded::Char(character, length)) = decode(self.bytes, end) {
            if !ZERO_WIDTH.contains(&character) {
                break;
            }
            faults.push(space_fault(character, end, length, ""));
            end += length;
        }
        (end > from && self.precedes_word_byte(end)).then_some((end, faults))
    }

    fn number_end(&self, start: usize) -> usize {
        let mut end = start;
        loop {
            while self.precedes_word_byte(end) {
                end += 1;
            }
            if let Some((run_end, _)) = self.zero_width_run_inside_word(end) {
                end = run_end;
                continue;
            }
            let after_exponent_letter = end > start && self.byte(end - 1) == Some(b'e');
            let continues = match (self.byte(end), self.byte(end + 1)) {
                (Some(b'.'), Some(next)) => next != b'.',
                (Some(b'.'), None) => true,
                (Some(b'+' | b'-'), _) => after_exponent_letter,
                _ => false,
            };
            if !continues {
                return end;
            }
            end += 1;
        }
    }

    /// The bytes of the number at `start..end` without its zero-width code points, the source
    /// offset of each remaining byte (and of the end), and the fault of each code point dropped.
    fn number_lexeme(&self, start: usize, end: usize) -> (Vec<u8>, Vec<usize>, Vec<Fault>) {
        let mut lexeme = Vec::new();
        let mut offsets = Vec::new();
        let mut faults = Vec::new();
        let mut position = start;
        while position < end {
            if let Some(Decoded::Char(character, length)) = decode(self.bytes, position)
                && ZERO_WIDTH.contains(&character)
            {
                faults.push(space_fault(character, position, length, ""));
                position += length;
                continue;
            }
            lexeme.extend(self.byte(position));
            offsets.push(position);
            position += 1;
        }
        offsets.push(end);
        (lexeme, offsets, faults)
    }

    fn scan_number(&self, start: usize) -> Scanned {
        let end = self.number_end(start);
        let (lexeme, offsets, mut faults) = self.number_lexeme(start, end);
        let spelled = String::from_utf8_lossy(&lexeme).into_owned();
        let mut literal = match number::classify(&lexeme, &offsets) {
            Ok(literal) => literal,
            Err(rejection) => {
                let message = rejection.message(&spelled);
                faults.push(fault(Kind::InvalidNumber, message, Vec::new()));
                return Scanned {
                    kind: TokenKind::Error,
                    end,
                    faults,
                };
            }
        };
        let kind = if literal.is_float {
            TokenKind::Float(literal.spelling.clone())
        } else {
            TokenKind::Int(literal.spelling.clone())
        };
        let starts_with_dot = lexeme.first() == Some(&b'.');
        if let Some(first) = literal
            .edits
            .first_mut()
            .filter(|_| starts_with_dot && self.follows_word_byte(start))
        {
            first.1 = " 0".to_string();
        }
        if literal.edits.is_empty() {
            return Scanned { kind, end, faults };
        }
        let message = format!(
            "`{spelled}` is not a valid number; write `{}`",
            literal.spelling
        );
        faults.push(fault(Kind::InvalidNumber, message, literal.edits));
        faults.sort_by_key(|fault| fault.repair.first().map_or(usize::MAX, |edit| edit.0.start));
        Scanned { kind, end, faults }
    }

    fn scan_unicode(&self, start: usize, character: char, length: usize) -> Scanned {
        let end = start + length;
        match character {
            SIGMA => Scanned::plain(TokenKind::Sum(C1Variant::Unicode), end),
            ELEMENT_OF => Scanned::plain(TokenKind::In(C1Variant::Unicode), end),
            FOR_ALL => Scanned::plain(TokenKind::Forall(C1Variant::Unicode), end),
            ALMOST_EQUAL => Scanned::plain(TokenKind::AlmostEqual, end),
            ELLIPSIS => {
                let message = format!(
                    "{} is not a range; write `lo..<hi`",
                    code_point_name(ELLIPSIS)
                );
                Scanned::rejected(Kind::RangeOperator, message, end)
            }
            SMALL_ELEMENT_OF => self.scan_c1_lookalike(start, character, length, C1Symbol::In),
            TURNED_A => self.scan_c1_lookalike(start, character, length, C1Symbol::Forall),
            lookalike if MEMBERSHIP_LOOKALIKES.contains(&lookalike) => {
                Scanned::plain(TokenKind::MembershipLookalike, end)
            }
            lookalike if SUM_LOOKALIKES.contains(&lookalike) => {
                self.scan_c1_lookalike(start, character, length, C1Symbol::Sum)
            }
            lookalike if UNREPAIRED_LOOKALIKES.contains(&lookalike) => {
                let message = format!(
                    "{} is another symbol, not in the language",
                    code_point_name(character)
                );
                Scanned::rejected(Kind::Lookalike, message, end)
            }
            _ => self.scan_operator_lookalike(start, character, length),
        }
    }

    fn scan_c1_lookalike(
        &self,
        start: usize,
        character: char,
        length: usize,
        symbol: C1Symbol,
    ) -> Scanned {
        let end = start + length;
        let spelling = symbol.spelling(self.variant);
        let replacement = symbol.replacement(self.variant, self.bytes, range(start, end));
        let message = format!("{} looks like `{spelling}`", code_point_name(character));
        Scanned {
            kind: symbol.token(self.variant),
            end,
            faults: vec![fault(
                Kind::Lookalike,
                message,
                vec![(range(start, end), replacement)],
            )],
        }
    }

    fn scan_operator_lookalike(&self, start: usize, character: char, length: usize) -> Scanned {
        let end = start + length;
        let found = OPERATOR_LOOKALIKES
            .iter()
            .find(|(lookalike, _, _)| *lookalike == character);
        let Some((_, kind, spelling)) = found else {
            let message = format!("{} is not ASCII", code_point_name(character));
            return Scanned::rejected(Kind::NonAscii, message, end);
        };
        let message = format!("{} looks like `{spelling}`", code_point_name(character));
        let edit = (range(start, end), (*spelling).to_string());
        Scanned {
            kind: kind.clone(),
            end,
            faults: vec![fault(Kind::Lookalike, message, vec![edit])],
        }
    }
}

fn space_fault(character: char, start: usize, length: usize, replacement: &str) -> Fault {
    let wanted = if replacement.is_empty() {
        "delete it"
    } else {
        "use a space"
    };
    let message = format!(
        "{} is a lookalike of whitespace; {wanted}",
        code_point_name(character)
    );
    let edit = (range(start, start + length), replacement.to_string());
    fault(Kind::Lookalike, message, vec![edit])
}
