//! Expressions: one loop per precedence level, recursion only where a level opens.

use lemma_diagnostics::Kind;

use super::nesting::Delimiter;
use super::{AN_EXPRESSION, ParseResult, Parser};
use crate::ast::{BinaryOperator, Binder, Expression, ExpressionKind, UnaryOperator};
use crate::token::{C1Symbol, Keyword, TextRange, TokenKind, VectorType};

type OperatorOf = fn(&TokenKind) -> Option<BinaryOperator>;
type OperandParser<'a> = fn(&mut Parser<'a>) -> ParseResult<Expression>;

fn or_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    matches!(kind, TokenKind::OrOr).then_some(BinaryOperator::Or)
}

fn and_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    matches!(kind, TokenKind::AndAnd).then_some(BinaryOperator::And)
}

fn comparison_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    match kind {
        TokenKind::EqualEqual => Some(BinaryOperator::Equal),
        TokenKind::BangEqual => Some(BinaryOperator::NotEqual),
        TokenKind::Less => Some(BinaryOperator::Less),
        TokenKind::LessEqual => Some(BinaryOperator::LessEqual),
        TokenKind::Greater => Some(BinaryOperator::Greater),
        TokenKind::GreaterEqual => Some(BinaryOperator::GreaterEqual),
        _ => None,
    }
}

fn additive_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    match kind {
        TokenKind::Plus => Some(BinaryOperator::Add),
        TokenKind::Minus => Some(BinaryOperator::Subtract),
        _ => None,
    }
}

fn multiplicative_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    match kind {
        TokenKind::Star => Some(BinaryOperator::Multiply),
        TokenKind::Slash => Some(BinaryOperator::Divide),
        TokenKind::Percent => Some(BinaryOperator::Remainder),
        _ => None,
    }
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    let range = TextRange {
        start: left.range.start,
        end: right.range.end,
    };
    let kind = ExpressionKind::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    };
    Expression { kind, range }
}

impl<'a> Parser<'a> {
    pub(super) fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_or()
    }

    fn node(&self, start: usize, kind: ExpressionKind) -> Expression {
        Expression {
            kind,
            range: self.range_from(start),
        }
    }

    /// Parses `operand (operator operand)*`, grouping to the left.
    fn parse_chain(
        &mut self,
        operator_of: OperatorOf,
        operand: OperandParser<'a>,
    ) -> ParseResult<Expression> {
        let mut left = operand(self)?;
        while let Some(operator) = operator_of(self.peek(0)) {
            self.bump()?;
            let right = operand(self)?;
            left = binary(operator, left, right);
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> ParseResult<Expression> {
        self.parse_chain(or_operator, Self::parse_and)
    }

    fn parse_and(&mut self) -> ParseResult<Expression> {
        self.parse_chain(and_operator, Self::parse_comparison)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let left = self.parse_additive()?;
        let Some(operator) = self.consume_comparison_operator()? else {
            return Ok(left);
        };
        let right = self.parse_additive()?;
        Ok(binary(operator, left, right))
    }

    /// Consumes a comparison operator, if one is next; `=` reads as `==`, reported as an
    /// `assignment-in-expression` with that repair.
    fn consume_comparison_operator(&mut self) -> ParseResult<Option<BinaryOperator>> {
        if self.at(&TokenKind::Equals) {
            let range = self.bump()?;
            let message = "`=` is not a comparison; equality is `==`".to_string();
            let edit = (range, "==".to_string());
            let record = self.record(Kind::AssignmentInExpression, message, range, &[edit]);
            self.diagnostics.push(record);
            return Ok(Some(BinaryOperator::Equal));
        }
        let Some(operator) = comparison_operator(self.peek(0)) else {
            return Ok(None);
        };
        self.bump()?;
        Ok(Some(operator))
    }

    pub(super) fn parse_additive(&mut self) -> ParseResult<Expression> {
        self.parse_chain(additive_operator, Self::parse_multiplicative)
    }

    /// The innermost chain; a `&` after one of its operands is a binary `&`, which the language
    /// does not have.
    fn parse_multiplicative(&mut self) -> ParseResult<Expression> {
        let chain = self.parse_chain(multiplicative_operator, Self::parse_unary)?;
        if self.at(&TokenKind::Ampersand) {
            let range = self.current().range;
            let message = "`&` is not a binary operator of the language".to_string();
            return Err(self.stop(Kind::UnsupportedOperator, message, range));
        }
        Ok(chain)
    }

    fn parse_unary(&mut self) -> ParseResult<Expression> {
        let start = self.start();
        let (operator, parse_operand): (_, OperandParser<'a>) = match self.peek(0) {
            TokenKind::Minus => (UnaryOperator::Negate, Self::parse_unary),
            TokenKind::Bang => (UnaryOperator::Not, Self::parse_primary),
            _ => return self.parse_primary(),
        };
        let operand = Box::new(self.unary_level(parse_operand)?);
        Ok(self.node(start, ExpressionKind::Unary { operator, operand }))
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        let start = self.start();
        let kind = match self.peek(0) {
            TokenKind::Int(spelling) => ExpressionKind::Int(spelling.clone()),
            TokenKind::Float(spelling) => ExpressionKind::Float(spelling.clone()),
            TokenKind::Keyword(Keyword::True) => ExpressionKind::Bool(true),
            TokenKind::Keyword(Keyword::False) => ExpressionKind::Bool(false),
            TokenKind::Keyword(Keyword::Result) => ExpressionKind::Result,
            TokenKind::Name(_) => return self.parse_name_primary(),
            TokenKind::VectorType(vector) => {
                let vector = *vector;
                return self.parse_member(vector);
            }
            TokenKind::LeftParen => return self.parse_parenthesized(),
            other => {
                return match other.c1_symbol() {
                    Some((C1Symbol::Sum, _)) => self.parse_quantifier(C1Symbol::Sum),
                    Some((C1Symbol::Forall, _)) => self.parse_quantifier(C1Symbol::Forall),
                    _ => Err(self.fail(&[AN_EXPRESSION])),
                };
            }
        };
        self.bump()?;
        Ok(self.node(start, kind))
    }

    fn parse_name_primary(&mut self) -> ParseResult<Expression> {
        let start = self.start();
        let name = self.parse_name()?;
        let kind = match self.peek(0) {
            TokenKind::LeftBracket => {
                let index = self.delimited(Delimiter::Bracket, &[], Self::parse_expression)?;
                ExpressionKind::Index {
                    base: name,
                    index: Box::new(index),
                }
            }
            TokenKind::LeftParen => {
                let arguments = self.parse_arguments()?;
                ExpressionKind::Call {
                    callee: name,
                    arguments,
                }
            }
            _ => ExpressionKind::Name(name.text),
        };
        Ok(self.node(start, kind))
    }

    fn parse_arguments(&mut self) -> ParseResult<Vec<Expression>> {
        self.delimited_list(Delimiter::Parenthesis, true, Self::parse_expression)
    }

    fn parse_member(&mut self, vector: VectorType) -> ParseResult<Expression> {
        let start = self.start();
        self.bump()?;
        self.consume(TokenKind::Dot)?;
        let member = self.parse_name()?;
        let arguments = self.parse_arguments()?;
        let kind = ExpressionKind::Member {
            vector,
            member,
            arguments,
        };
        Ok(self.node(start, kind))
    }

    fn parse_parenthesized(&mut self) -> ParseResult<Expression> {
        let start = self.start();
        let inner = self.delimited(Delimiter::Parenthesis, &[], Self::parse_expression)?;
        Ok(self.node(start, ExpressionKind::Paren(Box::new(inner))))
    }

    /// Parses `binder IN low..<high`: the head shared by loops, sums and quantifiers.
    pub(super) fn parse_range_head(&mut self) -> ParseResult<(Binder, Expression, Expression)> {
        let binder = self.parse_binder()?;
        self.consume_c1(C1Symbol::In)?;
        let low = self.parse_additive()?;
        self.consume(TokenKind::DotDotLess)?;
        let high = self.parse_additive()?;
        Ok((binder, low, high))
    }

    fn parse_quantifier(&mut self, symbol: C1Symbol) -> ParseResult<Expression> {
        let start = self.start();
        self.consume_c1(symbol)?;
        let (binder, low, high) =
            self.delimited(Delimiter::Parenthesis, &[], Self::parse_range_head)?;
        let body = Box::new(self.delimited(Delimiter::Brace, &[], Self::parse_expression)?);
        let (low, high) = (Box::new(low), Box::new(high));
        let kind = match symbol {
            C1Symbol::Forall => ExpressionKind::Forall {
                binder,
                low,
                high,
                body,
            },
            _ => ExpressionKind::Sum {
                binder,
                low,
                high,
                body,
            },
        };
        Ok(self.node(start, kind))
    }
}
