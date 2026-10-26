# LEM-12 ticket report (ticket-reader, 2026-10-25)

Issue #12, "feat(syntax): lexer, parser, AST and canonical printer for the dot subset". Labels syntax, size L, milestone Prototype. No comments. Dependencies #8 (diagnostic records) and #9 (grammar) both closed. Blocks #13.

## Request
- Regular lexer, no parser feedback. Lookalikes rejected with a repair.
- C1 variant behind a switch: one variant active, the other's spellings recognized only to be rejected, with their own kind and a repair.
- No cascades: at most one syntax diagnostic per function, resuming at the next function.
- The canonical printer is the formatter.
- examples/dot.lm rewritten in the new grammar; the proof-engineer rewrites the contract lines (spelling, not meaning).

## Acceptance criteria
- Both dot forms parse under both C1 variants.
- Property test: the P6 whitespace property holds.
- Property test: the printer is idempotent.
- Property test: parsing the printer's output gives the same AST.
- AST and diagnostic snapshots exist.
- A fuzz target never panics.

## Design references
- docs/grammar.md: §1 two dot forms (variant A default, variant U diff), §2 lexical rules, keywords, C1 symbols, lookalike table with repairs, §3 EBNF, precedence, 11 structure rules, §4 meaning the parser relies on, §5 canonical form (comments kept by formatter, parens kept in AST), §7 closed list of syntax kinds with repairs and recovery rule.
- docs/adr/0008-prototype-grammar.md: choices 13 (parens kept), 15 (no literal suffixes), 16-18 (lexical, reserved words, lookalikes), 20 (recovery through repairs).
- docs/adr/0006-optimized-for-its-writer.md: P1-P9, C1 switch (default A), "For the parser" consequences ~lines 399-406.
- docs/adr/0009-diagnostic-records.md, docs/diagnostics.md: record format, 1-based positions, columns in code points, repairs as edit lists all-or-none.
- crates/lemma-diagnostics landed (kind.rs, record.rs, span.rs, text.rs, records.rs, parse.rs, rules.rs, tests/).
- crates/lemma-syntax is a stub (lib.rs doc line only).
- examples/dot.lm already appears to use the variant A spelling.

## Constraints
1. Parser emits the syntax-class kinds of grammar.md §7 as diagnostics.md records; checker kinds out of scope.
2. Recovery: a diagnostic with a repair is reported and parsing continues as if repaired; the first one without a repair ends the function's parse; resume at the next fn or export.
3. Repairs never applied to the source, never change meaning on the contract.
4. Lexing is regular (P7). sum, in, forall reserved in both variants. requires/ensures/effects are not keywords; the parser rejects them with foreign-keyword in clause position.
5. Parse wo parameters and bounded types; not-in-subset is the checker's.
6. Parens kept in AST and printed as written.
7. Comments not in AST, but the formatter keeps them.
8. Printer layout fixed (§5).
9. P6: with comments removed, replacing each whitespace run with one space yields the same AST.

## Gray areas
1. "At most one diagnostic per function" (issue) vs docs: repaired diagnostics continue, only an unrepaired one ends the function.
2. Not every lookalike has a repair (table rows with "none").
3. #13 says kind `type_layout`; docs use `type-or-layout` class.
4. C1 default A provisional; no freeze ADR yet; both variants needed behind the switch.
5. examples/dot.lm already rewritten? Status unclear.
6. Scope: dot-only, or full §3 EBNF (SUM/FORALL, bounded types, element bounds, wo)?
7. Must every §7 syntax kind be emitted?
8. AST shape unspecified.
9. Tooling (snapshot, proptest, fuzz harness) unnamed; no fuzz/ dir; new deps trigger devsecops.
10. Overlap with lemma-diagnostics rendering snapshots.
11. Comment attachment for the formatter not specified beyond §5.
