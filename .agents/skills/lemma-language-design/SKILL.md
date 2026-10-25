---
name: lemma-language-design
description: Use when a change proposes or alters a Lemma language construct (syntax, types and their layout or bounds, effects, the memory model, the spec/impl/equiv contract and its allow options, the C ABI or WebAssembly boundary, JIT hot replacement), when it reverses or extends an architecture decision, or when it changes behavior described in README.md, docs/design.md or AGENTS.md. Covers judging the proposal against the written scope and principles, and recording the decision as an ADR.
---

# Lemma language design

Every feature added for an off-target domain dilutes the core, and every semantic decision taken
silently in code becomes a contradiction someone discovers later. A language change is decided in
writing first, where the next reader will look for it.

## Read what exists

- [`docs/adr/0002-scope.md`](../../../docs/adr/0002-scope.md): the target domains and what is out
  of scope.
- Every ADR in [`docs/adr/`](../../../docs/adr/): status, date, what it supersedes.
- [`docs/design.md`](../../../docs/design.md), [`docs/grammar.md`](../../../docs/grammar.md) and
  the `.lm` files in [`examples/`](../../../examples/) that use the construct.

## Judge the proposal

1. **Against the scope.** Which target domain of ADR 0002 does it serve, and what does it cost
   the primary constraints (allocation-free operation, bounded execution time, no standard
   library, tiny binaries)? A proposal that serves only an out-of-scope domain is rejected or left
   to host languages. That is a legitimate outcome. Changing the scope itself is a separate
   decision for the maintainer.
2. **Against the principles.** Pure SSA, types that carry memory layout, explicit effects, spec
   and impl separate and proven equivalent, nothing implicit (floating-point reassociation,
   overflow and traps are opt-in), hash-addressed functions. A construct that needs alias-analysis
   guesswork, hidden state or an implicit conversion contradicts them: say which.
3. **For verification and codegen.** State the proof obligations the construct generates, and
   which LLVM facts it lets the compiler emit and on what proven basis (see
   [lemma-specs-and-proofs](../lemma-specs-and-proofs/SKILL.md) and
   [lemma-code-review](../lemma-code-review/SKILL.md)).

Decide by the written scope, not by taste.

## Record the decision

- **New decision**: `docs/adr/NNNN-<slug>.md`, the next number, in the format of the existing
  ADRs: title, `Status`, `Date`, then *Context*, *Decision*, *Consequences*. A new ADR starts as
  `proposed`; the maintainer accepts it.
- **Reversed decision**: a new ADR that supersedes the old one; the old one's status becomes
  `superseded by NNNN` and nothing else in it changes. ADRs are not rewritten.
- **`docs/design.md`**: the construct as a reader of the language needs it, with an example in
  the style of [`examples/dot.lm`](../../../examples/dot.lm).
- **Documentation stays true**: a change that alters behavior described in `README.md`,
  `docs/design.md` or `AGENTS.md` updates that file in the same commit.

Plain terms only: an ADR states the context, the decision and its consequences, not the history of
the discussion, and carries no ticket key.
