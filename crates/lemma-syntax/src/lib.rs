//! Lemma lexer, parser and AST.

mod lexer;
mod token;

pub use lexer::lex;
pub use token::{
    C1Variant, Comment, Fault, Keyword, Lexed, ScalarClass, ScalarType, TextRange, Token,
    TokenKind, VectorType,
};
