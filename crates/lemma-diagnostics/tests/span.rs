//! Positions and spans computed from a source's bytes, per `docs/diagnostics.md` section 2.

use std::ops::Range;

use lemma_diagnostics::{Position, Source, Span, SpanError};

const FILE: &str = "examples/case.lm";

fn position((byte, line, column): (usize, usize, usize)) -> Position {
    Position { byte, line, column }
}

fn assert_span(
    text: &[u8],
    range: Range<usize>,
    start: (usize, usize, usize),
    end: (usize, usize, usize),
) {
    let source = Source { name: FILE, text };
    let expected = Span {
        file: FILE.to_string(),
        start: position(start),
        end: position(end),
    };

    let span = source.span(range).ok();

    assert_eq!(span, Some(expected));
}

#[test]
fn a_non_ascii_code_point_is_one_column_and_several_bytes() {
    assert_span(
        "  spec Σ(i: u64 ∈ 0..<n) { a[i] * b[i] }\n".as_bytes(),
        7..9,
        (7, 1, 8),
        (9, 1, 9),
    );
}

#[test]
fn a_tab_is_one_column() {
    assert_span(b"\tx\n", 1..2, (1, 1, 2), (2, 1, 3));
}

#[test]
fn a_cr_before_an_lf_belongs_to_its_line() {
    assert_span(b"a\r\nb\n", 1..3, (1, 1, 2), (3, 2, 1));
}

#[test]
fn the_offset_after_a_final_lf_is_column_1_of_the_next_line() {
    assert_span(b"ab\n", 3..3, (3, 2, 1), (3, 2, 1));
}

#[test]
fn on_invalid_utf8_a_column_counts_the_bytes_that_are_not_continuation_bytes() {
    assert_span(b"a\xff\x80b\n", 3..4, (3, 1, 3), (4, 1, 4));
}

#[test]
fn a_range_past_the_end_of_the_text_is_refused() {
    let source = Source {
        name: FILE,
        text: b"abc",
    };

    let span = source.span(1..4);

    assert!(matches!(span, Err(SpanError::OutOfBounds { .. })));
}

#[test]
fn a_reversed_range_is_refused() {
    let source = Source {
        name: FILE,
        text: b"abc",
    };

    let span = source.span(Range { start: 2, end: 1 });

    assert!(matches!(span, Err(SpanError::Reversed { .. })));
}
