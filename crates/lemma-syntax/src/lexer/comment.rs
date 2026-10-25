//! The characters a comment may hold (`docs/grammar.md` section 2, "Comments") and the one
//! `comment-character` fault for those it may not.

use lemma_diagnostics::Kind;

use super::{Decoded, decode, range};
use crate::token::{Fault, TextRange, byte_name, code_point_name};

/// The code points and bytes of one comment that a comment may not hold.
pub(super) struct Rejected {
    first: TextRange,
    first_name: String,
    further: usize,
    runs: Vec<TextRange>,
}

/// Printable ASCII and tab.
fn is_comment_character(character: char) -> bool {
    matches!(character, '\t' | ' '..='~')
}

/// Line ends can only occur inside a block comment, which spans lines.
fn is_allowed(bytes: &[u8], position: usize, character: char) -> bool {
    match character {
        '\n' => true,
        '\r' => bytes.get(position + 1) == Some(&b'\n'),
        _ => is_comment_character(character),
    }
}

/// The rejected code points and bytes in `from..to`, or `None` when the text is clean.
pub(super) fn find(bytes: &[u8], from: usize, to: usize) -> Option<Rejected> {
    let mut items: Vec<(TextRange, String)> = Vec::new();
    let mut position = from;
    while position < to {
        let Some(decoded) = decode(bytes, position) else {
            break;
        };
        let (length, rejected) = rejected_at(bytes, position, to, decoded);
        items.extend(rejected);
        position += length;
    }
    Rejected::from_items(&items)
}

/// The length of the decoded sequence at `position` and what it holds that a comment may not.
fn rejected_at(
    bytes: &[u8],
    position: usize,
    to: usize,
    decoded: Decoded,
) -> (usize, Vec<(TextRange, String)>) {
    match decoded {
        Decoded::Char(character, length) => {
            let name = code_point_name(character);
            let item = (range(position, position + length), name);
            let rejected = (!is_allowed(bytes, position, character)).then_some(item);
            (length, rejected.into_iter().collect())
        }
        Decoded::Invalid(length) => {
            let end = (position + length).min(to);
            let rejected = (position..end).filter_map(|offset| {
                let byte = bytes.get(offset)?;
                Some((range(offset, offset + 1), byte_name(*byte)))
            });
            (length, rejected.collect())
        }
    }
}

/// The comment's text from `from..to` without the rejected runs.
pub(super) fn text_without(
    bytes: &[u8],
    from: usize,
    to: usize,
    rejected: Option<&Rejected>,
) -> String {
    let mut kept: Vec<u8> = Vec::new();
    let mut position = from;
    for run in rejected.iter().flat_map(|rejected| &rejected.runs) {
        kept.extend_from_slice(bytes.get(position..run.start).unwrap_or_default());
        position = run.end;
    }
    kept.extend_from_slice(bytes.get(position..to).unwrap_or_default());
    String::from_utf8_lossy(&kept).into_owned()
}

impl Rejected {
    fn from_items(items: &[(TextRange, String)]) -> Option<Self> {
        let (first, first_name) = items.first()?.clone();
        let mut runs: Vec<TextRange> = Vec::new();
        for (item, _) in items {
            match runs.last_mut() {
                Some(run) if run.end == item.start => run.end = item.end,
                _ => runs.push(*item),
            }
        }
        Some(Self {
            first,
            first_name,
            further: items.len().saturating_sub(1),
            runs,
        })
    }

    fn found(&self) -> String {
        match self.further {
            0 => self.first_name.clone(),
            more => format!("{} and {more} more", self.first_name),
        }
    }

    fn sentence(&self) -> String {
        format!(
            "a comment holds printable ASCII and tab only; found {}",
            self.found()
        )
    }

    fn deletions(&self) -> Vec<(TextRange, String)> {
        self.runs.iter().map(|&run| (run, String::new())).collect()
    }

    /// The fault of a comment whose marker is fine: one deletion per run, reported on the first.
    pub(super) fn fault(&self) -> Fault {
        Fault {
            kind: Kind::CommentCharacter,
            message: self.sentence(),
            repair: self.deletions(),
            span: Some(self.first),
        }
    }

    /// `message` followed by what the comment's characters break.
    pub(super) fn note_to(&self, message: &str) -> String {
        format!("{message}; also, {}", self.sentence())
    }

    /// The fault of the comment's marker, which also deletes the runs when it has a repair; a
    /// deletion that touches the marker's edit joins it, so the edits stay separated.
    pub(super) fn merged_into(&self, mut marker: Fault) -> Fault {
        marker.message = self.note_to(&marker.message);
        let marker_span = marker
            .repair
            .first()
            .zip(marker.repair.last())
            .map(|(first, last)| range(first.0.start, last.0.end));
        let Some(marker_span) = marker_span else {
            return marker;
        };
        marker.span = Some(marker_span);
        let edits = std::mem::take(&mut marker.repair);
        marker.repair = joined(edits.into_iter().chain(self.deletions()));
        marker
    }
}

/// The edits, in source order, with each edit that starts where the previous one ends joined to
/// it.
fn joined(edits: impl Iterator<Item = (TextRange, String)>) -> Vec<(TextRange, String)> {
    let mut joined: Vec<(TextRange, String)> = Vec::new();
    for (edit_range, replacement) in edits {
        match joined.last_mut() {
            Some((previous, text)) if previous.end == edit_range.start => {
                previous.end = edit_range.end;
                text.push_str(&replacement);
            }
            _ => joined.push((edit_range, replacement)),
        }
    }
    joined
}
