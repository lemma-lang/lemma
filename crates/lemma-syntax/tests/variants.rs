//! Both C1 variants of `docs/grammar.md` section 1: each form of `dot` parses clean under its own
//! variant, the other form reports its C1 spellings with the active ones as repair, and all four
//! pairings give the same AST.

use lemma_diagnostics::{Diagnostic, Kind, Source};
use lemma_syntax::{C1Variant, dump, parse};

type EditView = (usize, usize, String);
type DiagnosticView = (Kind, usize, usize, Vec<EditView>);
type Pairing = (&'static str, C1Variant);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;

const DOT_ASCII_SPELLINGS: &str = include_str!("../../../examples/dot.lm");
const DOT_UNICODE_SPELLINGS: &str = include_str!("testdata/dot_unicode.lm");

const QUANTIFIER_ASCII_SPELLINGS: &str = concat!(
    "fn all_below(n: u64) -> bool\n",
    "  eff pure\n",
    "  spec forall(i: u64 in 0..<n) { i < n }\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return true\n",
    "  }\n",
);

const QUANTIFIER_UNICODE_SPELLINGS: &str = concat!(
    "fn all_below(n: u64) -> bool\n",
    "  eff pure\n",
    "  spec \u{2200}(i: u64 \u{2208} 0..<n) { i < n }\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return true\n",
    "  }\n",
);

const QUANTIFIER_DUMP: &str = concat!(
    "fn all_below\n",
    "  param n u64\n",
    "  result bool\n",
    "  eff pure\n",
    "  spec (forall i u64 0 n (< i n))\n",
    "  equiv\n",
    "  impl\n",
    "    return true\n",
);

fn source(text: &str) -> Source<'_> {
    Source {
        name: "variants.lm",
        text: text.as_bytes(),
    }
}

fn dump_of(text: &str, variant: C1Variant) -> String {
    dump(&parse(source(text), variant).file)
}

fn diagnostic_view(diagnostic: &Diagnostic) -> DiagnosticView {
    let edits = diagnostic
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
        diagnostic.kind,
        diagnostic.span.start.byte,
        diagnostic.span.end.byte,
        edits,
    )
}

fn diagnostics_of(text: &str, variant: C1Variant) -> Vec<DiagnosticView> {
    parse(source(text), variant)
        .diagnostics
        .iter()
        .map(diagnostic_view)
        .collect()
}

fn other_variant(start: usize, end: usize, replacement: &str) -> DiagnosticView {
    (
        Kind::C1OtherVariant,
        start,
        end,
        vec![(start, end, replacement.to_string())],
    )
}

#[test]
fn each_form_parses_without_diagnostics_under_its_own_variant() {
    let pairings: [Pairing; 4] = [
        (DOT_ASCII_SPELLINGS, ASCII),
        (DOT_UNICODE_SPELLINGS, UNICODE),
        (QUANTIFIER_ASCII_SPELLINGS, ASCII),
        (QUANTIFIER_UNICODE_SPELLINGS, UNICODE),
    ];

    let diagnostics = pairings.map(|(text, variant)| diagnostics_of(text, variant));

    assert_eq!(
        diagnostics,
        [Vec::new(), Vec::new(), Vec::new(), Vec::new()]
    );
}

#[test]
fn ascii_dot_under_variant_u_reports_each_c1_word_with_its_symbol_as_repair() {
    let text = DOT_ASCII_SPELLINGS;

    let diagnostics = diagnostics_of(text, UNICODE);

    assert_eq!(
        diagnostics,
        vec![
            other_variant(182, 185, "\u{03A3}"),
            other_variant(193, 195, "\u{2208}"),
            other_variant(283, 285, "\u{2208}"),
        ]
    );
}

#[test]
fn unicode_dot_under_variant_a_reports_each_c1_symbol_with_its_word_as_repair() {
    let text = DOT_UNICODE_SPELLINGS;

    let diagnostics = diagnostics_of(text, ASCII);

    assert_eq!(
        diagnostics,
        vec![
            other_variant(182, 184, "sum"),
            other_variant(192, 195, "in"),
            other_variant(283, 286, "in"),
        ]
    );
}

#[test]
fn a_crossed_quantifier_spelling_reports_the_active_spelling_as_repair() {
    let pairings: [Pairing; 2] = [
        (QUANTIFIER_ASCII_SPELLINGS, UNICODE),
        (QUANTIFIER_UNICODE_SPELLINGS, ASCII),
    ];

    let diagnostics = pairings.map(|(text, variant)| diagnostics_of(text, variant));

    assert_eq!(
        diagnostics,
        [
            vec![
                other_variant(47, 53, "\u{2200}"),
                other_variant(61, 63, "\u{2208}"),
            ],
            vec![other_variant(47, 50, "forall"), other_variant(58, 61, "in"),],
        ]
    );
}

#[test]
fn every_pairing_of_dot_form_and_variant_dumps_the_same_ast() {
    let reference = dump_of(DOT_ASCII_SPELLINGS, ASCII);
    let others: [Pairing; 3] = [
        (DOT_ASCII_SPELLINGS, UNICODE),
        (DOT_UNICODE_SPELLINGS, ASCII),
        (DOT_UNICODE_SPELLINGS, UNICODE),
    ];

    let dumps = others.map(|(text, variant)| dump_of(text, variant));

    assert_eq!(dumps, [reference.clone(), reference.clone(), reference]);
}

#[test]
fn every_pairing_of_quantifier_form_and_variant_dumps_the_same_ast() {
    let pairings: [Pairing; 4] = [
        (QUANTIFIER_ASCII_SPELLINGS, ASCII),
        (QUANTIFIER_ASCII_SPELLINGS, UNICODE),
        (QUANTIFIER_UNICODE_SPELLINGS, ASCII),
        (QUANTIFIER_UNICODE_SPELLINGS, UNICODE),
    ];

    let dumps = pairings.map(|(text, variant)| dump_of(text, variant));

    assert_eq!(dumps, [QUANTIFIER_DUMP; 4]);
}

#[test]
fn dot_dumps_to_its_recorded_snapshot() {
    let dump = dump_of(DOT_ASCII_SPELLINGS, ASCII);

    insta::assert_snapshot!("dot_dump", dump);
}
