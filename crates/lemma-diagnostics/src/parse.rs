//! Parsing the R rendering back into records, per `docs/diagnostics.md` section 5, "Parsing".

use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, MapAccess, Visitor};

use crate::kind::Kind;
use crate::record::{Assignment, Counterexample, Diagnostic, Edit, Obligation, Outputs, Related};
use crate::rules::RuleViolation;
use crate::span::{Position, Span};

/// Why a line of the R rendering is not a valid record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordErrorReason {
    /// The JSON parser or the record's shape refused the line: syntax, type, unknown, duplicate or
    /// missing key, or a `null`. Holds the parser's message.
    Json(String),
    /// The line is empty.
    EmptyLine {},
    /// The `kind` is not in the closed set.
    UnknownKind {
        /// The name found.
        name: String,
    },
    /// The `class` is not the class of the `kind`.
    ClassMismatch {
        /// The record's kind.
        kind: Kind,
        /// The class found.
        class: String,
    },
    /// An array that must hold at least one element is empty.
    EmptyList {
        /// The key of the array: `related` or `repair`.
        key: &'static str,
    },
    /// The record breaks a rule of `docs/diagnostics.md` sections 1 to 3.
    Rule(RuleViolation),
}

impl fmt::Display for RecordErrorReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => formatter.write_str(message),
            Self::EmptyLine {} => formatter.write_str("the line is empty"),
            Self::UnknownKind { name } => write!(formatter, "`{name}` is not a kind"),
            Self::ClassMismatch { kind, class } => write!(
                formatter,
                "`{class}` is not the class of `{}`, which is `{}`",
                kind.name(),
                kind.failure_class().name()
            ),
            Self::EmptyList { key } => write!(formatter, "the `{key}` array is empty"),
            Self::Rule(violation) => violation.fmt(formatter),
        }
    }
}

/// A line of the R rendering that is not a valid record, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordError {
    /// The 1-based number of the line in the input.
    pub line: usize,
    /// The 1-based column where the JSON parser located the error, or 1 for a rule on the record
    /// as a whole.
    pub column: usize,
    /// What is wrong with the line.
    pub reason: RecordErrorReason,
}

impl fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}, column {}: {}",
            self.line, self.column, self.reason
        )
    }
}

impl std::error::Error for RecordError {}

/// Parses the R rendering of `docs/diagnostics.md` section 5 into records, in the order of the
/// input.
///
/// The input is lines separated by LF, the last one with or without its LF, each holding one
/// record; the empty input holds none. Rendering the result gives the records in the order of
/// section 4.
///
/// # Errors
///
/// The first line that section 5 rejects, with its number and the reason.
///
/// # Examples
///
/// ```
/// assert_eq!(lemma_diagnostics::parse_records("").map(|records| records.len()), Ok(0));
/// ```
pub fn parse_records(input: &str) -> Result<Vec<Diagnostic>, RecordError> {
    let mut lines: Vec<&str> = input.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| parse_line(line).map_err(|error| error.on_line(index + 1)))
        .collect()
}

/// A failure on one line, before the line is known.
struct LineError {
    column: usize,
    reason: RecordErrorReason,
}

impl LineError {
    fn whole_record(reason: RecordErrorReason) -> Self {
        Self { column: 1, reason }
    }

    fn on_line(self, line: usize) -> RecordError {
        RecordError {
            line,
            column: self.column,
            reason: self.reason,
        }
    }
}

impl From<serde_json::Error> for LineError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            column: error.column(),
            reason: RecordErrorReason::Json(error.to_string()),
        }
    }
}

fn parse_line(line: &str) -> Result<Diagnostic, LineError> {
    if line.is_empty() {
        return Err(LineError::whole_record(RecordErrorReason::EmptyLine {}));
    }
    let Object(wire) = serde_json::from_str::<Object<WireRecord>>(line)?;
    let Some(kind) = Kind::from_name(&wire.kind) else {
        let name = wire.kind;
        return Err(LineError::whole_record(RecordErrorReason::UnknownKind {
            name,
        }));
    };
    if wire.class != kind.failure_class().name() {
        let class = wire.class;
        return Err(LineError::whole_record(RecordErrorReason::ClassMismatch {
            kind,
            class,
        }));
    }
    refuse_empty("related", wire.related.as_deref())?;
    refuse_empty("repair", wire.repair.as_deref())?;
    let record = wire.into_record(kind);
    record
        .validate()
        .map_err(|violation| LineError::whole_record(RecordErrorReason::Rule(violation)))?;
    Ok(record)
}

fn refuse_empty<T>(key: &'static str, list: Option<&[T]>) -> Result<(), LineError> {
    match list {
        Some([]) => Err(LineError::whole_record(RecordErrorReason::EmptyList {
            key,
        })),
        _ => Ok(()),
    }
}

/// A value that must be a JSON object: unlike the derived implementation, it refuses an array
/// holding the values of the fields in order.
struct Object<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

struct ObjectVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
    type Value = Object<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(Object)
    }
}

/// Reads a present optional field as `T` itself, so that `null` is refused instead of meaning
/// absent.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRecord {
    kind: String,
    class: String,
    message: String,
    span: Object<WireSpan>,
    #[serde(default, deserialize_with = "present")]
    related: Option<Vec<Object<WireRelated>>>,
    #[serde(default, deserialize_with = "present")]
    obligation: Option<Object<WireObligation>>,
    #[serde(default, deserialize_with = "present")]
    counterexample: Option<Object<WireCounterexample>>,
    #[serde(default, deserialize_with = "present")]
    repair: Option<Vec<Object<WireEdit>>>,
}

impl WireRecord {
    fn into_record(self, kind: Kind) -> Diagnostic {
        Diagnostic {
            kind,
            message: self.message,
            span: self.span.0.into(),
            related: converted(self.related.unwrap_or_default()),
            obligation: self.obligation.map(|Object(wire)| wire.into()),
            counterexample: self.counterexample.map(|Object(wire)| wire.into()),
            repair: converted(self.repair.unwrap_or_default()),
        }
    }
}

fn converted<W, T: From<W>>(wires: Vec<Object<W>>) -> Vec<T> {
    wires.into_iter().map(|Object(wire)| wire.into()).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSpan {
    file: String,
    start: Object<WirePosition>,
    end: Object<WirePosition>,
}

impl From<WireSpan> for Span {
    fn from(wire: WireSpan) -> Self {
        Self {
            file: wire.file,
            start: wire.start.0.into(),
            end: wire.end.0.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePosition {
    byte: usize,
    line: usize,
    column: usize,
}

impl From<WirePosition> for Position {
    fn from(wire: WirePosition) -> Self {
        Self {
            byte: wire.byte,
            line: wire.line,
            column: wire.column,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRelated {
    span: Object<WireSpan>,
    label: String,
}

impl From<WireRelated> for Related {
    fn from(wire: WireRelated) -> Self {
        Self {
            span: wire.span.0.into(),
            label: wire.label,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObligation {
    about: String,
    assumes: Vec<String>,
}

impl From<WireObligation> for Obligation {
    fn from(wire: WireObligation) -> Self {
        Self {
            about: wire.about,
            assumes: wire.assumes,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCounterexample {
    inputs: Vec<Object<WireAssignment>>,
    #[serde(default, deserialize_with = "present")]
    outputs: Option<Object<WireOutputs>>,
}

impl From<WireCounterexample> for Counterexample {
    fn from(wire: WireCounterexample) -> Self {
        Self {
            inputs: converted(wire.inputs),
            outputs: wire.outputs.map(|Object(outputs)| outputs.into()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireAssignment {
    name: String,
    value: String,
}

impl From<WireAssignment> for Assignment {
    fn from(wire: WireAssignment) -> Self {
        Self {
            name: wire.name,
            value: wire.value,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireOutputs {
    spec: String,
    #[serde(rename = "impl")]
    implementation: String,
}

impl From<WireOutputs> for Outputs {
    fn from(wire: WireOutputs) -> Self {
        Self {
            spec: wire.spec,
            implementation: wire.implementation,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEdit {
    span: Object<WireSpan>,
    replacement: String,
}

impl From<WireEdit> for Edit {
    fn from(wire: WireEdit) -> Self {
        Self {
            span: wire.span.0.into(),
            replacement: wire.replacement,
        }
    }
}
