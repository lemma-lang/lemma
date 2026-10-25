//! Lemma lexer, parser and AST.

mod ast;
mod dump;
mod lexer;
mod parser;
mod printer;
mod token;

pub use ast::{
    AccessMode, BinaryOperator, Binder, Block, Bound, BoundOperator, Carry, Clause, EffClause,
    Effect, EquivClause, Expression, ExpressionKind, File, Function, Limit, Name, Param, ParamKind,
    Permission, ReferenceType, SignatureType, Statement, StatementKind, UnaryOperator, ValueType,
};
pub use dump::dump;
pub use lexer::lex;
pub use parser::{Parsed, parse};
pub use printer::{format, print};
pub use token::{
    C1Variant, Comment, Fault, Keyword, Lexed, ScalarClass, ScalarType, TextRange, Token,
    TokenKind, VectorType,
};
