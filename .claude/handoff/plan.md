# LEM-12 plan: lexer, parser, AST and canonical printer for the prototype subset

Ticket report: `.claude/state/LEM-12-ticket.md` (authoritative). Branch `claude/focused-ramanujan-lgmayo`,
base `origin/main` fc3f9cc. The user lifted the stop between steps: the plan runs to the pull request.
Revised after step 1, ADR 0010 (a02726e), step 2 (2ec01a4 tests, eb386a2 parser) and ADR 0011
(comments, proposed, uncommitted in the tree with its `docs/grammar.md` and `docs/diagnostics.md` changes).

## Objective

`lemma-syntax` turns a source file into an AST plus the syntax-class diagnostic records of
`docs/grammar.md` section 7, for the whole section 3 EBNF, under either C1 variant (switch, default A);
and prints any AST back in the canonical form of section 5, comments kept by the formatter. The
ticket's acceptance criteria (both dot forms under both variants, P6, idempotence, round trip, AST and
diagnostic snapshots, a fuzz target that never panics) hold at the end.

## Current state

- **Step 1 is done**: tests ac1df43 (`crates/lemma-syntax/tests/lexer.rs`), lexer f08316e
  (`crates/lemma-syntax/src/token.rs`, `src/lexer.rs`, `src/lexer/number.rs`). Public surface, re-exported at
  `crates/lemma-syntax/src/lib.rs:6-9`: `lex(text: &[u8], variant: C1Variant) -> Lexed` (`src/lexer.rs:56`);
  `C1Variant { Ascii (default), Unicode }` (`src/token.rs:7`); `TextRange { start, end }` (`:17`);
  `ScalarClass { Unsigned, Signed, Float }` (`:26`), `ScalarType { class, bits: u16 }` (`:37`),
  `VectorType { element, lanes: u8 }` (`:46`); `Keyword` (`:55`, the words of `grammar.md:94-99` minus
  `sum`/`in`/`forall`, `Bool` included, `Keyword::from_word`); `TokenKind` (`:142`: `Name`, `Keyword`,
  `ScalarType`, `VectorType`, `Int`/`Float`/`Align(String)`, punctuation and operators, `Sum`/`In`/`Forall(C1Variant)`
  carrying the spelling's variant, `MembershipLookalike`, `AlmostEqual`, `Error`, `Eof`); `Fault { kind,
  message, repair: Vec<(TextRange, String)> }` (`:229`); `Token { kind, range, faults }` (`:240`; faults of
  the trivia before the token, then of the token, in source order; an unrepaired fault sits on an `Error`
  token); `Comment { range, text }` (`:251`); `Lexed { tokens, comments }` (`:260`).
- Step 1 behaviors later steps rely on: a `.` before a digit outside a number repairs with `0`, or ` 0`
  after a word character; a zero-width code point inside a word is skipped (the word continues), but
  inside a number it now continues the number too (eb386a2); comment text keeps a trailing CR on CRLF
  input, any code point is accepted in a comment, and invalid UTF-8 inside a comment gives an unrepaired
  `invalid-encoding` fault on the next token: all three change in step 3 (ADR 0011).
- **Step 2 is done**: `parse`, `Parsed`, `dump`, the AST (`crates/lemma-syntax/src/ast.rs`), parser
  modules; `Kind::NestingLimit` exists (`crates/lemma-diagnostics/src/kind.rs:139`).
- Wiring already in place: `crates/lemma-syntax/Cargo.toml` has `lemma-diagnostics` (path), dev-deps
  `insta = "1"` and `proptest`; `.gitignore:17-18` ignores `*.snap.new` and `*.pending-snap`.
- **Red now**: `crates/lemma-diagnostics/tests/kinds.rs`
  `the_code_has_the_documented_kinds_with_their_classes`: `grammar.md:609-610` list `comment-character`
  and `misplaced-comment`, `Kind` does not. Recheck: `cargo test -p lemma-diagnostics --test kinds`.
- `lemma-diagnostics` to reuse: `Diagnostic` (`src/record.rs:64`), `Source` and the fallible O(n)
  `Source::span` (`src/span.rs:83`), `Kind::ALL` (`src/kind.rs:184`), `Diagnostic::validate()`
  (`src/rules.rs:142`), `render_records`, `render_text`.
- `examples/dot.lm` is byte-identical to variant A: `diff <(sed -n 20,39p docs/grammar.md) examples/dot.lm`
  is empty. Variant U lines: `docs/grammar.md:45-46`.
- Normative sources (line numbers of the current tree): lexical `docs/grammar.md:52-240`, **Comments
  `:71-122`** (place `:79`, characters `:108`, one record per comment `:116`), lookalikes `:210`, EBNF
  from `:242`, precedence `:309`, structure rules `:332`, Nesting depth `:359`, canonical form `:521`
  (**Comments** `:551`, **Comment text** `:556`), recovery `:592`, `comment-character` and
  `misplaced-comment` rows `:609-610`, `invalid-number` row `:614`, `nesting-limit` row `:654`, switch
  `:676`; `docs/adr/0010-nesting-limit.md`; **`docs/adr/0011-comments.md`** (decisions 1-7,
  Consequences "For the lexer", "For the parser", "For the printer"); `docs/diagnostics.md` section 3
  "Between records" (`:113`, inserts at one position apply in record order) and message shapes
  `:324-343` (`comment-character` `:331-338`); ADR 0008 decisions 13, 20; ADR 0007 hash input.
- **Environment**: LLVM 22 is not installable in this container, so `just check` (workspace clippy and
  tests) cannot build `lemma-codegen` here. Local gate: `cargo clippy -p lemma-syntax -p lemma-diagnostics
  --all-targets -- -D warnings`, `cargo nextest run -p lemma-syntax -p lemma-diagnostics`,
  `cargo test --doc -p lemma-syntax -p lemma-diagnostics`, `cargo fmt --all --check`, `just spell`,
  `just rules`; CI covers the rest. No nightly toolchain, no `cargo-fuzz`.

## Gray areas: positions

1. **Scope: full section 3, not dot only** (`grammar.md:3-4`; ADR 0008 Consequences). Bounded types,
   element bounds, `forall`, `wo`, internal `fn`, `let` are parsed; `not-in-subset` and the type-or-layout
   kinds stay the checker's.
2. **"One diagnostic per function" vs recovery.** ADR 0006 names it "the simplest compliant strategy";
   ADR 0008 decision 20 and `grammar.md:592` choose: repaired diagnostics continue, the first unrepaired
   one ends the function, resume at the next `fn`/`export`. Implemented; the PR description says so.
3. **Lookalikes without repair** are reported unrepaired and end the function.
4. **`type_layout` in #13**: not this ticket; the class is `type-or-layout`.
5. **C1 default A, both variants**; no freeze ADR needed.
6. **`examples/dot.lm`: no rewrite** (identical, above); no contract change, no proof-engineer pass.
7. **Every section 7 syntax kind is emitted**, `nesting-limit`, `comment-character` and
   `misplaced-comment` included; a coverage test ties
   `Kind::ALL`'s syntax-class kinds to the corpus.
8. Overlap with lemma-diagnostics: none (snapshots go through the existing renderers).
9. Comment attachment: settled by ADR 0011 (one place: own line before an item, clause or statement;
   the printer never moves a comment). The step 3 language-designer pass is done (8f10026, then ADR 0011).
10. **Readings this plan fixes** (no doc change): `unclosed-delimiter`'s "exactly one delimiter is open"
    counts open `(` and `[` only; input ending after a complete clause with clauses missing is
    `missing-clause`, inside a construct `unexpected-end`; clause keywords after the `impl` block still
    belong to the function (`clause-order`); `equiv-relation` applies when `allow` followed by `(` occurs
    after `equiv` before any `{`, `fn`, `export`, second `equiv` or end of input, else
    `equiv-without-allow`; a misplaced comment with no token before it (file start, next token not an
    item, clause or statement start) gets no repair, like the no-item case of `grammar.md:610`;
    "the next token begins a clause or statement" is judged on the token as repaired (`requires` counts
    as `req`).

## Settled decisions

1. Nesting: kind `nesting-limit`, no repair, limit 16, counted per `grammar.md:359` and ADR 0010.
2. Fuzzing: a `just fuzz` recipe only; the CI job is issue #72.
3. Step 6 stays in this ticket.
4. Long binary chains: no limit here (follow-up issue); the parser parses chains by iteration.
5. Comments (ADR 0011): own line before an item, clause or statement, printable ASCII and tab;
   `comment-character` (lexer, step 3) and `misplaced-comment` (parser, step 5); one record per comment.

## Chosen approach

- **Lexer attaches faults, parser reports them** (done in step 1 on the lexer side): one recovery policy
  for lexical and syntactic faults. Rejected: lexer-side reporting (cannot end the function).
- **AST nodes carry `TextRange`; "same AST" is the range-free `dump`** (handwritable expected values,
  AST snapshots, equality of the three properties). Rejected: range-ignoring `PartialEq`, Debug snapshots,
  a range-erasing visitor.
- **`insta`** snapshots: AST dumps handwritten red-first; diagnostic R/T snapshots blessed by the tester
  after the coder (second tests commit for that step). **`proptest`** properties and robustness suite;
  **cargo-fuzz** crate in `crates/lemma-syntax/fuzz/` with its own `[workspace]`.
- **Depth by an explicit counter** on the tokens as written (ADR 0010 Consequences), not by the parser's
  recursion depth, so the count is the documented one whatever the implementation.

## Public API of lemma-syntax (still to create)

```rust
pub fn parse(source: Source<'_>, variant: C1Variant) -> Parsed;
pub struct Parsed { pub file: File, pub comments: Vec<Comment>, pub diagnostics: Vec<Diagnostic> }
pub fn print(file: &File, variant: C1Variant) -> String;   // canonical form, no comments
pub fn format(source: Source<'_>, variant: C1Variant) -> Result<String, Vec<Diagnostic>>; // Err if any diagnostic
pub fn dump(file: &File) -> String;
```

`Source` is `lemma_diagnostics::Source`. `Parsed.file.items` holds the functions parsed to their end
(repairs applied as if in the text); a function ended by an unrepaired diagnostic is absent. Diagnostics
in source order. New modules: `ast.rs`, `dump.rs`, `parser.rs` with `parser/expression.rs` and
`parser/statement.rs`, `printer.rs`.

**AST** (`ast.rs`, every node with `range: TextRange`, derive `Debug, Clone, PartialEq, Eq`; reuses
`ScalarType`, `VectorType`, `ScalarClass` from `token.rs`, no duplicate type):
`File { items }`; `Function { export: Option<TextRange>, name: Name, params: Vec<Param>, result: SignatureType,
effects: EffClause, requires: Vec<Clause>, ensures: Vec<Clause>, spec: Clause, equiv: EquivClause, body: Block }`
(clauses stored by role); `Name { text }`; `ValueType = Scalar(ScalarType) | Vector(VectorType) | Bool`;
`SignatureType { value: ValueType, bound: Option<Bound> }` (a bound is parsed after any value type);
`Bound { operator: Less|LessEqual, limit: Int(String)|Name(Name) }`; `Param { name, kind: Value(SignatureType)
| Reference(ReferenceType) }`; `ReferenceType { element: ScalarType, element_bound: Option<Bound>, extent:
Int|Name, alignment: String, mode: ReadOnly|WriteOnly }`; `EffClause { effects: Vec<Effect> }`;
`Clause { expression }`; `EquivClause { permissions }` sorted in table order at parse; `Block { statements }`;
`Statement = Let { binder, value } | Loop { index: Binder, low, high, step: String, carries, body } |
Next { name, value } | Return { value }`; `Binder { name, value_type }`; `Carry { binder, value }`;
`Expression { kind, range }` with `Int(String) | Float(String) | Bool(bool) | Result | Name(String) |
Index { base: Name, index } | Call { callee: Name, arguments } | Member { vector: VectorType, member: Name,
arguments } | Sum { binder, low, high, body } | Forall { .. } | Paren(Box) | Unary { operator: Negate|Not,
operand } | Binary { operator, left, right }`. No C1 variant and no comment in the AST.

**Dump format** (one line per construct, two spaces per level, expressions as s-expressions, LF after
every line): `[export ]fn <name>`; at 2: `param <name> <type>`, `result <type>`, `eff pure`, `req <e>`,
`ens <e>`, `spec <e>`, `equiv[ <perm>…]` (table order), `impl`; statements at 4, nested two deeper:
`let <name> <type> <e>`, `loop <name> <type> <lo> <hi> step <int>` followed at +2 by `carry <name> <type> <e>`
lines then body lines, `next <name> <e>`, `return <e>`. Types: `u64`, `f32x4`, `bool`, `(bounded u64 < n)`,
`(ref f32 n @a128 ro)`, `(ref (bounded u64 < n) m @a8 ro)`. Expressions: literals as written, `true`,
`false`, `result`, names bare, `(index a i)`, `(call fma a b c)`, `(member f32x4 load a i)`,
`(sum i u64 <lo> <hi> <body>)`, `(forall …)`, `(paren e)`, `(neg e)`, `(not e)`, `(<op> l r)`. Example:
`spec (sum i u64 0 n (* (index a i) (index b i)))`,
`return (call reduce_sum (+ (paren (+ c0 c1)) (paren (+ c2 c3))))`.

## Breakdown

Seven steps (one done): the properties, the fuzz target and the kind coverage only hold on the complete
parser, and #13 is blocked on all of it. All tests are integration tests in `crates/lemma-syntax/tests/`
(no `mod tests;` wiring), shared helpers in `tests/common/mod.rs`; the coder runs suites with
`INSTA_UPDATE=no`. Done criteria use the local gate of "Current state"; `just check` runs in CI.

1. **Tokens, lexer, C1 switch.** Done (ac1df43, f08316e).
2. **Done** (2ec01a4, eb386a2). **AST, dump, parser for well-formed input** (~1200 lines: `ast.rs`, `dump.rs`, `parser*.rs`; plus
   ~5 in lemma-diagnostics). First, the coder adds `Kind::NestingLimit` to
   `crates/lemma-diagnostics/src/kind.rs` (variant, `ALL`, name `nesting-limit`, class `Syntax`): its red
   test already exists (the kinds table test above), so no new test; it is done here rather than in step 4
   so that suite is green from this step on. Tests: `tests/parse.rs` (hand-written dumps: precedence table,
   left associativity, parens kept, each primary form, the three signatures of `grammar.md:413-415`,
   `forall`, `wo`, internal `fn`, several items, `allow` in any order dumps in table order),
   `tests/variants.rs` (dot A and dot U under each variant: matching pairs give no diagnostic, crossed
   pairs exactly the `c1-other-variant` records with the active spelling as repair, all four the same
   dump), handwritten `tests/snapshots/variants__dot_dump.snap`; one lexer row in `tests/lexer.rs` for
   `1<ZWSP>2` (below). Malformed input here only ends the function with a plain `unexpected-token` or
   `unexpected-end` (refined in step 4). Done: suites green, kinds test green.
3. **Printer, formatter, and the comment lexer rules** (~350 lines `printer.rs`, ~80 in `lexer.rs`, ~10
   in lemma-diagnostics). The language-designer pass on section 5 is done (`grammar.md:521-560` as
   amended by ADR 0011). Coder, in this order:
   (a) `Kind::CommentCharacter` and `Kind::MisplacedComment` in `crates/lemma-diagnostics/src/kind.rs`
   (variant, `ALL`, names, class `Syntax`): the kinds table test is their red test, and adding both now
   turns it green; `MisplacedComment` is only emitted in step 5.
   (b) Lexer, ADR 0011 Consequences "For the lexer": a comment ends before the CR of a CRLF line end, so
   `Comment.range` and `Comment.text` exclude it (the printer then strips only trailing spaces and tabs);
   comment text is checked against U+0020-U+007E and tab; each comment with rejected code points or bytes
   yields one `comment-character` fault (span: its first rejected code point, or first invalid byte alone;
   repair: one deletion per maximal run; message exactly `diagnostics.md:331-338`, counting each further code
   point or byte, never quoting it), attached as trivia to the next token; invalid UTF-8 inside a comment
   becomes that fault, and the unrepaired `invalid-encoding` fault on the next token goes; a CR not
   followed by LF inside a comment is rejected, the comment continues to the LF. Lookalikes inside a
   comment are `comment-character`, never `lookalike`. A comment whose marker is rejected (`;`, `#`,
   `/* */`) and that also holds rejected characters yields one fault of the marker kind whose message says
   the characters are rejected too and whose repair, when the marker has one, also deletes the runs
   (grammar.md `:116`); without a marker repair, no repair.
   (c) Printer, section 5 as amended: a comment is printed on its own line just before the item, clause or
   statement whose first token follows it, at that construct's indentation, as written minus trailing
   spaces and tabs; comments attached to one construct keep their order; the printer never moves a
   comment. A comment's construct is found by its position: the first item, clause or statement start
   token after it. Until step 5 diagnoses misplaced comments, formatting a text with one is unspecified.
   Tests: `tests/printer.rs` (each section 5 bullet; `format(examples/dot.lm)` is the file byte for byte
   under A; dot U under U likewise; scrambled dot formats to `examples/dot.lm`; comments before an item,
   a clause and a statement at each depth, several before one construct; CRLF input: no CR in the
   output); `tests/lexer.rs` rows for the comment rules (CRLF range, each class of rejected code point:
   non-ASCII, tag character, bidi control, ZWSP, DEL, lone CR, invalid byte; run grouping; message count;
   marker merge); `tests/properties.rs` with an AST generator in `tests/common/mod.rs` (parser-producible
   ASTs only: names not reserved, `Paren` where precedence requires, structure rules respected, depth at
   most 16; comments of printable ASCII and only at the allowed places): idempotence of `format`, round
   trip `dump(parse(print(ast))) == dump(ast)`, P6 (whitespace varied between tokens, comments only on
   their own line before an item, clause or statement), under both variants. The three committed rows the
   tester adjusts (`tests/parse.rs` comment-only file and trailing comment, `tests/lexer.rs` non-ASCII
   comment) assert only what this step delivers: comment ranges and `comment-character`; any expectation
   of `misplaced-comment` belongs to step 5's corpus, or step 3 cannot go green. Done: green, 256 cases
   each, kinds test green.
4. **Recovery, token-level kinds and the nesting limit** (~500 lines). Kinds: `unexpected-token` (message
   per `diagnostics.md:326-330`; classes `a name`, `a type`, `an alignment`, `an expression`, `an integer
   literal`), `unexpected-end`, `unclosed-delimiter`, `unmatched-delimiter`, `trailing-comma` (lists),
   `misplaced-export`, `reserved-word`, `foreign-keyword`, `assignment`, `assignment-in-expression`,
   `unsupported-operator` for binary `&`, positional `c1-other-variant`, `ϵ`/`ε`, `≈` outside `equiv`, and
   **`nesting-limit`** per `grammar.md:359`: a counter, +1 at every `(`, `[`, `{` (parameter list, call
   arguments, `carry(`, `allow(`, sum/forall binder list, parenthesized expression or element type, index,
   extent, every block including `impl` and loop bodies, sum/forall bodies), -1 at its closer; +1 at each
   unary `-` and `!`, released after its operand; nothing for binary operators, clauses, statements,
   `loop`, `let`, or the parser's precedence levels; a delimiter inserted by a repair counts as written, a
   deleted one does not. The first token opening depth 17 is the record, its span that token alone, checked
   before descending; the function ends there. Message, exactly per `diagnostics.md:339-343`: in the impl
   ``nesting exceeds the limit of 16 levels; bind an inner expression to a name with `let` ``, in the
   signature or a contract clause `nesting exceeds the limit of 16 levels`; 16 is one named constant.
   Tests: `tests/diagnostics.rs` table (input, kinds, ranges, edits) over `tests/testdata/diagnostics/*.lm`
   (step 1's lexical kinds and `comment-character` join here), resync cases, R and T snapshots blessed after the coder; nesting
   rows: dot reaches 4 and parses; depth 16 accepted and 17 rejected for each opener kind (paren, bracket,
   brace, call, unary `-`, `!`, `-(x)` counting two, `!(!c)` three), in impl and in a contract clause
   (both messages), siblings not adding up (`(c0 + c1) + (c2 + c3)`), a repair-inserted `)` counted;
   `tests/robustness.rs` (proptest over arbitrary bytes and a token soup of every spelling, lookalike,
   foreign keyword and delimiter, plus runs of 10^4 `(` or `-`): no panic or stack overflow, every record
   `validate()`s and is syntax class, spans in bounds on code point boundaries, between two unrepaired
   records lies a `fn` or `export` token, a clean parse round-trips. **Competing passes: yes.** A: the
   simplest version (recursion with `Result<_, Abort>`, delimiter stack, depth counter beside it). B: the
   most robust to malformed input (depth and delimiter accounting in one structure that cannot drift,
   linear time on adversarial input, iterative resync that never loses the resume token). Why: steps 5
   and 6 and every later diagnostic reuse this recovery pattern.
5. **Clause, signature and comment structure** (~600 lines). **`misplaced-comment`** (ADR 0011
   decisions 1, 2, 5; `grammar.md:79-122`): for each comment the parser reaches (comments in a region
   skipped after an unrepaired diagnostic are not judged), it is well placed when only spaces and tabs
   precede it on its line and the next token, other comments skipped and judged as repaired, starts an
   item (`export`, `fn`), a clause (`eff`, `req`, `ens`, `spec`, `equiv`, `impl`) or a statement (`let`,
   `loop`, `next`, `return`). Otherwise one record, repaired by two edits: an insertion at the start of
   the first token of the innermost item, clause or statement containing the last token before the
   comment (the parser keeps a stack of open item/clause/statement start tokens; a carry is not a
   statement, so its loop is the target; `impl` before `{` and after the last `}` target the `impl`
   clause), of the comment's text with `//` as marker and rejected runs removed, then LF, then the
   token's leading spaces and tabs when it is first on its line, or preceded by LF when it is not; and a
   deletion of the comment with the spaces and tabs before it, or of its whole line with its line end when
   nothing else is on it. No repair in a file with no item, nor with no token before the comment. One
   record per comment: a comment that already carries a marker or `comment-character` fault keeps that
   kind (order: marker kinds, `comment-character`, `misplaced-comment`), its message adds the
   misplacement, and its repair becomes the move (whose insertion already applies the other fixes) when
   every broken rule has a repair, else none. The record is reported where the comment's fault would be
   (as trivia of the next token), and `format` returns it. Then the clause and signature kinds:
   `clause-order` (one record per function at
   the end of its clauses; one edit replacing the region from the first misplaced clause to the last clause
   with the clause texts in grammar order, `req`/`ens` keeping their relative order, joined by LF and two
   spaces), `missing-clause` (one record, empty span where the first missing clause belongs, message names
   all), `duplicate-clause`, `equiv-without-allow` (insert ` allow()` after `equiv`), `equiv-relation`,
   `duplicate-permission`, `duplicate-effect` (delete from the end of the previous item through the
   duplicate), `trailing-comma` in `eff`, `bound-operator`, `bound-expression`, `bound-on-binder` (delete
   the bound on an index binder), `unparenthesized-element-bound` (two edits; its inserted `(` counts for
   depth), `missing-alignment`, `missing-mode`, `missing-type`, `chained-comparison`, `ambiguous-not`
   (either side, unary minus included). Tests: corpus rows and snapshots as in step 4 (misplaced comments:
   trailing a clause, inside a parameter list, inside an expression, after a carry, between `impl` and
   `{`, before a closing `}`, after the last item, several moved before one token, a misplaced `#`
   comment with a non-ASCII code point giving one `hash-comment` record, a file of comments only giving no
   repair), and a repair-consistency test: for each corpus file whose records all carry repairs, applying
   them (several records inserting at one position apply in record order, `diagnostics.md:113`) gives a
   text that parses clean to the dump of the recovered AST and keeps the comments in source order. Done:
   green.
6. **Statement structure and kind coverage** (~250 lines). Kinds: `missing-step`, `invalid-step`,
   `missing-carry`, `carry-without-next` (at the loop's `}`), `duplicate-next`, `misplaced-next`,
   `missing-return` (at the impl's `}`), `misplaced-return` (span the `return`, related the statement
   after it), `result-outside-ens`; all in source order during the parse. Tests: corpus rows, snapshots,
   `every_syntax_kind_is_produced_by_the_corpus` over `Kind::ALL` filtered on `FailureClass::Syntax`
   (which now includes `comment-character` and `misplaced-comment`).
   Done: green.
7. **Fuzz target.** Tester: `crates/lemma-syntax/fuzz/` (`Cargo.toml` with its own `[workspace]` and
   `libfuzzer-sys`, `fuzz_targets/parse.rs` running step 4's robustness oracle under both variants,
   `.gitignore` for `target`, `corpus`, `artifacts`), seeds from `tests/testdata/diagnostics/` and
   `examples/dot.lm`. **release-engineer**: a `fuzz` recipe in `justfile`
   (`cargo +nightly fuzz run parse -- -max_total_time=60` from that directory), not in `check`, no CI job
   (issue #72); `_typos.toml` if `just spell` flags test spellings (`ret`, `fun`, `val`). Done:
   `just --list` shows the recipe, `just spell` green; the 60 s run is the user's (nightly and cargo-fuzz
   are not installed here).

## Coding directives

- No `unwrap`/`expect`; byte range to `Span` goes through one helper; a `SpanError` there yields an
  `internal-error` record on the empty span at file start built from public fields, never a panic.
- Recovery: a token's faults are reported when the parser consumes it or stops at it with an unrepaired
  diagnostic; at an `Error` token the parser reports its fault, not `unexpected-token`. Resync scans
  tokens (no recursion) to the next `fn` or `export`, dropping the faults of skipped tokens and of the
  resume token's trivia. A record with a repair continues as if every edit were applied; the AST holds
  the repaired spelling.
- **Binary chains by iteration**: each precedence level is a loop over `( op operand )*`, never a
  recursive call per chain element; recursion happens only where a level opens (ADR 0010), so parser
  stack is bounded by a constant times 16. `dump`, `print` and the AST generator may recurse on the tree
  (chain length is the follow-up issue's); the robustness generators keep inputs at most 4096 bytes.
- `c1-other-variant` is reported only where the parser expects that C1 token (SUM or FORALL as primary,
  IN after a binder); a C1 word or symbol where a name is expected is `reserved-word`.
- ASCII-word replacements of a symbol are padded with a space on each side touching a letter, digit or
  `_` (step 1 does it for `0`; the parser's own repairs follow the same rule).
- `foreign-keyword` contexts, each on a name token: at item start or after `export`, `function` `func`
  `fun` `def` (repair `fn`), `pub` `extern` (none); where a clause is expected, `requires` `ensures`
  `effects` (repairs `req` `ens` `eff`); at statement start, `ret` (repair `return`), `var` `val` `const`
  followed by a name and `:` (repair `let`), `for` `while` `if` `else` `match` `break` `continue` `mut`
  (none); after `let`, `mut` followed by a name (none). Any other statement starting with a name is
  `assignment`.
- Printer never adds or removes parentheses; prints literal and alignment spellings as stored; prints C1
  tokens in the requested variant (ADR 0007 calls it with `Ascii`); strips trailing spaces and tabs from
  comment text (the CR is no longer in it); never moves a comment; signature on one line.
- No message about a comment contains a code point the comment rules reject (ADR 0011 decision 3).
- Messages: one sentence, one line, found spelling in backticks; fixed shapes of `diagnostics.md:324-343`
  exactly; no message text in table tests except those fixed shapes.
- In `crates/lemma-diagnostics`, only the new `Kind` variants (`NestingLimit` done; `CommentCharacter`,
  `MisplacedComment` in step 3). Test names and files never contain
  the ticket key. No `for`/`if` in `#[test]` bodies (table + one body).

## Risks

- **Snapshot flow**: steps 4-6 get a second tests commit (the tester's blessing after the coder); the
  frozen reference for that step moves to it.
- **Quadratic spans**: `Source::span` scans from the file start per record; acceptable for the prototype.
- **Long chains** still overflow `dump`/`print` and later passes on huge inputs (ADR 0010 open point;
  follow-up issue): the fuzz target keeps the default `-max_len` 4096.
- Edge cases for review: CRLF, tabs, empty file, comments only, `0..<n` vs `0.5`, `->` vs `- >`,
  `!a == b`, ZWSP inside words and numbers, a comment at end of file without LF, a CRLF comment, a lone
  CR in a comment, a comment between two comments before one construct, `specΣ(` under A, a repair at end of input, `export` at end
  of input, depth exactly 16 / 17 in each clause kind, a repair-inserted delimiter at depth 16.
- **ADR 0011 is uncommitted**: its docs must land with this work (the committer's job at the end), or
  the repository describes rules the code does not follow, and the reverse.
- Repairs of different records may insert at one position: any tool or test applying several records
  must follow the record-order tie rule; within one comment record, the move's deletion covers the
  character deletions, so no record carries overlapping edits.
- The T rendering's snippet still shows a rejected code point of a comment (ADR 0011 open point, the
  renderer's, not this ticket's).
- Local gate is per crate (LLVM 22 missing here); a workspace-only failure shows first in CI.
- The fuzz crate is outside the workspace: lint and `cargo deny` do not see it.
- **Exit gate**: `reviewer`; `auditor` (untrusted text reaches the parser; recovery and depth
  accounting; compares step 4's competing passes); `devsecops` (`insta` dev-dependency tree,
  `libfuzzer-sys`, the `justfile` change). No `proof-engineer`, no `perf-engineer`.
