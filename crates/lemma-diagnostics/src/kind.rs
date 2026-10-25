//! The closed set of diagnostic kinds and the failure classes they belong to.
//!
//! The kinds are the rows of the tables listed in `docs/diagnostics.md` section 7.

/// The cause of a failure, one of ADR 0004's causes that a diagnostic can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FailureClass {
    /// The text is not in the grammar.
    Syntax,
    /// A name, a type, a bound or a layout is wrong.
    TypeOrLayout,
    /// An effect is not allowed. No kind belongs to this class yet.
    Effect,
    /// A bound obligation is refuted.
    BoundProof,
    /// An equivalence or `ens` obligation is refuted.
    EquivalenceProof,
    /// The solver could not decide an obligation.
    SolverTimeout,
    /// The compiler failed on an input that does not explain it.
    ToolchainBug,
}

impl FailureClass {
    /// The class name written in the `class` key of the R rendering.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::TypeOrLayout => "type-or-layout",
            Self::Effect => "effect",
            Self::BoundProof => "bound-proof",
            Self::EquivalenceProof => "equivalence-proof",
            Self::SolverTimeout => "solver-timeout",
            Self::ToolchainBug => "toolchain-bug",
        }
    }
}

/// What went wrong, from the closed set of `docs/diagnostics.md` section 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// The `invalid-encoding` kind, of class `syntax`.
    InvalidEncoding,
    /// The `invalid-character` kind, of class `syntax`.
    InvalidCharacter,
    /// The `non-ascii` kind, of class `syntax`.
    NonAscii,
    /// The `lookalike` kind, of class `syntax`.
    Lookalike,
    /// The `c1-other-variant` kind, of class `syntax`.
    C1OtherVariant,
    /// The `semicolon` kind, of class `syntax`.
    Semicolon,
    /// The `block-comment` kind, of class `syntax`.
    BlockComment,
    /// The `hash-comment` kind, of class `syntax`.
    HashComment,
    /// The `invalid-number` kind, of class `syntax`.
    InvalidNumber,
    /// The `invalid-identifier` kind, of class `syntax`.
    InvalidIdentifier,
    /// The `reserved-word` kind, of class `syntax`.
    ReservedWord,
    /// The `unsupported-operator` kind, of class `syntax`.
    UnsupportedOperator,
    /// The `assignment-in-expression` kind, of class `syntax`.
    AssignmentInExpression,
    /// The `path-separator` kind, of class `syntax`.
    PathSeparator,
    /// The `range-operator` kind, of class `syntax`.
    RangeOperator,
    /// The `unexpected-token` kind, of class `syntax`.
    UnexpectedToken,
    /// The `unclosed-delimiter` kind, of class `syntax`.
    UnclosedDelimiter,
    /// The `unmatched-delimiter` kind, of class `syntax`.
    UnmatchedDelimiter,
    /// The `trailing-comma` kind, of class `syntax`.
    TrailingComma,
    /// The `foreign-keyword` kind, of class `syntax`.
    ForeignKeyword,
    /// The `assignment` kind, of class `syntax`.
    Assignment,
    /// The `clause-order` kind, of class `syntax`.
    ClauseOrder,
    /// The `missing-clause` kind, of class `syntax`.
    MissingClause,
    /// The `duplicate-clause` kind, of class `syntax`.
    DuplicateClause,
    /// The `equiv-without-allow` kind, of class `syntax`.
    EquivWithoutAllow,
    /// The `equiv-relation` kind, of class `syntax`.
    EquivRelation,
    /// The `duplicate-permission` kind, of class `syntax`.
    DuplicatePermission,
    /// The `duplicate-effect` kind, of class `syntax`.
    DuplicateEffect,
    /// The `chained-comparison` kind, of class `syntax`.
    ChainedComparison,
    /// The `ambiguous-not` kind, of class `syntax`.
    AmbiguousNot,
    /// The `missing-type` kind, of class `syntax`.
    MissingType,
    /// The `bound-operator` kind, of class `syntax`.
    BoundOperator,
    /// The `bound-expression` kind, of class `syntax`.
    BoundExpression,
    /// The `bound-on-binder` kind, of class `syntax`.
    BoundOnBinder,
    /// The `unparenthesized-element-bound` kind, of class `syntax`.
    UnparenthesizedElementBound,
    /// The `missing-alignment` kind, of class `syntax`.
    MissingAlignment,
    /// The `missing-mode` kind, of class `syntax`.
    MissingMode,
    /// The `missing-step` kind, of class `syntax`.
    MissingStep,
    /// The `invalid-step` kind, of class `syntax`.
    InvalidStep,
    /// The `missing-carry` kind, of class `syntax`.
    MissingCarry,
    /// The `carry-without-next` kind, of class `syntax`.
    CarryWithoutNext,
    /// The `duplicate-next` kind, of class `syntax`.
    DuplicateNext,
    /// The `misplaced-next` kind, of class `syntax`.
    MisplacedNext,
    /// The `missing-return` kind, of class `syntax`.
    MissingReturn,
    /// The `misplaced-return` kind, of class `syntax`.
    MisplacedReturn,
    /// The `result-outside-ens` kind, of class `syntax`.
    ResultOutsideEns,
    /// The `misplaced-export` kind, of class `syntax`.
    MisplacedExport,
    /// The `unexpected-end` kind, of class `syntax`.
    UnexpectedEnd,
    /// The `nesting-limit` kind, of class `syntax`.
    NestingLimit,
    /// The `unknown-name` kind, of class `type-or-layout`.
    UnknownName,
    /// The `duplicate-name` kind, of class `type-or-layout`.
    DuplicateName,
    /// The `builtin-name` kind, of class `type-or-layout`.
    BuiltinName,
    /// The `unknown-member` kind, of class `type-or-layout`.
    UnknownMember,
    /// The `bound-order` kind, of class `type-or-layout`.
    BoundOrder,
    /// The `bound-type-mismatch` kind, of class `type-or-layout`.
    BoundTypeMismatch,
    /// The `bound-on-non-integer` kind, of class `type-or-layout`.
    BoundOnNonInteger,
    /// The `bound-in-clause` kind, of class `type-or-layout`.
    BoundInClause,
    /// The `index-not-unsigned` kind, of class `type-or-layout`.
    IndexNotUnsigned,
    /// The `literal-out-of-range` kind, of class `type-or-layout`.
    LiteralOutOfRange,
    /// The `not-in-subset` kind, of class `type-or-layout`.
    NotInSubset,
    /// The `export-vector-type` kind, of class `type-or-layout`.
    ExportVectorType,
    /// The `export-unsupported-type` kind, of class `type-or-layout`.
    ExportUnsupportedType,
    /// The `export-c-reserved-name` kind, of class `type-or-layout`.
    ExportCReservedName,
    /// The `export-symbol-collision` kind, of class `type-or-layout`.
    ExportSymbolCollision,
    /// The `export-module-name` kind, of class `type-or-layout`.
    ExportModuleName,
    /// The `bound-unproven` kind, of class `bound-proof`.
    BoundUnproven,
    /// The `equivalence-unproven` kind, of class `equivalence-proof`.
    EquivalenceUnproven,
    /// The `ens-unproven` kind, of class `equivalence-proof`.
    EnsUnproven,
    /// The `solver-unknown` kind, of class `solver-timeout`.
    SolverUnknown,
    /// The `internal-error` kind, of class `toolchain-bug`.
    InternalError,
}

impl Kind {
    /// Every kind, in the order of the documents that define them.
    pub const ALL: &[Kind] = &[
        Self::InvalidEncoding,
        Self::InvalidCharacter,
        Self::NonAscii,
        Self::Lookalike,
        Self::C1OtherVariant,
        Self::Semicolon,
        Self::BlockComment,
        Self::HashComment,
        Self::InvalidNumber,
        Self::InvalidIdentifier,
        Self::ReservedWord,
        Self::UnsupportedOperator,
        Self::AssignmentInExpression,
        Self::PathSeparator,
        Self::RangeOperator,
        Self::UnexpectedToken,
        Self::UnclosedDelimiter,
        Self::UnmatchedDelimiter,
        Self::TrailingComma,
        Self::ForeignKeyword,
        Self::Assignment,
        Self::ClauseOrder,
        Self::MissingClause,
        Self::DuplicateClause,
        Self::EquivWithoutAllow,
        Self::EquivRelation,
        Self::DuplicatePermission,
        Self::DuplicateEffect,
        Self::ChainedComparison,
        Self::AmbiguousNot,
        Self::MissingType,
        Self::BoundOperator,
        Self::BoundExpression,
        Self::BoundOnBinder,
        Self::UnparenthesizedElementBound,
        Self::MissingAlignment,
        Self::MissingMode,
        Self::MissingStep,
        Self::InvalidStep,
        Self::MissingCarry,
        Self::CarryWithoutNext,
        Self::DuplicateNext,
        Self::MisplacedNext,
        Self::MissingReturn,
        Self::MisplacedReturn,
        Self::ResultOutsideEns,
        Self::MisplacedExport,
        Self::UnexpectedEnd,
        Self::NestingLimit,
        Self::UnknownName,
        Self::DuplicateName,
        Self::BuiltinName,
        Self::UnknownMember,
        Self::BoundOrder,
        Self::BoundTypeMismatch,
        Self::BoundOnNonInteger,
        Self::BoundInClause,
        Self::IndexNotUnsigned,
        Self::LiteralOutOfRange,
        Self::NotInSubset,
        Self::ExportVectorType,
        Self::ExportUnsupportedType,
        Self::ExportCReservedName,
        Self::ExportSymbolCollision,
        Self::ExportModuleName,
        Self::BoundUnproven,
        Self::EquivalenceUnproven,
        Self::EnsUnproven,
        Self::SolverUnknown,
        Self::InternalError,
    ];

    /// The kind name written in the `kind` key of the R rendering.
    pub const fn name(self) -> &'static str {
        match self {
            Self::InvalidEncoding => "invalid-encoding",
            Self::InvalidCharacter => "invalid-character",
            Self::NonAscii => "non-ascii",
            Self::Lookalike => "lookalike",
            Self::C1OtherVariant => "c1-other-variant",
            Self::Semicolon => "semicolon",
            Self::BlockComment => "block-comment",
            Self::HashComment => "hash-comment",
            Self::InvalidNumber => "invalid-number",
            Self::InvalidIdentifier => "invalid-identifier",
            Self::ReservedWord => "reserved-word",
            Self::UnsupportedOperator => "unsupported-operator",
            Self::AssignmentInExpression => "assignment-in-expression",
            Self::PathSeparator => "path-separator",
            Self::RangeOperator => "range-operator",
            Self::UnexpectedToken => "unexpected-token",
            Self::UnclosedDelimiter => "unclosed-delimiter",
            Self::UnmatchedDelimiter => "unmatched-delimiter",
            Self::TrailingComma => "trailing-comma",
            Self::ForeignKeyword => "foreign-keyword",
            Self::Assignment => "assignment",
            Self::ClauseOrder => "clause-order",
            Self::MissingClause => "missing-clause",
            Self::DuplicateClause => "duplicate-clause",
            Self::EquivWithoutAllow => "equiv-without-allow",
            Self::EquivRelation => "equiv-relation",
            Self::DuplicatePermission => "duplicate-permission",
            Self::DuplicateEffect => "duplicate-effect",
            Self::ChainedComparison => "chained-comparison",
            Self::AmbiguousNot => "ambiguous-not",
            Self::MissingType => "missing-type",
            Self::BoundOperator => "bound-operator",
            Self::BoundExpression => "bound-expression",
            Self::BoundOnBinder => "bound-on-binder",
            Self::UnparenthesizedElementBound => "unparenthesized-element-bound",
            Self::MissingAlignment => "missing-alignment",
            Self::MissingMode => "missing-mode",
            Self::MissingStep => "missing-step",
            Self::InvalidStep => "invalid-step",
            Self::MissingCarry => "missing-carry",
            Self::CarryWithoutNext => "carry-without-next",
            Self::DuplicateNext => "duplicate-next",
            Self::MisplacedNext => "misplaced-next",
            Self::MissingReturn => "missing-return",
            Self::MisplacedReturn => "misplaced-return",
            Self::ResultOutsideEns => "result-outside-ens",
            Self::MisplacedExport => "misplaced-export",
            Self::UnexpectedEnd => "unexpected-end",
            Self::NestingLimit => "nesting-limit",
            Self::UnknownName => "unknown-name",
            Self::DuplicateName => "duplicate-name",
            Self::BuiltinName => "builtin-name",
            Self::UnknownMember => "unknown-member",
            Self::BoundOrder => "bound-order",
            Self::BoundTypeMismatch => "bound-type-mismatch",
            Self::BoundOnNonInteger => "bound-on-non-integer",
            Self::BoundInClause => "bound-in-clause",
            Self::IndexNotUnsigned => "index-not-unsigned",
            Self::LiteralOutOfRange => "literal-out-of-range",
            Self::NotInSubset => "not-in-subset",
            Self::ExportVectorType => "export-vector-type",
            Self::ExportUnsupportedType => "export-unsupported-type",
            Self::ExportCReservedName => "export-c-reserved-name",
            Self::ExportSymbolCollision => "export-symbol-collision",
            Self::ExportModuleName => "export-module-name",
            Self::BoundUnproven => "bound-unproven",
            Self::EquivalenceUnproven => "equivalence-unproven",
            Self::EnsUnproven => "ens-unproven",
            Self::SolverUnknown => "solver-unknown",
            Self::InternalError => "internal-error",
        }
    }

    /// The kind that bears `name`, if the closed set has one.
    pub fn from_name(name: &str) -> Option<Kind> {
        Self::ALL.iter().copied().find(|kind| kind.name() == name)
    }

    /// The failure class this kind belongs to.
    pub const fn failure_class(self) -> FailureClass {
        match self {
            Self::InvalidEncoding
            | Self::InvalidCharacter
            | Self::NonAscii
            | Self::Lookalike
            | Self::C1OtherVariant
            | Self::Semicolon
            | Self::BlockComment
            | Self::HashComment
            | Self::InvalidNumber
            | Self::InvalidIdentifier
            | Self::ReservedWord
            | Self::UnsupportedOperator
            | Self::AssignmentInExpression
            | Self::PathSeparator
            | Self::RangeOperator
            | Self::UnexpectedToken
            | Self::UnclosedDelimiter
            | Self::UnmatchedDelimiter
            | Self::TrailingComma
            | Self::ForeignKeyword
            | Self::Assignment
            | Self::ClauseOrder
            | Self::MissingClause
            | Self::DuplicateClause
            | Self::EquivWithoutAllow
            | Self::EquivRelation
            | Self::DuplicatePermission
            | Self::DuplicateEffect
            | Self::ChainedComparison
            | Self::AmbiguousNot
            | Self::MissingType
            | Self::BoundOperator
            | Self::BoundExpression
            | Self::BoundOnBinder
            | Self::UnparenthesizedElementBound
            | Self::MissingAlignment
            | Self::MissingMode
            | Self::MissingStep
            | Self::InvalidStep
            | Self::MissingCarry
            | Self::CarryWithoutNext
            | Self::DuplicateNext
            | Self::MisplacedNext
            | Self::MissingReturn
            | Self::MisplacedReturn
            | Self::ResultOutsideEns
            | Self::MisplacedExport
            | Self::UnexpectedEnd
            | Self::NestingLimit => FailureClass::Syntax,
            Self::UnknownName
            | Self::DuplicateName
            | Self::BuiltinName
            | Self::UnknownMember
            | Self::BoundOrder
            | Self::BoundTypeMismatch
            | Self::BoundOnNonInteger
            | Self::BoundInClause
            | Self::IndexNotUnsigned
            | Self::LiteralOutOfRange
            | Self::NotInSubset
            | Self::ExportVectorType
            | Self::ExportUnsupportedType
            | Self::ExportCReservedName
            | Self::ExportSymbolCollision
            | Self::ExportModuleName => FailureClass::TypeOrLayout,
            Self::BoundUnproven => FailureClass::BoundProof,
            Self::EquivalenceUnproven | Self::EnsUnproven => FailureClass::EquivalenceProof,
            Self::SolverUnknown => FailureClass::SolverTimeout,
            Self::InternalError => FailureClass::ToolchainBug,
        }
    }
}
