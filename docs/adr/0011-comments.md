# 11. Comments: on their own line, in printable ASCII

- Status: proposed
- Date: 2026-10-25

## Context

A comment is the only free text of a `.lm` file. It is not in the AST, not in the contract the
toolchain emits for the spec review (ADR 0006, P8), and not in the content hash (ADR 0007), so it
cannot change a verdict of the compiler, the verifier or the reviewer. But the models that read the
whole file read it: the writer fixing its code after a diagnostic, and a later model changing the
function. Two risks follow.

- **Injected text.** A comment can carry instructions addressed to the model that reads it. Some of
  it can be invisible to a person auditing the canonical text while a model reads it in full: the
  Unicode tag characters (U+E0000 to U+E007F) spell ASCII text no renderer shows; the bidirectional
  controls reorder what an editor displays, which is the Trojan Source attack (CVE-2021-42574); the
  zero-width characters hide text between visible letters; a CR not followed by LF makes a terminal
  overwrite the line it ends; escape sequences drive the terminal itself. ADR 0006 keeps the
  canonical text "available for a human audit"; text a person cannot see defeats that audit.
- **Wrong or stale comments.** Nothing checks a comment against the code. A model trusts a comment
  that contradicts the code it sits on, and P4 already says why: a statement the compiler cannot
  check "drifts, and a model reading it may trust it."

In Lemma most of what comments usually say is said by checked constructs: the contract (`spec`,
`req`, `ens`, `eff`), types with their layout and bounds, and the `allow` list of `equiv`. What a
comment still adds for a model is what the compiler cannot check: the reason for an impl choice
(`dot`'s four accumulators, which hide the latency of `fma`) and provenance (the standard or the
algorithm a function follows).

ADR 0008 lets a comment stand anywhere a line can end. Specifying the canonical form showed the
cost: a comment trailing a clause, a comment between two tokens the canonical form prints on one
line, a comment before a closing `}` or before the `) {` of a carry list, and comments after the
last item each needed a rule of their own, and a comment inside a construct had to move when
printed. Every place a comment may stand is one more form for the formatter, the tests and every
later tool (P2), and a model that reads a comment trailing a line cannot tell whether it is about
that line or about the next.

## Decision

This ADR extends ADR 0008's lexical rules (decisions 16 to 18) and changes none of its decisions.
It amends ADR 0006, P6, by one sentence (below). The rules are written in
[`docs/grammar.md`](../grammar.md), section 2, "Comments", and section 7.

1. **A comment stands on a line of its own, before an item, a clause or a statement**, and is
   attached to it: only spaces and tabs precede it on its line, and the next token, other comments
   skipped, is the first token of an item (`export` or `fn`), of a clause (`eff`, `req`, `ens`,
   `spec`, `equiv`, `impl`) or of a statement (`let`, `loop`, `next`, `return`). Any other comment
   is a `misplaced-comment` diagnostic: one that follows code on its line; one inside a construct,
   such as a parameter list, an expression, a carry list (whether the canonical form prints it on
   one line or several), or between `impl` and its `{`; one before a closing `}`; and one after the
   last item. A carry is not a statement, so a comment before a carry is misplaced, and the allowed
   places do not depend on how the canonical form lays out a construct. A misplaced comment is an
   error, not a layout the formatter corrects: comments elsewhere are not wanted, and the
   diagnostic tells the writer where comments belong instead of fixing them without a word.
2. **Its repair moves it.** A comment carries no meaning, so where it stands changes nothing the
   compiler checks, and the repair is certain. The comment goes to its own line just before the
   first token of the innermost item, clause or statement that contains the last token before the
   comment: `req n % 16 == 0 // why` goes above `req`, a comment after a carry above its `loop`, a
   comment inside the parameter list above the `fn` line (above `export`), a comment before the
   `}` of a loop body above the last statement of that body, a comment after the last item above
   that item's `impl`. As an edit list (ADR 0009, decision 1), the repair is two edits: an insertion
   at the start of that first token, of the comment's text and a line feed, followed by the spaces
   and tabs before the token when it is the first of its line, and preceded by a line feed when it
   is not; and a deletion of the comment with the spaces and tabs before it, and, when nothing else
   is on its line, of its whole line with its line end. The insertion lies before the deletion, so
   the two are separated. In a file with no item, there is no construct to move a comment to, and
   `misplaced-comment` has no repair.
3. **A comment's text is printable ASCII and tab**: U+0020 to U+007E, and U+0009. Tab stays, since
   it is the whitespace the language already allows between tokens and hides nothing. Any other
   code point in a comment (a non-ASCII code point, a control character, DEL, a CR that does not
   end the line) and any byte that is not UTF-8 is a `comment-character` diagnostic, a new kind,
   whose repair deletes each maximal run of them. The kind is new rather than `non-ascii`,
   `invalid-character` or `invalid-encoding` because those have no repair (in code, the intent of a
   stray code point is not certain) and because the calibration must count this restriction
   separately (below). One record per comment: its span is the first rejected code point or byte,
   its repair one edit per run. The message names what it rejects as `U+` and hexadecimal digits,
   or as a byte in hexadecimal, never the code point itself, so a diagnostic does not pass hidden
   text on to the model that reads it. A file under variant A is then ASCII, and a file under
   variant U holds no non-ASCII code point but the three C1 symbols. ADR 0006, P7, which allows
   non-ASCII text "only in comments" and C1's symbols, stays true as an upper bound.
4. **Invalid UTF-8 inside a comment is a `comment-character` diagnostic**, repaired by deleting
   the invalid bytes, not an `invalid-encoding` diagnostic without repair. Comment text has no
   meaning to decode, so deleting what cannot be decoded is certain; reported without repair, the
   fault would end the parse of the function that follows the comment for a slip in text that
   cannot change it. `invalid-encoding` stays for bytes outside comments.
5. **A comment yields at most one record.** A comment can break several rules at once: its marker
   (`;`, `#` or `/* */`, rejected with a `//` repair under ADR 0008), its characters, and its
   place. Separate records would carry overlapping repairs (the move deletes the text the other
   repairs edit), which no tool can apply together and which recovery cannot assume together. The
   record's kind is the first that applies among the marker kinds, `comment-character` and
   `misplaced-comment`; its message says what else the comment breaks, and its repair, when every
   broken rule has one, fixes them all: a comment that moves is inserted with `//` and without its
   rejected code points.
6. **Repairs of several records that insert at one position apply in record order.** Several
   comments moved before the same token, by different records, insert at the same position.
   `docs/diagnostics.md`, section 3, now says that their texts are applied in record order (its
   section 4), the earlier record's text first, which keeps the comments in source order. This
   extends ADR 0009's decision 1, which orders the edits within one repair.
7. **The canonical form follows.** A comment is printed on its own line just before the item,
   clause or statement it is attached to, at that construct's indentation, as written minus the
   spaces and tabs at its end; comments attached to one construct keep their order. The CR of a CRLF
   line end belongs to the line end, not to the comment. Since the formatter formats only text with
   no syntax diagnostic, these are all the rules it needs for comments (`docs/grammar.md`, section
   5).

The principles of ADR 0006 this rests on:

- **P2, one canonical form.** One place for a comment instead of five, so one way to print it, and
  a formatter that never moves a comment.
- **P3, locality.** What a comment is about is the construct below it, the place a model reading
  the line looks for it; a trailing comment leaves the reader to guess between its own line and the
  next.
- **P5, diagnostics are data.** Both kinds are records with a certain repair, so recovery continues
  and every comment slip of a submission is reported in one round; the messages carry no rejected
  code point.
- **P6, no meaning in layout.** The whitespace property is unchanged: it is stated once comments
  are removed. The rule that a comment stands on its own line is a rule on layout, though:
  `req n % 16 == 0 // why` is rejected where the same comment one line break later is accepted. P6
  says that "layout in a submission is never an error"; ADR 0006 now carries a one-sentence
  amendment under P6 that makes the place of a comment the one exception, and states that the
  whitespace property is unchanged. Code layout stays free.

### Alternatives rejected

- **No comments at all.** It removes the free-text channel entirely, and the stale-comment problem
  with it. Rejected for now: models write comments by habit, so every habitual comment would cost a
  round, and one round costs the whole kernel again (ADR 0006, "Candidates reshaped or rejected");
  and the reason for an impl choice, which nothing else in the language can state, would be lost to
  the next model that changes the function.
- **Comments anywhere, with only invisible characters rejected.** It addresses invisible text and
  nothing else: a list of invisible code points must be maintained against Unicode's growth, the
  other non-ASCII text that reads differently to a person and to a model (lookalikes, mixed
  scripts) stays, and the five placement rules of the canonical form stay with it. Printable ASCII
  is a closed set that needs no list.
- **No error for a misplaced comment, which the formatter moves.** A comment would be accepted
  anywhere a line can end, and the canonical form would print it on its own line before the item,
  clause or statement decision 2 moves it to. This keeps P6's "layout in a submission is never an
  error" without exception, and costs no round. Rejected for now: comments outside those places are
  not wanted, and a correction the formatter makes silently does not tell the writer where comments
  belong, where the error does. The calibration measures the rounds the error costs (below), and may
  relax the rule to this alternative.

## Consequences

- **For the lexer**: a comment runs to its line end, the CR of a CRLF line end excluded, so a
  `Comment`'s range stops before it; the comment's text is checked against printable ASCII and tab,
  and each comment with a rejected code point or byte yields one `comment-character` fault whose
  repair is one deletion per run; invalid UTF-8 in a comment no longer yields an unrepaired
  `invalid-encoding` fault on the next token.
- **For the parser**: it knows, for each comment, whether the next token begins an item, a clause or
  a statement, and the innermost such construct of the token before it, to emit
  `misplaced-comment` with its two-edit repair; it merges the comment's faults into one record
  (decision 5). A comment in a region that the parse skips after a diagnostic without repair is not
  judged, as nothing in that region is.
- **For the printer**: the comment rules shrink to one placement and one text normalization; the
  formatter never moves a comment, since a text in which a comment is misplaced has a diagnostic and
  does not format.
- **For the verifier, codegen and the JIT**: nothing. Comments reach none of them, add no
  obligation and justify no LLVM fact.
- **For `docs/diagnostics.md`**: the tie rule of decision 6 in section 3, and the fixed message
  shape of `comment-character` in section 7.
- **For the experiment**: both kinds are syntax-class failures, counted by P1's metric. The
  restriction remains open to injected instructions written in plain, visible ASCII; it makes them
  visible to an audit, and the spec reviewer never sees comments (P8), but whether a model that
  changes code should be told to treat comments as data is a matter for the harness, not the
  language.
- **The choice is revisited at the calibration**, by the ADR that freezes the grammar, with three
  measurements read from the calibration runs: how often model writers emit comments at all, how
  many rounds `misplaced-comment` and `comment-character` cost (the records of those kinds, and the
  marker-kind records whose repair also moves the comment, counted per run), and whether kernels
  written without comments, under a card that asks for none, do worse on ADR 0004's metrics than
  kernels written with them. Rounds lost to the restriction with no measurable gain from comments
  argue for forbidding them; many rounds lost to `misplaced-comment` argue for relaxing the
  placement rule to the rejected alternative in which the formatter moves the comment and no error
  is raised. It is not a calibration arm: the calibration's design does not change.
- ADR 0006 carries a one-sentence amendment under P6 pointing here.
- `examples/dot.lm` already conforms: its one comment is ASCII and stands on its own line before the
  item.

## Open points

- **The T rendering's snippet** shows the source line as decoded (`docs/diagnostics.md`, section 6),
  so a line that holds a rejected code point is shown with it. The message never names the code
  point itself; whether the snippet should also escape what the comment rules reject is a decision
  on the renderer, left open.
- **The repair deletes**, where an ASCII spelling would keep the comment's sense for the common
  cases (`≤` as `<=`, `→` as `->`, `×` as `*`, an em dash as `-`), as the lookalike table already
  does in code. Deleting is the simpler rule; transliterating would keep more of a comment the
  restriction otherwise mutilates.
