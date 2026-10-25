//! The syntax tree of `docs/grammar.md` section 3.
//!
//! Every node carries the byte range of the text it was parsed from. Parentheses are kept as
//! [`ExpressionKind::Paren`] nodes, and literal and alignment spellings as written. The tree holds
//! no comment and no C1 variant.

use crate::token::{ScalarType, TextRange, VectorType};

/// A source file: its functions, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The functions that were parsed to their end.
    pub items: Vec<Function>,
    /// The whole text.
    pub range: TextRange,
}

/// A function: signature, contract clauses and implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The range of the `export` keyword, when the function is exported.
    pub export: Option<TextRange>,
    /// The function's name.
    pub name: Name,
    /// The parameters, in order.
    pub params: Vec<Param>,
    /// The result type.
    pub result: SignatureType,
    /// The `eff` clause.
    pub effects: EffClause,
    /// The `req` clauses, in source order.
    pub requires: Vec<Clause>,
    /// The `ens` clauses, in source order.
    pub ensures: Vec<Clause>,
    /// The `spec` clause.
    pub spec: Clause,
    /// The `equiv` clause.
    pub equiv: EquivClause,
    /// The `impl` block.
    pub body: Block,
    /// From `export` or `fn` to the closing `}` of the block.
    pub range: TextRange,
}

/// An identifier with its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The identifier.
    pub text: String,
    /// Its bytes.
    pub range: TextRange,
}

/// A type that holds a value: a scalar, a vector or `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// A scalar type such as `u64`.
    Scalar(ScalarType),
    /// A vector type such as `f32x4`.
    Vector(VectorType),
    /// `bool`.
    Bool,
}

/// The comparison of a bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundOperator {
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
}

impl BoundOperator {
    /// The operator's spelling.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessEqual => "<=",
        }
    }
}

/// An integer literal or a name, as a bound's limit or a reference's extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Limit {
    /// An integer literal, spelling kept.
    Int(String),
    /// A name.
    Name(Name),
}

/// A bound on a type: `< limit` or `<= limit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// The comparison.
    pub operator: BoundOperator,
    /// What the value is compared to.
    pub limit: Limit,
    /// From the operator to the limit.
    pub range: TextRange,
}

/// A type of a signature: a value type with an optional bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureType {
    /// The type that holds the value.
    pub value: ValueType,
    /// The bound written after it, if any.
    pub bound: Option<Bound>,
    /// From the type to the end of its bound.
    pub range: TextRange,
}

/// Whether a reference parameter is read or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// `ro`.
    ReadOnly,
    /// `wo`.
    WriteOnly,
}

impl AccessMode {
    /// The mode's spelling.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::ReadOnly => "ro",
            Self::WriteOnly => "wo",
        }
    }
}

/// A reference to a region of memory: `&T[extent] @aN mode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceType {
    /// The element type.
    pub element: ScalarType,
    /// The bound written on the element, inside parentheses.
    pub element_bound: Option<Bound>,
    /// The number of elements.
    pub extent: Limit,
    /// The digits of the alignment `@aN`, as written.
    pub alignment: String,
    /// Read or write access.
    pub mode: AccessMode,
    /// From the `&` to the mode.
    pub range: TextRange,
}

/// The type of a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind {
    /// A value.
    Value(SignatureType),
    /// A reference.
    Reference(ReferenceType),
}

/// A parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The parameter's name.
    pub name: Name,
    /// Its type.
    pub kind: ParamKind,
    /// From the name to the end of the type.
    pub range: TextRange,
}

/// An effect a function may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// `pure`.
    Pure,
}

/// The `eff` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffClause {
    /// The effects, in source order.
    pub effects: Vec<Effect>,
    /// From `eff` to the last effect.
    pub range: TextRange,
}

/// A `req`, `ens` or `spec` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// What the clause states.
    pub expression: Expression,
    /// From the keyword to the end of the expression.
    pub range: TextRange,
}

/// A floating-point reordering the implementation may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    /// `fp_reassoc`.
    FpReassoc,
    /// `fp_contract`.
    FpContract,
}

impl Permission {
    /// The permission's spelling.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::FpReassoc => "fp_reassoc",
            Self::FpContract => "fp_contract",
        }
    }
}

/// The `equiv` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquivClause {
    /// The permissions of the `allow` list, sorted in the order of the permission table.
    pub permissions: Vec<Permission>,
    /// From `equiv` to the `)` of its `allow` list.
    pub range: TextRange,
}

/// A `{ ... }` block of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The statements, in order.
    pub statements: Vec<Statement>,
    /// From the `{` to the `}`.
    pub range: TextRange,
}

/// A name with its value type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binder {
    /// The bound name.
    pub name: Name,
    /// Its type.
    pub value_type: ValueType,
    /// From the name to the type.
    pub range: TextRange,
}

/// A loop-carried value: `binder = initial`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carry {
    /// The carried name and its type.
    pub binder: Binder,
    /// The initial value.
    pub value: Expression,
    /// From the binder to the end of the value.
    pub range: TextRange,
}

/// A statement of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Which statement it is.
    pub kind: StatementKind,
    /// From the first keyword to the end of the statement.
    pub range: TextRange,
}

/// The forms of a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    /// `let binder = value`.
    Let {
        /// The defined name and its type.
        binder: Binder,
        /// The value.
        value: Expression,
    },
    /// `loop index in low..<high step N carry(...) { body }`.
    Loop {
        /// The index.
        index: Binder,
        /// The first value of the index.
        low: Expression,
        /// The bound the index stays below.
        high: Expression,
        /// The step, as written.
        step: String,
        /// The carried values.
        carries: Vec<Carry>,
        /// The loop body.
        body: Block,
    },
    /// `next name = value`.
    Next {
        /// The carry that is updated.
        name: Name,
        /// Its next value.
        value: Expression,
    },
    /// `return value`.
    Return {
        /// The returned value.
        value: Expression,
    },
}

/// A prefix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `-`.
    Negate,
    /// `!`.
    Not,
}

impl UnaryOperator {
    /// The operator's spelling.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

/// An infix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
    /// `/`.
    Divide,
    /// `%`.
    Remainder,
    /// `==`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterEqual,
    /// `&&`.
    And,
    /// `||`.
    Or,
}

impl BinaryOperator {
    /// The operator's spelling.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}

/// An expression with the bytes it was parsed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    /// Which expression it is.
    pub kind: ExpressionKind,
    /// The bytes of the expression, parentheses included.
    pub range: TextRange,
}

/// The forms of an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    /// An integer literal, spelling kept.
    Int(String),
    /// A floating-point literal, spelling kept.
    Float(String),
    /// `true` or `false`.
    Bool(bool),
    /// `result`.
    Result,
    /// A name.
    Name(String),
    /// `base[index]`.
    Index {
        /// The indexed name.
        base: Name,
        /// The index.
        index: Box<Expression>,
    },
    /// `callee(arguments)`.
    Call {
        /// The called name.
        callee: Name,
        /// The arguments, in order.
        arguments: Vec<Expression>,
    },
    /// `vector.member(arguments)`, such as `f32x4.load(a, i)`.
    Member {
        /// The vector type.
        vector: VectorType,
        /// The member.
        member: Name,
        /// The arguments, in order.
        arguments: Vec<Expression>,
    },
    /// `sum(binder in low..<high) { body }`.
    Sum {
        /// The summation index.
        binder: Binder,
        /// The first value of the index.
        low: Box<Expression>,
        /// The bound the index stays below.
        high: Box<Expression>,
        /// The summed expression.
        body: Box<Expression>,
    },
    /// `forall(binder in low..<high) { body }`.
    Forall {
        /// The quantified index.
        binder: Binder,
        /// The first value of the index.
        low: Box<Expression>,
        /// The bound the index stays below.
        high: Box<Expression>,
        /// The quantified expression.
        body: Box<Expression>,
    },
    /// `(inner)`.
    Paren(Box<Expression>),
    /// A prefix operator applied to its operand.
    Unary {
        /// The operator.
        operator: UnaryOperator,
        /// The operand.
        operand: Box<Expression>,
    },
    /// An infix operator applied to two operands.
    Binary {
        /// The operator.
        operator: BinaryOperator,
        /// The left operand.
        left: Box<Expression>,
        /// The right operand.
        right: Box<Expression>,
    },
}
