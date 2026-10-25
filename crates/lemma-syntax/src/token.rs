//! Tokens, their byte ranges, and the lexical faults attached to them.

use lemma_diagnostics::Kind;

/// Which spelling of the C1 symbols is active: `sum`, `in`, `forall` or `Σ`, `∈`, `∀`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum C1Variant {
    /// Variant A, the provisional default: `sum`, `in`, `forall`.
    #[default]
    Ascii,
    /// Variant U: `Σ`, `∈`, `∀`.
    Unicode,
}

/// A half-open range of byte offsets in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    /// Offset of the first byte.
    pub start: usize,
    /// Offset one past the last byte.
    pub end: usize,
}

/// The class of a scalar type: unsigned integer, signed integer or floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarClass {
    /// `u8` to `u128`.
    Unsigned,
    /// `i8` to `i128`.
    Signed,
    /// `f16` to `f64`.
    Float,
}

/// A reserved scalar type name such as `u64` or `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScalarType {
    /// Unsigned, signed or floating point.
    pub class: ScalarClass,
    /// Width in bits.
    pub bits: u16,
}

/// A reserved vector type name such as `f32x4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VectorType {
    /// The type of one lane.
    pub element: ScalarType,
    /// The number of lanes.
    pub lanes: u8,
}

/// A reserved word that is neither a type name nor one of the three C1 words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// `export`
    Export,
    /// `fn`
    Fn,
    /// `eff`
    Eff,
    /// `req`
    Req,
    /// `ens`
    Ens,
    /// `spec`
    Spec,
    /// `equiv`
    Equiv,
    /// `allow`
    Allow,
    /// `impl`
    Impl,
    /// `let`
    Let,
    /// `loop`
    Loop,
    /// `step`
    Step,
    /// `carry`
    Carry,
    /// `next`
    Next,
    /// `return`
    Return,
    /// `result`
    Result,
    /// `true`
    True,
    /// `false`
    False,
    /// `ro`
    Ro,
    /// `wo`
    Wo,
    /// `pure`
    Pure,
    /// `fp_reassoc`
    FpReassoc,
    /// `fp_contract`
    FpContract,
    /// `bool`
    Bool,
}

impl Keyword {
    /// The keyword spelled `word`, if there is one.
    pub fn from_word(word: &str) -> Option<Self> {
        let keyword = match word {
            "export" => Self::Export,
            "fn" => Self::Fn,
            "eff" => Self::Eff,
            "req" => Self::Req,
            "ens" => Self::Ens,
            "spec" => Self::Spec,
            "equiv" => Self::Equiv,
            "allow" => Self::Allow,
            "impl" => Self::Impl,
            "let" => Self::Let,
            "loop" => Self::Loop,
            "step" => Self::Step,
            "carry" => Self::Carry,
            "next" => Self::Next,
            "return" => Self::Return,
            "result" => Self::Result,
            "true" => Self::True,
            "false" => Self::False,
            "ro" => Self::Ro,
            "wo" => Self::Wo,
            "pure" => Self::Pure,
            "fp_reassoc" => Self::FpReassoc,
            "fp_contract" => Self::FpContract,
            "bool" => Self::Bool,
            _ => return None,
        };
        Some(keyword)
    }
}

/// What a token is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// An identifier: `[a-z][a-z0-9_]*` that is no keyword and no reserved type name.
    Name(String),
    /// A keyword.
    Keyword(Keyword),
    /// A reserved scalar type name.
    ScalarType(ScalarType),
    /// A reserved vector type name.
    VectorType(VectorType),
    /// An integer literal, spelling kept.
    Int(String),
    /// A floating-point literal, spelling kept.
    Float(String),
    /// An alignment `@aN`; the digits as written.
    Align(String),
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `.`
    Dot,
    /// `->`
    Arrow,
    /// `=`
    Equals,
    /// `&`
    Ampersand,
    /// `..<`
    DotDotLess,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `==`
    EqualEqual,
    /// `!=`
    BangEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
    /// The sum symbol, with the variant of its spelling: `sum` or `Σ`.
    Sum(C1Variant),
    /// The range membership symbol, with the variant of its spelling: `in` or `∈`.
    In(C1Variant),
    /// The universal quantifier, with the variant of its spelling: `forall` or `∀`.
    Forall(C1Variant),
    /// `ϵ` or `ε`, which the parser accepts as a membership lookalike only where one is expected.
    MembershipLookalike,
    /// `≈`, which the parser accepts only inside `equiv`.
    AlmostEqual,
    /// Text rejected by an unrepaired lexical fault.
    Error,
    /// The end of the input.
    Eof,
}

/// A lexical fault: a diagnostic kind, its message, and the edits of its repair, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The syntax-class kind.
    pub kind: Kind,
    /// One sentence on one line, with the found spelling in backticks.
    pub message: String,
    /// The edits of the repair, applied together; empty when the fault has no repair.
    pub repair: Vec<(TextRange, String)>,
}

/// A token with its byte range and the faults found in it and in the trivia before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What the token is; a repaired token has the kind of its repair.
    pub kind: TokenKind,
    /// The bytes of the token, without the trivia before it.
    pub range: TextRange,
    /// Faults of the trivia before the token, then of the token itself, in source order.
    pub faults: Vec<Fault>,
}

/// A `//` comment, from its `//` to before the line feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The bytes of the comment in the source text.
    pub range: TextRange,
    /// The comment's text, including its `//`.
    pub text: String,
}

/// The result of lexing a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexed {
    /// Every token, in order, ending with [`TokenKind::Eof`].
    pub tokens: Vec<Token>,
    /// Every comment, in order.
    pub comments: Vec<Comment>,
}
