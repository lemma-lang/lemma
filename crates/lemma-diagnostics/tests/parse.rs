//! Parsing the R rendering back into records, per `docs/diagnostics.md` section 5, "Parsing".

use lemma_diagnostics::{
    Diagnostic, Kind, Position, RecordError, RecordErrorReason, RuleViolation, Span, parse_records,
    render_records,
};

type Expected = fn(&RecordErrorReason) -> bool;

const SPAN: &str = r#"{"file":"case.lm","start":{"byte":4,"line":1,"column":5},"end":{"byte":9,"line":1,"column":10}}"#;

fn record_line(kind: &str, class: &str, span: &str, rest: &str) -> String {
    format!(r#"{{"kind":"{kind}","class":"{class}","message":"a message","span":{span}{rest}}}"#)
}

fn syntax_line(rest: &str) -> String {
    record_line("unexpected-token", "syntax", SPAN, rest)
}

fn minimal_line() -> String {
    syntax_line("")
}

/// The line between two valid records, so that it is line 2 of the input.
fn between(line: &str) -> String {
    let minimal = minimal_line();
    format!("{minimal}\n{line}\n{minimal}\n")
}

/// An edit on line 1 of `case.lm`, where a column is the byte offset plus one.
fn edit(start: usize, end: usize, replacement: &str) -> String {
    let start_column = start + 1;
    let end_column = end + 1;
    format!(
        r#"{{"span":{{"file":"case.lm","start":{{"byte":{start},"line":1,"column":{start_column}}},"end":{{"byte":{end},"line":1,"column":{end_column}}}}},"replacement":"{replacement}"}}"#
    )
}

fn repair_line(edits: &[String]) -> String {
    record_line(
        "semicolon",
        "syntax",
        SPAN,
        &format!(r#","repair":[{}]"#, edits.join(",")),
    )
}

fn minimal_record() -> Diagnostic {
    Diagnostic {
        kind: Kind::UnexpectedToken,
        message: "a message".to_string(),
        span: Span {
            file: "case.lm".to_string(),
            start: Position {
                byte: 4,
                line: 1,
                column: 5,
            },
            end: Position {
                byte: 9,
                line: 1,
                column: 10,
            },
        },
        related: Vec::new(),
        obligation: None,
        counterexample: None,
        repair: Vec::new(),
    }
}

fn rejected_cases() -> Vec<(&'static str, String, usize, Expected)> {
    vec![
        (
            "an empty line between two records",
            format!("{0}\n\n{0}\n", minimal_line()),
            2,
            |reason| matches!(reason, RecordErrorReason::EmptyLine { .. }),
        ),
        (
            "an empty last line",
            format!("{}\n\n", minimal_line()),
            2,
            |reason| matches!(reason, RecordErrorReason::EmptyLine { .. }),
        ),
        ("a lone LF", "\n".to_string(), 1, |reason| {
            matches!(reason, RecordErrorReason::EmptyLine { .. })
        }),
        (
            "a line that is not JSON",
            between(r#"{"kind":x}"#),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a JSON array of the record's values",
            between(&format!(
                r#"["unexpected-token","syntax","a message",{SPAN}]"#
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a nested object written as an array of its values",
            between(&record_line(
                "unexpected-token",
                "syntax",
                r#"["case.lm",{"byte":4,"line":1,"column":5},{"byte":9,"line":1,"column":10}]"#,
                "",
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "two objects on one line",
            between(&format!("{0}{0}", minimal_line())),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "an unknown key",
            between(&syntax_line(r#","severity":"error""#)),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a duplicate key",
            between(&format!(
                r#"{{"kind":"unexpected-token",{}"#,
                &minimal_line()[1..]
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a missing required key",
            between(&format!(
                r#"{{"kind":"unexpected-token","class":"syntax","span":{SPAN}}}"#
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a value of the wrong type",
            between(&format!(
                r#"{{"kind":"unexpected-token","class":"syntax","message":1,"span":{SPAN}}}"#
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a null optional field",
            between(&record_line(
                "solver-unknown",
                "solver-timeout",
                SPAN,
                r#","obligation":null"#,
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a null inside a nested object",
            between(&record_line(
                "equivalence-unproven",
                "equivalence-proof",
                SPAN,
                r#","obligation":{"about":"x","assumes":[]},"counterexample":{"inputs":[],"outputs":null}"#,
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "an obligation without its assumes",
            between(&record_line(
                "solver-unknown",
                "solver-timeout",
                SPAN,
                r#","obligation":{"about":"x"}"#,
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "a counterexample without its inputs",
            between(&record_line(
                "bound-unproven",
                "bound-proof",
                SPAN,
                r#","obligation":{"about":"x","assumes":[]},"counterexample":{}"#,
            )),
            2,
            |reason| matches!(reason, RecordErrorReason::Json { .. }),
        ),
        (
            "an empty related array",
            between(&syntax_line(r#","related":[]"#)),
            2,
            |reason| matches!(reason, RecordErrorReason::EmptyList { .. }),
        ),
        (
            "an empty repair array",
            between(&syntax_line(r#","repair":[]"#)),
            2,
            |reason| matches!(reason, RecordErrorReason::EmptyList { .. }),
        ),
        (
            "an unknown kind",
            between(&record_line("no-such-kind", "syntax", SPAN, "")),
            2,
            |reason| matches!(reason, RecordErrorReason::UnknownKind { .. }),
        ),
        (
            "a class that is not the kind's",
            between(&record_line("unexpected-token", "type-or-layout", SPAN, "")),
            2,
            |reason| matches!(reason, RecordErrorReason::ClassMismatch { .. }),
        ),
        (
            "a position on line 0",
            between(&record_line(
                "unexpected-token",
                "syntax",
                r#"{"file":"case.lm","start":{"byte":4,"line":0,"column":5},"end":{"byte":9,"line":1,"column":10}}"#,
                "",
            )),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::ZeroLineOrColumn { .. })
                )
            },
        ),
        (
            "a position in column 0",
            between(&record_line(
                "unexpected-token",
                "syntax",
                r#"{"file":"case.lm","start":{"byte":4,"line":1,"column":0},"end":{"byte":9,"line":1,"column":10}}"#,
                "",
            )),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::ZeroLineOrColumn { .. })
                )
            },
        ),
        (
            "a reversed span",
            between(&record_line(
                "unexpected-token",
                "syntax",
                r#"{"file":"case.lm","start":{"byte":9,"line":1,"column":10},"end":{"byte":4,"line":1,"column":5}}"#,
                "",
            )),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::ReversedSpan { .. })
                )
            },
        ),
        (
            "unsorted edits",
            between(&repair_line(&[edit(5, 6, ""), edit(0, 1, "")])),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::UnsortedEdits { .. })
                )
            },
        ),
        (
            "edits that are not separated",
            between(&repair_line(&[edit(0, 3, ""), edit(3, 5, "")])),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::EditsNotSeparated { .. })
                )
            },
        ),
        (
            "an empty edit",
            between(&repair_line(&[edit(3, 3, "")])),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::EmptyEdit { .. })
                )
            },
        ),
        (
            "a field the kind does not allow",
            between(&syntax_line(r#","obligation":{"about":"x","assumes":[]}"#)),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::UnexpectedObligation { .. })
                )
            },
        ),
        (
            "a field the kind requires, missing",
            between(&record_line(
                "bound-unproven",
                "bound-proof",
                SPAN,
                r#","obligation":{"about":"x","assumes":[]}"#,
            )),
            2,
            |reason| {
                matches!(
                    reason,
                    RecordErrorReason::Rule(RuleViolation::MissingCounterexample { .. })
                )
            },
        ),
    ]
}

/// One input per reason that is not a JSON error: each is a rule on the record as a whole.
fn whole_record_cases() -> Vec<(&'static str, String)> {
    vec![
        ("an empty line", between("")),
        (
            "an unknown kind",
            between(&record_line("no-such-kind", "syntax", SPAN, "")),
        ),
        (
            "a class that is not the kind's",
            between(&record_line("unexpected-token", "type-or-layout", SPAN, "")),
        ),
        (
            "an empty related array",
            between(&syntax_line(r#","related":[]"#)),
        ),
        (
            "a record rule",
            between(&syntax_line(r#","obligation":{"about":"x","assumes":[]}"#)),
        ),
    ]
}

fn accepted_cases() -> Vec<(&'static str, String, Vec<Diagnostic>)> {
    vec![
        ("the empty input", String::new(), Vec::new()),
        (
            "a last line without its LF",
            minimal_line(),
            vec![minimal_record()],
        ),
        (
            "keys in another order",
            r#"{"span":{"end":{"column":10,"line":1,"byte":9},"start":{"line":1,"column":5,"byte":4},"file":"case.lm"},"message":"a message","class":"syntax","kind":"unexpected-token"}"#
                .to_string()
                + "\n",
            vec![minimal_record()],
        ),
        (
            "JSON whitespace between tokens",
            "{ \"kind\" : \"unexpected-token\",\t\"class\":\"syntax\" ,\r\"message\":\"a message\", \"span\": {\"file\":\"case.lm\",\"start\":{\"byte\":4,\"line\":1,\"column\":5},\"end\":{ \"byte\":9,\"line\":1,\"column\":10 } } }\n"
                .to_string(),
            vec![minimal_record()],
        ),
    ]
}

fn golden_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("empty", include_str!("testdata/records/empty.jsonl")),
        ("minimal", include_str!("testdata/records/minimal.jsonl")),
        ("related", include_str!("testdata/records/related.jsonl")),
        (
            "obligation",
            include_str!("testdata/records/obligation.jsonl"),
        ),
        (
            "empty_assumes",
            include_str!("testdata/records/empty_assumes.jsonl"),
        ),
        (
            "empty_inputs",
            include_str!("testdata/records/empty_inputs.jsonl"),
        ),
        ("outputs", include_str!("testdata/records/outputs.jsonl")),
        ("repair", include_str!("testdata/records/repair.jsonl")),
        ("escaping", include_str!("testdata/records/escaping.jsonl")),
        ("pick", include_str!("testdata/records/pick.jsonl")),
        ("dot", include_str!("testdata/records/dot.jsonl")),
        ("order", include_str!("testdata/records/order.jsonl")),
        (
            "duplicates",
            include_str!("testdata/records/duplicates.jsonl"),
        ),
    ]
}

#[test]
fn each_rejected_line_is_reported_with_its_line_and_reason() {
    let cases = rejected_cases();

    let misreported: Vec<(&str, Result<Vec<Diagnostic>, RecordError>)> = cases
        .iter()
        .map(|(name, input, line, expected)| (*name, parse_records(input), *line, expected))
        .filter(|(_, result, line, expected)| {
            !result
                .as_ref()
                .err()
                .is_some_and(|error| error.line == *line && expected(&error.reason))
        })
        .map(|(name, result, _, _)| (name, result))
        .collect();

    assert!(misreported.is_empty(), "misreported: {misreported:#?}");
}

#[test]
fn a_json_syntax_error_carries_the_column_where_the_parser_located_it() {
    let input = between(r#"{"kind":x}"#);

    let location = parse_records(&input)
        .err()
        .map(|error| (error.line, error.column));

    assert_eq!(location, Some((2, 9)));
}

#[test]
fn a_rule_on_the_whole_record_is_located_at_column_1() {
    let cases = whole_record_cases();

    let columns: Vec<(&str, Option<usize>)> = cases
        .iter()
        .map(|(name, input)| (*name, parse_records(input).err().map(|error| error.column)))
        .collect();

    let expected: Vec<(&str, Option<usize>)> =
        cases.iter().map(|(name, _)| (*name, Some(1))).collect();
    assert_eq!(columns, expected);
}

#[test]
fn accepted_inputs_parse_to_their_records() {
    let cases = accepted_cases();

    let wrong: Vec<(&str, Result<Vec<Diagnostic>, RecordError>)> = cases
        .iter()
        .map(|(name, input, expected)| (*name, parse_records(input), expected))
        .filter(|(_, result, expected)| result.as_ref().ok() != Some(*expected))
        .map(|(name, result, _)| (name, result))
        .collect();

    assert!(wrong.is_empty(), "wrong: {wrong:#?}");
}

#[test]
fn every_golden_file_parses_to_records_that_render_to_the_same_bytes() {
    let goldens = golden_files();

    let differing: Vec<(&str, Result<String, RecordError>)> = goldens
        .iter()
        .map(|(name, golden)| {
            (
                *name,
                parse_records(golden).map(|records: Vec<Diagnostic>| render_records(&records)),
                *golden,
            )
        })
        .filter(|(_, rendered, golden)| rendered.as_deref().ok() != Some(*golden))
        .map(|(name, rendered, _)| (name, rendered))
        .collect();

    assert!(differing.is_empty(), "differing: {differing:#?}");
}
