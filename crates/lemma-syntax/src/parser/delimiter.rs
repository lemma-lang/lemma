//! Delimiters and the nesting depth: the stack of open `(`, `[` and `{`, the level counter beside
//! it, and the records for a missing or surplus delimiter.

use lemma_diagnostics::Kind;

use super::{Abort, Expectation, ParseResult, Parser, token};
use crate::token::{Keyword, TextRange, TokenKind};

/// The deepest nesting of levels `docs/grammar.md` section 3 accepts.
const NESTING_LIMIT: usize = 16;

const IMPL_ADVICE: &str = "; bind an inner expression to a name with `let`";

/// A kind of delimiter that opens a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delimiter {
    /// `(` and `)`.
    Paren,
    /// `[` and `]`.
    Bracket,
    /// `{` and `}`.
    Brace,
}

impl Delimiter {
    pub(super) fn opener(self) -> TokenKind {
        match self {
            Self::Paren => TokenKind::LeftParen,
            Self::Bracket => TokenKind::LeftBracket,
            Self::Brace => TokenKind::LeftBrace,
        }
    }

    pub(super) fn closer(self) -> TokenKind {
        match self {
            Self::Paren => TokenKind::RightParen,
            Self::Bracket => TokenKind::RightBracket,
            Self::Brace => TokenKind::RightBrace,
        }
    }

    /// The delimiter that `kind` closes, if it is a closing delimiter.
    pub(super) fn closed_by(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::RightParen => Some(Self::Paren),
            TokenKind::RightBracket => Some(Self::Bracket),
            TokenKind::RightBrace => Some(Self::Brace),
            _ => None,
        }
    }

    fn opening_spelling(self) -> &'static str {
        self.opener().spelling().unwrap_or_default()
    }

    fn closing_spelling(self) -> &'static str {
        self.closer().spelling().unwrap_or_default()
    }
}

/// Whether the keyword begins a clause or a statement.
fn begins_clause_or_statement(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::Eff
            | Keyword::Req
            | Keyword::Ens
            | Keyword::Spec
            | Keyword::Equiv
            | Keyword::Impl
            | Keyword::Let
            | Keyword::Loop
            | Keyword::Next
            | Keyword::Return
    )
}

impl Parser<'_> {
    /// Forgets the levels of a function that ended early.
    pub(super) fn reset_nesting(&mut self) {
        self.delimiters.clear();
        self.depth = 0;
        self.in_impl = false;
    }

    /// Opens a level for the token about to be consumed, or ends the function at the limit.
    pub(super) fn enter_level(&mut self) -> ParseResult<()> {
        if self.depth >= NESTING_LIMIT {
            return Err(self.nesting_limit());
        }
        self.depth += 1;
        Ok(())
    }

    pub(super) fn leave_level(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn nesting_limit(&mut self) -> Abort {
        let advice = if self.in_impl { IMPL_ADVICE } else { "" };
        let message = format!("nesting exceeds the limit of {NESTING_LIMIT} levels{advice}");
        let range = self.current().range;
        self.stop(Kind::NestingLimit, message, range)
    }

    /// Consumes the opening token of `delimiter` and opens its level.
    pub(super) fn open(&mut self, delimiter: Delimiter) -> ParseResult<TextRange> {
        if !self.at(&delimiter.opener()) {
            return Err(self.fail(&[token(delimiter.opener())]));
        }
        self.enter_level()?;
        self.delimiters.push(delimiter);
        self.bump()
    }

    /// Consumes the closing token of `delimiter`, or reports what stands in its place:
    /// `expected` says what the grammar allows there.
    pub(super) fn close(
        &mut self,
        delimiter: Delimiter,
        expected: &[Expectation],
    ) -> ParseResult<TextRange> {
        if self.at(&delimiter.closer()) {
            let range = self.bump()?;
            self.delimiters.pop();
            self.leave_level();
            return Ok(range);
        }
        if self.ends_delimiter(delimiter) {
            return self.report_unclosed(delimiter);
        }
        Err(self.fail(expected))
    }

    /// Whether the current token cannot occur inside `delimiter`.
    fn ends_delimiter(&self, delimiter: Delimiter) -> bool {
        match (self.peek(0), delimiter) {
            (TokenKind::Keyword(Keyword::Fn | Keyword::Export) | TokenKind::Eof, _) => true,
            (TokenKind::Keyword(word), Delimiter::Paren | Delimiter::Bracket) => {
                begins_clause_or_statement(*word)
            }
            _ => false,
        }
    }

    /// Records the unclosed `delimiter`; a lone `(` or `[` is closed as if its closer were
    /// written before the current token.
    fn report_unclosed(&mut self, delimiter: Delimiter) -> ParseResult<TextRange> {
        let at = TextRange {
            start: self.previous_end,
            end: self.previous_end,
        };
        let found = match self.peek(0) {
            TokenKind::Eof => "end of input".to_string(),
            _ => format!("`{}`", self.token_text(self.current().range)),
        };
        let opener = delimiter.opening_spelling();
        let open_count = self
            .delimiters
            .iter()
            .filter(|open| **open != Delimiter::Brace)
            .count();
        if delimiter == Delimiter::Brace || open_count != 1 {
            let message = format!("`{opener}` is not closed before {found}");
            return Err(self.stop(Kind::UnclosedDelimiter, message, at));
        }
        let closer = delimiter.closing_spelling();
        let message = format!("`{opener}` is not closed before {found}; write `{closer}`");
        let edit = (at, closer.to_string());
        let record = self.record(Kind::UnclosedDelimiter, message, at, &[edit]);
        self.diagnostics.push(record);
        self.delimiters.pop();
        self.leave_level();
        Ok(at)
    }

    /// Records a comma that ends a list and goes on as if it were deleted.
    pub(super) fn report_trailing_comma(&mut self, comma: TextRange) {
        let message = "a list takes no trailing comma; delete it".to_string();
        let edit = (comma, String::new());
        let record = self.record(Kind::TrailingComma, message, comma, &[edit]);
        self.diagnostics.push(record);
    }

    /// The message for a closing delimiter that is not the innermost open one's closer, or
    /// `None` when it is: then it only stands where the grammar allows something else.
    pub(super) fn unmatched_message(&self, closer: Delimiter) -> Option<String> {
        let spelling = closer.closing_spelling();
        match self.delimiters.last() {
            Some(open) if *open == closer => None,
            Some(open) => Some(format!(
                "`{spelling}` does not close the open `{}`; write `{}`",
                open.opening_spelling(),
                open.closing_spelling()
            )),
            None => Some(format!("`{spelling}` closes nothing: no delimiter is open")),
        }
    }
}
