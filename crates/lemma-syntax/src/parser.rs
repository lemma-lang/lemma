//! The parser of `docs/grammar.md` section 3: tokens to the syntax tree, with the diagnostics of
//! the lexer's faults and of the parser's own findings.
//!
//! A diagnostic with a repair is reported and parsing goes on as if the repair were applied. The
//! first diagnostic without one ends the function being parsed; parsing resumes at the next `fn`
//! or `export`.

mod expression;
mod signature;
mod statement;

use lemma_diagnostics::{Diagnostic, Edit, Kind, Position, Source, Span, SpanError};

use crate::ast::{Clause, EffClause, Effect, EquivClause, File, Function, Name, Permission};
use crate::lexer::lex;
use crate::token::{C1Symbol, C1Variant, Comment, Fault, Keyword, TextRange, Token, TokenKind};

/// What `parse` found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The functions parsed to their end, repairs applied as if made in the text.
    pub file: File,
    /// Every comment of the source, in order.
    pub comments: Vec<Comment>,
    /// The syntax-class diagnostics, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses `source` under the given C1 variant.
///
/// The parser never fails as a whole: input it cannot read yields diagnostics and the functions
/// that could be parsed.
pub fn parse(source: Source<'_>, variant: C1Variant) -> Parsed {
    let lexed = lex(source.text, variant);
    let mut parser = Parser {
        source,
        variant,
        tokens: lexed.tokens,
        position: 0,
        previous_end: 0,
        diagnostics: Vec::new(),
    };
    let file = parser.parse_file();
    Parsed {
        file,
        comments: lexed.comments,
        diagnostics: parser.diagnostics,
    }
}

/// The function being parsed is over: its diagnostic is already recorded.
struct Abort;

type ParseResult<T> = Result<T, Abort>;

/// What the grammar allows at a place where the parser found something else.
enum Expectation {
    /// A token with a fixed spelling.
    Token(TokenKind),
    /// A class of tokens, named in words.
    Class(&'static str),
}

const A_NAME: Expectation = Expectation::Class("a name");
const A_TYPE: Expectation = Expectation::Class("a type");
const AN_ALIGNMENT: Expectation = Expectation::Class("an alignment");
const AN_EXPRESSION: Expectation = Expectation::Class("an expression");
const AN_INTEGER_LITERAL: Expectation = Expectation::Class("an integer literal");

static END_OF_INPUT: Token = Token {
    kind: TokenKind::Eof,
    range: TextRange { start: 0, end: 0 },
    faults: Vec::new(),
};

struct Parser<'a> {
    source: Source<'a>,
    variant: C1Variant,
    tokens: Vec<Token>,
    position: usize,
    previous_end: usize,
    diagnostics: Vec<Diagnostic>,
}

fn keyword(keyword: Keyword) -> TokenKind {
    TokenKind::Keyword(keyword)
}

fn token(kind: TokenKind) -> Expectation {
    Expectation::Token(kind)
}

/// The spellings and classes joined as `x`, `x or y`, `x, y or z`: spellings in backticks in byte
/// order, then classes in alphabetical order.
fn expectation_list(expected: &[Expectation]) -> String {
    let mut spelled: Vec<&str> = expected
        .iter()
        .filter_map(|expectation| match expectation {
            Expectation::Token(kind) => kind.spelling(),
            Expectation::Class(_) => None,
        })
        .collect();
    let mut classes: Vec<&str> = expected
        .iter()
        .filter_map(|expectation| match expectation {
            Expectation::Class(class) => Some(*class),
            Expectation::Token(_) => None,
        })
        .collect();
    spelled.sort_unstable();
    spelled.dedup();
    classes.sort_unstable();
    classes.dedup();
    let mut items: Vec<String> = spelled
        .into_iter()
        .map(|spelling| format!("`{spelling}`"))
        .collect();
    items.extend(classes.into_iter().map(str::to_string));
    let Some(last) = items.pop() else {
        return String::new();
    };
    if items.is_empty() {
        return last;
    }
    format!("{} or {last}", items.join(", "))
}

/// The range a fault is reported on: the one it names, else the token when the repair edits it,
/// else the edited text.
fn fault_range(fault: &Fault, token_range: TextRange) -> TextRange {
    if let Some(span) = fault.span {
        return span;
    }
    let start = fault.repair.iter().map(|edit| edit.0.start).min();
    let end = fault.repair.iter().map(|edit| edit.0.end).max();
    match (start, end) {
        (Some(start), Some(end)) if start < token_range.start || end > token_range.end => {
            TextRange { start, end }
        }
        _ => token_range,
    }
}

impl Parser<'_> {
    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&END_OF_INPUT)
    }

    fn peek(&self, ahead: usize) -> &TokenKind {
        self.tokens
            .get(self.position + ahead)
            .or_else(|| self.tokens.last())
            .map_or(&END_OF_INPUT.kind, |token| &token.kind)
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek(0) == kind
    }

    fn at_keyword(&self, wanted: Keyword) -> bool {
        self.at(&keyword(wanted))
    }

    fn at_end(&self) -> bool {
        self.at(&TokenKind::Eof)
    }

    fn range_from(&self, start: usize) -> TextRange {
        TextRange {
            start,
            end: self.previous_end,
        }
    }

    fn start(&self) -> usize {
        self.current().range.start
    }

    fn token_text(&self, range: TextRange) -> String {
        let bytes = self
            .source
            .text
            .get(range.start..range.end)
            .unwrap_or_default();
        String::from_utf8_lossy(bytes).into_owned()
    }

    fn record(
        &self,
        kind: Kind,
        message: String,
        range: TextRange,
        edits: &[(TextRange, String)],
    ) -> Diagnostic {
        self.build_record(kind, &message, range, edits)
            .unwrap_or_else(|error| self.internal_error(&error))
    }

    fn build_record(
        &self,
        kind: Kind,
        message: &str,
        range: TextRange,
        edits: &[(TextRange, String)],
    ) -> Result<Diagnostic, SpanError> {
        let repair = edits
            .iter()
            .map(|(edit_range, replacement)| {
                Ok(Edit {
                    span: self.source.span(edit_range.start..edit_range.end)?,
                    replacement: replacement.clone(),
                })
            })
            .collect::<Result<Vec<_>, SpanError>>()?;
        Ok(Diagnostic {
            kind,
            message: message.to_string(),
            span: self.source.span(range.start..range.end)?,
            related: Vec::new(),
            obligation: None,
            counterexample: None,
            repair,
        })
    }

    fn internal_error(&self, error: &SpanError) -> Diagnostic {
        let origin = Position {
            byte: 0,
            line: 1,
            column: 1,
        };
        Diagnostic {
            kind: Kind::InternalError,
            message: format!("the parser built a range outside its source: {error}"),
            span: Span {
                file: self.source.name.to_string(),
                start: origin,
                end: origin,
            },
            related: Vec::new(),
            obligation: None,
            counterexample: None,
            repair: Vec::new(),
        }
    }

    /// Reports the faults of the current token; `true` when one of them has no repair.
    fn report_faults(&mut self) -> bool {
        let position = self.position;
        let Some(token) = self.tokens.get_mut(position) else {
            return false;
        };
        let faults = std::mem::take(&mut token.faults);
        let token_range = token.range;
        let mut unrepaired = false;
        for fault in faults {
            unrepaired |= fault.repair.is_empty();
            let range = fault_range(&fault, token_range);
            let record = self.record(fault.kind, fault.message, range, &fault.repair);
            self.diagnostics.push(record);
        }
        unrepaired
    }

    /// Consumes the current token, reporting its faults; the end of input is never passed.
    fn bump(&mut self) -> ParseResult<TextRange> {
        let range = self.current().range;
        let unrepaired = self.report_faults();
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
            self.previous_end = range.end;
        }
        if unrepaired { Err(Abort) } else { Ok(range) }
    }

    fn eat(&mut self, kind: &TokenKind) -> ParseResult<Option<TextRange>> {
        if self.at(kind) {
            return self.bump().map(Some);
        }
        Ok(None)
    }

    fn consume(&mut self, kind: TokenKind) -> ParseResult<TextRange> {
        if self.at(&kind) {
            return self.bump();
        }
        Err(self.fail(&[token(kind)]))
    }

    fn consume_keyword(&mut self, wanted: Keyword) -> ParseResult<TextRange> {
        self.consume(keyword(wanted))
    }

    fn parse_name(&mut self) -> ParseResult<Name> {
        let TokenKind::Name(text) = self.peek(0) else {
            return Err(self.fail(&[A_NAME]));
        };
        let text = text.clone();
        let range = self.bump()?;
        Ok(Name { text, range })
    }

    fn parse_integer(&mut self) -> ParseResult<String> {
        let TokenKind::Int(spelling) = self.peek(0) else {
            return Err(self.fail(&[AN_INTEGER_LITERAL]));
        };
        let spelling = spelling.clone();
        self.bump()?;
        Ok(spelling)
    }

    /// Consumes the C1 symbol `symbol` in either spelling; the other variant's is reported with
    /// the active spelling as its repair.
    fn consume_c1(&mut self, symbol: C1Symbol) -> ParseResult<TextRange> {
        let spelled = match self.peek(0).c1_symbol() {
            Some((found, spelled)) if found == symbol => spelled,
            _ => return Err(self.fail(&[token(symbol.token(self.variant))])),
        };
        let range = self.bump()?;
        if spelled != self.variant {
            let found = self.token_text(range);
            let active = symbol.spelling(self.variant);
            let replacement = symbol.replacement(self.variant, self.source.text, range);
            let message = format!("`{found}` is the other C1 variant's spelling; write `{active}`");
            let record = self.record(
                Kind::C1OtherVariant,
                message,
                range,
                &[(range, replacement)],
            );
            self.diagnostics.push(record);
        }
        Ok(range)
    }

    /// Parses `item ("," item)*` and the `close` token, or `close` alone when the list may be
    /// empty. The opening token is already consumed.
    fn parse_list<T>(
        &mut self,
        close: TokenKind,
        may_be_empty: bool,
        mut item: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        if may_be_empty && self.at(&close) {
            self.bump()?;
            return Ok(items);
        }
        loop {
            items.push(item(self)?);
            if self.eat(&TokenKind::Comma)?.is_none() {
                break;
            }
        }
        if !self.at(&close) {
            return Err(self.fail(&[token(TokenKind::Comma), token(close)]));
        }
        self.bump()?;
        Ok(items)
    }

    /// Records the unexpected token, or the fault that ended the function, and returns the abort.
    fn fail(&mut self, expected: &[Expectation]) -> Abort {
        let unrepaired = self.report_faults();
        let current = self.current();
        let (kind, range) = (current.kind.clone(), current.range);
        if unrepaired || kind == TokenKind::Error {
            return Abort;
        }
        let at_end = kind == TokenKind::Eof;
        let found = if at_end {
            "end of input".to_string()
        } else {
            format!("`{}`", self.token_text(range))
        };
        let message = format!(
            "unexpected {found}, expected {}",
            expectation_list(expected)
        );
        let kind = if at_end {
            Kind::UnexpectedEnd
        } else {
            Kind::UnexpectedToken
        };
        let record = self.record(kind, message, range, &[]);
        self.diagnostics.push(record);
        Abort
    }

    fn parse_file(&mut self) -> File {
        let mut items = Vec::new();
        while !self.at_end() {
            let start = self.position;
            match self.parse_function() {
                Ok(function) => items.push(function),
                Err(Abort) => self.resynchronize(start),
            }
        }
        self.report_faults();
        File {
            items,
            range: TextRange {
                start: 0,
                end: self.source.text.len(),
            },
        }
    }

    /// Skips to the next `fn` or `export` after the item that began at token `start`.
    fn resynchronize(&mut self, start: usize) {
        let last = self.tokens.len().saturating_sub(1);
        self.position = self.position.max(start + 1).min(last);
        while !matches!(
            self.peek(0),
            TokenKind::Keyword(Keyword::Fn | Keyword::Export) | TokenKind::Eof
        ) {
            self.position += 1;
        }
    }

    fn parse_function(&mut self) -> ParseResult<Function> {
        let start = self.start();
        let export = self.eat(&keyword(Keyword::Export))?;
        if !self.at_keyword(Keyword::Fn) {
            let mut expected = vec![token(keyword(Keyword::Fn))];
            if export.is_none() {
                expected.push(token(keyword(Keyword::Export)));
            }
            return Err(self.fail(&expected));
        }
        self.bump()?;
        let name = self.parse_name()?;
        self.consume(TokenKind::LeftParen)?;
        let params = self.parse_list(TokenKind::RightParen, true, Self::parse_param)?;
        self.consume(TokenKind::Arrow)?;
        let result = self.parse_signature_type()?;
        let effects = self.parse_effects()?;
        let requires = self.parse_clauses(Keyword::Req)?;
        let ensures = self.parse_clauses(Keyword::Ens)?;
        let spec = self.parse_clause(Keyword::Spec)?;
        let equiv = self.parse_equiv()?;
        self.consume_keyword(Keyword::Impl)?;
        let body = self.parse_block()?;
        Ok(Function {
            export,
            name,
            params,
            result,
            effects,
            requires,
            ensures,
            spec,
            equiv,
            body,
            range: self.range_from(start),
        })
    }

    fn parse_effects(&mut self) -> ParseResult<EffClause> {
        let start = self.start();
        self.consume_keyword(Keyword::Eff)?;
        let mut effects = Vec::new();
        loop {
            self.consume_keyword(Keyword::Pure)?;
            effects.push(Effect::Pure);
            if self.eat(&TokenKind::Comma)?.is_none() {
                break;
            }
        }
        Ok(EffClause {
            effects,
            range: self.range_from(start),
        })
    }

    fn parse_clauses(&mut self, wanted: Keyword) -> ParseResult<Vec<Clause>> {
        let mut clauses = Vec::new();
        while self.at_keyword(wanted) {
            clauses.push(self.parse_clause(wanted)?);
        }
        Ok(clauses)
    }

    fn parse_clause(&mut self, wanted: Keyword) -> ParseResult<Clause> {
        let start = self.start();
        self.consume_keyword(wanted)?;
        let expression = self.parse_expression()?;
        Ok(Clause {
            expression,
            range: self.range_from(start),
        })
    }

    fn parse_equiv(&mut self) -> ParseResult<EquivClause> {
        let start = self.start();
        self.consume_keyword(Keyword::Equiv)?;
        self.consume_keyword(Keyword::Allow)?;
        self.consume(TokenKind::LeftParen)?;
        let mut permissions =
            self.parse_list(TokenKind::RightParen, true, Self::parse_permission)?;
        permissions.sort();
        Ok(EquivClause {
            permissions,
            range: self.range_from(start),
        })
    }

    fn parse_permission(&mut self) -> ParseResult<Permission> {
        let permission = match self.peek(0) {
            TokenKind::Keyword(Keyword::FpReassoc) => Permission::FpReassoc,
            TokenKind::Keyword(Keyword::FpContract) => Permission::FpContract,
            _ => {
                let expected = [
                    token(keyword(Keyword::FpReassoc)),
                    token(keyword(Keyword::FpContract)),
                ];
                return Err(self.fail(&expected));
            }
        };
        self.bump()?;
        Ok(permission)
    }
}
