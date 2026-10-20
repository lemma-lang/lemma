//! The diagnostic record and its parts, per `docs/diagnostics.md` section 1.

use crate::kind::Kind;
use crate::span::Span;

/// A secondary place a diagnostic points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related {
    /// The place.
    pub span: Span,
    /// What the place is.
    pub label: String,
}

/// The proof obligation that failed or could not be decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    /// The construct the obligation is about and what it requires.
    pub about: String,
    /// The clauses the obligation assumes, each printed as an expression without its keyword.
    pub assumes: Vec<String>,
}

/// The value of one free variable of an obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// The variable's name.
    pub name: String,
    /// The canonical text of a Lemma literal of the variable's type.
    pub value: String,
}

/// What the two sides of an equivalence compute on a counterexample's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    /// The value the spec computes.
    pub spec: String,
    /// The value the impl computes.
    pub implementation: String,
}

/// Concrete values on which an obligation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterexample {
    /// One assignment per free variable, possibly none, in the order of section 1.
    pub inputs: Vec<Assignment>,
    /// What both sides compute on those inputs, when the obligation compares two sides.
    pub outputs: Option<Outputs>,
}

/// One replacement of a repair, against the text as submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The text replaced.
    pub span: Span,
    /// The text put in its place, possibly empty.
    pub replacement: String,
}

/// One diagnostic: what went wrong, where, and what is known about it.
///
/// An empty `related` or `repair` list means the field is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// What went wrong.
    pub kind: Kind,
    /// One sentence, on one line.
    pub message: String,
    /// Where it went wrong.
    pub span: Span,
    /// Secondary places.
    pub related: Vec<Related>,
    /// The proof obligation, for the classes that carry one.
    pub obligation: Option<Obligation>,
    /// The counterexample, for the classes that carry one.
    pub counterexample: Option<Counterexample>,
    /// The fix, as edits applied together, when it is certain.
    pub repair: Vec<Edit>,
}
