//! Parameters and the types of a signature.

use super::delimiter::Delimiter;
use super::{
    A_NAME, A_TYPE, AN_ALIGNMENT, AN_INTEGER_LITERAL, ParseResult, Parser, keyword, token,
};
use crate::ast::{
    AccessMode, Bound, BoundOperator, Limit, Param, ParamKind, ReferenceType, SignatureType,
    ValueType,
};
use crate::token::{Keyword, ScalarType, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_param(&mut self) -> ParseResult<Param> {
        let start = self.start();
        let name = self.parse_name()?;
        self.consume(TokenKind::Colon)?;
        let kind = if self.at(&TokenKind::Ampersand) {
            ParamKind::Reference(self.parse_reference_type()?)
        } else {
            ParamKind::Value(self.parse_signature_type()?)
        };
        Ok(Param {
            name,
            kind,
            range: self.range_from(start),
        })
    }

    pub(super) fn parse_value_type(&mut self) -> ParseResult<ValueType> {
        let value = match self.peek(0) {
            TokenKind::ScalarType(scalar) => ValueType::Scalar(*scalar),
            TokenKind::VectorType(vector) => ValueType::Vector(*vector),
            TokenKind::Keyword(Keyword::Bool) => ValueType::Bool,
            _ => return Err(self.fail(&[A_TYPE])),
        };
        self.bump()?;
        Ok(value)
    }

    pub(super) fn parse_signature_type(&mut self) -> ParseResult<SignatureType> {
        let start = self.start();
        let value = self.parse_value_type()?;
        let bound = match self.peek(0) {
            TokenKind::Less | TokenKind::LessEqual => Some(self.parse_bound()?),
            _ => None,
        };
        Ok(SignatureType {
            value,
            bound,
            range: self.range_from(start),
        })
    }

    fn parse_bound(&mut self) -> ParseResult<Bound> {
        let start = self.start();
        let operator = match self.peek(0) {
            TokenKind::LessEqual => BoundOperator::LessEqual,
            _ => BoundOperator::Less,
        };
        self.bump()?;
        let limit = self.parse_limit()?;
        Ok(Bound {
            operator,
            limit,
            range: self.range_from(start),
        })
    }

    fn parse_limit(&mut self) -> ParseResult<Limit> {
        match self.peek(0) {
            TokenKind::Int(_) => self.parse_integer().map(Limit::Int),
            TokenKind::Name(_) => self.parse_name().map(Limit::Name),
            _ => Err(self.fail(&[AN_INTEGER_LITERAL, A_NAME])),
        }
    }

    fn parse_scalar_type(&mut self) -> ParseResult<ScalarType> {
        let TokenKind::ScalarType(scalar) = self.peek(0) else {
            return Err(self.fail(&[A_TYPE]));
        };
        let scalar = *scalar;
        self.bump()?;
        Ok(scalar)
    }

    fn parse_reference_type(&mut self) -> ParseResult<ReferenceType> {
        let start = self.start();
        self.consume(TokenKind::Ampersand)?;
        let (element, element_bound) = self.parse_element()?;
        self.open(Delimiter::Bracket)?;
        let extent = self.parse_limit()?;
        self.close(Delimiter::Bracket, &[token(TokenKind::RightBracket)])?;
        let alignment = self.parse_alignment()?;
        let mode = self.parse_mode()?;
        Ok(ReferenceType {
            element,
            element_bound,
            extent,
            alignment,
            mode,
            range: self.range_from(start),
        })
    }

    fn parse_element(&mut self) -> ParseResult<(ScalarType, Option<Bound>)> {
        if !self.at(&TokenKind::LeftParen) {
            return Ok((self.parse_scalar_type()?, None));
        }
        self.open(Delimiter::Paren)?;
        let element = self.parse_scalar_type()?;
        if !matches!(self.peek(0), TokenKind::Less | TokenKind::LessEqual) {
            let expected = [token(TokenKind::Less), token(TokenKind::LessEqual)];
            return Err(self.fail(&expected));
        }
        let bound = self.parse_bound()?;
        self.close(Delimiter::Paren, &[token(TokenKind::RightParen)])?;
        Ok((element, Some(bound)))
    }

    fn parse_alignment(&mut self) -> ParseResult<String> {
        let TokenKind::Align(digits) = self.peek(0) else {
            return Err(self.fail(&[AN_ALIGNMENT]));
        };
        let digits = digits.clone();
        self.bump()?;
        Ok(digits)
    }

    fn parse_mode(&mut self) -> ParseResult<AccessMode> {
        let mode = match self.peek(0) {
            TokenKind::Keyword(Keyword::Ro) => AccessMode::ReadOnly,
            TokenKind::Keyword(Keyword::Wo) => AccessMode::WriteOnly,
            _ => {
                let expected = [token(keyword(Keyword::Ro)), token(keyword(Keyword::Wo))];
                return Err(self.fail(&expected));
            }
        };
        self.bump()?;
        Ok(mode)
    }
}
