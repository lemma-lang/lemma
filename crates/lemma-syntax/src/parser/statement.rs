//! Blocks, statements and binders.

use super::foreign::{Context, Foreign};
use super::nesting::Delimiter;
use super::{ParseResult, Parser, keyword, token};
use crate::ast::{Binder, Block, Carry, Statement, StatementKind};
use crate::token::{Keyword, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.start();
        let statements = self.delimited(Delimiter::Brace, &[], Self::parse_statements)?;
        Ok(Block {
            statements,
            range: self.range_from(start),
        })
    }

    fn parse_statements(&mut self) -> ParseResult<Vec<Statement>> {
        let mut statements = Vec::new();
        while !self.at(&TokenKind::RightBrace) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
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

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let start = self.start();
        let kind = match self.consume_statement_keyword()? {
            Keyword::Let => self.parse_let()?,
            Keyword::Loop => self.parse_loop()?,
            Keyword::Next => self.parse_next()?,
            _ => self.parse_return()?,
        };
        Ok(Statement {
            kind,
            range: self.range_from(start),
        })
    }

    /// Consumes the keyword that begins a statement, or another language's word for one; a
    /// statement that begins with any other name is an `assignment`.
    fn consume_statement_keyword(&mut self) -> ParseResult<Keyword> {
        if let Some(keyword) = self.consume_foreign(Context::Statement)? {
            return Ok(keyword);
        }
        let found = match self.peek(0) {
            TokenKind::Keyword(
                found @ (Keyword::Let | Keyword::Loop | Keyword::Next | Keyword::Return),
            ) => *found,
            TokenKind::Name(_) => return Err(self.assignment()),
            _ => {
                let expected = [
                    token(keyword(Keyword::Let)),
                    token(keyword(Keyword::Loop)),
                    token(keyword(Keyword::Next)),
                    token(keyword(Keyword::Return)),
                    token(TokenKind::RightBrace),
                ];
                return Err(self.fail(&expected));
            }
        };
        self.bump()?;
        Ok(found)
    }

    fn parse_let(&mut self) -> ParseResult<StatementKind> {
        if self.foreign(Context::AfterLet) == Some(Foreign::Reject) {
            return Err(self.reject_foreign());
        }
        let binder = self.parse_binder()?;
        self.consume(TokenKind::Equals)?;
        let value = self.parse_expression()?;
        Ok(StatementKind::Let { binder, value })
    }

    fn parse_loop(&mut self) -> ParseResult<StatementKind> {
        let (index, low, high) = self.parse_range_head()?;
        self.consume_keyword(Keyword::Step)?;
        let step = self.parse_integer()?;
        self.consume_keyword(Keyword::Carry)?;
        let carries = self.delimited_list(Delimiter::Parenthesis, false, Self::parse_carry)?;
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
        let name = self.parse_name()?;
        self.consume(TokenKind::Equals)?;
        let value = self.parse_expression()?;
        Ok(StatementKind::Next { name, value })
    }

    fn parse_return(&mut self) -> ParseResult<StatementKind> {
        let value = self.parse_expression()?;
        Ok(StatementKind::Return { value })
    }
}
