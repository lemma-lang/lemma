//! The canonical form of `docs/grammar.md` section 5: the printer of the syntax tree and the
//! formatter, which keeps the comments of a text that has no syntax diagnostic.

use lemma_diagnostics::{Diagnostic, Source};

use crate::ast::{
    Binder, Block, Bound, Carry, EquivClause, Expression, ExpressionKind, File, Function, Limit,
    Param, ParamKind, ReferenceType, SignatureType, Statement, StatementKind, UnaryOperator,
    ValueType,
};
use crate::parser::parse;
use crate::token::{C1Symbol, C1Variant, Comment};

/// Prints `file` in the canonical form, with the C1 symbols spelled for `variant`.
///
/// The tree holds no comment, so none is printed; [`format()`] keeps them.
pub fn print(file: &File, variant: C1Variant) -> String {
    Printer::new(variant, &[]).file(file)
}

/// Formats `source` in the canonical form, comments kept, under the given C1 variant.
///
/// # Errors
///
/// Returns every diagnostic the parser reports when it reports any, with or without a repair:
/// the formatter never formats around an error.
pub fn format(source: Source<'_>, variant: C1Variant) -> Result<String, Vec<Diagnostic>> {
    let parsed = parse(source, variant);
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics);
    }
    Ok(Printer::new(variant, &parsed.comments).file(&parsed.file))
}

const INDENTATION: &str = "  ";

struct Printer<'a> {
    variant: C1Variant,
    comments: &'a [Comment],
    next_comment: usize,
    out: String,
}

impl<'a> Printer<'a> {
    fn new(variant: C1Variant, comments: &'a [Comment]) -> Self {
        Self {
            variant,
            comments,
            next_comment: 0,
            out: String::new(),
        }
    }

    fn file(mut self, file: &File) -> String {
        for (index, function) in file.items.iter().enumerate() {
            if index > 0 {
                self.out.push('\n');
            }
            self.function(function);
        }
        self.remaining_comments();
        self.out
    }

    fn symbol(&self, symbol: C1Symbol) -> &'static str {
        symbol.spelling(self.variant)
    }

    fn line(&mut self, depth: usize, text: &str) {
        self.out.push_str(&INDENTATION.repeat(depth));
        self.out.push_str(text);
        self.out.push('\n');
    }

    /// Prints the comments that precede the byte `start`, then the line of the construct there.
    fn construct(&mut self, depth: usize, start: usize, text: &str) {
        let pending = self.comments.get(self.next_comment..).unwrap_or_default();
        let before = pending
            .iter()
            .take_while(|comment| comment.range.start < start)
            .count();
        for comment in pending.iter().take(before) {
            self.line(depth, comment.text.trim_end_matches([' ', '\t']));
        }
        self.next_comment += before;
        self.line(depth, text);
    }

    /// Prints the comments no construct follows, at column 0.
    fn remaining_comments(&mut self) {
        let pending = self.comments.get(self.next_comment..).unwrap_or_default();
        for comment in pending {
            self.line(0, comment.text.trim_end_matches([' ', '\t']));
        }
        self.next_comment = self.comments.len();
    }

    fn function(&mut self, function: &Function) {
        let export = if function.export.is_some() {
            "export "
        } else {
            ""
        };
        let params: Vec<String> = function.params.iter().map(param).collect();
        let header = format!(
            "{export}fn {}({}) -> {}",
            function.name.text,
            params.join(", "),
            signature_type(&function.result)
        );
        self.construct(0, function.range.start, &header);
        let effects = vec!["pure"; function.effects.effects.len()].join(", ");
        self.construct(1, function.effects.range.start, &format!("eff {effects}"));
        for clause in &function.requires {
            let text = format!("req {}", self.expression(&clause.expression));
            self.construct(1, clause.range.start, &text);
        }
        for clause in &function.ensures {
            let text = format!("ens {}", self.expression(&clause.expression));
            self.construct(1, clause.range.start, &text);
        }
        let spec = format!("spec {}", self.expression(&function.spec.expression));
        self.construct(1, function.spec.range.start, &spec);
        self.construct(1, function.equiv.range.start, &equiv(&function.equiv));
        self.construct(1, function.body.range.start, "impl {");
        self.statements(2, &function.body);
        self.line(1, "}");
    }

    fn statements(&mut self, depth: usize, block: &Block) {
        for statement in &block.statements {
            self.statement(depth, statement);
        }
    }

    fn statement(&mut self, depth: usize, statement: &Statement) {
        let start = statement.range.start;
        match &statement.kind {
            StatementKind::Let {
                binder: bound,
                value,
            } => {
                let text = format!("let {} = {}", binder(bound), self.expression(value));
                self.construct(depth, start, &text);
            }
            StatementKind::Loop {
                index,
                low,
                high,
                step,
                carries,
                body,
            } => {
                let header = format!(
                    "loop {} {} {}..<{} step {step}",
                    binder(index),
                    self.symbol(C1Symbol::In),
                    self.expression(low),
                    self.expression(high)
                );
                self.loop_header(depth, start, &header, carries);
                self.statements(depth + 1, body);
                self.line(depth, "}");
            }
            StatementKind::Next { name, value } => {
                let text = format!("next {} = {}", name.text, self.expression(value));
                self.construct(depth, start, &text);
            }
            StatementKind::Return { value } => {
                let text = format!("return {}", self.expression(value));
                self.construct(depth, start, &text);
            }
        }
    }

    /// A loop line: one carry stays on it, several take a line each.
    fn loop_header(&mut self, depth: usize, start: usize, header: &str, carries: &[Carry]) {
        let texts: Vec<String> = carries
            .iter()
            .map(|carry| {
                format!(
                    "{} = {}",
                    binder(&carry.binder),
                    self.expression(&carry.value)
                )
            })
            .collect();
        if texts.len() <= 1 {
            let text = format!("{header} carry({}) {{", texts.join(""));
            self.construct(depth, start, &text);
            return;
        }
        self.construct(depth, start, &format!("{header} carry("));
        let count = texts.len();
        for (index, text) in texts.iter().enumerate() {
            let separator = if index + 1 == count { "" } else { "," };
            self.line(depth + 1, &format!("{text}{separator}"));
        }
        self.line(depth, ") {");
    }

    fn expression(&self, expression: &Expression) -> String {
        match &expression.kind {
            ExpressionKind::Int(spelling) | ExpressionKind::Float(spelling) => spelling.clone(),
            ExpressionKind::Bool(value) => value.to_string(),
            ExpressionKind::Result => "result".to_string(),
            ExpressionKind::Name(name) => name.clone(),
            ExpressionKind::Index { base, index } => {
                format!("{}[{}]", base.text, self.expression(index))
            }
            ExpressionKind::Call { callee, arguments } => {
                format!("{}({})", callee.text, self.arguments(arguments))
            }
            ExpressionKind::Member {
                vector,
                member,
                arguments,
            } => format!("{vector}.{}({})", member.text, self.arguments(arguments)),
            ExpressionKind::Sum {
                binder,
                low,
                high,
                body,
            } => self.quantifier(C1Symbol::Sum, binder, low, high, body),
            ExpressionKind::Forall {
                binder,
                low,
                high,
                body,
            } => self.quantifier(C1Symbol::Forall, binder, low, high, body),
            ExpressionKind::Paren(inner) => format!("({})", self.expression(inner)),
            ExpressionKind::Unary { operator, operand } => self.unary(*operator, operand),
            ExpressionKind::Binary {
                operator,
                left,
                right,
            } => format!(
                "{} {} {}",
                self.expression(left),
                operator.symbol(),
                self.expression(right)
            ),
        }
    }

    fn arguments(&self, arguments: &[Expression]) -> String {
        let texts: Vec<String> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        texts.join(", ")
    }

    /// A lone `--` lexes as an error token, so a minus before a minus keeps a space.
    fn unary(&self, operator: UnaryOperator, operand: &Expression) -> String {
        let operand = self.expression(operand);
        let separator = if operator == UnaryOperator::Negate && operand.starts_with('-') {
            " "
        } else {
            ""
        };
        format!("{}{separator}{operand}", operator.symbol())
    }

    fn quantifier(
        &self,
        symbol: C1Symbol,
        bound: &Binder,
        low: &Expression,
        high: &Expression,
        body: &Expression,
    ) -> String {
        format!(
            "{}({} {} {}..<{}) {{ {} }}",
            self.symbol(symbol),
            binder(bound),
            self.symbol(C1Symbol::In),
            self.expression(low),
            self.expression(high),
            self.expression(body)
        )
    }
}

fn binder(binder: &Binder) -> String {
    format!("{}: {}", binder.name.text, value_type(binder.value_type))
}

fn value_type(value: ValueType) -> String {
    match value {
        ValueType::Scalar(scalar) => scalar.to_string(),
        ValueType::Vector(vector) => vector.to_string(),
        ValueType::Bool => "bool".to_string(),
    }
}

fn limit(limit: &Limit) -> &str {
    match limit {
        Limit::Int(spelling) => spelling,
        Limit::Name(name) => &name.text,
    }
}

fn with_bound(base: &str, bound: Option<&Bound>) -> String {
    match bound {
        Some(bound) => format!("{base} {} {}", bound.operator.symbol(), limit(&bound.limit)),
        None => base.to_string(),
    }
}

fn signature_type(signature: &SignatureType) -> String {
    with_bound(&value_type(signature.value), signature.bound.as_ref())
}

fn reference_type(reference: &ReferenceType) -> String {
    let base = reference.element.to_string();
    let element = match &reference.element_bound {
        Some(bound) => format!("({})", with_bound(&base, Some(bound))),
        None => base,
    };
    format!(
        "&{element}[{}] @a{} {}",
        limit(&reference.extent),
        reference.alignment,
        reference.mode.symbol()
    )
}

fn param(param: &Param) -> String {
    let kind = match &param.kind {
        ParamKind::Value(value) => signature_type(value),
        ParamKind::Reference(reference) => reference_type(reference),
    };
    format!("{}: {kind}", param.name.text)
}

fn equiv(clause: &EquivClause) -> String {
    let permissions: Vec<&str> = clause
        .permissions
        .iter()
        .map(|permission| permission.symbol())
        .collect();
    format!("equiv allow({})", permissions.join(", "))
}
