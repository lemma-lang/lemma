# 10. A limit on syntactic nesting

- Status: proposed
- Date: 2026-10-25

## Context

The parser of the prototype subset ([`docs/grammar.md`](../grammar.md), ADR 0008) descends
recursively: each parenthesis, bracket, brace and unary operator is parsed by a call inside the
call that parses what encloses it. Its stack depth therefore grows with the nesting of the input,
and nothing in the grammar bounds that nesting. A model writer that falls into runaway generation
(`((((…` repeated, a `-` repeated) produces input that overflows the stack or, if parsed, sends a
huge term to Z3. Either way the writer receives a crash or a timeout with no span, a toolchain-bug
or solver-timeout failure in ADR 0004's classes, where the cause is in its own text: the repair
loop of ADR 0006 has nothing to act on.

None of the closed list of syntax kinds of ADR 0008 fits this case, and reporting it as
`internal-error` would count, as a fault of the compiler, a failure the input explains.

Evidence, weak to moderate:

- **Parser caps are far higher.** Read in source: CPython's tokenizer caps parenthesis nesting at
  `MAXLEVEL` 200 (`Parser/lexer/state.h`), serde_json's recursion limit is 128 (`src/de.rs`), Go's
  parser stops at `maxNestLev` 100 000 (`go/parser/parser.go`). From search summaries only, not
  verified: Clang's `-fbracket-depth` defaults to 256, and C11 (5.2.4.1) requires implementations
  to accept at least 63 nested parenthesized expressions and 127 nested blocks. These caps protect
  a parser's stack; none measures what a writer needs.
- **Readability rules sit at 3 to 5 levels of block nesting**: the Linux kernel coding style warns
  past 3, pylint's `max-nested-blocks` defaults to 5, ESLint's `max-depth` to 4. They count blocks
  only, not parentheses.
- **Real kernels, counted with the rule below**: CMSIS-DSP's `arm_dot_prod_f32` about 4, reference
  BLAS `sgemv` about 5, RFC 6234's SHA-256 written in Lemma's style about 3, `examples/dot.lm`
  exactly 4; other kernels estimated, not counted, at 3 to 9.

## Decision

This ADR extends ADR 0008 and changes none of its decisions.

1. **Nesting deeper than 16 levels is a syntax error**, of kind `nesting-limit`, in ADR 0004's
   syntax class, listed in `docs/grammar.md` section 7.
2. **Only what is visible in the source counts.** A level is opened by each `(`, `[` and `{`,
   whatever construct it belongs to (a call's argument list is its `(`, one level), and by each
   unary `-` or `!`, whose level covers its operand. A binary operator opens none, however long its
   chain (`a + b + c`); nor does a clause, a statement, or any recursion internal to the parser,
   such as its precedence levels. The exact counting rule, with `dot` worked through, is in
   `docs/grammar.md` section 3, "Nesting depth". A writer can count the depth of its own text by
   reading it, and the count does not change when the parser's implementation does (P3).
3. **No repair.** Lowering the depth means binding an inner expression with `let`, and that
   requires inventing a name: a guess, which P5 forbids. The message states the limit and, in the
   impl, suggests binding an inner expression with `let`.
4. **The span is the token that opens the seventeenth level**, the first such token in source
   order: its `(`, `[`, `{`, or unary operator. The parse of the function ends there, as for every
   diagnostic without a repair (ADR 0008, choice 20), so the end of the construct is not known, and
   in runaway input it may never come. Parsing resumes at the next `fn` or `export`.
5. **16 is provisional.** The ADR that freezes the grammar after ADR 0006's calibration revisits
   it, from the number of `nesting-limit` records in the calibration runs, read through ADR 0004's
   failure classification. It is a constant revisited at the freeze, not a calibration arm: the
   calibration's design does not change.

Why 16:

- **For the model writer.** A model following the human code it learned from nests 3 to 6 levels
  in such kernels, so 16 never blocks normal code, and it catches runaway generation long before
  the stack is at risk. With the cap, that input yields a syntax-class record with a span, which
  the repair loop can act on, instead of a failure with none.
- **Too low costs more than too high.** Each false rejection costs a round, and one extra round of
  feedback costs the whole kernel again (ADR 0006, "Candidates reshaped or rejected").
  Raising the limit later breaks no program; lowering it does. Readability for model readers is
  steered by the canonical form, the reference card and `let` (P3), not by a hard error, so the
  limit is set for robustness, not for style.

## Consequences

- For the parser: a depth counter, incremented where a level opens and decremented where it
  closes, checked before descending; binary chains are parsed by iteration, not recursion. The
  parser's stack is then bounded by a constant times 16, whatever the input. Resynchronization at
  the next `fn` or `export` scans tokens and does not recurse. Depth is counted on the text the
  recovery parses: a delimiter a repair inserts opens or closes a level as if it were written.
- For the printer and every later pass over the AST: the levels of an AST from the parser nest at
  most 16 deep; the length of a binary chain is not bounded (open points).
- For the verifier and codegen: no obligation and no LLVM fact. For the JIT: nothing.
- For the experiment: `nesting-limit` is counted with the syntax-class failures (P1), and its
  count is the input of the revision in decision 5.
- `docs/grammar.md` gains structure rule 12, the "Nesting depth" subsection and the kind;
  `docs/diagnostics.md` gives the message its fixed shape.

## Open points

- **The limit bounds nesting, not the size of the tree.** A binary chain of any length is accepted
  and parses into a left-deep tree as deep as the chain is long. The parser handles it without
  recursion, but any later pass that recurses on the tree (the checker, the encoding of terms for
  Z3, codegen) does so to the depth of the chain, and a long enough chain still overflows its stack
  or sends a huge term to the solver. Either those passes walk chains without recursion, or chains
  get a limit of their own; this ADR decides neither.
- **`let` exists only in the impl.** The signature, `req`, `ens` and `spec` have no binding form,
  so the message's suggestion does not apply there, and the subset offers no other way to lower
  the depth of a contract expression until calls between functions arrive. The kind is the same
  wherever the limit is reached.
