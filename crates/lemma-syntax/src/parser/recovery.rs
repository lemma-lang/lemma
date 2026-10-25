//! What the parser does where the text leaves the grammar (`docs/grammar.md` section 7,
//! "Recovery"): the record reported at the token it stops at, the closing delimiter a repair
//! inserts, the levels it opens and closes, and the token parsing resumes at.

use lemma_diagnostics::Kind;

use super::nesting::{Delimiter, Level, NESTING_LIMIT};
use super::{A_NAME, Abort, Expectation, ParseResult, Parser, expectation_list, token};
use crate::token::{Keyword, TextRange, Token, TokenKind, code_point_name, is_printable_ascii};

/// What the `nesting-limit` message adds in the impl, where `let` exists.
const IMPL_NESTING_ADVICE: &str = "; bind an inner expression to a name with `let`";

/// A token that cannot occur inside a `(` or a `[`: a clause or statement keyword, an item start
/// or the end of input.
fn ends_delimited_text(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Eof
            | TokenKind::Keyword(
                Keyword::Fn
                    | Keyword::Export
                    | Keyword::Eff
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
    )
}

/// A token that cannot occur inside a `{`: an item start or the end of input.
fn ends_block(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Eof | TokenKind::Keyword(Keyword::Fn | Keyword::Export)
    )
}

/// What a stopping parser reports, unless the token it stops at has an unrepaired fault.
struct StopRecord {
    kind: Kind,
    message: String,
    range: TextRange,
}

impl Parser<'_> {
    /// The token's spelling as the parser reads it, repairs applied, in backticks; `end of
    /// input` at the end. A spelling outside printable ASCII is named by its first code point
    /// instead, without backticks, unless it is the active C1 variant's symbol.
    pub(super) fn found(&self, token: &Token) -> String {
        let spelling = match &token.kind {
            TokenKind::Eof => return "end of input".to_string(),
            TokenKind::Name(text) | TokenKind::Int(text) | TokenKind::Float(text) => text.clone(),
            TokenKind::Align(digits) => format!("@a{digits}"),
            TokenKind::ScalarType(scalar) => scalar.to_string(),
            TokenKind::VectorType(vector) => vector.to_string(),
            kind => kind
                .spelling()
                .map_or_else(|| self.token_text(token.range), str::to_string),
        };
        let active_symbol = token
            .kind
            .c1_symbol()
            .is_some_and(|(_, spelled)| spelled == self.variant);
        if active_symbol || spelling.chars().all(is_printable_ascii) {
            return format!("`{spelling}`");
        }
        spelling
            .chars()
            .next()
            .map_or_else(String::new, code_point_name)
    }

    /// The code point of a one-code-point token, named by its number.
    pub(super) fn code_point(&self, range: TextRange) -> String {
        let text = self.token_text(range);
        code_point_name(text.chars().next().unwrap_or_default())
    }

    /// Ends the function at the current token: its faults are reported, then `record` unless
    /// one of those faults already has no repair.
    pub(super) fn stop(&mut self, kind: Kind, message: String, range: TextRange) -> Abort {
        if self.report_faults() {
            return Abort;
        }
        let record = self.record(kind, message, range, &[]);
        self.diagnostics.push(record);
        Abort
    }

    /// Ends the function at the current token, where the grammar allows only `expected`.
    pub(super) fn fail(&mut self, expected: &[Expectation]) -> Abort {
        let StopRecord {
            kind,
            message,
            range,
        } = self.stop_record(expected);
        self.stop(kind, message, range)
    }

    fn stop_record(&self, expected: &[Expectation]) -> StopRecord {
        let current = self.current();
        if expected.contains(&A_NAME) && current.kind.is_reserved() {
            let message = format!(
                "{} is reserved and cannot be used as a name",
                self.found(current)
            );
            return self.stop_record_of(Kind::ReservedWord, message, current.range);
        }
        if let Some(open) = self.unclosed_at(&current.kind) {
            return self.unclosed(open, None);
        }
        if let Some(closed) = Delimiter::closed_by(&current.kind)
            && self.nesting.innermost_delimiter() != Some(closed)
        {
            return self.unmatched(closed, current.range);
        }
        match current.kind {
            TokenKind::Eof => self.unexpected(Kind::UnexpectedEnd, expected),
            TokenKind::AlmostEqual => {
                let message = format!(
                    "{} is not an operator of the language",
                    self.code_point(current.range)
                );
                self.stop_record_of(Kind::Lookalike, message, current.range)
            }
            TokenKind::MembershipLookalike => {
                let message = format!("{} is not ASCII", self.code_point(current.range));
                self.stop_record_of(Kind::NonAscii, message, current.range)
            }
            _ => self.unexpected(Kind::UnexpectedToken, expected),
        }
    }

    fn stop_record_of(&self, kind: Kind, message: String, range: TextRange) -> StopRecord {
        StopRecord {
            kind,
            message,
            range,
        }
    }

    fn unexpected(&self, kind: Kind, expected: &[Expectation]) -> StopRecord {
        let current = self.current();
        let message = format!(
            "unexpected {}, expected {}",
            self.found(current),
            expectation_list(expected)
        );
        self.stop_record_of(kind, message, current.range)
    }

    /// The open delimiter a token of `kind` cannot occur inside, when there is one.
    fn unclosed_at(&self, kind: &TokenKind) -> Option<Delimiter> {
        if ends_block(kind) {
            return self.nesting.innermost_delimiter();
        }
        if ends_delimited_text(kind) {
            return self.nesting.innermost_parenthesis_or_bracket();
        }
        None
    }

    /// The `unclosed-delimiter` record for `open`, at the end of the last token read; `inserted`
    /// is the closer its repair inserts. The message counts the other delimiters the current token
    /// cannot occur inside: every open one before an item start or the end, else the open `(` and
    /// `[`.
    fn unclosed(&self, open: Delimiter, inserted: Option<Delimiter>) -> StopRecord {
        let current = self.current();
        let before = self.found(current);
        let insertion = inserted.map_or_else(String::new, |closer| {
            format!("; insert `{}`", closer.close_spelling())
        });
        let candidates = if ends_block(&current.kind) {
            self.nesting.delimiter_count()
        } else {
            self.nesting.parentheses_and_brackets()
        };
        let opened = match candidates.saturating_sub(1) {
            0 => format!("`{}` is", open.open_spelling()),
            more => format!("`{}` and {more} more are", open.open_spelling()),
        };
        let message = format!("{opened} not closed before {before}{insertion}");
        let range = TextRange {
            start: self.previous_end,
            end: self.previous_end,
        };
        self.stop_record_of(Kind::UnclosedDelimiter, message, range)
    }

    fn unmatched(&self, closed: Delimiter, range: TextRange) -> StopRecord {
        let message = match self.nesting.innermost_delimiter() {
            Some(open) => format!(
                "`{}` does not close the open `{}`",
                closed.close_spelling(),
                open.open_spelling()
            ),
            None => format!("`{}` closes nothing", closed.close_spelling()),
        };
        self.stop_record_of(Kind::UnmatchedDelimiter, message, range)
    }

    /// Opens `level` at the current token and consumes it; the token opening a level past the
    /// limit is a `nesting-limit` record, checked before anything inside the level is read.
    fn enter(&mut self, level: Level) -> ParseResult<TextRange> {
        let range = self.current().range;
        if self.nesting.open(level).is_err() {
            let advice = if self.in_impl {
                IMPL_NESTING_ADVICE
            } else {
                ""
            };
            let message = format!("nesting exceeds the limit of {NESTING_LIMIT} levels{advice}");
            return Err(self.stop(Kind::NestingLimit, message, range));
        }
        self.bump()
    }

    /// Closes the innermost level, which must be `expected`; any other level is a parser bug,
    /// reported as an `internal-error` that ends the function.
    fn leave(&mut self, expected: Level) -> ParseResult<()> {
        if self.nesting.close() == Some(expected) {
            return Ok(());
        }
        let message = "the parser closed a nesting level other than the one it opened".to_string();
        let record = self.internal_error(message);
        self.diagnostics.push(record);
        Err(Abort)
    }

    /// Parses the operand of the unary operator at the current token, one level deeper.
    pub(super) fn unary_level<T>(
        &mut self,
        operand: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        self.enter(Level::Unary)?;
        let inner = operand(self)?;
        self.leave(Level::Unary)?;
        Ok(inner)
    }

    /// Parses `body` between `delimiter`'s opener and closer, one level deeper; `also` lists
    /// what the body accepts where the closer is missing.
    pub(super) fn delimited<T>(
        &mut self,
        delimiter: Delimiter,
        also: &[TokenKind],
        body: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let opener = delimiter.opener();
        if !self.at(&opener) {
            return Err(self.fail(&[token(opener)]));
        }
        self.enter(Level::Delimiter(delimiter))?;
        let inner = body(self)?;
        self.close_delimiter(delimiter, also)?;
        Ok(inner)
    }

    /// Parses `item ("," item)*` between `delimiter`'s opener and closer, or nothing when the
    /// list may be empty.
    pub(super) fn delimited_list<T>(
        &mut self,
        delimiter: Delimiter,
        may_be_empty: bool,
        item: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let closer = delimiter.closer();
        self.delimited(delimiter, &[TokenKind::Comma], |parser| {
            parser.list_items(&closer, may_be_empty, item)
        })
    }

    /// The items of a list up to its closer, which is left in place; a comma before the closer
    /// is a `trailing-comma`, deleted.
    fn list_items<T>(
        &mut self,
        closer: &TokenKind,
        may_be_empty: bool,
        mut item: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        if may_be_empty && self.at(closer) {
            return Ok(items);
        }
        loop {
            items.push(item(self)?);
            let Some(comma) = self.eat(&TokenKind::Comma)? else {
                return Ok(items);
            };
            if self.at(closer) {
                let message = "a list takes no trailing comma".to_string();
                let record = self.record(
                    Kind::TrailingComma,
                    message,
                    comma,
                    &[(comma, String::new())],
                );
                self.diagnostics.push(record);
                return Ok(items);
            }
        }
    }

    /// Consumes `delimiter`'s closer. When it is missing before a token that cannot occur inside
    /// it, and it is the only `(` or `[` open, the repair inserts it and the parse goes on as if
    /// it were written.
    fn close_delimiter(&mut self, delimiter: Delimiter, also: &[TokenKind]) -> ParseResult<()> {
        let closer = delimiter.closer();
        if self.at(&closer) {
            self.bump()?;
            return self.leave(Level::Delimiter(delimiter));
        }
        let insertable = delimiter != Delimiter::Brace
            && ends_delimited_text(self.peek(0))
            && self.nesting.parentheses_and_brackets() == 1;
        if !insertable {
            let expected: Vec<Expectation> = std::iter::once(closer)
                .chain(also.iter().cloned())
                .map(token)
                .collect();
            return Err(self.fail(&expected));
        }
        let StopRecord {
            kind,
            message,
            range,
        } = self.unclosed(delimiter, Some(delimiter));
        let insertion = (range, delimiter.close_spelling().to_string());
        let record = self.record(kind, message, range, &[insertion]);
        self.diagnostics.push(record);
        self.leave(Level::Delimiter(delimiter))
    }

    /// Skips to the next `fn` or `export` after the item that began at token `start`, or to the
    /// end of input; the token the function stopped at is kept when it is one of them.
    pub(super) fn resynchronize(&mut self, start: usize) {
        self.nesting.clear();
        let last = self.tokens.len().saturating_sub(1);
        let mut position = self.position.max(start + 1).min(last);
        while let Some(skipped) = self.tokens.get(position).filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Keyword(Keyword::Fn | Keyword::Export) | TokenKind::Eof
            )
        }) {
            self.previous_end = skipped.range.end;
            position += 1;
        }
        self.position = position.min(last);
    }
}
