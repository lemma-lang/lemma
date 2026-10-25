//! Other languages' words where a Lemma keyword is expected (`foreign-keyword`), and the
//! statements that begin with a name (`assignment`).

use lemma_diagnostics::Kind;

use super::{Abort, ParseResult, Parser};
use crate::token::{Keyword, TextRange, TokenKind};

/// Where the parser expects a keyword another language spells differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    /// At item start, or after `export`.
    Item,
    /// Where a clause is expected.
    Clause,
    /// At statement start.
    Statement,
    /// Right after `let`.
    AfterLet,
}

/// What a foreign word gets in its context.
#[derive(Debug, Clone, Copy)]
enum Rule {
    /// Respelled as the keyword.
    Respell(Keyword),
    /// Respelled as the keyword when a binder `name :` follows; otherwise an ordinary name.
    RespellBeforeBinder(Keyword),
    /// No repair.
    Reject,
    /// No repair when a name follows; otherwise the word is an ordinary name.
    RejectBeforeName,
}

const FOREIGN_WORDS: [(&str, Context, Rule); 22] = [
    ("function", Context::Item, Rule::Respell(Keyword::Fn)),
    ("func", Context::Item, Rule::Respell(Keyword::Fn)),
    ("fun", Context::Item, Rule::Respell(Keyword::Fn)),
    ("def", Context::Item, Rule::Respell(Keyword::Fn)),
    ("pub", Context::Item, Rule::Reject),
    ("extern", Context::Item, Rule::Reject),
    ("requires", Context::Clause, Rule::Respell(Keyword::Req)),
    ("ensures", Context::Clause, Rule::Respell(Keyword::Ens)),
    ("effects", Context::Clause, Rule::Respell(Keyword::Eff)),
    ("ret", Context::Statement, Rule::Respell(Keyword::Return)),
    (
        "var",
        Context::Statement,
        Rule::RespellBeforeBinder(Keyword::Let),
    ),
    (
        "val",
        Context::Statement,
        Rule::RespellBeforeBinder(Keyword::Let),
    ),
    (
        "const",
        Context::Statement,
        Rule::RespellBeforeBinder(Keyword::Let),
    ),
    ("for", Context::Statement, Rule::Reject),
    ("while", Context::Statement, Rule::Reject),
    ("if", Context::Statement, Rule::Reject),
    ("else", Context::Statement, Rule::Reject),
    ("match", Context::Statement, Rule::Reject),
    ("break", Context::Statement, Rule::Reject),
    ("continue", Context::Statement, Rule::Reject),
    ("mut", Context::Statement, Rule::Reject),
    ("mut", Context::AfterLet, Rule::RejectBeforeName),
];

/// What the parser does with a foreign word at the current token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Foreign {
    /// Report it with the keyword as its repair, and read it as that keyword.
    Respell(Keyword),
    /// Report it without repair: the function ends.
    Reject,
}

impl Parser<'_> {
    /// The foreign word at the current token in `context`, if it is one.
    pub(super) fn foreign(&self, context: Context) -> Option<Foreign> {
        let TokenKind::Name(word) = self.peek(0) else {
            return None;
        };
        let (_, _, rule) = FOREIGN_WORDS
            .iter()
            .find(|(spelling, place, _)| spelling == word && *place == context)?;
        let name_follows = matches!(self.peek(1), TokenKind::Name(_));
        let binder_follows = name_follows && self.peek(2) == &TokenKind::Colon;
        match *rule {
            Rule::Respell(keyword) => Some(Foreign::Respell(keyword)),
            Rule::RespellBeforeBinder(keyword) if binder_follows => Some(Foreign::Respell(keyword)),
            Rule::RespellBeforeBinder(_) => None,
            Rule::Reject => Some(Foreign::Reject),
            Rule::RejectBeforeName if name_follows => Some(Foreign::Reject),
            Rule::RejectBeforeName => None,
        }
    }

    /// Consumes the foreign word at the current token, reported with `keyword` as its repair.
    pub(super) fn respell(&mut self, keyword: Keyword) -> ParseResult<TextRange> {
        let word = self.found(self.current());
        let range = self.bump()?;
        let spelling = keyword.as_str();
        let message = format!("{word} is another language's keyword; write `{spelling}`");
        let record = self.record(
            Kind::ForeignKeyword,
            message,
            range,
            &[(range, spelling.to_string())],
        );
        self.diagnostics.push(record);
        Ok(range)
    }

    /// Ends the function at the foreign word at the current token, which has no repair.
    pub(super) fn reject_foreign(&mut self) -> Abort {
        let current = self.current();
        let range = current.range;
        let message = format!(
            "{} is another language's keyword, not part of Lemma",
            self.found(current)
        );
        self.stop(Kind::ForeignKeyword, message, range)
    }

    /// Consumes the foreign word at the current token in `context`; `Ok(None)` when there is
    /// none, the keyword it reads as when it has a repair.
    pub(super) fn consume_foreign(&mut self, context: Context) -> ParseResult<Option<Keyword>> {
        match self.foreign(context) {
            Some(Foreign::Respell(keyword)) => self.respell(keyword).map(|_| Some(keyword)),
            Some(Foreign::Reject) => Err(self.reject_foreign()),
            None => Ok(None),
        }
    }

    /// Ends the function at the name that begins the current statement.
    pub(super) fn assignment(&mut self) -> Abort {
        let current = self.current();
        let range = current.range;
        let message = format!(
            "a statement cannot begin with {}; a value is defined once, by `let`, a carry or `next`",
            self.found(current)
        );
        self.stop(Kind::Assignment, message, range)
    }
}
