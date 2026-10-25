//! The lexer of `docs/grammar.md` section 2: tokens, their byte ranges, and the lexical faults of
//! section 7 attached to them, under each C1 variant.

use lemma_diagnostics::Kind;
use lemma_syntax::{C1Variant, TextRange, TokenKind, lex};

type Edit = (TextRange, String);
type FaultView = (Kind, Vec<Edit>);
type TokenView = (TokenKind, TextRange, Vec<FaultView>);
type StreamCase = (&'static str, C1Variant, Vec<TokenView>);
type StreamMismatch = (&'static str, C1Variant, Vec<TokenView>, Vec<TokenView>);
type CommentView = (TextRange, String);
type KindAndRange = (TokenKind, TextRange);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;
const VARIANTS: [C1Variant; 2] = [ASCII, UNICODE];

const SIGMA: &str = "\u{03A3}";
const ELEMENT_OF: &str = "\u{2208}";
const FOR_ALL: &str = "\u{2200}";

const PUNCTUATION: [&str; 27] = [
    "(", ")", "[", "]", "{", "}", ",", ":", ".", "->", "=", "&", "..<", "+", "-", "*", "/", "%",
    "==", "!=", "<", "<=", ">", ">=", "&&", "||", "!",
];

const KEYWORDS: [&str; 24] = [
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
];

const SCALAR_TYPES: [&str; 13] = [
    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f16", "f32", "f64",
];

const VECTOR_TYPES: [&str; 8] = [
    "f32x4", "u8x16", "i64x2", "f64x8", "u16x32", "i8x64", "u128x2", "f16x4",
];

const NAMES: [&str; 19] = [
    "a",
    "c0",
    "reduce_sum",
    "i0",
    "i1",
    "f2",
    "u7",
    "u256",
    "f8",
    "f128",
    "u8x3",
    "f32x128",
    "u8x",
    "x4",
    "requires",
    "ensures",
    "effects",
    "ret",
    "function",
];

const SUM_LOOKALIKES: [&str; 8] = [
    "\u{2211}",
    "\u{01A9}",
    "\u{2140}",
    "\u{1D6BA}",
    "\u{1D6F4}",
    "\u{1D72E}",
    "\u{1D768}",
    "\u{1D7A2}",
];

const MEMBERSHIP_LOOKALIKE: &str = "\u{220A}";
const QUANTIFIER_LOOKALIKE: &str = "\u{2C6F}";

const OPERATOR_LOOKALIKES: [(&str, &str); 11] = [
    ("\u{2264}", "<="),
    ("\u{2265}", ">="),
    ("\u{2260}", "!="),
    ("\u{00D7}", "*"),
    ("\u{22C5}", "*"),
    ("\u{00B7}", "*"),
    ("\u{2212}", "-"),
    ("\u{2192}", "->"),
    ("\u{2227}", "&&"),
    ("\u{2228}", "||"),
    ("\u{00AC}", "!"),
];

const UNREPAIRED_SPELLINGS: [(&str, Kind); 38] = [
    ("..", Kind::RangeOperator),
    ("...", Kind::RangeOperator),
    ("..=", Kind::RangeOperator),
    ("\u{2026}", Kind::RangeOperator),
    ("++", Kind::UnsupportedOperator),
    ("--", Kind::UnsupportedOperator),
    ("**", Kind::UnsupportedOperator),
    ("+=", Kind::UnsupportedOperator),
    ("-=", Kind::UnsupportedOperator),
    ("*=", Kind::UnsupportedOperator),
    ("/=", Kind::UnsupportedOperator),
    ("%=", Kind::UnsupportedOperator),
    ("|", Kind::UnsupportedOperator),
    ("^", Kind::UnsupportedOperator),
    ("~", Kind::UnsupportedOperator),
    ("<<", Kind::UnsupportedOperator),
    (">>", Kind::UnsupportedOperator),
    ("\u{2209}", Kind::Lookalike),
    ("\u{220B}", Kind::Lookalike),
    ("\u{220C}", Kind::Lookalike),
    ("\u{220D}", Kind::Lookalike),
    ("\u{2203}", Kind::Lookalike),
    ("\u{2204}", Kind::Lookalike),
    ("$", Kind::InvalidCharacter),
    ("`", Kind::InvalidCharacter),
    ("'", Kind::InvalidCharacter),
    ("\"", Kind::InvalidCharacter),
    ("?", Kind::InvalidCharacter),
    ("\\", Kind::InvalidCharacter),
    ("\u{7}", Kind::InvalidCharacter),
    ("@", Kind::InvalidCharacter),
    ("\u{E9}", Kind::NonAscii),
    ("\u{1F600}", Kind::NonAscii),
    ("Abc", Kind::InvalidIdentifier),
    ("_x", Kind::InvalidIdentifier),
    ("aB", Kind::InvalidIdentifier),
    ("007", Kind::InvalidNumber),
    ("0x1F", Kind::InvalidNumber),
];

fn at(start: usize, end: usize) -> TextRange {
    TextRange { start, end }
}

fn edit(start: usize, end: usize, replacement: &str) -> Edit {
    (at(start, end), replacement.to_string())
}

fn view(text: &[u8], variant: C1Variant) -> Vec<TokenView> {
    lex(text, variant)
        .tokens
        .into_iter()
        .map(|token| {
            let faults = token
                .faults
                .into_iter()
                .map(|fault| (fault.kind, fault.repair))
                .collect();
            (token.kind, token.range, faults)
        })
        .collect()
}

fn view_of(text: &str, variant: C1Variant) -> Vec<TokenView> {
    view(text.as_bytes(), variant)
}

fn kind_of(spelling: &str, variant: C1Variant) -> TokenKind {
    view_of(spelling, variant).remove(0).0
}

fn first_kind_and_range(text: &str, variant: C1Variant) -> Option<KindAndRange> {
    let first = view_of(text, variant).into_iter().next()?;
    Some((first.0, first.1))
}

fn name(text: &str) -> TokenKind {
    TokenKind::Name(text.to_string())
}

fn plain(kind: TokenKind, start: usize, end: usize) -> TokenView {
    (kind, at(start, end), Vec::new())
}

fn faulted(kind: TokenKind, start: usize, end: usize, fault: Kind, edits: Vec<Edit>) -> TokenView {
    (kind, at(start, end), vec![(fault, edits)])
}

fn unrepaired(start: usize, end: usize, fault: Kind) -> TokenView {
    faulted(TokenKind::Error, start, end, fault, Vec::new())
}

fn eof(offset: usize) -> TokenView {
    plain(TokenKind::Eof, offset, offset)
}

/// The kind of the single fault-free token spanning `text`, when `text` lexes as exactly that.
fn lone_token_kind(text: &str, variant: C1Variant) -> Option<TokenKind> {
    let tokens = view_of(text, variant);
    let kind = tokens.first()?.0.clone();
    let expected = vec![plain(kind.clone(), 0, text.len()), eof(text.len())];
    (tokens == expected).then_some(kind)
}

fn labelled_kinds(spellings: &[&str]) -> Vec<(String, TokenKind)> {
    spellings
        .iter()
        .map(|spelling| (spelling.to_string(), kind_of(spelling, ASCII)))
        .collect()
}

/// Every pair of labels whose kinds are equal.
fn shared_kinds(entries: &[(String, TokenKind)]) -> Vec<(String, String)> {
    entries
        .iter()
        .enumerate()
        .flat_map(|(index, (label, kind))| {
            entries[index + 1..]
                .iter()
                .filter(move |(_, other)| other == kind)
                .map(move |(other_label, _)| (label.clone(), other_label.clone()))
        })
        .collect()
}

fn mismatched_streams(cases: Vec<StreamCase>) -> Vec<StreamMismatch> {
    cases
        .into_iter()
        .map(|(text, variant, expected)| (text, variant, view_of(text, variant), expected))
        .filter(|(_, _, actual, expected)| actual != expected)
        .collect()
}

/// A spelling that lexes alone as the token its repair lexes as, carrying that one fault.
fn repaired(
    text: &'static str,
    variant: C1Variant,
    repair: &str,
    fault: Kind,
    edits: Vec<Edit>,
) -> StreamCase {
    let token = faulted(kind_of(repair, variant), 0, text.len(), fault, edits);
    (text, variant, vec![token, eof(text.len())])
}

fn replaced_whole(
    text: &'static str,
    variant: C1Variant,
    repair: &'static str,
    fault: Kind,
) -> StreamCase {
    repaired(
        text,
        variant,
        repair,
        fault,
        vec![edit(0, text.len(), repair)],
    )
}

fn c1_lookalike_cases(
    text: &'static str,
    ascii: &'static str,
    unicode: &'static str,
) -> Vec<StreamCase> {
    vec![
        replaced_whole(text, ASCII, ascii, Kind::Lookalike),
        replaced_whole(text, UNICODE, unicode, Kind::Lookalike),
    ]
}

fn operator_lookalike_cases() -> Vec<StreamCase> {
    OPERATOR_LOOKALIKES
        .into_iter()
        .flat_map(|(text, repair)| {
            VARIANTS
                .into_iter()
                .map(move |variant| replaced_whole(text, variant, repair, Kind::Lookalike))
        })
        .collect()
}

fn invalid_number_cases() -> Vec<StreamCase> {
    vec![
        repaired(
            ".5",
            ASCII,
            "0.5",
            Kind::InvalidNumber,
            vec![edit(0, 0, "0")],
        ),
        repaired(
            "5.",
            ASCII,
            "5.0",
            Kind::InvalidNumber,
            vec![edit(2, 2, "0")],
        ),
        repaired(
            "1e5",
            ASCII,
            "1.0e5",
            Kind::InvalidNumber,
            vec![edit(1, 1, ".0")],
        ),
        repaired(
            "1e-5",
            ASCII,
            "1.0e-5",
            Kind::InvalidNumber,
            vec![edit(1, 1, ".0")],
        ),
        repaired(
            "1.0e+5",
            ASCII,
            "1.0e5",
            Kind::InvalidNumber,
            vec![edit(4, 5, "")],
        ),
        repaired(
            "1_000",
            ASCII,
            "1000",
            Kind::InvalidNumber,
            vec![edit(1, 2, "")],
        ),
        repaired(
            "0.0f",
            ASCII,
            "0.0",
            Kind::InvalidNumber,
            vec![edit(3, 4, "")],
        ),
        repaired(
            "16u64",
            ASCII,
            "16",
            Kind::InvalidNumber,
            vec![edit(2, 5, "")],
        ),
        repaired(
            "1.0_f32",
            ASCII,
            "1.0",
            Kind::InvalidNumber,
            vec![edit(3, 7, "")],
        ),
    ]
}

fn repaired_spelling_cases() -> Vec<StreamCase> {
    let sums = SUM_LOOKALIKES
        .into_iter()
        .flat_map(|text| c1_lookalike_cases(text, "sum", SIGMA));
    let memberships = c1_lookalike_cases(MEMBERSHIP_LOOKALIKE, "in", ELEMENT_OF);
    let quantifiers = c1_lookalike_cases(QUANTIFIER_LOOKALIKE, "forall", FOR_ALL);
    let path_separator = replaced_whole("::", ASCII, ".", Kind::PathSeparator);
    sums.chain(memberships)
        .chain(quantifiers)
        .chain(operator_lookalike_cases())
        .chain(invalid_number_cases())
        .chain([path_separator])
        .collect()
}

fn unrepaired_spelling_cases() -> Vec<StreamCase> {
    UNREPAIRED_SPELLINGS
        .into_iter()
        .flat_map(|(text, fault)| {
            VARIANTS.into_iter().map(move |variant| {
                (
                    text,
                    variant,
                    vec![unrepaired(0, text.len(), fault), eof(text.len())],
                )
            })
        })
        .collect()
}

#[test]
fn the_default_variant_is_ascii() {
    let variant = C1Variant::default();

    assert_eq!(variant, C1Variant::Ascii);
}

#[test]
fn each_punctuation_and_operator_lexes_as_one_fault_free_token() {
    let spellings = PUNCTUATION;

    let rejected: Vec<&str> = spellings
        .into_iter()
        .filter(|spelling| lone_token_kind(spelling, ASCII).is_none())
        .collect();

    assert!(
        rejected.is_empty(),
        "not one fault-free token: {rejected:?}"
    );
}

#[test]
fn punctuation_and_operators_have_distinct_kinds() {
    let mut entries = labelled_kinds(&PUNCTUATION);
    entries.push(("error".to_string(), TokenKind::Error));
    entries.push(("end of input".to_string(), TokenKind::Eof));

    let shared = shared_kinds(&entries);

    assert!(shared.is_empty(), "same kind: {shared:?}");
}

#[test]
fn longest_match_splits_adjacent_spellings() {
    let cases: Vec<StreamCase> = vec![
        (
            "0..<n",
            ASCII,
            vec![
                plain(TokenKind::Int("0".to_string()), 0, 1),
                plain(kind_of("..<", ASCII), 1, 4),
                plain(name("n"), 4, 5),
                eof(5),
            ],
        ),
        (
            "0..n",
            ASCII,
            vec![
                plain(TokenKind::Int("0".to_string()), 0, 1),
                unrepaired(1, 3, Kind::RangeOperator),
                plain(name("n"), 3, 4),
                eof(4),
            ],
        ),
        (
            "a->b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                plain(kind_of("->", ASCII), 1, 3),
                plain(name("b"), 3, 4),
                eof(4),
            ],
        ),
        (
            "- >",
            ASCII,
            vec![
                plain(kind_of("-", ASCII), 0, 1),
                plain(kind_of(">", ASCII), 2, 3),
                eof(3),
            ],
        ),
        (
            "a<=b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                plain(kind_of("<=", ASCII), 1, 3),
                plain(name("b"), 3, 4),
                eof(4),
            ],
        ),
        (
            "! =",
            ASCII,
            vec![
                plain(kind_of("!", ASCII), 0, 1),
                plain(kind_of("=", ASCII), 2, 3),
                eof(3),
            ],
        ),
        (
            "a&&b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                plain(kind_of("&&", ASCII), 1, 3),
                plain(name("b"), 3, 4),
                eof(4),
            ],
        ),
        (
            "f32x4.splat(0.0)",
            ASCII,
            vec![
                plain(kind_of("f32x4", ASCII), 0, 5),
                plain(kind_of(".", ASCII), 5, 6),
                plain(name("splat"), 6, 11),
                plain(kind_of("(", ASCII), 11, 12),
                plain(TokenKind::Float("0.0".to_string()), 12, 15),
                plain(kind_of(")", ASCII), 15, 16),
                eof(16),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn each_keyword_lexes_as_a_keyword_token() {
    let words = KEYWORDS;

    let rejected: Vec<&str> = words
        .into_iter()
        .filter(|word| !matches!(lone_token_kind(word, ASCII), Some(TokenKind::Keyword(_))))
        .collect();

    assert!(rejected.is_empty(), "not a lone keyword: {rejected:?}");
}

#[test]
fn keywords_have_distinct_kinds() {
    let entries = labelled_kinds(&KEYWORDS);

    let shared = shared_kinds(&entries);

    assert!(shared.is_empty(), "same kind: {shared:?}");
}

#[test]
fn words_outside_the_reserved_patterns_lex_as_names() {
    let words = NAMES;

    let rejected: Vec<&str> = words
        .into_iter()
        .filter(|word| lone_token_kind(word, ASCII) != Some(name(word)))
        .collect();

    assert!(rejected.is_empty(), "not a lone name: {rejected:?}");
}

#[test]
fn each_reserved_scalar_type_name_lexes_as_a_scalar_type() {
    let words = SCALAR_TYPES;

    let rejected: Vec<&str> = words
        .into_iter()
        .filter(|word| !matches!(lone_token_kind(word, ASCII), Some(TokenKind::ScalarType(_))))
        .collect();

    assert!(rejected.is_empty(), "not a lone scalar type: {rejected:?}");
}

#[test]
fn reserved_vector_type_names_lex_as_vector_types() {
    let words = VECTOR_TYPES;

    let rejected: Vec<&str> = words
        .into_iter()
        .filter(|word| !matches!(lone_token_kind(word, ASCII), Some(TokenKind::VectorType(_))))
        .collect();

    assert!(rejected.is_empty(), "not a lone vector type: {rejected:?}");
}

#[test]
fn reserved_type_names_have_distinct_kinds() {
    let entries = labelled_kinds(&[SCALAR_TYPES.as_slice(), VECTOR_TYPES.as_slice()].concat());

    let shared = shared_kinds(&entries);

    assert!(shared.is_empty(), "same kind: {shared:?}");
}

#[test]
fn valid_literals_and_alignments_keep_their_spelling() {
    let cases: [(&str, TokenKind); 8] = [
        ("0", TokenKind::Int("0".to_string())),
        ("42", TokenKind::Int("42".to_string())),
        ("0.0", TokenKind::Float("0.0".to_string())),
        ("3.25", TokenKind::Float("3.25".to_string())),
        ("1.5e3", TokenKind::Float("1.5e3".to_string())),
        ("2.0e-7", TokenKind::Float("2.0e-7".to_string())),
        ("@a128", TokenKind::Align("128".to_string())),
        ("@a8", TokenKind::Align("8".to_string())),
    ];

    let mismatches: Vec<(&str, Option<TokenKind>)> = cases
        .into_iter()
        .map(|(text, expected)| (text, lone_token_kind(text, ASCII), expected))
        .filter(|(_, actual, expected)| actual.as_ref() != Some(expected))
        .map(|(text, actual, _)| (text, actual))
        .collect();

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn c1_spellings_lex_as_c1_tokens_carrying_the_variant_of_their_spelling() {
    let cases: Vec<StreamCase> = VARIANTS
        .into_iter()
        .flat_map(|variant| {
            [
                ("sum", TokenKind::Sum(C1Variant::Ascii)),
                (SIGMA, TokenKind::Sum(C1Variant::Unicode)),
                ("in", TokenKind::In(C1Variant::Ascii)),
                (ELEMENT_OF, TokenKind::In(C1Variant::Unicode)),
                ("forall", TokenKind::Forall(C1Variant::Ascii)),
                (FOR_ALL, TokenKind::Forall(C1Variant::Unicode)),
            ]
            .map(|(text, kind)| {
                (
                    text,
                    variant,
                    vec![plain(kind, 0, text.len()), eof(text.len())],
                )
            })
        })
        .collect();

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn membership_and_almost_equal_lookalikes_lex_as_their_own_tokens() {
    let cases: Vec<(&str, C1Variant, TokenKind)> = VARIANTS
        .into_iter()
        .flat_map(|variant| {
            [
                ("\u{03F5}", variant, TokenKind::MembershipLookalike),
                ("\u{03B5}", variant, TokenKind::MembershipLookalike),
                ("\u{2248}", variant, TokenKind::AlmostEqual),
            ]
        })
        .collect();

    let mismatches: Vec<(&str, C1Variant, Option<KindAndRange>)> = cases
        .into_iter()
        .map(|(text, variant, kind)| (text, variant, first_kind_and_range(text, variant), kind))
        .filter(|(text, _, actual, kind)| *actual != Some((kind.clone(), at(0, text.len()))))
        .map(|(text, variant, actual, _)| (text, variant, actual))
        .collect();

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn repaired_spellings_lex_as_their_repair_carrying_one_fault() {
    let cases = repaired_spelling_cases();

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn ascii_word_repairs_are_padded_where_they_touch_a_word_character() {
    let cases: Vec<StreamCase> = vec![
        (
            "spec\u{2211}(",
            ASCII,
            vec![
                plain(kind_of("spec", ASCII), 0, 4),
                faulted(
                    TokenKind::Sum(C1Variant::Ascii),
                    4,
                    7,
                    Kind::Lookalike,
                    vec![edit(4, 7, " sum")],
                ),
                plain(kind_of("(", ASCII), 7, 8),
                eof(8),
            ],
        ),
        (
            "spec\u{2211}(",
            UNICODE,
            vec![
                plain(kind_of("spec", UNICODE), 0, 4),
                faulted(
                    TokenKind::Sum(C1Variant::Unicode),
                    4,
                    7,
                    Kind::Lookalike,
                    vec![edit(4, 7, SIGMA)],
                ),
                plain(kind_of("(", UNICODE), 7, 8),
                eof(8),
            ],
        ),
        (
            "\u{2211}x",
            ASCII,
            vec![
                faulted(
                    TokenKind::Sum(C1Variant::Ascii),
                    0,
                    3,
                    Kind::Lookalike,
                    vec![edit(0, 3, "sum ")],
                ),
                plain(name("x"), 3, 4),
                eof(4),
            ],
        ),
        (
            "i\u{220A}0",
            ASCII,
            vec![
                plain(name("i"), 0, 1),
                faulted(
                    TokenKind::In(C1Variant::Ascii),
                    1,
                    4,
                    Kind::Lookalike,
                    vec![edit(1, 4, " in ")],
                ),
                plain(TokenKind::Int("0".to_string()), 4, 5),
                eof(5),
            ],
        ),
        (
            "i\u{220A}0",
            UNICODE,
            vec![
                plain(name("i"), 0, 1),
                faulted(
                    TokenKind::In(C1Variant::Unicode),
                    1,
                    4,
                    Kind::Lookalike,
                    vec![edit(1, 4, ELEMENT_OF)],
                ),
                plain(TokenKind::Int("0".to_string()), 4, 5),
                eof(5),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn unrepaired_spellings_lex_as_one_error_token_carrying_their_fault() {
    let cases = unrepaired_spelling_cases();

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn invalid_encoding_is_one_error_token_per_maximal_invalid_run() {
    let cases: [(&[u8], Vec<TokenView>); 3] = [
        (
            b"a\xff\xfeb",
            vec![
                plain(name("a"), 0, 1),
                unrepaired(1, 3, Kind::InvalidEncoding),
                plain(name("b"), 3, 4),
                eof(4),
            ],
        ),
        (
            b"a\xffb\xffc",
            vec![
                plain(name("a"), 0, 1),
                unrepaired(1, 2, Kind::InvalidEncoding),
                plain(name("b"), 2, 3),
                unrepaired(3, 4, Kind::InvalidEncoding),
                plain(name("c"), 4, 5),
                eof(5),
            ],
        ),
        (
            b"\xe2\x82a",
            vec![
                unrepaired(0, 2, Kind::InvalidEncoding),
                plain(name("a"), 2, 3),
                eof(3),
            ],
        ),
    ];

    let mismatches: Vec<(&[u8], Vec<TokenView>, Vec<TokenView>)> = cases
        .into_iter()
        .map(|(text, expected)| (text, view(text, ASCII), expected))
        .filter(|(_, actual, expected)| actual != expected)
        .collect();

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_dot_before_a_digit_outside_a_number_is_an_invalid_number() {
    let cases: Vec<StreamCase> = vec![
        (
            "(.5)",
            ASCII,
            vec![
                plain(kind_of("(", ASCII), 0, 1),
                faulted(
                    TokenKind::Float("0.5".to_string()),
                    1,
                    3,
                    Kind::InvalidNumber,
                    vec![edit(1, 1, "0")],
                ),
                plain(kind_of(")", ASCII), 3, 4),
                eof(4),
            ],
        ),
        (
            "x.5",
            ASCII,
            vec![
                plain(name("x"), 0, 1),
                faulted(
                    TokenKind::Float("0.5".to_string()),
                    1,
                    3,
                    Kind::InvalidNumber,
                    vec![edit(1, 1, " 0")],
                ),
                eof(3),
            ],
        ),
        (
            "x_.5",
            ASCII,
            vec![
                plain(name("x_"), 0, 2),
                faulted(
                    TokenKind::Float("0.5".to_string()),
                    2,
                    4,
                    Kind::InvalidNumber,
                    vec![edit(2, 2, " 0")],
                ),
                eof(4),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn an_at_sign_not_followed_by_a_and_digits_is_an_invalid_character() {
    let cases: Vec<StreamCase> = vec![
        (
            "@a x",
            ASCII,
            vec![
                unrepaired(0, 1, Kind::InvalidCharacter),
                plain(name("a"), 1, 2),
                plain(name("x"), 3, 4),
                eof(4),
            ],
        ),
        (
            "@16",
            ASCII,
            vec![
                unrepaired(0, 1, Kind::InvalidCharacter),
                plain(TokenKind::Int("16".to_string()), 1, 3),
                eof(3),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn whitespace_and_comments_separate_tokens_without_faults() {
    let cases: Vec<StreamCase> = vec![
        ("", ASCII, vec![eof(0)]),
        (" \t\r\n", ASCII, vec![eof(4)]),
        (
            "a \tb\r\nc",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                plain(name("b"), 3, 4),
                plain(name("c"), 6, 7),
                eof(7),
            ],
        ),
        (
            "a// b\nc",
            ASCII,
            vec![plain(name("a"), 0, 1), plain(name("c"), 6, 7), eof(7)],
        ),
        (
            "// \u{03A3} \u{E9} \u{2211}\nfn",
            ASCII,
            vec![plain(kind_of("fn", ASCII), 13, 15), eof(15)],
        ),
        (
            "// \u{03A3} \u{E9} \u{2211}\nfn",
            UNICODE,
            vec![plain(kind_of("fn", UNICODE), 13, 15), eof(15)],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn comments_are_collected_with_their_ranges() {
    let cases: [(&str, Vec<CommentView>); 6] = [
        ("a // note\nb", vec![(at(2, 9), "// note".to_string())]),
        ("/// doc\nfn", vec![(at(0, 7), "/// doc".to_string())]),
        (
            "// one\n// two",
            vec![
                (at(0, 6), "// one".to_string()),
                (at(7, 13), "// two".to_string()),
            ],
        ),
        ("a // x", vec![(at(2, 6), "// x".to_string())]),
        (
            "// \u{03A3} \u{E9} \u{2211}\nfn",
            vec![(at(0, 12), "// \u{03A3} \u{E9} \u{2211}".to_string())],
        ),
        ("a / b", Vec::new()),
    ];

    let mismatches: Vec<(&str, Vec<CommentView>, Vec<CommentView>)> = cases
        .into_iter()
        .map(|(text, expected)| {
            let comments = lex(text.as_bytes(), ASCII)
                .comments
                .into_iter()
                .map(|comment| (comment.range, comment.text))
                .collect();
            (text, comments, expected)
        })
        .filter(|(_, actual, expected)| actual != expected)
        .collect();

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn trivia_faults_attach_to_the_next_token() {
    let cases: Vec<StreamCase> = vec![
        (
            "a\u{00A0}b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 3, 4, Kind::Lookalike, vec![edit(1, 3, " ")]),
                eof(4),
            ],
        ),
        (
            "a\u{2007}b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 4, 5, Kind::Lookalike, vec![edit(1, 4, " ")]),
                eof(5),
            ],
        ),
        (
            "a\u{2009}b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 4, 5, Kind::Lookalike, vec![edit(1, 4, " ")]),
                eof(5),
            ],
        ),
        (
            "a\u{202F}b",
            UNICODE,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 4, 5, Kind::Lookalike, vec![edit(1, 4, " ")]),
                eof(5),
            ],
        ),
        (
            "a \u{200B}b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 5, 6, Kind::Lookalike, vec![edit(2, 5, "")]),
                eof(6),
            ],
        ),
        (
            "\u{FEFF}fn",
            ASCII,
            vec![
                faulted(
                    kind_of("fn", ASCII),
                    3,
                    5,
                    Kind::Lookalike,
                    vec![edit(0, 3, "")],
                ),
                eof(5),
            ],
        ),
        (
            "a \u{00A0}",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(TokenKind::Eof, 4, 4, Kind::Lookalike, vec![edit(2, 4, " ")]),
            ],
        ),
        (
            "\u{FEFF}\u{00A0}fn",
            ASCII,
            vec![
                (
                    kind_of("fn", ASCII),
                    at(5, 7),
                    vec![
                        (Kind::Lookalike, vec![edit(0, 3, "")]),
                        (Kind::Lookalike, vec![edit(3, 5, " ")]),
                    ],
                ),
                eof(7),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_zero_width_space_inside_a_word_lexes_as_one_word() {
    let text = "ab\u{200B}cd";

    let tokens = view_of(text, ASCII);

    assert_eq!(
        tokens,
        vec![
            faulted(name("abcd"), 0, 7, Kind::Lookalike, vec![edit(2, 5, "")]),
            eof(7),
        ]
    );
}

#[test]
fn semicolon_repairs_depend_on_their_line() {
    let cases: Vec<StreamCase> = vec![
        (
            "; note\nfn",
            ASCII,
            vec![
                faulted(
                    kind_of("fn", ASCII),
                    7,
                    9,
                    Kind::Semicolon,
                    vec![edit(0, 1, "//")],
                ),
                eof(9),
            ],
        ),
        (
            ";; note\nfn",
            ASCII,
            vec![
                faulted(
                    kind_of("fn", ASCII),
                    8,
                    10,
                    Kind::Semicolon,
                    vec![edit(0, 2, "//")],
                ),
                eof(10),
            ],
        ),
        (
            "  ; note\nfn",
            ASCII,
            vec![
                faulted(
                    kind_of("fn", ASCII),
                    9,
                    11,
                    Kind::Semicolon,
                    vec![edit(2, 3, "//")],
                ),
                eof(11),
            ],
        ),
        (
            "a;\nb",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(name("b"), 3, 4, Kind::Semicolon, vec![edit(1, 2, "")]),
                eof(4),
            ],
        ),
        (
            "a;}",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(
                    kind_of("}", ASCII),
                    2,
                    3,
                    Kind::Semicolon,
                    vec![edit(1, 2, "")],
                ),
                eof(3),
            ],
        ),
        (
            "a;return b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(
                    kind_of("return", ASCII),
                    2,
                    8,
                    Kind::Semicolon,
                    vec![edit(1, 2, "")],
                ),
                plain(name("b"), 9, 10),
                eof(10),
            ],
        ),
        (
            "a;",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(TokenKind::Eof, 2, 2, Kind::Semicolon, vec![edit(1, 2, "")]),
            ],
        ),
        (
            "a; b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                unrepaired(1, 2, Kind::Semicolon),
                plain(name("b"), 3, 4),
                eof(4),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn block_comment_repairs_depend_on_their_line() {
    let cases: Vec<StreamCase> = vec![
        (
            "a /*note*/\nb",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                faulted(
                    name("b"),
                    11,
                    12,
                    Kind::BlockComment,
                    vec![edit(2, 10, "//note")],
                ),
                eof(12),
            ],
        ),
        (
            "a /*x*/ b",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                unrepaired(2, 7, Kind::BlockComment),
                plain(name("b"), 8, 9),
                eof(9),
            ],
        ),
        (
            "/* x\ny */fn",
            ASCII,
            vec![
                unrepaired(0, 9, Kind::BlockComment),
                plain(kind_of("fn", ASCII), 9, 11),
                eof(11),
            ],
        ),
        (
            "/* x",
            ASCII,
            vec![unrepaired(0, 4, Kind::BlockComment), eof(4)],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn hash_comment_repairs_depend_on_their_line() {
    let cases: Vec<StreamCase> = vec![
        (
            "# note\nfn",
            ASCII,
            vec![
                faulted(
                    kind_of("fn", ASCII),
                    7,
                    9,
                    Kind::HashComment,
                    vec![edit(0, 1, "//")],
                ),
                eof(9),
            ],
        ),
        (
            "#[test]\nfn",
            ASCII,
            vec![
                unrepaired(0, 7, Kind::HashComment),
                plain(kind_of("fn", ASCII), 8, 10),
                eof(10),
            ],
        ),
        (
            "a # x\nb",
            ASCII,
            vec![
                plain(name("a"), 0, 1),
                unrepaired(2, 5, Kind::HashComment),
                plain(name("b"), 6, 7),
                eof(7),
            ],
        ),
    ];

    let mismatches = mismatched_streams(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}
