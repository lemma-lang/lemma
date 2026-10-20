//! Parsing the R rendering of valid records gives them back, in the order of `docs/diagnostics.md`
//! section 4.

use std::slice;

use lemma_diagnostics::{
    Assignment, Counterexample, Diagnostic, Edit, FailureClass, Kind, Obligation, Outputs,
    Position, Related, Span, parse_records, render_records,
};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;

const FILES: [&str; 4] = ["a.lm", "B.lm", "dir/c.lm", "é.lm"];

/// Control characters, `"`, `\`, `/`, U+007F and non-ASCII, without a line break.
fn single_line_text() -> impl Strategy<Value = String> {
    "[a-z0-9 \"\\\\/\\t\\x01\\x08\\x0c\\x1b\\x1f\\x7féΣ]{0,12}"
}

fn multi_line_text() -> impl Strategy<Value = String> {
    "[a-z0-9 \"\\\\/\\t\\n\\r\\x01\\x1f\\x7féΣ]{0,8}"
}

fn end_column(start_column: usize, extra_lines: usize, shift: usize) -> usize {
    if extra_lines == 0 {
        start_column + shift
    } else {
        1 + shift
    }
}

/// A well-formed span: its end is not before its start, by byte nor by line and column.
fn span() -> impl Strategy<Value = Span> {
    (
        select(FILES.to_vec()),
        0usize..2000,
        0usize..40,
        1usize..300,
        1usize..120,
        0usize..3,
        0usize..40,
    )
        .prop_map(
            |(file, byte, length, line, column, extra_lines, shift)| Span {
                file: file.to_string(),
                start: Position { byte, line, column },
                end: Position {
                    byte: byte + length,
                    line: line + extra_lines,
                    column: end_column(column, extra_lines, shift),
                },
            },
        )
}

fn related() -> impl Strategy<Value = Related> {
    (span(), single_line_text()).prop_map(|(span, label)| Related { span, label })
}

fn assignment() -> impl Strategy<Value = Assignment> {
    (single_line_text(), single_line_text()).prop_map(|(name, value)| Assignment { name, value })
}

fn obligation() -> impl Strategy<Value = Obligation> {
    (single_line_text(), vec(single_line_text(), 0..3))
        .prop_map(|(about, assumes)| Obligation { about, assumes })
}

fn outputs() -> impl Strategy<Value = Outputs> {
    (single_line_text(), single_line_text()).prop_map(|(spec, implementation)| Outputs {
        spec,
        implementation,
    })
}

fn line_1_span(start: usize, end: usize) -> Span {
    Span {
        file: "a.lm".to_string(),
        start: Position {
            byte: start,
            line: 1,
            column: start + 1,
        },
        end: Position {
            byte: end,
            line: 1,
            column: end + 1,
        },
    }
}

/// Edits of one file, sorted, separated by at least one byte, none empty.
fn edits() -> impl Strategy<Value = Vec<Edit>> {
    vec((1usize..5, 0usize..4, multi_line_text()), 0..4).prop_map(|parts| {
        let mut cursor = 0;
        parts
            .into_iter()
            .map(|(gap, length, replacement)| {
                let start = cursor + gap;
                let length = length.max(usize::from(replacement.is_empty()));
                cursor = start + length;
                Edit {
                    span: line_1_span(start, cursor),
                    replacement,
                }
            })
            .collect()
    })
}

fn outputs_for(kind: Kind, outputs: Outputs, keep: bool) -> Option<Outputs> {
    match kind {
        Kind::EquivalenceUnproven => Some(outputs),
        Kind::EnsUnproven => keep.then_some(outputs),
        _ => None,
    }
}

struct Parts {
    kind: Kind,
    message: String,
    span: Span,
    related: Vec<Related>,
    obligation: Obligation,
    keep_obligation: bool,
    inputs: Vec<Assignment>,
    outputs: Outputs,
    keep_outputs: bool,
    repair: Vec<Edit>,
}

/// The record the parts make once the fields its kind's class forbids are dropped.
fn shaped(parts: Parts) -> Diagnostic {
    let class = parts.kind.failure_class();
    let obligation = match class {
        FailureClass::Syntax | FailureClass::TypeOrLayout | FailureClass::Effect => None,
        FailureClass::ToolchainBug => parts.keep_obligation.then_some(parts.obligation),
        FailureClass::BoundProof | FailureClass::EquivalenceProof | FailureClass::SolverTimeout => {
            Some(parts.obligation)
        }
    };
    let counterexample = match class {
        FailureClass::BoundProof | FailureClass::EquivalenceProof => Some(Counterexample {
            inputs: parts.inputs,
            outputs: outputs_for(parts.kind, parts.outputs, parts.keep_outputs),
        }),
        _ => None,
    };
    let repair = match class {
        FailureClass::Syntax | FailureClass::TypeOrLayout => parts.repair,
        _ => Vec::new(),
    };
    Diagnostic {
        kind: parts.kind,
        message: parts.message,
        span: parts.span,
        related: parts.related,
        obligation,
        counterexample,
        repair,
    }
}

fn record() -> impl Strategy<Value = Diagnostic> {
    (
        select(Kind::ALL.to_vec()),
        single_line_text(),
        span(),
        vec(related(), 0..3),
        obligation(),
        any::<bool>(),
        vec(assignment(), 0..3),
        outputs(),
        any::<bool>(),
        edits(),
    )
        .prop_map(
            |(
                kind,
                message,
                span,
                related,
                obligation,
                keep_obligation,
                inputs,
                outputs,
                keep_outputs,
                repair,
            )| {
                shaped(Parts {
                    kind,
                    message,
                    span,
                    related,
                    obligation,
                    keep_obligation,
                    inputs,
                    outputs,
                    keep_outputs,
                    repair,
                })
            },
        )
}

/// Section 4: file name bytes, start byte, end byte, then the record's R bytes.
fn in_rendered_order(records: &[Diagnostic]) -> Vec<Diagnostic> {
    let mut sorted = records.to_vec();
    sorted.sort_by_cached_key(|record| {
        (
            record.span.file.clone(),
            record.span.start.byte,
            record.span.end.byte,
            render_records(slice::from_ref(record)),
        )
    });
    sorted
}

proptest! {
    #[test]
    fn parsing_the_rendering_gives_the_records_in_rendered_order(records in vec(record(), 0..6)) {
        let rendered = render_records(&records);

        let parsed = parse_records(&rendered).map_err(|error| format!("{error:?}"));

        prop_assert_eq!(parsed, Ok(in_rendered_order(&records)));
    }

    #[test]
    fn rendering_the_parsed_records_gives_the_same_bytes(records in vec(record(), 0..6)) {
        let rendered = render_records(&records);
        let parsed = parse_records(&rendered).map_err(|error| format!("{error:?}"));

        let rendered_again = parsed.map(|records: Vec<Diagnostic>| render_records(&records));

        prop_assert_eq!(rendered_again, Ok(rendered));
    }
}
