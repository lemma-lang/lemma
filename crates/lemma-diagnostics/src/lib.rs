//! Lemma diagnostic records and their two renderings: the compact JSON form read by agents and the text form read by humans.

mod kind;
mod parse;
mod record;
mod records;
mod rules;
mod span;
mod text;

pub use kind::{FailureClass, Kind};
pub use parse::{RecordError, RecordErrorReason, parse_records};
pub use record::{Assignment, Counterexample, Diagnostic, Edit, Obligation, Outputs, Related};
pub use records::render_records;
pub use rules::RuleViolation;
pub use span::{Position, Source, Span, SpanError};
pub use text::render_text;
