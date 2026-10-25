//! Tokens, their byte ranges, and the lexical faults attached to them.

use std::fmt;

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

/// One of the three symbols whose spelling the C1 variant chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum C1Symbol {
    Sum,
    In,
    Forall,
}

impl C1Symbol {
    pub(crate) fn token(self, variant: C1Variant) -> TokenKind {
        match self {
            Self::Sum => TokenKind::Sum(variant),
            Self::In => TokenKind::In(variant),
            Self::Forall => TokenKind::Forall(variant),
        }
    }

    pub(crate) fn spelling(self, variant: C1Variant) -> &'static str {
        match (self, variant) {
            (Self::Sum, C1Variant::Ascii) => "sum",
            (Self::In, C1Variant::Ascii) => "in",
            (Self::Forall, C1Variant::Ascii) => "forall",
            (Self::Sum, C1Variant::Unicode) => "\u{03A3}",
            (Self::In, C1Variant::Unicode) => "\u{2208}",
            (Self::Forall, C1Variant::Unicode) => "\u{2200}",
        }
    }

    /// The text that replaces the symbol at `range` in `text` under `variant`: a word is padded
    /// with a space on each side that touches a word character.
    pub(crate) fn replacement(self, variant: C1Variant, text: &[u8], range: TextRange) -> String {
        let spelling = self.spelling(variant);
        if variant == C1Variant::Unicode {
            return spelling.to_string();
        }
        let follows_word = range
            .start
            .checked_sub(1)
            .and_then(|before| text.get(before))
            .copied()
            .is_some_and(is_word_byte);
        let precedes_word = text.get(range.end).copied().is_some_and(is_word_byte);
        let leading = if follows_word { " " } else { "" };
        let trailing = if precedes_word { " " } else { "" };
        format!("{leading}{spelling}{trailing}")
    }
}

/// Whether `byte` can be part of a word: an ASCII letter, a digit or `_`.
pub(crate) fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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

impl fmt::Display for ScalarType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = match self.class {
            ScalarClass::Unsigned => 'u',
            ScalarClass::Signed => 'i',
            ScalarClass::Float => 'f',
        };
        write!(formatter, "{letter}{}", self.bits)
    }
}

/// A reserved vector type name such as `f32x4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VectorType {
    /// The type of one lane.
    pub element: ScalarType,
    /// The number of lanes.
    pub lanes: u8,
}

impl fmt::Display for VectorType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}x{}", self.element, self.lanes)
    }
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

const KEYWORDS: [(&str, Keyword); 24] = [
    ("export", Keyword::Export),
    ("fn", Keyword::Fn),
    ("eff", Keyword::Eff),
    ("req", Keyword::Req),
    ("ens", Keyword::Ens),
    ("spec", Keyword::Spec),
    ("equiv", Keyword::Equiv),
    ("allow", Keyword::Allow),
    ("impl", Keyword::Impl),
    ("let", Keyword::Let),
    ("loop", Keyword::Loop),
    ("step", Keyword::Step),
    ("carry", Keyword::Carry),
    ("next", Keyword::Next),
    ("return", Keyword::Return),
    ("result", Keyword::Result),
    ("true", Keyword::True),
    ("false", Keyword::False),
    ("ro", Keyword::Ro),
    ("wo", Keyword::Wo),
    ("pure", Keyword::Pure),
    ("fp_reassoc", Keyword::FpReassoc),
    ("fp_contract", Keyword::FpContract),
    ("bool", Keyword::Bool),
];

impl Keyword {
    /// The keyword spelled `word`, if there is one.
    pub fn from_word(word: &str) -> Option<Self> {
        KEYWORDS
            .iter()
            .find(|(spelling, _)| *spelling == word)
            .map(|(_, keyword)| *keyword)
    }

    /// The keyword's spelling.
    pub fn as_str(self) -> &'static str {
        KEYWORDS
            .iter()
            .find(|(_, keyword)| *keyword == self)
            .map_or("", |(spelling, _)| spelling)
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

impl TokenKind {
    /// The fixed spelling of a keyword, punctuation, operator or C1 token; `None` for names,
    /// types, literals, lookalikes and the markers of rejected text and of the end.
    pub fn spelling(&self) -> Option<&'static str> {
        let spelling = match self {
            Self::Keyword(keyword) => keyword.as_str(),
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::Comma => ",",
            Self::Colon => ":",
            Self::Dot => ".",
            Self::Arrow => "->",
            Self::Equals => "=",
            Self::Ampersand => "&",
            Self::DotDotLess => "..<",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::EqualEqual => "==",
            Self::BangEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::AndAnd => "&&",
            Self::OrOr => "||",
            Self::Bang => "!",
            Self::Sum(variant) => C1Symbol::Sum.spelling(*variant),
            Self::In(variant) => C1Symbol::In.spelling(*variant),
            Self::Forall(variant) => C1Symbol::Forall.spelling(*variant),
            Self::AlmostEqual => "\u{2248}",
            Self::Name(_)
            | Self::ScalarType(_)
            | Self::VectorType(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Align(_)
            | Self::MembershipLookalike
            | Self::Error
            | Self::Eof => return None,
        };
        Some(spelling)
    }

    /// Whether the token is a word the language reserves: a keyword, a type name or a C1 word or
    /// symbol.
    pub(crate) fn is_reserved(&self) -> bool {
        matches!(
            self,
            Self::Keyword(_)
                | Self::ScalarType(_)
                | Self::VectorType(_)
                | Self::Sum(_)
                | Self::In(_)
                | Self::Forall(_)
        )
    }

    /// The C1 symbol this token spells and the variant of its spelling.
    pub(crate) fn c1_symbol(&self) -> Option<(C1Symbol, C1Variant)> {
        match self {
            Self::Sum(variant) => Some((C1Symbol::Sum, *variant)),
            Self::In(variant) => Some((C1Symbol::In, *variant)),
            Self::Forall(variant) => Some((C1Symbol::Forall, *variant)),
            _ => None,
        }
    }
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
    /// The range the record is reported on, when it is neither the token nor the text its
    /// repair edits.
    pub span: Option<TextRange>,
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

/// A `//` comment, from its `//` to before the line end, the CR of a CRLF line end excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The bytes of the comment in the source text.
    pub range: TextRange,
    /// The comment's text, including its `//`, with the code points and bytes a comment may
    /// not hold removed.
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
