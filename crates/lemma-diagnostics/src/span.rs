//! Positions and spans computed from a source's bytes, per `docs/diagnostics.md` section 2.

use std::fmt;
use std::ops::Range;

/// A point in a file: a byte offset with the line and column it falls on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// The offset in the file's bytes, 0-based.
    pub byte: usize,
    /// The line, 1-based; lines are split on LF only.
    pub line: usize,
    /// The column, 1-based, counting code points from the start of the line.
    pub column: usize,
}

/// A half-open range of a file, from `start` to the position just after the last byte covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// The file's name exactly as the compiler was given it.
    pub file: String,
    /// The first byte covered.
    pub start: Position,
    /// The position just after the last byte covered.
    pub end: Position,
}

/// A named file's bytes, from which spans are computed.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    /// The file's name exactly as the compiler was given it.
    pub name: &'a str,
    /// The file's bytes, valid UTF-8 or not.
    pub text: &'a [u8],
}

/// Why a byte range is not a span of a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// The range ends after the end of the text.
    OutOfBounds {
        /// The first byte of the range.
        start: usize,
        /// The end of the range.
        end: usize,
        /// The length of the text.
        length: usize,
    },
    /// The range starts after it ends.
    Reversed {
        /// The first byte of the range.
        start: usize,
        /// The end of the range.
        end: usize,
    },
}

impl fmt::Display for SpanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { start, end, length } => write!(
                formatter,
                "range {start}..{end} ends after the end of a text of {length} bytes"
            ),
            Self::Reversed { start, end } => {
                write!(formatter, "range {start}..{end} starts after it ends")
            }
        }
    }
}

impl std::error::Error for SpanError {}

impl Source<'_> {
    /// The span covering `range`, with the line and column of both ends.
    ///
    /// An offset equal to the text length is valid.
    ///
    /// # Errors
    ///
    /// [`SpanError::Reversed`] when the range starts after it ends, and
    /// [`SpanError::OutOfBounds`] when it ends after the text.
    pub fn span(&self, range: Range<usize>) -> Result<Span, SpanError> {
        let Range { start, end } = range;
        if start > end {
            return Err(SpanError::Reversed { start, end });
        }
        if end > self.text.len() {
            return Err(SpanError::OutOfBounds {
                start,
                end,
                length: self.text.len(),
            });
        }
        Ok(Span {
            file: self.name.to_string(),
            start: self.position(start),
            end: self.position(end),
        })
    }

    /// The position of `byte`, which must not exceed the text length.
    fn position(&self, byte: usize) -> Position {
        let before = &self.text[..byte];
        let line_start = before
            .iter()
            .rposition(|&candidate| candidate == b'\n')
            .map_or(0, |last_line_feed| last_line_feed + 1);
        let line = 1 + before
            .iter()
            .filter(|&&candidate| candidate == b'\n')
            .count();
        let column = 1 + before[line_start..]
            .iter()
            .filter(|&&candidate| candidate & 0xC0 != 0x80)
            .count();
        Position { byte, line, column }
    }
}
