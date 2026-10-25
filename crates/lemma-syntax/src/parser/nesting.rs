//! The nesting depth of `docs/grammar.md` section 3, "Nesting depth" (ADR 0010).
//!
//! Every open level sits on one stack, delimiters and unary operators alike, so the depth and the
//! open delimiters are read from the same data and cannot disagree. The stack never holds more
//! than [`NESTING_LIMIT`] levels: opening one more is refused, which also bounds the parser's
//! recursion, since the parser only recurses where a level opens.

use crate::token::TokenKind;

/// The deepest level a function may open.
pub(super) const NESTING_LIMIT: usize = 16;

/// A pair of delimiters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delimiter {
    /// `(` and `)`.
    Parenthesis,
    /// `[` and `]`.
    Bracket,
    /// `{` and `}`.
    Brace,
}

impl Delimiter {
    /// The token that opens the pair.
    pub(super) fn opener(self) -> TokenKind {
        match self {
            Self::Parenthesis => TokenKind::LeftParen,
            Self::Bracket => TokenKind::LeftBracket,
            Self::Brace => TokenKind::LeftBrace,
        }
    }

    /// The token that closes the pair.
    pub(super) fn closer(self) -> TokenKind {
        match self {
            Self::Parenthesis => TokenKind::RightParen,
            Self::Bracket => TokenKind::RightBracket,
            Self::Brace => TokenKind::RightBrace,
        }
    }

    /// The pair `kind` closes, when it is a closing delimiter.
    pub(super) fn closed_by(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::RightParen => Some(Self::Parenthesis),
            TokenKind::RightBracket => Some(Self::Bracket),
            TokenKind::RightBrace => Some(Self::Brace),
            _ => None,
        }
    }

    /// The spelling of the opener.
    pub(super) fn open_spelling(self) -> &'static str {
        match self {
            Self::Parenthesis => "(",
            Self::Bracket => "[",
            Self::Brace => "{",
        }
    }

    /// The spelling of the closer.
    pub(super) fn close_spelling(self) -> &'static str {
        match self {
            Self::Parenthesis => ")",
            Self::Bracket => "]",
            Self::Brace => "}",
        }
    }
}

/// What opened a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Level {
    /// A delimiter, closed by its closer.
    Delimiter(Delimiter),
    /// A unary `-` or `!`, closed after its operand.
    Unary,
}

/// Opening one more level would pass [`NESTING_LIMIT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LimitReached;

/// The levels open at the parser's position, outermost first.
#[derive(Debug, Default)]
pub(super) struct Nesting {
    levels: Vec<Level>,
}

impl Nesting {
    /// Opens `level`, unless the depth is already at the limit.
    pub(super) fn open(&mut self, level: Level) -> Result<(), LimitReached> {
        if self.levels.len() >= NESTING_LIMIT {
            return Err(LimitReached);
        }
        self.levels.push(level);
        Ok(())
    }

    /// Closes the innermost level.
    pub(super) fn close(&mut self) {
        self.levels.pop();
    }

    /// Closes every level: the depth is 0 between items.
    pub(super) fn clear(&mut self) {
        self.levels.clear();
    }

    fn delimiters(&self) -> impl DoubleEndedIterator<Item = Delimiter> + '_ {
        self.levels.iter().filter_map(|level| match level {
            Level::Delimiter(delimiter) => Some(*delimiter),
            Level::Unary => None,
        })
    }

    /// The innermost open delimiter.
    pub(super) fn innermost_delimiter(&self) -> Option<Delimiter> {
        self.delimiters().next_back()
    }

    /// The innermost open `(` or `[`.
    pub(super) fn innermost_parenthesis_or_bracket(&self) -> Option<Delimiter> {
        self.delimiters()
            .rev()
            .find(|delimiter| *delimiter != Delimiter::Brace)
    }

    /// How many `(` and `[` are open.
    pub(super) fn parentheses_and_brackets(&self) -> usize {
        self.delimiters()
            .filter(|delimiter| *delimiter != Delimiter::Brace)
            .count()
    }
}
