//! Generators of syntax trees the parser can produce, and of texts that lay out their printed form.
//!
//! A generated tree respects what the parser checks: names are not reserved, parentheses stand
//! where precedence requires them, the structure rules of `docs/grammar.md` section 3 hold, and no
//! token opens a level deeper than 16 by the count of its "Nesting depth" section.

use std::collections::BTreeSet;

use lemma_syntax::{
    AccessMode, BinaryOperator, Binder, Block, Bound, BoundOperator, C1Variant, Carry, Clause,
    EffClause, Effect, EquivClause, Expression, ExpressionKind, File, Function, Limit, Name, Param,
    ParamKind, Permission, ReferenceType, ScalarClass, ScalarType, SignatureType, Statement,
    StatementKind, TextRange, UnaryOperator, ValueType, VectorType, print,
};
use proptest::collection::{btree_set, vec};
use proptest::prelude::*;
use proptest::sample::select;
use proptest::strategy::Union;

const NOWHERE: TextRange = TextRange { start: 0, end: 0 };

const NESTING_LIMIT: usize = 16;

/// How many expressions deep a generated expression goes, whatever levels they open.
const EXPRESSION_FUEL: u32 = 4;

/// How many loops deep a generated block goes.
const LOOP_FUEL: u32 = 2;

const PRIMARY: u8 = 0;
const UNARY: u8 = 1;
const MULTIPLICATIVE: u8 = 2;
const ADDITIVE: u8 = 3;
const COMPARISON: u8 = 4;
const CONJUNCTION: u8 = 5;
const DISJUNCTION: u8 = 6;

const KEYWORDS: [&str; 27] = [
    "export",
    "fn",
    "eff",
    "req",
    "ens",
    "spec",
    "equiv",
    "allow",
    "impl",
    "let",
    "loop",
    "step",
    "carry",
    "next",
    "return",
    "result",
    "true",
    "false",
    "ro",
    "wo",
    "pure",
    "fp_reassoc",
    "fp_contract",
    "sum",
    "in",
    "forall",
    "bool",
];

const SCALAR_TYPES: [ScalarType; 13] = [
    scalar(ScalarClass::Unsigned, 8),
    scalar(ScalarClass::Unsigned, 16),
    scalar(ScalarClass::Unsigned, 32),
    scalar(ScalarClass::Unsigned, 64),
    scalar(ScalarClass::Unsigned, 128),
    scalar(ScalarClass::Signed, 8),
    scalar(ScalarClass::Signed, 16),
    scalar(ScalarClass::Signed, 32),
    scalar(ScalarClass::Signed, 64),
    scalar(ScalarClass::Signed, 128),
    scalar(ScalarClass::Float, 16),
    scalar(ScalarClass::Float, 32),
    scalar(ScalarClass::Float, 64),
];

const INTEGER_TYPES: [ScalarType; 10] = [
    scalar(ScalarClass::Unsigned, 8),
    scalar(ScalarClass::Unsigned, 16),
    scalar(ScalarClass::Unsigned, 32),
    scalar(ScalarClass::Unsigned, 64),
    scalar(ScalarClass::Unsigned, 128),
    scalar(ScalarClass::Signed, 8),
    scalar(ScalarClass::Signed, 16),
    scalar(ScalarClass::Signed, 32),
    scalar(ScalarClass::Signed, 64),
    scalar(ScalarClass::Signed, 128),
];

const LANES: [u8; 6] = [2, 4, 8, 16, 32, 64];

const ALIGNMENTS: [&str; 8] = ["1", "2", "4", "8", "16", "32", "64", "128"];

/// The words that begin a line of the canonical form a comment may stand before: an item, a
/// clause or a statement.
const COMMENTABLE_LINE_STARTS: [&str; 12] = [
    "export", "fn", "eff", "req", "ens", "spec", "equiv", "impl", "let", "loop", "next", "return",
];

const FORMS: [Form; 21] = [
    Form::Negate,
    Form::Not,
    Form::Paren,
    Form::Index,
    Form::Call,
    Form::Member,
    Form::Sum,
    Form::Forall,
    Form::Binary(BinaryOperator::Multiply),
    Form::Binary(BinaryOperator::Divide),
    Form::Binary(BinaryOperator::Remainder),
    Form::Binary(BinaryOperator::Add),
    Form::Binary(BinaryOperator::Subtract),
    Form::Binary(BinaryOperator::Equal),
    Form::Binary(BinaryOperator::NotEqual),
    Form::Binary(BinaryOperator::Less),
    Form::Binary(BinaryOperator::LessEqual),
    Form::Binary(BinaryOperator::Greater),
    Form::Binary(BinaryOperator::GreaterEqual),
    Form::Binary(BinaryOperator::And),
    Form::Binary(BinaryOperator::Or),
];

/// What an expression may still spend: nesting levels below the limit, generation depth, and
/// whether `result` may appear.
#[derive(Debug, Clone, Copy)]
struct Budget {
    remaining_levels: usize,
    fuel: u32,
    result_allowed: bool,
}

/// The place an expression goes: the loosest precedence level it may have without parentheses,
/// whether a `!` expression may stand there bare, and its budget.
#[derive(Debug, Clone, Copy)]
struct Slot {
    loosest: u8,
    bare_not: bool,
    budget: Budget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Negate,
    Not,
    Paren,
    Index,
    Call,
    Member,
    Sum,
    Forall,
    Binary(BinaryOperator),
}

impl Form {
    fn level(self) -> u8 {
        match self {
            Self::Negate | Self::Not => UNARY,
            Self::Binary(operator) => binary_level(operator),
            Self::Paren | Self::Index | Self::Call | Self::Member | Self::Sum | Self::Forall => {
                PRIMARY
            }
        }
    }

    fn opens_a_level(self) -> bool {
        !matches!(self, Self::Binary(_))
    }
}

#[derive(Debug, Clone, Copy)]
enum Ending {
    LineFeed,
    CarriageReturnLineFeed,
    Space,
}

/// How one line of the canonical form is laid out in a generated text.
#[derive(Debug, Clone)]
struct LineLayout {
    empty_lines_before: usize,
    comments: Vec<String>,
    indentation: String,
    trailing_whitespace: String,
    ending: Ending,
}

const fn scalar(class: ScalarClass, bits: u16) -> ScalarType {
    ScalarType { class, bits }
}

fn binary_level(operator: BinaryOperator) -> u8 {
    match operator {
        BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder => {
            MULTIPLICATIVE
        }
        BinaryOperator::Add | BinaryOperator::Subtract => ADDITIVE,
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => COMPARISON,
        BinaryOperator::And => CONJUNCTION,
        BinaryOperator::Or => DISJUNCTION,
    }
}

fn is_reserved(word: &str) -> bool {
    let is_scalar_name = SCALAR_TYPES.iter().any(|scalar| scalar.to_string() == word);
    let is_vector_name = SCALAR_TYPES.iter().any(|&element| {
        LANES
            .iter()
            .any(|&lanes| VectorType { element, lanes }.to_string() == word)
    });
    KEYWORDS.contains(&word) || is_scalar_name || is_vector_name
}

fn named(text: String) -> Name {
    Name {
        text,
        range: NOWHERE,
    }
}

fn expression_of(kind: ExpressionKind) -> Expression {
    Expression {
        kind,
        range: NOWHERE,
    }
}

fn parenthesized(inner: Expression) -> Expression {
    expression_of(ExpressionKind::Paren(Box::new(inner)))
}

fn unary(operator: UnaryOperator, operand: Expression) -> Expression {
    expression_of(ExpressionKind::Unary {
        operator,
        operand: Box::new(operand),
    })
}

fn statement_of(kind: StatementKind) -> Statement {
    Statement {
        kind,
        range: NOWHERE,
    }
}

fn identifier() -> BoxedStrategy<String> {
    "[a-z][a-z0-9_]{0,5}"
        .prop_filter("a reserved word is not a name", |word| !is_reserved(word))
        .boxed()
}

fn name() -> BoxedStrategy<Name> {
    identifier().prop_map(named).boxed()
}

fn int_literal() -> BoxedStrategy<String> {
    "0|[1-9][0-9]{0,5}".boxed()
}

fn float_literal() -> BoxedStrategy<String> {
    "(0|[1-9][0-9]{0,2})\\.[0-9]{1,3}(e-?[0-9]{1,2})?".boxed()
}

fn scalar_type() -> BoxedStrategy<ScalarType> {
    select(SCALAR_TYPES.to_vec()).boxed()
}

fn integer_type() -> BoxedStrategy<ScalarType> {
    select(INTEGER_TYPES.to_vec()).boxed()
}

fn vector_type() -> BoxedStrategy<VectorType> {
    (scalar_type(), select(LANES.to_vec()))
        .prop_map(|(element, lanes)| VectorType { element, lanes })
        .boxed()
}

fn value_type() -> BoxedStrategy<ValueType> {
    prop_oneof![
        scalar_type().prop_map(ValueType::Scalar),
        vector_type().prop_map(ValueType::Vector),
        Just(ValueType::Bool),
    ]
    .boxed()
}

fn binder() -> BoxedStrategy<Binder> {
    (name(), value_type())
        .prop_map(|(name, value_type)| Binder {
            name,
            value_type,
            range: NOWHERE,
        })
        .boxed()
}

fn limit() -> BoxedStrategy<Limit> {
    prop_oneof![
        int_literal().prop_map(Limit::Int),
        name().prop_map(Limit::Name),
    ]
    .boxed()
}

fn bound() -> BoxedStrategy<Bound> {
    (
        select(vec![BoundOperator::Less, BoundOperator::LessEqual]),
        limit(),
    )
        .prop_map(|(operator, limit)| Bound {
            operator,
            limit,
            range: NOWHERE,
        })
        .boxed()
}

fn signature_type() -> BoxedStrategy<SignatureType> {
    let unbounded = value_type().prop_map(|value| SignatureType {
        value,
        bound: None,
        range: NOWHERE,
    });
    let bounded = (integer_type(), bound()).prop_map(|(scalar, bound)| SignatureType {
        value: ValueType::Scalar(scalar),
        bound: Some(bound),
        range: NOWHERE,
    });
    prop_oneof![unbounded, bounded].boxed()
}

fn reference_type() -> BoxedStrategy<ReferenceType> {
    let element = prop_oneof![
        scalar_type().prop_map(|element| (element, None)),
        (integer_type(), bound()).prop_map(|(element, bound)| (element, Some(bound))),
    ];
    (
        element,
        limit(),
        select(ALIGNMENTS.to_vec()),
        select(vec![AccessMode::ReadOnly, AccessMode::WriteOnly]),
    )
        .prop_map(
            |((element, element_bound), extent, alignment, mode)| ReferenceType {
                element,
                element_bound,
                extent,
                alignment: alignment.to_string(),
                mode,
                range: NOWHERE,
            },
        )
        .boxed()
}

fn param() -> BoxedStrategy<Param> {
    let kind = prop_oneof![
        signature_type().prop_map(ParamKind::Value),
        reference_type().prop_map(ParamKind::Reference),
    ];
    (name(), kind)
        .prop_map(|(name, kind)| Param {
            name,
            kind,
            range: NOWHERE,
        })
        .boxed()
}

fn leaf(result_allowed: bool) -> BoxedStrategy<Expression> {
    let mut choices = vec![
        int_literal().prop_map(ExpressionKind::Int).boxed(),
        float_literal().prop_map(ExpressionKind::Float).boxed(),
        any::<bool>().prop_map(ExpressionKind::Bool).boxed(),
        identifier().prop_map(ExpressionKind::Name).boxed(),
    ];
    choices.extend(result_allowed.then(|| Just(ExpressionKind::Result).boxed()));
    Union::new(choices).prop_map(expression_of).boxed()
}

/// An expression for `slot`: a leaf once the fuel is spent, otherwise a leaf or any form the
/// remaining levels can hold, parenthesized when the slot does not take it bare.
fn expression(slot: Slot) -> BoxedStrategy<Expression> {
    let leaves = leaf(slot.budget.result_allowed);
    if slot.budget.fuel == 0 {
        return leaves;
    }
    let mut choices = vec![(8, leaves)];
    choices.extend(
        FORMS
            .iter()
            .filter_map(|&form| placed(form, slot))
            .map(|strategy| (1, strategy)),
    );
    Union::new_weighted(choices).boxed()
}

/// `form` in `slot`, or `None` when the levels it opens would pass the limit.
fn placed(form: Form, slot: Slot) -> Option<BoxedStrategy<Expression>> {
    let wrapped = form.level() > slot.loosest || (form == Form::Not && !slot.bare_not);
    let cost = usize::from(wrapped) + usize::from(form.opens_a_level());
    let inner = Budget {
        remaining_levels: slot.budget.remaining_levels.checked_sub(cost)?,
        fuel: slot.budget.fuel.saturating_sub(1),
        result_allowed: slot.budget.result_allowed,
    };
    let bare = Just(())
        .prop_flat_map(move |()| natural(form, inner))
        .boxed();
    let placed = if wrapped {
        bare.prop_map(parenthesized).boxed()
    } else {
        bare
    };
    Some(placed)
}

fn full_slot(budget: Budget) -> Slot {
    Slot {
        loosest: DISJUNCTION,
        bare_not: true,
        budget,
    }
}

fn range_slot(budget: Budget) -> Slot {
    Slot {
        loosest: ADDITIVE,
        bare_not: false,
        budget,
    }
}

/// `form` itself, its operands drawn within `inner`.
fn natural(form: Form, inner: Budget) -> BoxedStrategy<Expression> {
    let full = full_slot(inner);
    match form {
        Form::Negate => expression(Slot {
            loosest: UNARY,
            bare_not: false,
            budget: inner,
        })
        .prop_map(|operand| unary(UnaryOperator::Negate, operand))
        .boxed(),
        Form::Not => expression(Slot {
            loosest: PRIMARY,
            bare_not: false,
            budget: inner,
        })
        .prop_map(|operand| unary(UnaryOperator::Not, operand))
        .boxed(),
        Form::Paren => expression(full).prop_map(parenthesized).boxed(),
        Form::Index => (name(), expression(full))
            .prop_map(|(base, index)| {
                expression_of(ExpressionKind::Index {
                    base,
                    index: Box::new(index),
                })
            })
            .boxed(),
        Form::Call => (name(), vec(expression(full), 0..3))
            .prop_map(|(callee, arguments)| {
                expression_of(ExpressionKind::Call { callee, arguments })
            })
            .boxed(),
        Form::Member => (vector_type(), name(), vec(expression(full), 0..3))
            .prop_map(|(vector, member, arguments)| {
                expression_of(ExpressionKind::Member {
                    vector,
                    member,
                    arguments,
                })
            })
            .boxed(),
        Form::Sum => quantified(inner)
            .prop_map(|(binder, low, high, body)| {
                expression_of(ExpressionKind::Sum {
                    binder,
                    low: Box::new(low),
                    high: Box::new(high),
                    body: Box::new(body),
                })
            })
            .boxed(),
        Form::Forall => quantified(inner)
            .prop_map(|(binder, low, high, body)| {
                expression_of(ExpressionKind::Forall {
                    binder,
                    low: Box::new(low),
                    high: Box::new(high),
                    body: Box::new(body),
                })
            })
            .boxed(),
        Form::Binary(operator) => binary(operator, inner),
    }
}

/// The binder, range and body of a sum or a quantifier: the range sits inside its `(`, the body
/// inside its `{`, both one level below the expression.
fn quantified(
    inner: Budget,
) -> impl Strategy<Value = (Binder, Expression, Expression, Expression)> {
    (
        binder(),
        expression(range_slot(inner)),
        expression(range_slot(inner)),
        expression(full_slot(inner)),
    )
}

/// A binary operation: a comparison takes additive operands on both sides, the other operators
/// their own level on the left and a tighter one on the right; only `&&` and `||` take a bare `!`.
fn binary(operator: BinaryOperator, inner: Budget) -> BoxedStrategy<Expression> {
    let level = binary_level(operator);
    let left_loosest = if level == COMPARISON { ADDITIVE } else { level };
    let bare_not = level >= CONJUNCTION;
    let left = Slot {
        loosest: left_loosest,
        bare_not,
        budget: inner,
    };
    let right = Slot {
        loosest: level - 1,
        bare_not,
        budget: inner,
    };
    (expression(left), expression(right))
        .prop_map(move |(left, right)| {
            expression_of(ExpressionKind::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            })
        })
        .boxed()
}

/// The budget of an expression whose innermost open level has depth `depth`.
fn budget_at(depth: usize, result_allowed: bool) -> Budget {
    Budget {
        remaining_levels: NESTING_LIMIT - depth,
        fuel: EXPRESSION_FUEL,
        result_allowed,
    }
}

fn clause(result_allowed: bool) -> BoxedStrategy<Clause> {
    expression(full_slot(budget_at(0, result_allowed)))
        .prop_map(|expression| Clause {
            expression,
            range: NOWHERE,
        })
        .boxed()
}

fn equiv_clause() -> BoxedStrategy<EquivClause> {
    select(vec![
        Vec::new(),
        vec![Permission::FpReassoc],
        vec![Permission::FpContract],
        vec![Permission::FpReassoc, Permission::FpContract],
    ])
    .prop_map(|permissions| EquivClause {
        permissions,
        range: NOWHERE,
    })
    .boxed()
}

/// A `let` or, while loops may still nest, a `loop`, in a block whose `{` has depth `depth`.
fn inner_statement(depth: usize, loop_fuel: u32) -> BoxedStrategy<Statement> {
    let let_statement = (binder(), expression(full_slot(budget_at(depth, false))))
        .prop_map(|(binder, value)| statement_of(StatementKind::Let { binder, value }))
        .boxed();
    if loop_fuel == 0 || depth >= NESTING_LIMIT {
        return let_statement;
    }
    prop_oneof![2 => let_statement, 1 => loop_statement(depth, loop_fuel)].boxed()
}

/// A loop with distinct carries, whose body ends with exactly one `next` per carry.
fn loop_statement(depth: usize, loop_fuel: u32) -> BoxedStrategy<Statement> {
    let range = range_slot(budget_at(depth, false));
    let inside = depth + 1;
    (
        binder(),
        expression(range),
        expression(range),
        "[1-9][0-9]{0,2}",
        btree_set(identifier(), 1..4),
    )
        .prop_flat_map(move |(index, low, high, step, carry_names)| {
            let count = carry_names.len();
            (
                Just((index, low, high, step, carry_names)),
                vec(value_type(), count),
                vec(expression(full_slot(budget_at(inside, false))), count),
                vec(inner_statement(inside, loop_fuel - 1), 0..3),
                vec(expression(full_slot(budget_at(inside, false))), count),
            )
        })
        .prop_map(
            |((index, low, high, step, carry_names), types, initials, statements, nexts)| {
                assembled_loop(
                    (index, low, high, step),
                    carry_names,
                    (types, initials),
                    statements,
                    nexts,
                )
            },
        )
        .boxed()
}

fn assembled_loop(
    (index, low, high, step): (Binder, Expression, Expression, String),
    carry_names: BTreeSet<String>,
    (types, initials): (Vec<ValueType>, Vec<Expression>),
    mut statements: Vec<Statement>,
    nexts: Vec<Expression>,
) -> Statement {
    let carries = carry_names
        .iter()
        .zip(types)
        .zip(initials)
        .map(|((carry_name, value_type), value)| Carry {
            binder: Binder {
                name: named(carry_name.clone()),
                value_type,
                range: NOWHERE,
            },
            value,
            range: NOWHERE,
        })
        .collect();
    statements.extend(
        carry_names
            .into_iter()
            .zip(nexts)
            .map(|(carry_name, value)| {
                statement_of(StatementKind::Next {
                    name: named(carry_name),
                    value,
                })
            }),
    );
    statement_of(StatementKind::Loop {
        index,
        low,
        high,
        step,
        carries,
        body: Block {
            statements,
            range: NOWHERE,
        },
    })
}

/// An `impl` block: its `{` has depth 1, and it ends with its one `return`.
fn impl_block() -> BoxedStrategy<Block> {
    (
        vec(inner_statement(1, LOOP_FUEL), 0..3),
        expression(full_slot(budget_at(1, false))),
    )
        .prop_map(|(mut statements, value)| {
            statements.push(statement_of(StatementKind::Return { value }));
            Block {
                statements,
                range: NOWHERE,
            }
        })
        .boxed()
}

fn function() -> BoxedStrategy<Function> {
    (
        any::<bool>(),
        name(),
        vec(param(), 0..4),
        signature_type(),
        vec(clause(false), 0..3),
        vec(clause(true), 0..3),
        clause(false),
        equiv_clause(),
        impl_block(),
    )
        .prop_map(
            |(exported, name, params, result, requires, ensures, spec, equiv, body)| Function {
                export: exported.then_some(NOWHERE),
                name,
                params,
                result,
                effects: EffClause {
                    effects: vec![Effect::Pure],
                    range: NOWHERE,
                },
                requires,
                ensures,
                spec,
                equiv,
                body,
                range: NOWHERE,
            },
        )
        .boxed()
}

/// A file of zero to two functions the parser can produce.
pub fn file() -> BoxedStrategy<File> {
    vec(function(), 0..3)
        .prop_map(|items| File {
            items,
            range: NOWHERE,
        })
        .boxed()
}

/// A comment in printable ASCII and tab, trailing spaces and tabs included.
fn comment() -> BoxedStrategy<String> {
    "//[ -~\t]{0,12}".boxed()
}

fn line_layout() -> BoxedStrategy<LineLayout> {
    (
        0..3_usize,
        prop_oneof![3 => Just(Vec::new()), 1 => vec(comment(), 1..3)],
        "[ \t]{0,6}",
        "[ \t]{0,2}",
        select(vec![
            Ending::LineFeed,
            Ending::CarriageReturnLineFeed,
            Ending::Space,
        ]),
    )
        .prop_map(
            |(empty_lines_before, comments, indentation, trailing_whitespace, ending)| LineLayout {
                empty_lines_before,
                comments,
                indentation,
                trailing_whitespace,
                ending,
            },
        )
        .boxed()
}

fn takes_comments(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|word| COMMENTABLE_LINE_STARTS.contains(&word))
}

fn line_end(ending: Ending, comments_follow: bool) -> &'static str {
    match (ending, comments_follow) {
        (Ending::CarriageReturnLineFeed, _) => "\r\n",
        (Ending::Space, false) => " ",
        (Ending::LineFeed | Ending::Space, _) => "\n",
    }
}

/// The lines of a canonical text laid out as `layouts` say; a line is joined to the next by a
/// space only when no comment stands before the next one.
fn assembled_text(lines: &[&str], layouts: &[LineLayout]) -> String {
    let commented: Vec<bool> = lines
        .iter()
        .zip(layouts)
        .map(|(line, layout)| takes_comments(line) && !layout.comments.is_empty())
        .collect();
    let mut text = String::new();
    for (index, ((line, layout), &has_comments)) in
        lines.iter().zip(layouts).zip(&commented).enumerate()
    {
        let comments_follow = commented.get(index + 1).copied().unwrap_or(false);
        let comment_lines: String = layout
            .comments
            .iter()
            .filter(|_| has_comments)
            .map(|comment| std::format!("{}{comment}\n", layout.indentation))
            .collect();
        text.push_str(&"\n".repeat(layout.empty_lines_before));
        text.push_str(&comment_lines);
        text.push_str(&layout.indentation);
        text.push_str(line);
        text.push_str(&layout.trailing_whitespace);
        text.push_str(line_end(layout.ending, comments_follow));
    }
    text
}

/// A generated file with its printed text laid out anew: lines indented at random, ended by LF
/// or CRLF or joined by a space, empty lines between them, and comments on their own line before
/// items, clauses and statements.
pub fn laid_out_file(variant: C1Variant) -> BoxedStrategy<(File, String)> {
    file()
        .prop_flat_map(move |file| {
            let printed = print(&file, variant);
            let line_count = printed.lines().count();
            (Just(file), Just(printed), vec(line_layout(), line_count))
        })
        .prop_map(|(file, printed, layouts)| {
            let lines: Vec<&str> = printed.lines().collect();
            let text = assembled_text(&lines, &layouts);
            (file, text)
        })
        .boxed()
}

/// `text` with its comments removed and every run of whitespace replaced by one space.
pub fn on_one_line_without_comments(text: &str) -> String {
    let code: Vec<&str> = text
        .split('\n')
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect();
    let mut collapsed = String::new();
    let mut in_whitespace = false;
    for character in code.join("\n").chars() {
        let is_whitespace = matches!(character, ' ' | '\t' | '\n' | '\r');
        if !is_whitespace {
            collapsed.push(character);
        } else if !in_whitespace {
            collapsed.push(' ');
        }
        in_whitespace = is_whitespace;
    }
    collapsed
}
