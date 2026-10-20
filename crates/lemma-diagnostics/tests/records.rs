//! The R rendering and the order of records, per `docs/diagnostics.md` sections 4 and 5.

use lemma_diagnostics::{
    Assignment, Counterexample, Diagnostic, Edit, Kind, Obligation, Outputs, Position, Related,
    Span, render_records,
};

const DIAGNOSTICS: &str = include_str!("../../../docs/diagnostics.md");

const PICK: &str = include_str!("testdata/records/pick.jsonl");
const DOT: &str = include_str!("testdata/records/dot.jsonl");

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

fn record(kind: Kind) -> Diagnostic {
    Diagnostic {
        kind,
        message: "a message".to_string(),
        span: span_in("case.lm", (4, 1, 5), (9, 1, 10)),
        related: Vec::new(),
        obligation: None,
        counterexample: None,
        repair: Vec::new(),
    }
}

/// A record on line 1 of `file`, where a column is the byte offset plus one.
fn on_line_1(kind: Kind, file: &str, start: usize, end: usize) -> Diagnostic {
    Diagnostic {
        span: span_in(file, (start, 1, start + 1), (end, 1, end + 1)),
        ..record(kind)
    }
}

fn obligation(about: &str, assumes: &[&str]) -> Option<Obligation> {
    Some(Obligation {
        about: about.to_string(),
        assumes: assumes.iter().map(|clause| clause.to_string()).collect(),
    })
}

fn assignment(name: &str, value: &str) -> Assignment {
    Assignment {
        name: name.to_string(),
        value: value.to_string(),
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
        obligation: obligation(
            "the bound of `f32x4.load(a, i + 12)`: `i + 12 + 4 <= n`",
            &["n % 4 == 0"],
        ),
        counterexample: Some(Counterexample {
            inputs: vec![assignment("n", "12"), assignment("i", "0")],
            outputs: None,
        }),
        repair: Vec::new(),
    }
}

/// Sorted, this list is `order.jsonl`: file name by bytes (`B` before `a`), start byte, end byte,
/// then the record's R bytes (`semicolon` before `unexpected-token`).
fn unordered_records() -> Vec<Diagnostic> {
    vec![
        on_line_1(Kind::UnexpectedToken, "a.lm", 7, 8),
        on_line_1(Kind::UnexpectedToken, "a.lm", 2, 5),
        on_line_1(Kind::UnexpectedToken, "a.lm", 0, 4),
        on_line_1(Kind::UnexpectedToken, "B.lm", 9, 10),
        on_line_1(Kind::Semicolon, "a.lm", 7, 8),
        on_line_1(Kind::UnexpectedToken, "a.lm", 2, 3),
    ]
}

fn golden_cases() -> Vec<(&'static str, Vec<Diagnostic>, &'static str)> {
    vec![
        (
            "no records",
            Vec::new(),
            include_str!("testdata/records/empty.jsonl"),
        ),
        (
            "minimal record",
            vec![record(Kind::UnexpectedToken)],
            include_str!("testdata/records/minimal.jsonl"),
        ),
        (
            "related span",
            vec![Diagnostic {
                related: vec![Related {
                    span: span_in("case.lm", (0, 1, 1), (3, 1, 4)),
                    label: "the opening delimiter".to_string(),
                }],
                ..record(Kind::UnexpectedToken)
            }],
            include_str!("testdata/records/related.jsonl"),
        ),
        (
            "obligation",
            vec![Diagnostic {
                obligation: obligation("the bound of `a[k]`: `k < n`", &["n % 4 == 0", "k < n"]),
                ..record(Kind::SolverUnknown)
            }],
            include_str!("testdata/records/obligation.jsonl"),
        ),
        (
            "obligation assuming nothing",
            vec![Diagnostic {
                obligation: obligation("the bound of `a[k]`: `k < n`", &[]),
                ..record(Kind::SolverUnknown)
            }],
            include_str!("testdata/records/empty_assumes.jsonl"),
        ),
        (
            "counterexample with no inputs",
            vec![Diagnostic {
                obligation: obligation("the divisor `0` is not zero", &["n % 4 == 0"]),
                counterexample: Some(Counterexample {
                    inputs: Vec::new(),
                    outputs: None,
                }),
                ..record(Kind::BoundUnproven)
            }],
            include_str!("testdata/records/empty_inputs.jsonl"),
        ),
        (
            "counterexample with outputs",
            vec![Diagnostic {
                obligation: obligation("the impl computes the spec", &["n > 0"]),
                counterexample: Some(Counterexample {
                    inputs: vec![assignment("x", "0.5")],
                    outputs: Some(Outputs {
                        spec: "1.0".to_string(),
                        implementation: "0.5".to_string(),
                    }),
                }),
                ..record(Kind::EquivalenceUnproven)
            }],
            include_str!("testdata/records/outputs.jsonl"),
        ),
        (
            "one-edit repair",
            vec![Diagnostic {
                repair: vec![Edit {
                    span: span_in("case.lm", (9, 1, 10), (9, 1, 10)),
                    replacement: " allow()".to_string(),
                }],
                ..record(Kind::EquivWithoutAllow)
            }],
            include_str!("testdata/records/repair.jsonl"),
        ),
        (
            "every escaping rule of a string",
            vec![Diagnostic {
                message: "quote \" backslash \\ bs \u{8} ff \u{c} lf \n cr \r tab \t \
                          soh \u{1} us \u{1f} del \u{7f} e-acute é sigma Σ slash /"
                    .to_string(),
                ..record(Kind::UnexpectedToken)
            }],
            include_str!("testdata/records/escaping.jsonl"),
        ),
        ("worked example of pick.lm", vec![pick_record()], PICK),
        ("worked example of dot.lm", vec![dot_record()], DOT),
        (
            "records out of order",
            unordered_records(),
            include_str!("testdata/records/order.jsonl"),
        ),
        (
            "two identical records",
            vec![record(Kind::UnexpectedToken), record(Kind::UnexpectedToken)],
            include_str!("testdata/records/duplicates.jsonl"),
        ),
    ]
}

#[test]
fn records_render_to_their_golden_bytes() {
    let cases = golden_cases();

    let mismatched: Vec<(&str, String, &str)> = cases
        .iter()
        .map(|(name, records, golden)| (*name, render_records(records), *golden))
        .filter(|(_, rendered, golden)| rendered != golden)
        .collect();

    assert!(mismatched.is_empty(), "rendered, golden: {mismatched:#?}");
}

#[test]
fn rendering_twice_gives_the_same_bytes() {
    let records = unordered_records();
    let first = render_records(&records);

    let second = render_records(&records);

    assert_eq!(second, first);
}

#[test]
fn rendering_the_reversed_input_gives_the_same_bytes() {
    let records = unordered_records();
    let reversed: Vec<Diagnostic> = records.iter().rev().cloned().collect();

    let rendered = render_records(&reversed);

    assert_eq!(rendered, render_records(&records));
}

#[test]
fn the_worked_examples_are_the_lines_of_the_document() {
    let golden_lines = [PICK, DOT];

    let in_document = golden_lines.map(|golden| DIAGNOSTICS.contains(golden));

    assert_eq!(in_document, [true, true]);
}
