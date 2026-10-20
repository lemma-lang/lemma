//! The record rules of `docs/diagnostics.md` sections 1 to 3, as `Diagnostic::validate` checks them.

use lemma_diagnostics::{
    Assignment, Counterexample, Diagnostic, Edit, Kind, Obligation, Outputs, Position, Related,
    RuleViolation, Span,
};

type Expected = fn(&RuleViolation) -> bool;

const FILE: &str = "case.lm";

fn position((byte, line, column): (usize, usize, usize)) -> Position {
    Position { byte, line, column }
}

fn span_in(file: &str, start: (usize, usize, usize), end: (usize, usize, usize)) -> Span {
    Span {
        file: file.to_string(),
        start: position(start),
        end: position(end),
    }
}

fn span(start: (usize, usize, usize), end: (usize, usize, usize)) -> Span {
    span_in(FILE, start, end)
}

/// An edit on line 1, where a column is the byte offset plus one.
fn edit(file: &str, start: usize, end: usize, replacement: &str) -> Edit {
    Edit {
        span: span_in(file, (start, 1, start + 1), (end, 1, end + 1)),
        replacement: replacement.to_string(),
    }
}

fn related(span: Span) -> Vec<Related> {
    vec![Related {
        span,
        label: "the place".to_string(),
    }]
}

fn record(kind: Kind) -> Diagnostic {
    Diagnostic {
        kind,
        message: "what went wrong".to_string(),
        span: span((0, 1, 1), (1, 1, 2)),
        related: Vec::new(),
        obligation: None,
        counterexample: None,
        repair: Vec::new(),
    }
}

fn obligation() -> Option<Obligation> {
    Some(Obligation {
        about: "the bound of `a[k]`: `k < n`".to_string(),
        assumes: vec!["n % 4 == 0".to_string()],
    })
}

fn inputs() -> Vec<Assignment> {
    vec![Assignment {
        name: "n".to_string(),
        value: "12".to_string(),
    }]
}

fn counterexample() -> Option<Counterexample> {
    Some(Counterexample {
        inputs: inputs(),
        outputs: None,
    })
}

fn counterexample_with_outputs() -> Option<Counterexample> {
    Some(Counterexample {
        inputs: inputs(),
        outputs: Some(Outputs {
            spec: "1.0".to_string(),
            implementation: "0.5".to_string(),
        }),
    })
}

fn proof_failure(kind: Kind) -> Diagnostic {
    Diagnostic {
        obligation: obligation(),
        counterexample: counterexample(),
        ..record(kind)
    }
}

fn with_repair(kind: Kind, repair: Vec<Edit>) -> Diagnostic {
    Diagnostic {
        repair,
        ..record(kind)
    }
}

fn with_span(span: Span) -> Diagnostic {
    Diagnostic {
        span,
        ..record(Kind::UnexpectedToken)
    }
}

fn pick_record() -> Diagnostic {
    Diagnostic {
        kind: Kind::BoundInClause,
        message: "the clause `req k < n` states a bound, which belongs in the type of `k`"
            .to_string(),
        span: span_in("pick.lm", (70, 3, 3), (79, 3, 12)),
        related: vec![Related {
            span: span_in("pick.lm", (23, 1, 24), (24, 1, 25)),
            label: "the parameter the bound belongs to".to_string(),
        }],
        obligation: None,
        counterexample: None,
        repair: vec![
            Edit {
                span: span_in("pick.lm", (29, 1, 30), (29, 1, 30)),
                replacement: " < n".to_string(),
            },
            Edit {
                span: span_in("pick.lm", (68, 3, 1), (80, 4, 1)),
                replacement: String::new(),
            },
        ],
    }
}

fn dot_record() -> Diagnostic {
    Diagnostic {
        kind: Kind::BoundUnproven,
        message: "the load `f32x4.load(a, i + 12)` may read past the end of `a`".to_string(),
        span: span_in("dot.lm", (673, 17, 21), (694, 17, 42)),
        related: vec![Related {
            span: span_in("dot.lm", (159, 4, 3), (173, 4, 17)),
            label: "the clause the obligation assumes".to_string(),
        }],
        obligation: Some(Obligation {
            about: "the bound of `f32x4.load(a, i + 12)`: `i + 12 + 4 <= n`".to_string(),
            assumes: vec!["n % 4 == 0".to_string()],
        }),
        counterexample: Some(Counterexample {
            inputs: vec![
                Assignment {
                    name: "n".to_string(),
                    value: "12".to_string(),
                },
                Assignment {
                    name: "i".to_string(),
                    value: "0".to_string(),
                },
            ],
            outputs: None,
        }),
        repair: Vec::new(),
    }
}

fn valid_cases() -> Vec<(&'static str, Diagnostic)> {
    vec![
        ("syntax without repair", record(Kind::UnexpectedToken)),
        (
            "syntax with a one-edit repair",
            with_repair(Kind::Semicolon, vec![edit(FILE, 0, 1, "")]),
        ),
        ("type-or-layout worked example of pick.lm", pick_record()),
        ("bound-proof worked example of dot.lm", dot_record()),
        (
            "equivalence-unproven with outputs",
            Diagnostic {
                counterexample: counterexample_with_outputs(),
                ..proof_failure(Kind::EquivalenceUnproven)
            },
        ),
        (
            "ens-unproven with outputs",
            Diagnostic {
                counterexample: counterexample_with_outputs(),
                ..proof_failure(Kind::EnsUnproven)
            },
        ),
        (
            "ens-unproven without outputs",
            proof_failure(Kind::EnsUnproven),
        ),
        (
            "solver-unknown with an obligation",
            Diagnostic {
                obligation: obligation(),
                ..record(Kind::SolverUnknown)
            },
        ),
        (
            "internal-error without obligation",
            record(Kind::InternalError),
        ),
        (
            "internal-error with an obligation",
            Diagnostic {
                obligation: obligation(),
                ..record(Kind::InternalError)
            },
        ),
        (
            "counterexample with no inputs",
            Diagnostic {
                counterexample: Some(Counterexample {
                    inputs: Vec::new(),
                    outputs: None,
                }),
                ..proof_failure(Kind::BoundUnproven)
            },
        ),
        (
            "obligation assuming nothing",
            Diagnostic {
                obligation: Some(Obligation {
                    about: "the divisor `d` is not zero".to_string(),
                    assumes: Vec::new(),
                }),
                ..proof_failure(Kind::BoundUnproven)
            },
        ),
        ("empty main span", with_span(span((1, 1, 2), (1, 1, 2)))),
        (
            "edits sorted by file before start byte",
            with_repair(
                Kind::Semicolon,
                vec![edit("a.lm", 50, 51, ""), edit("b.lm", 0, 1, "")],
            ),
        ),
    ]
}

fn violation_cases() -> Vec<(&'static str, Diagnostic, Expected)> {
    vec![
        (
            "syntax with an obligation",
            Diagnostic {
                obligation: obligation(),
                ..record(Kind::UnexpectedToken)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedObligation { .. }),
        ),
        (
            "syntax with a counterexample",
            Diagnostic {
                counterexample: counterexample(),
                ..record(Kind::UnexpectedToken)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedCounterexample { .. }),
        ),
        (
            "type-or-layout with an obligation",
            Diagnostic {
                obligation: obligation(),
                ..record(Kind::UnknownName)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedObligation { .. }),
        ),
        (
            "type-or-layout with a counterexample",
            Diagnostic {
                counterexample: counterexample(),
                ..record(Kind::UnknownName)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedCounterexample { .. }),
        ),
        (
            "bound-proof without obligation",
            Diagnostic {
                obligation: None,
                ..proof_failure(Kind::BoundUnproven)
            },
            |violation| matches!(violation, RuleViolation::MissingObligation { .. }),
        ),
        (
            "bound-proof without counterexample",
            Diagnostic {
                counterexample: None,
                ..proof_failure(Kind::BoundUnproven)
            },
            |violation| matches!(violation, RuleViolation::MissingCounterexample { .. }),
        ),
        (
            "bound-proof with a repair",
            Diagnostic {
                repair: vec![edit(FILE, 0, 1, "")],
                ..proof_failure(Kind::BoundUnproven)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedRepair { .. }),
        ),
        (
            "equivalence-proof without obligation",
            Diagnostic {
                obligation: None,
                ..proof_failure(Kind::EnsUnproven)
            },
            |violation| matches!(violation, RuleViolation::MissingObligation { .. }),
        ),
        (
            "equivalence-proof without counterexample",
            Diagnostic {
                counterexample: None,
                ..proof_failure(Kind::EnsUnproven)
            },
            |violation| matches!(violation, RuleViolation::MissingCounterexample { .. }),
        ),
        (
            "equivalence-proof with a repair",
            Diagnostic {
                counterexample: counterexample_with_outputs(),
                repair: vec![edit(FILE, 0, 1, "")],
                ..proof_failure(Kind::EquivalenceUnproven)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedRepair { .. }),
        ),
        (
            "solver-timeout without obligation",
            record(Kind::SolverUnknown),
            |violation| matches!(violation, RuleViolation::MissingObligation { .. }),
        ),
        (
            "solver-timeout with a counterexample",
            Diagnostic {
                obligation: obligation(),
                counterexample: counterexample(),
                ..record(Kind::SolverUnknown)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedCounterexample { .. }),
        ),
        (
            "solver-timeout with a repair",
            Diagnostic {
                obligation: obligation(),
                repair: vec![edit(FILE, 0, 1, "")],
                ..record(Kind::SolverUnknown)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedRepair { .. }),
        ),
        (
            "toolchain-bug with a counterexample",
            Diagnostic {
                counterexample: counterexample(),
                ..record(Kind::InternalError)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedCounterexample { .. }),
        ),
        (
            "toolchain-bug with a repair",
            with_repair(Kind::InternalError, vec![edit(FILE, 0, 1, "")]),
            |violation| matches!(violation, RuleViolation::UnexpectedRepair { .. }),
        ),
        (
            "equivalence-unproven without outputs",
            proof_failure(Kind::EquivalenceUnproven),
            |violation| matches!(violation, RuleViolation::MissingOutputs { .. }),
        ),
        (
            "bound-unproven with outputs",
            Diagnostic {
                counterexample: counterexample_with_outputs(),
                ..proof_failure(Kind::BoundUnproven)
            },
            |violation| matches!(violation, RuleViolation::UnexpectedOutputs { .. }),
        ),
        (
            "line 0 in the main span",
            with_span(span((0, 0, 1), (1, 1, 2))),
            |violation| matches!(violation, RuleViolation::ZeroLineOrColumn { .. }),
        ),
        (
            "column 0 in a related span",
            Diagnostic {
                related: related(span((0, 1, 0), (1, 1, 2))),
                ..record(Kind::UnexpectedToken)
            },
            |violation| matches!(violation, RuleViolation::ZeroLineOrColumn { .. }),
        ),
        (
            "line 0 in an edit span",
            with_repair(
                Kind::Semicolon,
                vec![Edit {
                    span: span((0, 0, 1), (2, 1, 3)),
                    replacement: String::new(),
                }],
            ),
            |violation| matches!(violation, RuleViolation::ZeroLineOrColumn { .. }),
        ),
        (
            "main span reversed by byte",
            with_span(span((5, 1, 6), (3, 1, 6))),
            |violation| matches!(violation, RuleViolation::ReversedSpan { .. }),
        ),
        (
            "main span reversed by line",
            with_span(span((3, 2, 1), (5, 1, 9))),
            |violation| matches!(violation, RuleViolation::ReversedSpan { .. }),
        ),
        (
            "main span reversed by column on one line",
            with_span(span((3, 1, 6), (5, 1, 4))),
            |violation| matches!(violation, RuleViolation::ReversedSpan { .. }),
        ),
        (
            "related span reversed",
            Diagnostic {
                related: related(span((5, 1, 6), (3, 1, 4))),
                ..record(Kind::UnexpectedToken)
            },
            |violation| matches!(violation, RuleViolation::ReversedSpan { .. }),
        ),
        (
            "edit span reversed",
            with_repair(Kind::Semicolon, vec![edit(FILE, 5, 3, "x")]),
            |violation| matches!(violation, RuleViolation::ReversedSpan { .. }),
        ),
        (
            "edits unsorted by file",
            with_repair(
                Kind::Semicolon,
                vec![edit("b.lm", 0, 1, ""), edit("a.lm", 0, 1, "")],
            ),
            |violation| matches!(violation, RuleViolation::UnsortedEdits { .. }),
        ),
        (
            "edits unsorted by start byte in one file",
            with_repair(
                Kind::Semicolon,
                vec![edit(FILE, 20, 22, ""), edit(FILE, 0, 1, "")],
            ),
            |violation| matches!(violation, RuleViolation::UnsortedEdits { .. }),
        ),
        (
            "overlapping edits",
            with_repair(
                Kind::Semicolon,
                vec![edit(FILE, 0, 5, ""), edit(FILE, 3, 8, "")],
            ),
            |violation| matches!(violation, RuleViolation::EditsNotSeparated { .. }),
        ),
        (
            "edits where the first ends at the second's start",
            with_repair(
                Kind::Semicolon,
                vec![edit(FILE, 0, 3, ""), edit(FILE, 3, 5, "")],
            ),
            |violation| matches!(violation, RuleViolation::EditsNotSeparated { .. }),
        ),
        (
            "two insertions at one point",
            with_repair(
                Kind::Semicolon,
                vec![edit(FILE, 3, 3, "x"), edit(FILE, 3, 3, "y")],
            ),
            |violation| matches!(violation, RuleViolation::EditsNotSeparated { .. }),
        ),
        (
            "edit with an empty span and an empty replacement",
            with_repair(Kind::Semicolon, vec![edit(FILE, 3, 3, "")]),
            |violation| matches!(violation, RuleViolation::EmptyEdit { .. }),
        ),
    ]
}

#[test]
fn valid_records_pass() {
    let cases = valid_cases();

    let refused: Vec<(&str, Result<(), RuleViolation>)> = cases
        .iter()
        .map(|(name, record)| (*name, record.validate()))
        .filter(|(_, result)| result.is_err())
        .collect();

    assert!(refused.is_empty(), "refused: {refused:#?}");
}

#[test]
fn each_broken_rule_is_reported_with_its_violation() {
    let cases = violation_cases();

    let misreported: Vec<(&str, Result<(), RuleViolation>)> = cases
        .iter()
        .map(|(name, record, expected)| (*name, record.validate(), expected))
        .filter(|(_, result, expected)| !result.as_ref().err().is_some_and(expected))
        .map(|(name, result, _)| (name, result))
        .collect();

    assert!(misreported.is_empty(), "misreported: {misreported:#?}");
}
