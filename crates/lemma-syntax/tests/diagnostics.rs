//! The syntax diagnostics of `docs/grammar.md` section 7 as `parse` reports them: kinds, spans and
//! repairs over the corpus in `testdata/diagnostics/`, the recovery rule (a repaired record
//! continues the parse, an unrepaired one ends its function and parsing resumes at the next `fn` or
//! `export`), the nesting limit of section 3, and the R and T renderings of every corpus file.

use std::collections::BTreeSet;
use std::error::Error;
use std::io;

use lemma_diagnostics::{Diagnostic, Kind, Source, render_records, render_text};
use lemma_syntax::{C1Variant, File, dump, parse};

type ExpectedEdit = (usize, usize, &'static str);
type ExpectedRecord = (Kind, usize, usize, &'static [ExpectedEdit]);
type CorpusCase = (&'static str, C1Variant, &'static [ExpectedRecord]);
type EditView = (usize, usize, String);
type RecordView = (Kind, usize, usize, Vec<EditView>);
type CorpusMismatch = (&'static str, Vec<RecordView>, Vec<RecordView>);
type ItemCase = (&'static str, C1Variant, &'static [&'static str]);
type ItemMismatch = (&'static str, Vec<String>, Vec<String>);
type DumpCase = (&'static str, C1Variant, &'static str);
type DumpMismatch = (&'static str, String, String);
type NestingOutcome = (Vec<(Kind, usize, usize)>, usize);
type NestingCase = (String, String, NestingOutcome);
type NestingMismatch = (String, NestingOutcome, NestingOutcome);
type MessageCase = (&'static str, String, &'static str);
type MessageMismatch = (&'static str, Vec<String>, &'static str);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;

const CORPUS_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testdata/diagnostics");

const DOT: &str = include_str!("../../../examples/dot.lm");

const IMPL_MESSAGE: &str =
    "nesting exceeds the limit of 16 levels; bind an inner expression to a name with `let`";
const CONTRACT_MESSAGE: &str = "nesting exceeds the limit of 16 levels";

const LEXICAL_CASES: [CorpusCase; 5] = [
    (
        "lexical_unrepaired.lm",
        ASCII,
        &[
            (Kind::InvalidCharacter, 28, 29, &[]),
            (Kind::NonAscii, 90, 92, &[]),
            (Kind::InvalidIdentifier, 128, 129, &[]),
            (Kind::RangeOperator, 216, 218, &[]),
            (Kind::InvalidNumber, 280, 283, &[]),
        ],
    ),
    (
        "lexical_repaired.lm",
        ASCII,
        &[
            (Kind::Lookalike, 39, 42, &[(39, 42, ">=")]),
            (Kind::Lookalike, 52, 55, &[(52, 55, "sum")]),
            (Kind::Lookalike, 79, 82, &[(79, 82, "-")]),
            (Kind::InvalidNumber, 83, 85, &[(83, 83, "0")]),
            (Kind::PathSeparator, 135, 137, &[(135, 137, ".")]),
            (Kind::InvalidNumber, 143, 150, &[(144, 145, "")]),
        ],
    ),
    (
        "invalid_encoding.lm",
        ASCII,
        &[(Kind::InvalidEncoding, 58, 59, &[])],
    ),
    (
        "comment_markers.lm",
        ASCII,
        &[
            (Kind::Semicolon, 0, 1, &[(0, 1, "//")]),
            (Kind::HashComment, 38, 39, &[(38, 39, "//")]),
            (Kind::BlockComment, 55, 66, &[(55, 66, "// exact ")]),
            (Kind::Semicolon, 104, 105, &[(104, 105, "")]),
        ],
    ),
    (
        "comment_characters.lm",
        ASCII,
        &[
            (Kind::CommentCharacter, 6, 8, &[(6, 8, ""), (9, 10, "")]),
            (Kind::CommentCharacter, 50, 53, &[(50, 53, "")]),
        ],
    ),
];

const TOKEN_CASES: [CorpusCase; 17] = [
    (
        "unexpected_token.lm",
        ASCII,
        &[
            (Kind::UnexpectedToken, 3, 4, &[]),
            (Kind::UnexpectedToken, 69, 72, &[]),
            (Kind::UnexpectedToken, 131, 132, &[]),
            (Kind::UnexpectedToken, 209, 210, &[]),
            (Kind::UnexpectedToken, 258, 261, &[]),
            (Kind::UnexpectedToken, 379, 390, &[]),
            (Kind::UnexpectedToken, 472, 473, &[]),
        ],
    ),
    (
        "unexpected_end.lm",
        ASCII,
        &[(Kind::UnexpectedEnd, 16, 16, &[])],
    ),
    (
        "misplaced_export.lm",
        ASCII,
        &[
            (Kind::MisplacedExport, 0, 6, &[]),
            (Kind::MisplacedExport, 76, 82, &[]),
            (Kind::MisplacedExport, 149, 155, &[]),
        ],
    ),
    (
        "reserved_word.lm",
        ASCII,
        &[
            (Kind::ReservedWord, 3, 7, &[]),
            (Kind::ReservedWord, 68, 70, &[]),
            (Kind::ReservedWord, 189, 192, &[]),
            (Kind::ReservedWord, 276, 278, &[]),
        ],
    ),
    (
        "reserved_word_variant_u.lm",
        UNICODE,
        &[
            (Kind::ReservedWord, 5, 7, &[]),
            (Kind::ReservedWord, 130, 133, &[]),
        ],
    ),
    (
        "foreign_keyword_repaired.lm",
        ASCII,
        &[
            (Kind::ForeignKeyword, 0, 8, &[(0, 8, "fn")]),
            (Kind::ForeignKeyword, 28, 35, &[(28, 35, "eff")]),
            (Kind::ForeignKeyword, 43, 51, &[(43, 51, "req")]),
            (Kind::ForeignKeyword, 60, 67, &[(60, 67, "ens")]),
            (Kind::ForeignKeyword, 117, 120, &[(117, 120, "let")]),
            (Kind::ForeignKeyword, 136, 141, &[(136, 141, "let")]),
            (Kind::ForeignKeyword, 157, 160, &[(157, 160, "return")]),
            (Kind::ForeignKeyword, 174, 177, &[(174, 177, "fn")]),
        ],
    ),
    (
        "foreign_keyword_unrepaired.lm",
        ASCII,
        &[
            (Kind::ForeignKeyword, 0, 3, &[]),
            (Kind::ForeignKeyword, 123, 125, &[]),
            (Kind::ForeignKeyword, 194, 197, &[]),
        ],
    ),
    ("assignment.lm", ASCII, &[(Kind::Assignment, 57, 58, &[])]),
    (
        "assignment_in_expression.lm",
        ASCII,
        &[(Kind::AssignmentInExpression, 44, 45, &[(44, 45, "==")])],
    ),
    (
        "unsupported_operators.lm",
        ASCII,
        &[
            (Kind::UnsupportedOperator, 36, 37, &[]),
            (Kind::UnsupportedOperator, 108, 109, &[]),
        ],
    ),
    (
        "membership_lookalike.lm",
        ASCII,
        &[
            (Kind::Lookalike, 45, 47, &[(45, 47, "in")]),
            (Kind::NonAscii, 121, 123, &[]),
        ],
    ),
    (
        "membership_lookalike_variant_u.lm",
        UNICODE,
        &[(Kind::Lookalike, 44, 46, &[(44, 46, "\u{2208}")])],
    ),
    ("almost_equal.lm", ASCII, &[(Kind::Lookalike, 35, 38, &[])]),
    (
        "ampersand_in_loop_range.lm",
        ASCII,
        &[(Kind::UnsupportedOperator, 90, 91, &[])],
    ),
    (
        "clause_foreign_word_out_of_place.lm",
        ASCII,
        &[(Kind::ForeignKeyword, 33, 40, &[(33, 40, "eff")])],
    ),
    (
        "zero_width_space_in_assigned_name.lm",
        ASCII,
        &[
            (Kind::Lookalike, 69, 75, &[(71, 74, "")]),
            (Kind::Assignment, 69, 75, &[]),
        ],
    ),
    (
        "var_without_type.lm",
        ASCII,
        &[(Kind::Assignment, 63, 66, &[])],
    ),
];

const DELIMITER_CASES: [CorpusCase; 6] = [
    (
        "unclosed_delimiter_at_keyword.lm",
        ASCII,
        &[
            (Kind::UnclosedDelimiter, 86, 86, &[]),
            (Kind::UnclosedDelimiter, 146, 146, &[]),
        ],
    ),
    (
        "unclosed_delimiter_repaired.lm",
        ASCII,
        &[
            (Kind::UnclosedDelimiter, 59, 59, &[(59, 59, ")")]),
            (Kind::UnclosedDelimiter, 69, 69, &[(69, 69, "]")]),
            (Kind::UnclosedDelimiter, 124, 124, &[(124, 124, ")")]),
        ],
    ),
    (
        "unclosed_delimiter_at_end.lm",
        ASCII,
        &[
            (Kind::UnclosedDelimiter, 11, 11, &[(11, 11, ")")]),
            (Kind::UnexpectedEnd, 12, 12, &[]),
        ],
    ),
    (
        "unclosed_delimiter_unrepaired.lm",
        ASCII,
        &[
            (Kind::UnclosedDelimiter, 42, 42, &[]),
            (Kind::UnclosedDelimiter, 171, 171, &[]),
        ],
    ),
    (
        "unmatched_delimiter.lm",
        ASCII,
        &[
            (Kind::UnmatchedDelimiter, 35, 36, &[]),
            (Kind::UnmatchedDelimiter, 136, 137, &[]),
            (Kind::UnmatchedDelimiter, 202, 203, &[]),
        ],
    ),
    (
        "trailing_comma.lm",
        ASCII,
        &[
            (Kind::TrailingComma, 11, 12, &[(11, 12, "")]),
            (Kind::TrailingComma, 42, 43, &[(42, 43, "")]),
            (Kind::TrailingComma, 69, 70, &[(69, 70, "")]),
            (Kind::TrailingComma, 129, 130, &[(129, 130, "")]),
        ],
    ),
];

const RECOVERY_CASES: [CorpusCase; 3] = [
    (
        "skipped_faults.lm",
        ASCII,
        &[
            (Kind::ReservedWord, 5, 9, &[]),
            (Kind::Lookalike, 76, 78, &[(76, 78, " ")]),
        ],
    ),
    (
        "stop_token_is_resume_token.lm",
        ASCII,
        &[
            (Kind::Lookalike, 2, 4, &[(2, 4, " ")]),
            (Kind::ReservedWord, 4, 6, &[]),
        ],
    ),
    (
        "repaired_then_unrepaired.lm",
        ASCII,
        &[
            (Kind::TrailingComma, 11, 12, &[(11, 12, "")]),
            (Kind::Lookalike, 36, 39, &[(36, 39, ">=")]),
            (Kind::InvalidCharacter, 49, 50, &[]),
        ],
    ),
];

const NESTING_CORPUS_CASES: [CorpusCase; 2] = [
    (
        "nesting_limit_impl.lm",
        ASCII,
        &[(Kind::NestingLimit, 91, 92, &[])],
    ),
    (
        "nesting_limit_contract.lm",
        ASCII,
        &[(Kind::NestingLimit, 54, 55, &[])],
    ),
];

const RECOVERED_ITEM_CASES: [ItemCase; 20] = [
    ("lexical_unrepaired.lm", ASCII, &["fn g"]),
    ("invalid_encoding.lm", ASCII, &["fn g"]),
    ("comment_markers.lm", ASCII, &["fn f"]),
    ("comment_characters.lm", ASCII, &["fn f"]),
    ("unexpected_token.lm", ASCII, &["fn g"]),
    ("misplaced_export.lm", ASCII, &["export fn g", "fn h"]),
    ("reserved_word.lm", ASCII, &["fn g"]),
    ("foreign_keyword_unrepaired.lm", ASCII, &["fn a"]),
    ("assignment.lm", ASCII, &["fn g"]),
    ("unsupported_operators.lm", ASCII, &[]),
    ("membership_lookalike.lm", ASCII, &["fn f"]),
    ("unclosed_delimiter_unrepaired.lm", ASCII, &["fn g"]),
    ("unmatched_delimiter.lm", ASCII, &["fn g", "fn h"]),
    ("skipped_faults.lm", ASCII, &["fn g"]),
    ("stop_token_is_resume_token.lm", ASCII, &["fn g"]),
    ("repaired_then_unrepaired.lm", ASCII, &["fn g"]),
    ("unclosed_delimiter_at_keyword.lm", ASCII, &["fn g"]),
    ("ampersand_in_loop_range.lm", ASCII, &["fn g"]),
    ("zero_width_space_in_assigned_name.lm", ASCII, &["fn g"]),
    ("var_without_type.lm", ASCII, &["fn g"]),
];

const REPAIRED_DUMP_CASES: [DumpCase; 4] = [
    (
        "unclosed_delimiter_repaired.lm",
        ASCII,
        concat!(
            "fn f\n",
            "  param n u64\n",
            "  param a (ref u64 n @a8 ro)\n",
            "  result u64\n",
            "  eff pure\n",
            "  req (call g n)\n",
            "  req (index a n)\n",
            "  spec n\n",
            "  equiv\n",
            "  impl\n",
            "    let m u64 (call g n)\n",
            "    return m\n",
        ),
    ),
    (
        "trailing_comma.lm",
        ASCII,
        concat!(
            "fn f\n",
            "  param n u64\n",
            "  result u64\n",
            "  eff pure\n",
            "  spec (call g n)\n",
            "  equiv fp_reassoc\n",
            "  impl\n",
            "    loop i u64 0 n step 1\n",
            "      carry s u64 0\n",
            "      next s (+ s i)\n",
            "    return s\n",
        ),
    ),
    (
        "assignment_in_expression.lm",
        ASCII,
        concat!(
            "fn f\n",
            "  param n u64\n",
            "  result u64\n",
            "  eff pure\n",
            "  req (== (% n 16) 0)\n",
            "  spec n\n",
            "  equiv\n",
            "  impl\n",
            "    return n\n",
        ),
    ),
    (
        "foreign_keyword_repaired.lm",
        ASCII,
        concat!(
            "fn f\n",
            "  param n u64\n",
            "  result u64\n",
            "  eff pure\n",
            "  req (> n 0)\n",
            "  ens (> result 0)\n",
            "  spec n\n",
            "  equiv\n",
            "  impl\n",
            "    let m u64 n\n",
            "    let k u64 m\n",
            "    return k\n",
            "export fn g\n",
            "  result u64\n",
            "  eff pure\n",
            "  spec 0\n",
            "  equiv\n",
            "  impl\n",
            "    return 0\n",
        ),
    ),
];

const UNEXPECTED_TOKEN_MESSAGES: [&str; 7] = [
    "unexpected `5`, expected a name",
    "unexpected `u64`, expected `->`",
    "unexpected `5`, expected a type",
    "unexpected `*`, expected an expression",
    "unexpected `1.5`, expected a name or an integer literal",
    "unexpected `fp_contract`, expected `)` or `,`",
    "unexpected `x`, expected `export` or `fn`",
];

const SPEC_HEAD: &str = "fn f(n: u64, a: &u64[n] @a8 ro) -> u64\n  eff pure\n  spec ";
const SPEC_TAIL: &str = "\n  equiv allow()\n  impl {\n    return 0\n  }\n";
const IMPL_HEAD: &str = concat!(
    "fn f(n: u64, a: &u64[n] @a8 ro) -> u64\n",
    "  eff pure\n",
    "  spec 0\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return ",
);
const IMPL_TAIL: &str = "\n  }\n";
const REQ_HEAD: &str = "fn f(n: u64) -> u64\n  eff pure\n  req ";
const ENS_HEAD: &str = "fn f(n: u64) -> u64\n  eff pure\n  ens ";
const CLAUSE_TAIL: &str = "\n  spec n\n  equiv allow()\n  impl {\n    return n\n  }\n";
const LOOPS_HEAD: &str = "fn f(n: u64) -> u64\n  eff pure\n  spec n\n  equiv allow()\n  impl {\n";
const OPEN_CALL_IN_REQ: &str = "fn f(n: u64) -> u64\n  eff pure\n  req g(n";
const DOT_LOAD: &str = "f32x4.load(a, i + 4)";
const DOT_LOAD_HEAD: &str = "f32x4.load(a, ";

const SIBLINGS: &str = "(c0 + c1) + (c2 + c3) * c4 && c5 || c6";

/// A construct that opens one level, repeated to nest: `prefix` opens it at byte `opener` of the
/// prefix, and `suffix` closes it around the innermost `atom`.
#[derive(Clone, Copy)]
struct Opener {
    label: &'static str,
    prefix: &'static str,
    atom: &'static str,
    suffix: &'static str,
    opener: usize,
}

const PARENTHESIS: Opener = Opener {
    label: "parenthesis",
    prefix: "(",
    atom: "x",
    suffix: ")",
    opener: 0,
};

const OPENERS: [Opener; 5] = [
    PARENTHESIS,
    Opener {
        label: "index",
        prefix: "a[",
        atom: "0",
        suffix: "]",
        opener: 1,
    },
    Opener {
        label: "call",
        prefix: "f(",
        atom: "x",
        suffix: ")",
        opener: 1,
    },
    Opener {
        label: "unary minus",
        prefix: "- ",
        atom: "x",
        suffix: "",
        opener: 0,
    },
    Opener {
        label: "sum with its body",
        prefix: "sum(i: u64 in 0..<n) { ",
        atom: "x",
        suffix: " }",
        opener: 3,
    },
];

fn source<'a>(name: &'a str, text: &'a [u8]) -> Source<'a> {
    Source { name, text }
}

fn corpus_text(file: &str) -> Result<Vec<u8>, io::Error> {
    std::fs::read(format!("{CORPUS_DIRECTORY}/{file}"))
}

fn record_view(record: &Diagnostic) -> RecordView {
    let edits = record
        .repair
        .iter()
        .map(|edit| {
            (
                edit.span.start.byte,
                edit.span.end.byte,
                edit.replacement.clone(),
            )
        })
        .collect();
    (
        record.kind,
        record.span.start.byte,
        record.span.end.byte,
        edits,
    )
}

fn expected_view(&(kind, start, end, edits): &ExpectedRecord) -> RecordView {
    let edits = edits
        .iter()
        .map(|&(start, end, replacement)| (start, end, replacement.to_string()))
        .collect();
    (kind, start, end, edits)
}

fn corpus_records(file: &str, variant: C1Variant) -> Result<Vec<Diagnostic>, io::Error> {
    let text = corpus_text(file)?;
    Ok(parse(source(file, &text), variant).diagnostics)
}

fn corpus_file(file: &str, variant: C1Variant) -> Result<File, io::Error> {
    let text = corpus_text(file)?;
    Ok(parse(source(file, &text), variant).file)
}

/// The cases whose records differ from the expected kinds, spans and repairs.
fn corpus_mismatches(cases: &[CorpusCase]) -> Result<Vec<CorpusMismatch>, io::Error> {
    let mut mismatches = Vec::new();
    for &(file, variant, expected) in cases {
        let actual: Vec<RecordView> = corpus_records(file, variant)?
            .iter()
            .map(record_view)
            .collect();
        let expected: Vec<RecordView> = expected.iter().map(expected_view).collect();
        mismatches.extend((actual != expected).then_some((file, actual, expected)));
    }
    Ok(mismatches)
}

fn item_heads(file: &File) -> Vec<String> {
    file.items
        .iter()
        .map(|function| {
            let export = if function.export.is_some() {
                "export "
            } else {
                ""
            };
            format!("{export}fn {}", function.name.text)
        })
        .collect()
}

fn item_mismatches(cases: &[ItemCase]) -> Result<Vec<ItemMismatch>, io::Error> {
    let mut mismatches = Vec::new();
    for &(file, variant, expected) in cases {
        let actual = item_heads(&corpus_file(file, variant)?);
        let expected: Vec<String> = expected.iter().map(|head| head.to_string()).collect();
        mismatches.extend((actual != expected).then_some((file, actual, expected)));
    }
    Ok(mismatches)
}

fn dump_mismatches(cases: &[DumpCase]) -> Result<Vec<DumpMismatch>, io::Error> {
    let mut mismatches = Vec::new();
    for &(file, variant, expected) in cases {
        let actual = dump(&corpus_file(file, variant)?);
        mismatches.extend((actual != expected).then(|| (file, actual, expected.to_string())));
    }
    Ok(mismatches)
}

/// The messages of the file's records of `kind` that hold a code point outside printable ASCII.
fn unprintable_messages(
    file: &str,
    variant: C1Variant,
    kind: Kind,
) -> Result<Vec<String>, io::Error> {
    Ok(corpus_records(file, variant)?
        .into_iter()
        .filter(|record| record.kind == kind)
        .map(|record| record.message)
        .filter(|message| {
            !message
                .chars()
                .all(|character| matches!(character, ' '..='~'))
        })
        .collect())
}

fn corpus_messages(file: &str, variant: C1Variant) -> Result<Vec<String>, io::Error> {
    Ok(corpus_records(file, variant)?
        .into_iter()
        .map(|record| record.message)
        .collect())
}

fn all_corpus_cases() -> Vec<CorpusCase> {
    [
        LEXICAL_CASES.as_slice(),
        TOKEN_CASES.as_slice(),
        DELIMITER_CASES.as_slice(),
        RECOVERY_CASES.as_slice(),
        NESTING_CORPUS_CASES.as_slice(),
    ]
    .concat()
}

fn corpus_file_names() -> Result<BTreeSet<String>, io::Error> {
    std::fs::read_dir(CORPUS_DIRECTORY)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect()
}

/// The R rendering of the file's records, an empty line, then their T rendering.
fn renderings(file: &str, variant: C1Variant) -> Result<String, io::Error> {
    let text = corpus_text(file)?;
    let source = source(file, &text);
    let records = parse(source, variant).diagnostics;
    Ok(format!(
        "{}\n{}",
        render_records(&records),
        render_text(&records, &[source])
    ))
}

fn assert_snapshots(cases: &[CorpusCase]) -> Result<(), io::Error> {
    for &(file, variant, _) in cases {
        let name = file.trim_end_matches(".lm");
        insta::assert_snapshot!(name, renderings(file, variant)?);
    }
    Ok(())
}

fn nested(opener: &Opener, count: usize) -> String {
    format!(
        "{}{}{}",
        opener.prefix.repeat(count),
        opener.atom,
        opener.suffix.repeat(count)
    )
}

fn parenthesized(core: &str, count: usize) -> String {
    format!("{}{core}{}", "(".repeat(count), ")".repeat(count))
}

/// The byte of the opener of the `ordinal`-th unit, counted from 1, of an expression at `start`.
fn opener_offset(opener: &Opener, start: usize, ordinal: usize) -> usize {
    start + opener.prefix.len() * (ordinal - 1) + opener.opener
}

fn in_spec(expression: &str) -> String {
    format!("{SPEC_HEAD}{expression}{SPEC_TAIL}")
}

fn in_impl(expression: &str) -> String {
    format!("{IMPL_HEAD}{expression}{IMPL_TAIL}")
}

fn in_req(expression: &str) -> String {
    format!("{REQ_HEAD}{expression}{CLAUSE_TAIL}")
}

fn in_ens(expression: &str) -> String {
    format!("{ENS_HEAD}{expression}{CLAUSE_TAIL}")
}

fn loop_opening(ordinal: usize) -> String {
    format!("loop k{ordinal}: u64 in 0..<n step 1 carry(c{ordinal}: u64 = 0) {{\n")
}

fn loop_closing(ordinal: usize) -> String {
    format!("next c{ordinal} = c{ordinal}\n}}\n")
}

/// An impl of `count` loops, each the body of the one before.
fn nested_loops(count: usize) -> String {
    let openings: String = (1..=count).map(loop_opening).collect();
    let closings: String = (1..=count).rev().map(loop_closing).collect();
    format!("{LOOPS_HEAD}{openings}{closings}return n\n}}\n")
}

/// The byte of the `(` of `carry(` in the `ordinal`-th of nested loops.
fn carry_offset(ordinal: usize) -> usize {
    let before: usize = (1..ordinal)
        .map(|earlier| loop_opening(earlier).len())
        .sum();
    let head = format!("loop k{ordinal}: u64 in 0..<n step 1 carry");
    LOOPS_HEAD.len() + before + head.len()
}

/// `examples/dot.lm` with the index of its first load in `count` more parentheses.
fn dot_with_wrapped_load(count: usize) -> String {
    let wrapped = format!("{DOT_LOAD_HEAD}{})", parenthesized("i + 4", count));
    DOT.replacen(DOT_LOAD, &wrapped, 1)
}

fn dot_load_index_offset() -> Result<usize, String> {
    let load = DOT
        .find(DOT_LOAD)
        .ok_or_else(|| format!("`{DOT_LOAD}` is not in examples/dot.lm"))?;
    Ok(load + DOT_LOAD_HEAD.len())
}

fn accepted(label: &str, text: String) -> NestingCase {
    (label.to_string(), text, (Vec::new(), 1))
}

fn rejected(label: &str, text: String, opener: usize) -> NestingCase {
    let record = (Kind::NestingLimit, opener, opener + 1);
    (label.to_string(), text, (vec![record], 0))
}

fn sixteen_level_cases() -> Vec<NestingCase> {
    let mut cases: Vec<NestingCase> = OPENERS
        .iter()
        .flat_map(|opener| {
            [
                accepted(
                    &format!("16 levels of {} in spec", opener.label),
                    in_spec(&nested(opener, 16)),
                ),
                accepted(
                    &format!("15 levels of {} in impl", opener.label),
                    in_impl(&nested(opener, 15)),
                ),
            ]
        })
        .collect();
    cases.extend([
        accepted(
            "-(x) in 14 parentheses",
            in_spec(&parenthesized("-(x)", 14)),
        ),
        accepted(
            "!(!c) in 13 parentheses",
            in_spec(&parenthesized("!(!c)", 13)),
        ),
        accepted(
            "siblings in 15 parentheses in spec",
            in_spec(&parenthesized(SIBLINGS, 15)),
        ),
        accepted(
            "siblings in 14 parentheses in impl",
            in_impl(&parenthesized(SIBLINGS, 14)),
        ),
        accepted("15 nested loops", nested_loops(15)),
        accepted(
            "dot with 12 more levels in a load",
            dot_with_wrapped_load(12),
        ),
    ]);
    cases
}

fn seventeen_level_cases() -> Result<Vec<NestingCase>, String> {
    let spec_start = SPEC_HEAD.len();
    let impl_start = IMPL_HEAD.len();
    let mut cases: Vec<NestingCase> = OPENERS
        .iter()
        .flat_map(|opener| {
            [
                rejected(
                    &format!("17 levels of {} in spec", opener.label),
                    in_spec(&nested(opener, 17)),
                    opener_offset(opener, spec_start, 17),
                ),
                rejected(
                    &format!("16 levels of {} in impl", opener.label),
                    in_impl(&nested(opener, 16)),
                    opener_offset(opener, impl_start, 16),
                ),
            ]
        })
        .collect();
    cases.extend([
        rejected(
            "-(x) in 15 parentheses",
            in_spec(&parenthesized("-(x)", 15)),
            spec_start + 15 + 1,
        ),
        rejected(
            "!(!c) in 14 parentheses",
            in_spec(&parenthesized("!(!c)", 14)),
            spec_start + 14 + 2,
        ),
        rejected(
            "17 levels of parenthesis in req",
            in_req(&nested(&PARENTHESIS, 17)),
            REQ_HEAD.len() + 16,
        ),
        rejected(
            "17 levels of parenthesis in ens",
            in_ens(&nested(&PARENTHESIS, 17)),
            ENS_HEAD.len() + 16,
        ),
        rejected("16 nested loops", nested_loops(16), carry_offset(16)),
        rejected(
            "dot with 13 more levels in a load",
            dot_with_wrapped_load(13),
            dot_load_index_offset()? + 12,
        ),
    ]);
    Ok(cases)
}

fn nesting_outcome(text: &str) -> NestingOutcome {
    let parsed = parse(source("nesting.lm", text.as_bytes()), ASCII);
    let records = parsed
        .diagnostics
        .iter()
        .map(|record| (record.kind, record.span.start.byte, record.span.end.byte))
        .collect();
    (records, parsed.file.items.len())
}

fn nesting_mismatches(cases: Vec<NestingCase>) -> Vec<NestingMismatch> {
    cases
        .into_iter()
        .map(|(label, text, expected)| (label, nesting_outcome(&text), expected))
        .filter(|(_, actual, expected)| actual != expected)
        .collect()
}

fn message_cases() -> Vec<MessageCase> {
    vec![
        (
            "16 parentheses in impl",
            in_impl(&nested(&PARENTHESIS, 16)),
            IMPL_MESSAGE,
        ),
        ("16 nested loops", nested_loops(16), IMPL_MESSAGE),
        (
            "17 parentheses in spec",
            in_spec(&nested(&PARENTHESIS, 17)),
            CONTRACT_MESSAGE,
        ),
        (
            "17 parentheses in req",
            in_req(&nested(&PARENTHESIS, 17)),
            CONTRACT_MESSAGE,
        ),
        (
            "17 parentheses in ens",
            in_ens(&nested(&PARENTHESIS, 17)),
            CONTRACT_MESSAGE,
        ),
    ]
}

fn message_mismatches(cases: Vec<MessageCase>) -> Vec<MessageMismatch> {
    cases
        .into_iter()
        .map(|(label, text, expected)| {
            let messages: Vec<String> = parse(source("nesting.lm", text.as_bytes()), ASCII)
                .diagnostics
                .into_iter()
                .map(|record| record.message)
                .collect();
            (label, messages, expected)
        })
        .filter(|(_, messages, expected)| *messages != [*expected])
        .collect()
}

#[test]
fn lexical_faults_are_reported_through_the_parse() -> Result<(), Box<dyn Error>> {
    let cases = LEXICAL_CASES;

    let mismatches = corpus_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn token_level_kinds_are_reported_at_their_token() -> Result<(), Box<dyn Error>> {
    let cases = TOKEN_CASES;

    let mismatches = corpus_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn delimiter_kinds_are_reported_where_the_delimiter_is_missing_or_wrong()
-> Result<(), Box<dyn Error>> {
    let cases = DELIMITER_CASES;

    let mismatches = corpus_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn an_unrepaired_record_ends_its_function_without_reporting_what_follows()
-> Result<(), Box<dyn Error>> {
    let cases = RECOVERY_CASES;

    let mismatches = corpus_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn parsing_resumes_at_the_next_item_and_keeps_the_functions_parsed_to_their_end()
-> Result<(), Box<dyn Error>> {
    let cases = RECOVERED_ITEM_CASES;

    let mismatches = item_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn a_repaired_record_continues_the_parse_as_if_its_repair_were_applied()
-> Result<(), Box<dyn Error>> {
    let cases = REPAIRED_DUMP_CASES;

    let mismatches = dump_mismatches(&cases)?;

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn unexpected_token_messages_list_spelled_tokens_then_classes() -> Result<(), Box<dyn Error>> {
    let file = "unexpected_token.lm";

    let messages = corpus_messages(file, ASCII)?;

    assert_eq!(messages, UNEXPECTED_TOKEN_MESSAGES);
    Ok(())
}

#[test]
fn an_assignment_message_does_not_pass_on_a_zero_width_space_of_the_name()
-> Result<(), Box<dyn Error>> {
    let file = "zero_width_space_in_assigned_name.lm";

    let messages = unprintable_messages(file, ASCII, Kind::Assignment)?;

    assert!(messages.is_empty(), "{messages:#?}");
    Ok(())
}

#[test]
fn sixteen_levels_of_each_opener_parse_without_diagnostic() {
    let cases = sixteen_level_cases();

    let mismatches = nesting_mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn the_token_opening_the_seventeenth_level_is_a_nesting_limit_that_ends_the_function()
-> Result<(), Box<dyn Error>> {
    let cases = seventeen_level_cases()?;

    let mismatches = nesting_mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
    Ok(())
}

#[test]
fn a_delimiter_inserted_by_a_repair_closes_its_level() {
    let text = format!(
        "{OPEN_CALL_IN_REQ}\n  spec {}{SPEC_TAIL}",
        nested(&PARENTHESIS, 16)
    );
    let insertion = OPEN_CALL_IN_REQ.len();

    let outcome = nesting_outcome(&text);

    assert_eq!(
        outcome,
        (vec![(Kind::UnclosedDelimiter, insertion, insertion)], 1)
    );
}

#[test]
fn the_nesting_limit_message_suggests_let_in_the_impl_only() {
    let cases = message_cases();

    let mismatches = message_mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn every_corpus_file_has_a_row() -> Result<(), Box<dyn Error>> {
    let listed: BTreeSet<String> = all_corpus_cases()
        .iter()
        .map(|(file, _, _)| file.to_string())
        .collect();

    let files = corpus_file_names()?;

    assert_eq!(files, listed);
    Ok(())
}

#[test]
fn each_corpus_file_renders_as_its_snapshot() -> Result<(), Box<dyn Error>> {
    let cases = all_corpus_cases();

    assert_snapshots(&cases)?;

    Ok(())
}
