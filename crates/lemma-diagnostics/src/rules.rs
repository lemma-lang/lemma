//! The record rules of `docs/diagnostics.md` sections 1 to 3.

use std::fmt;

use crate::kind::{FailureClass, Kind};
use crate::record::{Diagnostic, Edit};
use crate::span::Span;

/// A rule of the record that a diagnostic breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleViolation {
    /// The kind's class requires an obligation and the record has none.
    MissingObligation {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind's class allows no obligation and the record has one.
    UnexpectedObligation {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind's class requires a counterexample and the record has none.
    MissingCounterexample {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind's class allows no counterexample and the record has one.
    UnexpectedCounterexample {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind's class allows no repair and the record has one.
    UnexpectedRepair {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind requires the outputs of both sides in its counterexample and it has none.
    MissingOutputs {
        /// The record's kind.
        kind: Kind,
    },
    /// The kind allows no outputs in its counterexample and it has some.
    UnexpectedOutputs {
        /// The record's kind.
        kind: Kind,
    },
    /// A position has a line or a column of 0.
    ZeroLineOrColumn {
        /// The span holding the position.
        span: Span,
    },
    /// A span starts after it ends, by byte or by line and column.
    ReversedSpan {
        /// The span that is not well formed.
        span: Span,
    },
    /// An edit comes before the edit that precedes it, by file then start byte.
    UnsortedEdits {
        /// The index of the edit that is out of order.
        index: usize,
    },
    /// An edit does not start strictly after the end of the edit that precedes it.
    EditsNotSeparated {
        /// The index of the edit that is too close to its predecessor.
        index: usize,
    },
    /// An edit has an empty span and an empty replacement.
    EmptyEdit {
        /// The index of the edit that changes nothing.
        index: usize,
    },
}

impl fmt::Display for RuleViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingObligation { kind } => {
                write!(formatter, "kind `{}` requires an obligation", kind.name())
            }
            Self::UnexpectedObligation { kind } => {
                write!(formatter, "kind `{}` allows no obligation", kind.name())
            }
            Self::MissingCounterexample { kind } => {
                write!(
                    formatter,
                    "kind `{}` requires a counterexample",
                    kind.name()
                )
            }
            Self::UnexpectedCounterexample { kind } => {
                write!(formatter, "kind `{}` allows no counterexample", kind.name())
            }
            Self::UnexpectedRepair { kind } => {
                write!(formatter, "kind `{}` allows no repair", kind.name())
            }
            Self::MissingOutputs { kind } => write!(
                formatter,
                "kind `{}` requires the outputs of both sides",
                kind.name()
            ),
            Self::UnexpectedOutputs { kind } => {
                write!(formatter, "kind `{}` allows no outputs", kind.name())
            }
            Self::ZeroLineOrColumn { span } => write!(
                formatter,
                "a position of a span in `{}` has line or column 0",
                span.file
            ),
            Self::ReversedSpan { span } => {
                write!(formatter, "a span in `{}` starts after it ends", span.file)
            }
            Self::UnsortedEdits { index } => write!(
                formatter,
                "edit {index} is not sorted after the edit before it"
            ),
            Self::EditsNotSeparated { index } => write!(
                formatter,
                "edit {index} does not start strictly after the edit before it ends"
            ),
            Self::EmptyEdit { index } => write!(
                formatter,
                "edit {index} has an empty span and an empty replacement"
            ),
        }
    }
}

impl std::error::Error for RuleViolation {}

/// Fails with `violation` when `condition` holds.
fn refuse_when(condition: bool, violation: RuleViolation) -> Result<(), RuleViolation> {
    if condition { Err(violation) } else { Ok(()) }
}

impl Diagnostic {
    /// Checks the record rules of `docs/diagnostics.md` sections 1 to 3.
    ///
    /// # Errors
    ///
    /// The first [`RuleViolation`] found: the fields the kind's class requires or forbids, the
    /// outputs the kind requires or forbids, the spans, then the edits of the repair.
    pub fn validate(&self) -> Result<(), RuleViolation> {
        self.validate_fields()?;
        self.validate_outputs()?;
        self.validate_spans()?;
        self.validate_repair()
    }

    fn validate_fields(&self) -> Result<(), RuleViolation> {
        let kind = self.kind;
        let lacks_obligation = RuleViolation::MissingObligation { kind };
        let has_obligation = RuleViolation::UnexpectedObligation { kind };
        let lacks_counterexample = RuleViolation::MissingCounterexample { kind };
        let has_counterexample = RuleViolation::UnexpectedCounterexample { kind };
        let has_repair = RuleViolation::UnexpectedRepair { kind };
        match kind.failure_class() {
            FailureClass::Syntax | FailureClass::TypeOrLayout => {
                refuse_when(self.obligation.is_some(), has_obligation)?;
                refuse_when(self.counterexample.is_some(), has_counterexample)
            }
            FailureClass::Effect => Ok(()),
            FailureClass::BoundProof | FailureClass::EquivalenceProof => {
                refuse_when(self.obligation.is_none(), lacks_obligation)?;
                refuse_when(self.counterexample.is_none(), lacks_counterexample)?;
                refuse_when(!self.repair.is_empty(), has_repair)
            }
            FailureClass::SolverTimeout => {
                refuse_when(self.obligation.is_none(), lacks_obligation)?;
                refuse_when(self.counterexample.is_some(), has_counterexample)?;
                refuse_when(!self.repair.is_empty(), has_repair)
            }
            FailureClass::ToolchainBug => {
                refuse_when(self.counterexample.is_some(), has_counterexample)?;
                refuse_when(!self.repair.is_empty(), has_repair)
            }
        }
    }

    fn validate_outputs(&self) -> Result<(), RuleViolation> {
        let Some(counterexample) = &self.counterexample else {
            return Ok(());
        };
        let kind = self.kind;
        let has_outputs = counterexample.outputs.is_some();
        match kind {
            Kind::EquivalenceUnproven => {
                refuse_when(!has_outputs, RuleViolation::MissingOutputs { kind })
            }
            Kind::EnsUnproven => Ok(()),
            _ => refuse_when(has_outputs, RuleViolation::UnexpectedOutputs { kind }),
        }
    }

    fn validate_spans(&self) -> Result<(), RuleViolation> {
        let related = self.related.iter().map(|related| &related.span);
        let edited = self.repair.iter().map(|edit| &edit.span);
        std::iter::once(&self.span)
            .chain(related)
            .chain(edited)
            .try_for_each(validate_span)
    }

    fn validate_repair(&self) -> Result<(), RuleViolation> {
        for (index, edit) in self.repair.iter().enumerate() {
            let changes_nothing =
                edit.span.start.byte == edit.span.end.byte && edit.replacement.is_empty();
            refuse_when(changes_nothing, RuleViolation::EmptyEdit { index })?;
        }
        for (index, pair) in self.repair.windows(2).enumerate() {
            refuse_when(
                sort_key(&pair[1]) < sort_key(&pair[0]),
                RuleViolation::UnsortedEdits { index: index + 1 },
            )?;
        }
        for (index, pair) in self.repair.windows(2).enumerate() {
            let touches = pair[0].span.file == pair[1].span.file
                && pair[0].span.end.byte >= pair[1].span.start.byte;
            refuse_when(
                touches,
                RuleViolation::EditsNotSeparated { index: index + 1 },
            )?;
        }
        Ok(())
    }
}

fn sort_key(edit: &Edit) -> (&str, usize) {
    (&edit.span.file, edit.span.start.byte)
}

fn validate_span(span: &Span) -> Result<(), RuleViolation> {
    let positions = [span.start, span.end];
    let has_zero = positions
        .iter()
        .any(|position| position.line == 0 || position.column == 0);
    refuse_when(
        has_zero,
        RuleViolation::ZeroLineOrColumn { span: span.clone() },
    )?;
    let by_byte = span.start.byte > span.end.byte;
    let by_line_and_column =
        (span.start.line, span.start.column) > (span.end.line, span.end.column);
    refuse_when(
        by_byte || by_line_and_column,
        RuleViolation::ReversedSpan { span: span.clone() },
    )
}
