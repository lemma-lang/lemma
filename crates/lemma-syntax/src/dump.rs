//! A range-free text rendering of the syntax tree, one line per construct.
//!
//! Two trees with the same dump differ at most in the byte ranges of their nodes, so the dump is
//! how tests compare what was parsed.

use crate::ast::{
    Binder, Block, Bound, Carry, Clause, EffClause, Effect, EquivClause, Expression,
    ExpressionKind, File, Function, Limit, Param, ParamKind, ReferenceType, SignatureType,
    Statement, StatementKind, UnaryOperator, ValueType,
};

/// Renders `file` as text: one line per construct, two spaces per level of nesting, expressions as
/// s-expressions, every line ending with a line feed.
pub fn dump(file: &File) -> String {
    let mut out = String::new();
    for function in &file.items {
        dump_function(&mut out, function);
    }
    out
}

fn line(out: &mut String, depth: usize, text: &str) {
    out.push_str(&"  ".repeat(depth));
    out.push_str(text);
    out.push('\n');
}

fn dump_function(out: &mut String, function: &Function) {
    let export = if function.export.is_some() {
        "export "
    } else {
        ""
    };
    line(out, 0, &format!("{export}fn {}", function.name.text));
    for param in &function.params {
        line(out, 1, &format!("param {}", param_text(param)));
    }
    line(
        out,
        1,
        &format!("result {}", signature_type(&function.result)),
    );
    line(out, 1, &effects_text(&function.effects));
    for clause in &function.requires {
        line(out, 1, &clause_text("req", clause));
    }
    for clause in &function.ensures {
        line(out, 1, &clause_text("ens", clause));
    }
    line(out, 1, &clause_text("spec", &function.spec));
    line(out, 1, &equiv_text(&function.equiv));
    line(out, 1, "impl");
    dump_block(out, 2, &function.body);
}

fn param_text(param: &Param) -> String {
    let kind = match &param.kind {
        ParamKind::Value(value) => signature_type(value),
        ParamKind::Reference(reference) => reference_type(reference),
    };
    format!("{} {kind}", param.name.text)
}

fn value_type(value: ValueType) -> String {
    match value {
        ValueType::Scalar(scalar) => scalar.to_string(),
        ValueType::Vector(vector) => vector.to_string(),
        ValueType::Bool => "bool".to_string(),
    }
}

fn limit_text(limit: &Limit) -> &str {
    match limit {
        Limit::Int(spelling) => spelling,
        Limit::Name(name) => &name.text,
    }
}

fn bounded(base: &str, bound: &Bound) -> String {
    format!(
        "(bounded {base} {} {})",
        bound.operator.symbol(),
        limit_text(&bound.limit)
    )
}

fn signature_type(signature: &SignatureType) -> String {
    let base = value_type(signature.value);
    match &signature.bound {
        Some(bound) => bounded(&base, bound),
        None => base,
    }
}

fn reference_type(reference: &ReferenceType) -> String {
    let base = reference.element.to_string();
    let element = match &reference.element_bound {
        Some(bound) => bounded(&base, bound),
        None => base,
    };
    format!(
        "(ref {element} {} @a{} {})",
        limit_text(&reference.extent),
        reference.alignment,
        reference.mode.symbol()
    )
}

fn effects_text(clause: &EffClause) -> String {
    let effects = clause.effects.iter().map(|effect| match effect {
        Effect::Pure => " pure",
    });
    format!("eff{}", effects.collect::<String>())
}

fn clause_text(keyword: &str, clause: &Clause) -> String {
    format!("{keyword} {}", expression_text(&clause.expression))
}

fn equiv_text(clause: &EquivClause) -> String {
    let permissions = clause
        .permissions
        .iter()
        .map(|permission| format!(" {}", permission.symbol()));
    format!("equiv{}", permissions.collect::<String>())
}

fn binder_text(binder: &Binder) -> String {
    format!("{} {}", binder.name.text, value_type(binder.value_type))
}

fn dump_block(out: &mut String, depth: usize, block: &Block) {
    for statement in &block.statements {
        dump_statement(out, depth, statement);
    }
}

fn dump_carry(out: &mut String, depth: usize, carry: &Carry) {
    let text = format!(
        "carry {} {}",
        binder_text(&carry.binder),
        expression_text(&carry.value)
    );
    line(out, depth, &text);
}

fn dump_statement(out: &mut String, depth: usize, statement: &Statement) {
    match &statement.kind {
        StatementKind::Let { binder, value } => {
            let text = format!("let {} {}", binder_text(binder), expression_text(value));
            line(out, depth, &text);
        }
        StatementKind::Loop {
            index,
            low,
            high,
            step,
            carries,
            body,
        } => {
            let text = format!(
                "loop {} {} {} step {step}",
                binder_text(index),
                expression_text(low),
                expression_text(high)
            );
            line(out, depth, &text);
            for carry in carries {
                dump_carry(out, depth + 1, carry);
            }
            dump_block(out, depth + 1, body);
        }
        StatementKind::Next { name, value } => {
            let text = format!("next {} {}", name.text, expression_text(value));
            line(out, depth, &text);
        }
        StatementKind::Return { value } => {
            line(out, depth, &format!("return {}", expression_text(value)));
        }
    }
}

fn list(expressions: &[Expression]) -> String {
    expressions
        .iter()
        .map(|expression| format!(" {}", expression_text(expression)))
        .collect()
}

fn quantifier_text(
    keyword: &str,
    binder: &Binder,
    low: &Expression,
    high: &Expression,
    body: &Expression,
) -> String {
    format!(
        "({keyword} {} {} {} {})",
        binder_text(binder),
        expression_text(low),
        expression_text(high),
        expression_text(body)
    )
}

fn expression_text(expression: &Expression) -> String {
    match &expression.kind {
        ExpressionKind::Int(spelling) | ExpressionKind::Float(spelling) => spelling.clone(),
        ExpressionKind::Bool(value) => value.to_string(),
        ExpressionKind::Result => "result".to_string(),
        ExpressionKind::Name(name) => name.clone(),
        ExpressionKind::Index { base, index } => {
            format!("(index {} {})", base.text, expression_text(index))
        }
        ExpressionKind::Call { callee, arguments } => {
            format!("(call {}{})", callee.text, list(arguments))
        }
        ExpressionKind::Member {
            vector,
            member,
            arguments,
        } => format!("(member {vector} {}{})", member.text, list(arguments)),
        ExpressionKind::Sum {
            binder,
            low,
            high,
            body,
        } => quantifier_text("sum", binder, low, high, body),
        ExpressionKind::Forall {
            binder,
            low,
            high,
            body,
        } => quantifier_text("forall", binder, low, high, body),
        ExpressionKind::Paren(inner) => format!("(paren {})", expression_text(inner)),
        ExpressionKind::Unary { operator, operand } => {
            let name = match operator {
                UnaryOperator::Negate => "neg",
                UnaryOperator::Not => "not",
            };
            format!("({name} {})", expression_text(operand))
        }
        ExpressionKind::Binary {
            operator,
            left,
            right,
        } => format!(
            "({} {} {})",
            operator.symbol(),
            expression_text(left),
            expression_text(right)
        ),
    }
}
