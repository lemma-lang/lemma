//! The parser on any input, under both C1 variants: it never panics or overflows its stack, and
//! every record it returns is a valid syntax-class record whose spans lie in the text on code
//! point boundaries; two records without repair always have an item start between them, and a
//! text that parses without a record formats to a text that parses to the same tree.

use lemma_diagnostics::{Diagnostic, FailureClass, Kind, Source, Span};
use lemma_syntax::{C1Variant, Keyword, TokenKind, dump, format, lex, parse};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;

type RunOutcome = (Vec<Kind>, usize);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;

const NAME: &str = "robustness.lm";

const MAX_INPUT_BYTES: usize = 4096;

const RUN_LENGTH: usize = 10_000;

const DOT: &str = include_str!("../../../examples/dot.lm");

const FUNCTION_AFTER: &str = "\nfn g() -> u64 eff pure spec 0 equiv allow() impl { return 0 }\n";

const SPEC_START: &str = "fn f(x: u64) -> u64 eff pure spec ";

const IMPL_START: &str = "fn f(x: u64) -> u64 eff pure spec x equiv allow() impl { return ";

/// Every spelling of the lexical structure, the other variant's, the lookalikes, other languages'
/// keywords and operators, and the delimiters; none longer than 11 bytes.
const SPELLINGS: [&str; 149] = [
    "export",
    "fn",
    "eff",
    "req",
    "ens",
    "spec",
    "equiv",
    "allow",
    "impl",
    "let",
    "loop",
    "step",
    "carry",
    "next",
    "return",
    "result",
    "true",
    "false",
    "ro",
    "wo",
    "pure",
    "fp_reassoc",
    "fp_contract",
    "bool",
    "sum",
    "in",
    "forall",
    "\u{03A3}",
    "\u{2208}",
    "\u{2200}",
    "u8",
    "u64",
    "i32",
    "f16",
    "f32",
    "f64",
    "u128",
    "f32x4",
    "u8x16",
    "a",
    "n",
    "x",
    "c0",
    "fma",
    "reduce_sum",
    "splat",
    "load",
    "0",
    "1",
    "16",
    "0.0",
    "1.5e3",
    "007",
    "0x1F",
    ".5",
    "5.",
    "1e5",
    "1_000",
    "0.0f",
    "16u64",
    "@a4",
    "@a128",
    "@",
    "@a",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    ",",
    ":",
    ".",
    "->",
    "=",
    "&",
    "..<",
    "+",
    "-",
    "*",
    "/",
    "%",
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "&&",
    "||",
    "!",
    ";",
    "::",
    "..",
    "...",
    "..=",
    "++",
    "--",
    "+=",
    "**",
    "|",
    "^",
    "~",
    "<<",
    ">>",
    "/*",
    "*/",
    "#",
    "#[",
    "$",
    "?",
    "\\",
    "\"",
    "Abc",
    "_x",
    "\u{2211}",
    "\u{01A9}",
    "\u{03F5}",
    "\u{03B5}",
    "\u{220A}",
    "\u{2248}",
    "\u{2264}",
    "\u{00D7}",
    "\u{2212}",
    "\u{2192}",
    "\u{2026}",
    "\u{2203}",
    "\u{00A0}",
    "\u{200B}",
    "\u{FEFF}",
    "\u{202E}",
    "\u{E9}",
    "// note\n",
    "// \u{E9}\n",
    "function",
    "def",
    "pub",
    "extern",
    "requires",
    "ensures",
    "effects",
    "ret",
    "var",
    "const",
    "mut",
    "for",
    "while",
    "if",
    "match",
];

const SEPARATORS: [&str; 5] = ["", " ", "\n", "\t", "\r\n"];

fn source(text: &[u8]) -> Source<'_> {
    Source { name: NAME, text }
}

/// What is wrong with `span` as a span of `text`: its file, its bounds, its line and column, and,
/// on valid UTF-8, its code point boundaries.
fn span_violations(text: &[u8], span: &Span) -> Vec<String> {
    let (start, end) = (span.start.byte, span.end.byte);
    let recomputed = source(text).span(start..end);
    let mut violations = Vec::new();
    if recomputed.as_ref() != Ok(span) {
        violations.push(std::format!(
            "{span:?} is not the span of {start}..{end}: {recomputed:?}"
        ));
    }
    let off_boundary = std::str::from_utf8(text)
        .is_ok_and(|valid| !valid.is_char_boundary(start) || !valid.is_char_boundary(end));
    if off_boundary {
        violations.push(std::format!(
            "{start}..{end} is not on code point boundaries"
        ));
    }
    violations
}

fn record_violations(text: &[u8], record: &Diagnostic) -> Vec<String> {
    let mut violations = Vec::new();
    if let Err(violation) = record.validate() {
        violations.push(std::format!("{record:?} breaks {violation:?}"));
    }
    if record.kind.failure_class() != FailureClass::Syntax {
        violations.push(std::format!("{record:?} is not syntax class"));
    }
    violations.extend(span_violations(text, &record.span));
    for edit in &record.repair {
        violations.extend(span_violations(text, &edit.span));
    }
    violations
}

/// Pairs of consecutive unrepaired records with no `fn` or `export` token starting from the first
/// one's start to the second one's: the token a function stops at may be the one parsing resumes at.
fn unseparated_unrepaired_records(
    text: &[u8],
    variant: C1Variant,
    records: &[Diagnostic],
) -> Vec<String> {
    let item_starts: Vec<usize> = lex(text, variant)
        .tokens
        .iter()
        .filter(|token| {
            matches!(
                token.kind,
                TokenKind::Keyword(Keyword::Fn | Keyword::Export)
            )
        })
        .map(|token| token.range.start)
        .collect();
    let mut unrepaired: Vec<&Span> = records
        .iter()
        .filter(|record| record.repair.is_empty())
        .map(|record| &record.span)
        .collect();
    unrepaired.sort_by_key(|span| (span.start.byte, span.end.byte));
    unrepaired
        .windows(2)
        .filter(|pair| {
            !item_starts
                .iter()
                .any(|&start| (pair[0].start.byte..=pair[1].start.byte).contains(&start))
        })
        .map(|pair| std::format!("no item start between {:?} and {:?}", pair[0], pair[1]))
        .collect()
}

/// For a text that parses without a record: what differs once it is formatted and parsed again.
fn round_trip_violations(text: &[u8], variant: C1Variant) -> Vec<String> {
    let parsed = parse(source(text), variant);
    if !parsed.diagnostics.is_empty() {
        return Vec::new();
    }
    let formatted = match format(source(text), variant) {
        Ok(formatted) => formatted,
        Err(records) => return vec![std::format!("a clean parse does not format: {records:?}")],
    };
    let reparsed = parse(source(formatted.as_bytes()), variant);
    let expected = (dump(&parsed.file), 0);
    let actual = (dump(&reparsed.file), reparsed.diagnostics.len());
    if actual == expected {
        return Vec::new();
    }
    vec![std::format!(
        "{formatted:?} parses to {actual:?}, not {expected:?}"
    )]
}

/// Everything the parse of `text` under `variant` breaks of what any input must give.
fn violations(text: &[u8], variant: C1Variant) -> Vec<String> {
    let records = parse(source(text), variant).diagnostics;
    let mut violations: Vec<String> = records
        .iter()
        .flat_map(|record| record_violations(text, record))
        .collect();
    violations.extend(unseparated_unrepaired_records(text, variant, &records));
    violations.extend(round_trip_violations(text, variant));
    violations
}

fn violations_under_both_variants(text: &[u8]) -> [Vec<String>; 2] {
    [violations(text, ASCII), violations(text, UNICODE)]
}

fn token_soup() -> BoxedStrategy<String> {
    vec(
        (select(SPELLINGS.to_vec()), select(SEPARATORS.to_vec())),
        0..300,
    )
    .prop_map(|pieces| {
        pieces
            .into_iter()
            .flat_map(|(spelling, separator)| [spelling, separator])
            .collect()
    })
    .boxed()
}

/// The last code point boundary of `text` at or before `byte`.
fn boundary_at_or_before(text: &str, byte: usize) -> usize {
    (0..=byte.min(text.len()))
        .rev()
        .find(|&candidate| text.is_char_boundary(candidate))
        .unwrap_or(0)
}

fn spliced(text: String, (start, length, spelling): (usize, usize, &str)) -> String {
    let start = boundary_at_or_before(&text, start);
    let end = boundary_at_or_before(&text, start + length);
    let mut spliced = text;
    spliced.replace_range(start..end, spelling);
    spliced
}

/// `examples/dot.lm` with up to four of its byte ranges replaced by spellings of the soup.
fn mutated_dot() -> BoxedStrategy<String> {
    let splice = (0..DOT.len(), 0..8_usize, select(SPELLINGS.to_vec()));
    vec(splice, 1..5)
        .prop_map(|splices| splices.into_iter().fold(DOT.to_string(), spliced))
        .boxed()
}

fn runs() -> Vec<String> {
    let parentheses = "(".repeat(RUN_LENGTH);
    let minuses = "- ".repeat(RUN_LENGTH);
    let joined_minuses = "-".repeat(RUN_LENGTH);
    vec![
        parentheses.clone(),
        std::format!("{SPEC_START}{parentheses}{FUNCTION_AFTER}"),
        std::format!("{IMPL_START}{parentheses}{FUNCTION_AFTER}"),
        std::format!("{SPEC_START}{minuses}x{FUNCTION_AFTER}"),
        std::format!("{SPEC_START}{joined_minuses}x{FUNCTION_AFTER}"),
        std::format!("{SPEC_START}{}x{FUNCTION_AFTER}", "!(".repeat(RUN_LENGTH)),
    ]
}

fn run_outcome(text: &str) -> RunOutcome {
    let parsed = parse(source(text.as_bytes()), ASCII);
    let kinds = parsed
        .diagnostics
        .iter()
        .map(|record| record.kind)
        .collect();
    (kinds, parsed.file.items.len())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn arbitrary_bytes_yield_only_well_formed_syntax_records(
        text in vec(any::<u8>(), 0..MAX_INPUT_BYTES)
    ) {
        let violations = violations_under_both_variants(&text);

        prop_assert_eq!(violations, [Vec::<String>::new(), Vec::new()]);
    }

    #[test]
    fn a_soup_of_tokens_yields_only_well_formed_syntax_records(text in token_soup()) {
        let violations = violations_under_both_variants(text.as_bytes());

        prop_assert_eq!(violations, [Vec::<String>::new(), Vec::new()]);
    }

    #[test]
    fn a_mutated_dot_yields_only_well_formed_syntax_records(text in mutated_dot()) {
        let violations = violations_under_both_variants(text.as_bytes());

        prop_assert_eq!(violations, [Vec::<String>::new(), Vec::new()]);
    }
}

#[test]
fn runs_of_ten_thousand_openers_yield_only_well_formed_syntax_records() {
    let texts = runs();

    let violations: Vec<[Vec<String>; 2]> = texts
        .iter()
        .map(|text| violations_under_both_variants(text.as_bytes()))
        .collect();

    assert_eq!(
        violations,
        vec![[Vec::<String>::new(), Vec::new()]; texts.len()]
    );
}

#[test]
fn a_run_of_ten_thousand_parentheses_in_a_clause_is_one_nesting_limit_record() {
    let text = std::format!("{SPEC_START}{}{FUNCTION_AFTER}", "(".repeat(RUN_LENGTH));

    let outcome = run_outcome(&text);

    assert_eq!(outcome, (vec![Kind::NestingLimit], 1));
}

#[test]
fn a_run_of_ten_thousand_unary_minuses_in_the_impl_is_one_nesting_limit_record() {
    let text = std::format!("{IMPL_START}{}x{FUNCTION_AFTER}", "- ".repeat(RUN_LENGTH));

    let outcome = run_outcome(&text);

    assert_eq!(outcome, (vec![Kind::NestingLimit], 1));
}
