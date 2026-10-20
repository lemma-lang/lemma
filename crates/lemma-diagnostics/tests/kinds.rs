//! The closed set of kinds, checked against the tables of the documents that define it.

use lemma_diagnostics::{FailureClass, Kind};

const GRAMMAR: &str = include_str!("../../../docs/grammar.md");
const EXPORT_MAPPING: &str =
    include_str!("../../../docs/adr/0007-export-mapping-and-content-hash.md");
const DIAGNOSTICS: &str = include_str!("../../../docs/diagnostics.md");

const CLASS_NAMES: [(FailureClass, &str); 7] = [
    (FailureClass::Syntax, "syntax"),
    (FailureClass::TypeOrLayout, "type-or-layout"),
    (FailureClass::Effect, "effect"),
    (FailureClass::BoundProof, "bound-proof"),
    (FailureClass::EquivalenceProof, "equivalence-proof"),
    (FailureClass::SolverTimeout, "solver-timeout"),
    (FailureClass::ToolchainBug, "toolchain-bug"),
];

type Table<'a> = Vec<Vec<&'a str>>;

fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

/// Every Markdown table of a document, as rows of cells, header and separator rows included.
fn tables(document: &str) -> Vec<Table<'_>> {
    let lines: Vec<&str> = document.lines().collect();
    lines
        .chunk_by(|first, second| first.starts_with('|') == second.starts_with('|'))
        .filter(|chunk| chunk[0].starts_with('|'))
        .map(|chunk| chunk.iter().map(|line| cells(line)).collect())
        .collect()
}

/// The tables whose first header cell is `Kind`, as `docs/diagnostics.md` section 7 selects them.
fn kind_tables(document: &str) -> Vec<Table<'_>> {
    tables(document)
        .into_iter()
        .filter(|table| table[0][0] == "Kind")
        .collect()
}

fn unquoted(cell: &str) -> String {
    cell.trim_matches('`').to_string()
}

fn with_class(table: &Table<'_>, class: &str) -> Vec<(String, String)> {
    table
        .iter()
        .skip(2)
        .map(|row| (unquoted(row[0]), class.to_string()))
        .collect()
}

fn with_class_column(table: &Table<'_>) -> Vec<(String, String)> {
    table
        .iter()
        .skip(2)
        .map(|row| (unquoted(row[0]), unquoted(row[1])))
        .collect()
}

#[test]
fn the_documents_hold_four_kind_tables_two_in_the_grammar() {
    let counts = [
        kind_tables(GRAMMAR).len(),
        kind_tables(EXPORT_MAPPING).len(),
        kind_tables(DIAGNOSTICS).len(),
    ];

    assert_eq!(counts, [2, 1, 1]);
}

#[test]
fn the_code_has_the_documented_kinds_with_their_classes() {
    let grammar = kind_tables(GRAMMAR);
    let mut documented = [
        with_class(&grammar[0], "syntax"),
        with_class(&grammar[1], "type-or-layout"),
        with_class(&kind_tables(EXPORT_MAPPING)[0], "type-or-layout"),
        with_class_column(&kind_tables(DIAGNOSTICS)[0]),
    ]
    .concat();
    documented.sort();

    let mut implemented: Vec<(String, String)> = Kind::ALL
        .iter()
        .map(|kind| {
            (
                kind.name().to_string(),
                kind.failure_class().name().to_string(),
            )
        })
        .collect();
    implemented.sort();

    assert_eq!(implemented, documented);
}

#[test]
fn every_kind_name_maps_back_to_its_kind() {
    let not_round_tripping: Vec<&str> = Kind::ALL
        .iter()
        .filter(|kind| Kind::from_name(kind.name()).as_ref() != Some(*kind))
        .map(|kind| kind.name())
        .collect();

    assert_eq!(not_round_tripping, Vec::<&str>::new());
}

#[test]
fn kind_names_are_unique() {
    let mut names: Vec<&str> = Kind::ALL.iter().map(|kind| kind.name()).collect();
    names.sort_unstable();
    let mut unique = names.clone();
    unique.dedup();

    assert_eq!(unique, names);
}

#[test]
fn an_unknown_name_maps_to_no_kind() {
    let kind = Kind::from_name("no-such-kind");

    assert!(kind.is_none());
}

#[test]
fn failure_classes_carry_the_names_of_section_7() {
    let expected: Vec<&str> = CLASS_NAMES.iter().map(|(_, name)| *name).collect();

    let names: Vec<&str> = CLASS_NAMES.iter().map(|(class, _)| class.name()).collect();

    assert_eq!(names, expected);
}
