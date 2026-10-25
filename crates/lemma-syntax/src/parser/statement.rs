//! Blocks, statements and binders.

use super::{ParseResult, Parser, keyword, token};
use crate::ast::{Binder, Block, Carry, Statement, StatementKind};
use crate::token::{Keyword, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.start();
        self.consume(TokenKind::LeftBrace)?;
        let mut statements = Vec::new();
        while !self.at(&TokenKind::RightBrace) {
            statements.push(self.parse_statement()?);
        }
        self.bump()?;
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

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let start = self.start();
        let kind = match self.peek(0) {
            TokenKind::Keyword(Keyword::Let) => self.parse_let()?,
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop()?,
            TokenKind::Keyword(Keyword::Next) => self.parse_next()?,
            TokenKind::Keyword(Keyword::Return) => self.parse_return()?,
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
        Ok(Statement {
            kind,
            range: self.range_from(start),
        })
    }

    fn parse_let(&mut self) -> ParseResult<StatementKind> {
        self.bump()?;
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
        self.consume(TokenKind::LeftParen)?;
        let carries = self.parse_list(TokenKind::RightParen, false, Self::parse_carry)?;
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
