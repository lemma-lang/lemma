//! Words of other languages where a Lemma keyword is expected, and the tokens a repair rewrites
//! in place so that parsing goes on as if the repair were applied.

use lemma_diagnostics::Kind;

use super::{Abort, ParseResult, Parser};
use crate::token::{Fault, Keyword, TokenKind};

/// Where the parser expects a keyword that another language spells differently.
#[derive(Debug, Clone, Copy)]
pub(super) enum Place {
    /// The start of an item, or after `export`.
    Item,
    /// Where a clause begins.
    Clause,
    /// The start of a statement.
    Statement,
    /// After `let`.
    Binder,
}

/// What the parser does with a foreign word.
enum Foreign {
    /// Reads it as this keyword and reports the repair.
    Repair(Keyword),
    /// Ends the function: no keyword stands for it.
    Reject,
}

/// Whether `word` is a foreign word at `place`, given the two tokens after it.
fn foreign_word(place: Place, word: &str, ahead: (&TokenKind, &TokenKind)) -> Option<Foreign> {
    match (place, word) {
        (Place::Item, "function" | "func" | "fun" | "def") => Some(Foreign::Repair(Keyword::Fn)),
        (Place::Item, "pub" | "extern") => Some(Foreign::Reject),
        (Place::Clause, "requires") => Some(Foreign::Repair(Keyword::Req)),
        (Place::Clause, "ensures") => Some(Foreign::Repair(Keyword::Ens)),
        (Place::Clause, "effects") => Some(Foreign::Repair(Keyword::Eff)),
        (Place::Statement, "ret") => Some(Foreign::Repair(Keyword::Return)),
        (Place::Statement, "var" | "val" | "const")
            if matches!(ahead, (TokenKind::Name(_), TokenKind::Colon)) =>
        {
            Some(Foreign::Repair(Keyword::Let))
        }
        (
            Place::Statement,
            "for" | "while" | "if" | "else" | "match" | "break" | "continue" | "mut",
        ) => Some(Foreign::Reject),
        (Place::Binder, "mut") if matches!(ahead.0, TokenKind::Name(_)) => Some(Foreign::Reject),
        _ => None,
    }
}

impl Parser<'_> {
    /// Rewrites the current token in place as `kind`, attaching the fault whose repair writes
    /// `replacement` over it.
    pub(super) fn repair_current(
        &mut self,
        kind: TokenKind,
        fault_kind: Kind,
        message: String,
        replacement: String,
    ) {
        let position = self.position;
        let Some(token) = self.tokens.get_mut(position) else {
            return;
        };
        token.kind = kind;
        token.faults.push(Fault {
            kind: fault_kind,
            message,
            repair: vec![(token.range, replacement)],
            span: None,
        });
    }

    /// Reads a foreign word at `place` as the keyword it stands for, or ends the function when
    /// no keyword does.
    pub(super) fn repair_foreign_word(&mut self, place: Place) -> ParseResult<()> {
        let TokenKind::Name(word) = self.peek(0) else {
            return Ok(());
        };
        let Some(foreign) = foreign_word(place, word, (self.peek(1), self.peek(2))) else {
            return Ok(());
        };
        let word = word.clone();
        match foreign {
            Foreign::Repair(replacement) => {
                let spelling = replacement.as_str();
                let message = format!("`{word}` is not a Lemma keyword; write `{spelling}`");
                let kind = TokenKind::Keyword(replacement);
                self.repair_current(kind, Kind::ForeignKeyword, message, spelling.to_string());
                Ok(())
            }
            Foreign::Reject => Err(self.reject_foreign_word(&word)),
        }
    }

    fn reject_foreign_word(&mut self, word: &str) -> Abort {
        let message = format!("`{word}` is another language's keyword, which Lemma has no use for");
        let range = self.current().range;
        self.stop(Kind::ForeignKeyword, message, range)
    }
}
