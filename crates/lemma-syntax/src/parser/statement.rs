//! Blocks, statements and binders.

use super::delimiter::Delimiter;
use super::foreign::Place;
use super::{Abort, Expectation, ParseResult, Parser, keyword, token};
use crate::ast::{Binder, Block, Carry, Statement, StatementKind};
use crate::token::{Keyword, TokenKind};
use lemma_diagnostics::Kind;

fn statement_expectations() -> [Expectation; 5] {
    [
        token(keyword(Keyword::Let)),
        token(keyword(Keyword::Loop)),
        token(keyword(Keyword::Next)),
        token(keyword(Keyword::Return)),
        token(TokenKind::RightBrace),
    ]
}

impl Parser<'_> {
    pub(super) fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.start();
        self.open(Delimiter::Brace)?;
        let mut statements = Vec::new();
        while self.at_statement_start() {
            statements.push(self.parse_statement()?);
        }
        self.close(Delimiter::Brace, &statement_expectations())?;
        Ok(Block {
            statements,
            range: self.range_from(start),
        })
    }

    pub(super) fn parse_binder(&mut self) -> ParseResult<Binder> {
        let start = self.start();
        let name = self.parse_name()?;
        self.consume(TokenKind::Colon)?;
        let value_type = self.parse_value_type()?;
        Ok(Binder {
            name,
            value_type,
            range: self.range_from(start),
        })
    }

    /// Whether the current token starts a statement or a name that stands where one belongs.
    fn at_statement_start(&self) -> bool {
        matches!(
            self.peek(0),
            TokenKind::Name(_)
                | TokenKind::Keyword(
                    Keyword::Let | Keyword::Loop | Keyword::Next | Keyword::Return
                )
        )
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let start = self.start();
        self.repair_foreign_word(Place::Statement)?;
        let kind = match self.peek(0) {
            TokenKind::Keyword(Keyword::Let) => self.parse_let()?,
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop()?,
            TokenKind::Keyword(Keyword::Next) => self.parse_next()?,
            TokenKind::Keyword(Keyword::Return) => self.parse_return()?,
            TokenKind::Name(_) => return Err(self.reject_assignment()),
            _ => return Err(self.fail(&statement_expectations())),
        };
        Ok(Statement {
            kind,
            range: self.range_from(start),
        })
    }

    /// Ends the function at a statement that begins with a name.
    fn reject_assignment(&mut self) -> Abort {
        let range = self.current().range;
        let name = self.token_text(range);
        let message = format!(
            "a statement cannot begin with the name `{name}`: a value is defined once, by `let`, a carry or `next`"
        );
        self.stop(Kind::Assignment, message, range)
    }

    fn parse_let(&mut self) -> ParseResult<StatementKind> {
        self.bump()?;
        self.repair_foreign_word(Place::Binder)?;
        let binder = self.parse_binder()?;
        self.consume(TokenKind::Equals)?;
        let value = self.parse_expression()?;
        Ok(StatementKind::Let { binder, value })
    }

    fn parse_loop(&mut self) -> ParseResult<StatementKind> {
        self.bump()?;
        let (index, low, high) = self.parse_range_head()?;
        self.consume_keyword(Keyword::Step)?;
        let step = self.parse_integer()?;
        self.consume_keyword(Keyword::Carry)?;
        let carries = self.parse_list(Delimiter::Paren, false, Self::parse_carry)?;
        let body = self.parse_block()?;
        Ok(StatementKind::Loop {
            index,
            low,
            high,
            step,
            carries,
            body,
        })
    }

    fn parse_carry(&mut self) -> ParseResult<Carry> {
        let start = self.start();
        let binder = self.parse_binder()?;
        self.consume(TokenKind::Equals)?;
        let value = self.parse_expression()?;
        Ok(Carry {
            binder,
            value,
            range: self.range_from(start),
        })
    }

    fn parse_next(&mut self) -> ParseResult<StatementKind> {
        self.bump()?;
        let name = self.parse_name()?;
        self.consume(TokenKind::Equals)?;
        let value = self.parse_expression()?;
        Ok(StatementKind::Next { name, value })
    }

    fn parse_return(&mut self) -> ParseResult<StatementKind> {
        self.bump()?;
        let value = self.parse_expression()?;
        Ok(StatementKind::Return { value })
    }
}
