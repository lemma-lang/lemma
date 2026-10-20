//! The R rendering, per `docs/diagnostics.md` sections 4 and 5, and the order both renderings share.

use serde::Serialize;

use crate::record::{Assignment, Counterexample, Diagnostic, Edit, Obligation, Outputs, Related};
use crate::span::{Position, Span};

/// Renders records as the compact JSON lines of `docs/diagnostics.md` section 5, in the order of
/// section 4. The input slice is left as it is.
///
/// # Examples
///
/// ```
/// assert_eq!(lemma_diagnostics::render_records(&[]), "");
/// ```
pub fn render_records(records: &[Diagnostic]) -> String {
    ordered(records)
        .into_iter()
        .map(|(_, line)| line + "\n")
        .collect()
}

/// The records in the order of `docs/diagnostics.md` section 4, each with its R rendering without
/// its LF. Records that are equal in every key stay, all of them.
pub(crate) fn ordered(records: &[Diagnostic]) -> Vec<(&Diagnostic, String)> {
    let mut lines: Vec<(&Diagnostic, String)> = records
        .iter()
        .map(|record| (record, render_line(record)))
        .collect();
    lines.sort_by(|(first, first_line), (second, second_line)| {
        let first_key = (
            &first.span.file,
            first.span.start.byte,
            first.span.end.byte,
            first_line,
        );
        let second_key = (
            &second.span.file,
            second.span.start.byte,
            second.span.end.byte,
            second_line,
        );
        first_key.cmp(&second_key)
    });
    lines
}

// Serializing strings and integers into a `String` cannot fail.
fn render_line(record: &Diagnostic) -> String {
    serde_json::to_string(&WireRecord::new(record)).unwrap_or_default()
}

#[derive(Serialize)]
struct WireRecord<'a> {
    kind: &'a str,
    class: &'a str,
    message: &'a str,
    span: WireSpan<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related: Vec<WireRelated<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obligation: Option<WireObligation<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    counterexample: Option<WireCounterexample<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    repair: Vec<WireEdit<'a>>,
}

impl<'a> WireRecord<'a> {
    fn new(record: &'a Diagnostic) -> Self {
        Self {
            kind: record.kind.name(),
            class: record.kind.failure_class().name(),
            message: &record.message,
            span: WireSpan::new(&record.span),
            related: record.related.iter().map(WireRelated::new).collect(),
            obligation: record.obligation.as_ref().map(WireObligation::new),
            counterexample: record.counterexample.as_ref().map(WireCounterexample::new),
            repair: record.repair.iter().map(WireEdit::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct WireSpan<'a> {
    file: &'a str,
    start: WirePosition,
    end: WirePosition,
}

impl<'a> WireSpan<'a> {
    fn new(span: &'a Span) -> Self {
        Self {
            file: &span.file,
            start: WirePosition::new(span.start),
            end: WirePosition::new(span.end),
        }
    }
}

#[derive(Serialize)]
struct WirePosition {
    byte: usize,
    line: usize,
    column: usize,
}

impl WirePosition {
    fn new(position: Position) -> Self {
        Self {
            byte: position.byte,
            line: position.line,
            column: position.column,
        }
    }
}

#[derive(Serialize)]
struct WireRelated<'a> {
    span: WireSpan<'a>,
    label: &'a str,
}

impl<'a> WireRelated<'a> {
    fn new(related: &'a Related) -> Self {
        Self {
            span: WireSpan::new(&related.span),
            label: &related.label,
        }
    }
}

#[derive(Serialize)]
struct WireObligation<'a> {
    about: &'a str,
    assumes: &'a [String],
}

impl<'a> WireObligation<'a> {
    fn new(obligation: &'a Obligation) -> Self {
        Self {
            about: &obligation.about,
            assumes: &obligation.assumes,
        }
    }
}

#[derive(Serialize)]
struct WireCounterexample<'a> {
    inputs: Vec<WireAssignment<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outputs: Option<WireOutputs<'a>>,
}

impl<'a> WireCounterexample<'a> {
    fn new(counterexample: &'a Counterexample) -> Self {
        Self {
            inputs: counterexample
                .inputs
                .iter()
                .map(WireAssignment::new)
                .collect(),
            outputs: counterexample.outputs.as_ref().map(WireOutputs::new),
        }
    }
}

#[derive(Serialize)]
struct WireAssignment<'a> {
    name: &'a str,
    value: &'a str,
}

impl<'a> WireAssignment<'a> {
    fn new(assignment: &'a Assignment) -> Self {
        Self {
            name: &assignment.name,
            value: &assignment.value,
        }
    }
}

#[derive(Serialize)]
struct WireOutputs<'a> {
    spec: &'a str,
    #[serde(rename = "impl")]
    implementation: &'a str,
}

impl<'a> WireOutputs<'a> {
    fn new(outputs: &'a Outputs) -> Self {
        Self {
            spec: &outputs.spec,
            implementation: &outputs.implementation,
        }
    }
}

#[derive(Serialize)]
struct WireEdit<'a> {
    span: WireSpan<'a>,
    replacement: &'a str,
}

impl<'a> WireEdit<'a> {
    fn new(edit: &'a Edit) -> Self {
        Self {
            span: WireSpan::new(&edit.span),
            replacement: &edit.replacement,
        }
    }
}
