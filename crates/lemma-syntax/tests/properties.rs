//! The properties of `docs/grammar.md` sections 2 and 5 over generated trees, under both C1
//! variants: the formatter is idempotent, printing then parsing gives the same tree back, and a
//! text without its comments, its whitespace runs collapsed to one space, parses to the same tree.

mod common;

use lemma_diagnostics::{Kind, Source};
use lemma_syntax::{C1Variant, dump, format, parse, print};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use common::{file, laid_out_file, on_one_line_without_comments};

type Outcome = (String, Vec<Kind>);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;

fn source(text: &str) -> Source<'_> {
    Source {
        name: "properties.lm",
        text: text.as_bytes(),
    }
}

fn outcome(text: &str, variant: C1Variant) -> Outcome {
    let parsed = parse(source(text), variant);
    let kinds = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind)
        .collect();
    (dump(&parsed.file), kinds)
}

fn formatted(text: &str, variant: C1Variant) -> Result<String, TestCaseError> {
    format(source(text), variant).map_err(|diagnostics| {
        TestCaseError::fail(std::format!(
            "the formatter refused {text:?}: {diagnostics:#?}"
        ))
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn formatting_twice_gives_the_text_of_formatting_once_under_variant_a(
        (_, text) in laid_out_file(ASCII)
    ) {
        let once = formatted(&text, ASCII)?;

        let twice = format(source(&once), ASCII);

        prop_assert_eq!(twice, Ok(once));
    }

    #[test]
    fn formatting_twice_gives_the_text_of_formatting_once_under_variant_u(
        (_, text) in laid_out_file(UNICODE)
    ) {
        let once = formatted(&text, UNICODE)?;

        let twice = format(source(&once), UNICODE);

        prop_assert_eq!(twice, Ok(once));
    }

    #[test]
    fn parsing_the_printed_tree_gives_it_back_under_variant_a(tree in file()) {
        let printed = print(&tree, ASCII);

        let reparsed = outcome(&printed, ASCII);

        prop_assert_eq!(reparsed, (dump(&tree), Vec::new()));
    }

    #[test]
    fn parsing_the_printed_tree_gives_it_back_under_variant_u(tree in file()) {
        let printed = print(&tree, UNICODE);

        let reparsed = outcome(&printed, UNICODE);

        prop_assert_eq!(reparsed, (dump(&tree), Vec::new()));
    }

    #[test]
    fn a_text_on_one_line_without_comments_parses_to_the_same_tree_under_variant_a(
        (tree, text) in laid_out_file(ASCII)
    ) {
        let collapsed = on_one_line_without_comments(&text);

        let reparsed = outcome(&collapsed, ASCII);

        prop_assert_eq!(reparsed, (dump(&tree), Vec::new()));
    }

    #[test]
    fn a_text_on_one_line_without_comments_parses_to_the_same_tree_under_variant_u(
        (tree, text) in laid_out_file(UNICODE)
    ) {
        let collapsed = on_one_line_without_comments(&text);

        let reparsed = outcome(&collapsed, UNICODE);

        prop_assert_eq!(reparsed, (dump(&tree), Vec::new()));
    }
}
