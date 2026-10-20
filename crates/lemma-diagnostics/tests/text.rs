//! The T rendering, per `docs/diagnostics.md` sections 4 and 6.

use lemma_diagnostics::{
    Assignment, Counterexample, Diagnostic, Edit, Kind, Obligation, Outputs, Position, Related,
    Source, Span, render_text,
};

const DIAGNOSTICS: &str = include_str!("../../../docs/diagnostics.md");

const PICK: &str = include_str!("testdata/text/pick.txt");
const DOT: &str = include_str!("testdata/text/dot.txt");

const CASE: Source<'static> = Source {
    name: "case.lm",
    text: include_bytes!("testdata/sources/case.lm"),
};

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

fn record_at(kind: Kind, span: Span) -> Diagnostic {
    Diagnostic {
        kind,
        message: "a message".to_string(),
        span,
        related: Vec::new(),
        obligation: None,
        counterexample: None,
        repair: Vec::new(),
    }
}

/// A record on the `f` of line 1 of `case.lm`.
fn record(kind: Kind) -> Diagnostic {
    record_at(kind, span_in("case.lm", (3, 1, 4), (4, 1, 5)))
}

fn on_case(start: (usize, usize, usize), end: (usize, usize, usize)) -> Diagnostic {
    record_at(Kind::UnexpectedToken, span_in("case.lm", start, end))
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

fn missing_file_record() -> Diagnostic {
    record_at(
        Kind::UnexpectedToken,
        span_in("other.lm", (0, 1, 1), (3, 1, 4)),
    )
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

/// Out of section 4 order: `other.lm`, then line 2 and line 1 of `case.lm`.
fn several_records() -> Vec<Diagnostic> {
    vec![
        missing_file_record(),
        on_case((29, 2, 2), (32, 2, 5)),
        record(Kind::UnexpectedToken),
    ]
}

fn golden_cases() -> Vec<(
    &'static str,
    Vec<Diagnostic>,
    Vec<Source<'static>>,
    &'static str,
)> {
    vec![
        (
            "no records",
            Vec::new(),
            vec![CASE],
            include_str!("testdata/text/empty.txt"),
        ),
        (
            "minimal record",
            vec![record(Kind::UnexpectedToken)],
            vec![CASE],
            include_str!("testdata/text/minimal.txt"),
        ),
        (
            "related span",
            vec![Diagnostic {
                related: vec![Related {
                    span: span_in("case.lm", (5, 1, 6), (6, 1, 7)),
                    label: "the parameter".to_string(),
                }],
                ..record(Kind::UnexpectedToken)
            }],
            vec![CASE],
            include_str!("testdata/text/related.txt"),
        ),
        (
            "obligation with its assumes",
            vec![Diagnostic {
                obligation: obligation("the bound of `k`", &["n % 4 == 0", "k < n"]),
                ..record(Kind::SolverUnknown)
            }],
            vec![CASE],
            include_str!("testdata/text/obligation.txt"),
        ),
        (
            "counterexample without outputs",
            vec![Diagnostic {
                obligation: obligation("the bound of `k`", &[]),
                counterexample: Some(Counterexample {
                    inputs: vec![assignment("n", "12"), assignment("k", "12")],
                    outputs: None,
                }),
                ..record(Kind::BoundUnproven)
            }],
            vec![CASE],
            include_str!("testdata/text/counterexample.txt"),
        ),
        (
            "counterexample with outputs",
            vec![Diagnostic {
                obligation: obligation("the impl computes the spec", &["n > 0"]),
                counterexample: Some(Counterexample {
                    inputs: vec![assignment("n", "3")],
                    outputs: Some(Outputs {
                        spec: "1.0".to_string(),
                        implementation: "0.5".to_string(),
                    }),
                }),
                ..record(Kind::EquivalenceUnproven)
            }],
            vec![CASE],
            include_str!("testdata/text/outputs.txt"),
        ),
        (
            "counterexample with no inputs",
            vec![Diagnostic {
                obligation: obligation("the divisor `0` is not zero", &[]),
                counterexample: Some(Counterexample {
                    inputs: Vec::new(),
                    outputs: None,
                }),
                ..record(Kind::BoundUnproven)
            }],
            vec![CASE],
            include_str!("testdata/text/no_inputs.txt"),
        ),
        (
            "one-edit repair",
            vec![Diagnostic {
                repair: vec![Edit {
                    span: span_in("case.lm", (27, 1, 28), (27, 1, 28)),
                    replacement: " allow()".to_string(),
                }],
                ..record(Kind::EquivWithoutAllow)
            }],
            vec![CASE],
            include_str!("testdata/text/repair.txt"),
        ),
        (
            "span continuing on a later line",
            vec![on_case((3, 1, 4), (32, 2, 5))],
            vec![CASE],
            include_str!("testdata/text/multiline.txt"),
        ),
        (
            "span continuing from a line ending with a CR",
            vec![on_case((50, 3, 3), (63, 5, 3))],
            vec![CASE],
            include_str!("testdata/text/multiline_cr.txt"),
        ),
        (
            "empty span",
            vec![on_case((4, 1, 5), (4, 1, 5))],
            vec![CASE],
            include_str!("testdata/text/empty_span.txt"),
        ),
        (
            "tab before the span",
            vec![on_case((29, 2, 2), (32, 2, 5))],
            vec![CASE],
            include_str!("testdata/text/tab.txt"),
        ),
        (
            "two-digit start line",
            vec![on_case((93, 10, 3), (97, 10, 7))],
            vec![CASE],
            include_str!("testdata/text/two_digit.txt"),
        ),
        (
            "empty source line",
            vec![on_case((60, 4, 1), (60, 4, 1))],
            vec![CASE],
            include_str!("testdata/text/empty_line.txt"),
        ),
        (
            "trailing spaces and tab of the source line",
            vec![on_case((63, 5, 3), (69, 5, 9))],
            vec![CASE],
            include_str!("testdata/text/trailing.txt"),
        ),
        (
            "invalid UTF-8 in the source line",
            vec![record_at(
                Kind::UnexpectedToken,
                span_in("invalid.lm", (2, 1, 3), (3, 1, 4)),
            )],
            vec![Source {
                name: "invalid.lm",
                text: include_bytes!("testdata/sources/invalid.lm"),
            }],
            include_str!("testdata/text/invalid_utf8.txt"),
        ),
        (
            "control characters in strings and in a replacement",
            vec![Diagnostic {
                message: "quote \" backslash \\ lf \n tab \t soh \u{1} del \u{7f}".to_string(),
                related: vec![Related {
                    span: span_in("case.lm", (5, 1, 6), (6, 1, 7)),
                    label: "a\tb".to_string(),
                }],
                repair: vec![Edit {
                    span: span_in("case.lm", (3, 1, 4), (4, 1, 5)),
                    replacement: "a\"b\\c\nd\u{1}".to_string(),
                }],
                ..record(Kind::Semicolon)
            }],
            vec![CASE],
            include_str!("testdata/text/escaping.txt"),
        ),
        (
            "file not among the sources",
            vec![missing_file_record()],
            vec![CASE],
            include_str!("testdata/text/missing_file.txt"),
        ),
        (
            "span ending past the source",
            vec![on_case((100, 10, 10), (120, 10, 30))],
            vec![CASE],
            include_str!("testdata/text/past_end.txt"),
        ),
        (
            "empty span past the source",
            vec![on_case((120, 12, 1), (120, 12, 1))],
            vec![CASE],
            include_str!("testdata/text/past_end_empty.txt"),
        ),
        (
            "two sources with one name",
            vec![record_at(
                Kind::UnexpectedToken,
                span_in("dup.lm", (0, 1, 1), (5, 1, 6)),
            )],
            vec![
                Source {
                    name: "dup.lm",
                    text: b"first line\n",
                },
                Source {
                    name: "dup.lm",
                    text: b"second line\n",
                },
            ],
            include_str!("testdata/text/same_name.txt"),
        ),
        (
            "several records out of order",
            several_records(),
            vec![CASE],
            include_str!("testdata/text/several.txt"),
        ),
        (
            "worked example of pick.lm",
            vec![pick_record()],
            vec![Source {
                name: "pick.lm",
                text: include_bytes!("testdata/sources/pick.lm"),
            }],
            PICK,
        ),
        (
            "worked example of dot.lm",
            vec![dot_record()],
            vec![Source {
                name: "dot.lm",
                text: include_bytes!("testdata/sources/dot.lm"),
            }],
            DOT,
        ),
    ]
}

#[test]
fn records_render_to_their_golden_text() {
    let cases = golden_cases();

    let mismatched: Vec<(&str, String, &str)> = cases
        .iter()
        .map(|(name, records, sources, golden)| (*name, render_text(records, sources), *golden))
        .filter(|(_, rendered, golden)| rendered != golden)
        .collect();

    assert!(mismatched.is_empty(), "rendered, golden: {mismatched:#?}");
}

#[test]
fn rendering_twice_gives_the_same_bytes() {
    let records = several_records();
    let first = render_text(&records, &[CASE]);

    let second = render_text(&records, &[CASE]);

    assert_eq!(second, first);
}

#[test]
fn rendering_the_reversed_input_gives_the_same_bytes() {
    let records = several_records();
    let reversed: Vec<Diagnostic> = records.iter().rev().cloned().collect();

    let rendered = render_text(&reversed, &[CASE]);

    assert_eq!(rendered, render_text(&records, &[CASE]));
}

#[test]
fn the_worked_examples_are_the_text_of_the_document() {
    let goldens = [PICK, DOT];

    let in_document = goldens.map(|golden| DIAGNOSTICS.contains(golden));

    assert_eq!(in_document, [true, true]);
}
