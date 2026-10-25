//! Classification of one numeric lexeme: valid literal, repairable malformed literal, or neither.

use crate::token::TextRange;

/// A literal spelled as the grammar requires, with the edits that turn the lexeme into it.
pub(super) struct Literal {
    /// The repaired spelling.
    pub spelling: String,
    /// Whether the repaired spelling is a FLOAT rather than an INT.
    pub is_float: bool,
    /// Edits in source offsets; empty when the lexeme was already valid.
    pub edits: Vec<(TextRange, String)>,
}

struct Reader<'a> {
    lexeme: &'a [u8],
    offsets: &'a [usize],
    index: usize,
    spelling: String,
    edits: Vec<(TextRange, String)>,
}

impl Reader<'_> {
    fn peek(&self, ahead: usize) -> Option<u8> {
        self.lexeme.get(self.index + ahead).copied()
    }

    fn keep(&mut self) {
        if let Some(byte) = self.peek(0) {
            self.spelling.push(char::from(byte));
            self.index += 1;
        }
    }

    /// The source offset of lexeme byte `index`, or of the lexeme's end past the last byte.
    fn offset(&self, index: usize) -> usize {
        self.offsets
            .get(index)
            .or_else(|| self.offsets.last())
            .copied()
            .unwrap_or_default()
    }

    fn edit(&mut self, from: usize, to: usize, replacement: &str) {
        let start = self.offset(from);
        let end = if to > from {
            self.offset(to - 1) + 1
        } else {
            start
        };
        self.edits
            .push((TextRange { start, end }, replacement.to_string()));
    }

    fn insert(&mut self, text: &str) {
        self.edit(self.index, self.index, text);
        self.spelling.push_str(text);
    }

    fn skip_one(&mut self) {
        self.edit(self.index, self.index + 1, "");
        self.index += 1;
    }

    /// Takes a run of digits, dropping a `_` that sits between two digits.
    fn take_digits(&mut self) -> usize {
        let mut count = 0;
        loop {
            match (self.peek(0), self.peek(1)) {
                (Some(byte), _) if byte.is_ascii_digit() => {
                    self.keep();
                    count += 1;
                }
                (Some(b'_'), Some(next)) if count > 0 && next.is_ascii_digit() => self.skip_one(),
                _ => return count,
            }
        }
    }

    /// Takes `e`, an optional sign, and digits; a `+` is dropped.
    fn take_exponent(&mut self) -> bool {
        self.keep();
        match self.peek(0) {
            Some(b'-') => self.keep(),
            Some(b'+') => self.skip_one(),
            _ => {}
        }
        self.take_digits() > 0
    }

    /// Drops a type suffix such as `f`, `d`, `u64` or `_f32`; a suffix that follows a bare `.`
    /// replaces it together with the `0` the fraction lacks, as one edit.
    fn drop_suffix(&mut self) -> bool {
        let rest = self.lexeme.get(self.index..).unwrap_or_default();
        if !is_type_suffix(rest) {
            return false;
        }
        let suffix_start = self.offset(self.index);
        let follows_bare_dot = self.index > 0 && self.lexeme.get(self.index - 1) == Some(&b'.');
        let missing_fraction = follows_bare_dot
            && self
                .edits
                .pop_if(|(range, _)| range.start == suffix_start && range.end == suffix_start)
                .is_some();
        if missing_fraction {
            self.edit(self.index - 1, self.lexeme.len(), ".0");
        } else {
            self.edit(self.index, self.lexeme.len(), "");
        }
        self.index = self.lexeme.len();
        true
    }
}

/// Whether `word` spells a type suffix: `f`, `d` or a scalar type name, after an optional `_`.
fn is_type_suffix(word: &[u8]) -> bool {
    let word = word.strip_prefix(b"_").unwrap_or(word);
    if matches!(word, b"f" | b"d") {
        return true;
    }
    std::str::from_utf8(word)
        .ok()
        .and_then(super::scalar_type)
        .is_some()
}

/// Reads `lexeme` as a literal; `offsets` holds the source offset of each of its bytes, then the
/// offset of its end.
///
/// `None` means no single repair makes it one: a leading zero, a hexadecimal literal, or any
/// other malformed text.
pub(super) fn classify(lexeme: &[u8], offsets: &[usize]) -> Option<Literal> {
    let mut reader = Reader {
        lexeme,
        offsets,
        index: 0,
        spelling: String::new(),
        edits: Vec::new(),
    };
    let mut is_float = false;
    let integer_digits = reader.take_digits();
    if integer_digits == 0 {
        reader.insert("0");
    }
    if reader.spelling.len() > 1 && reader.spelling.starts_with('0') {
        return None;
    }
    if reader.peek(0) == Some(b'.') {
        is_float = true;
        reader.keep();
        if reader.take_digits() == 0 {
            reader.insert("0");
        }
    }
    if reader.peek(0) == Some(b'e') {
        if !is_float {
            is_float = true;
            reader.insert(".0");
        }
        if !reader.take_exponent() {
            return None;
        }
    }
    let complete = reader.peek(0).is_none() || reader.drop_suffix();
    complete.then_some(Literal {
        spelling: reader.spelling,
        is_float,
        edits: reader.edits,
    })
}
