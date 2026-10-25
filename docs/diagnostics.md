# Diagnostics

Every message `lemma check` returns is first a record, and its two renderings are derived from it
([ADR 0006](adr/0006-optimized-for-its-writer.md), P5 and C2). This document is the normative
format: the record's fields and their types, how a position is counted, what a repair is, the order
of records, the exact bytes of the R and T renderings, and the closed set of kinds with their
ADR 0004 failure classes. The choices it makes beyond ADR 0006 are recorded in
[ADR 0009](adr/0009-diagnostic-records.md). Which rendering the writing model is shown is ADR 0006's
calibration choice C2; both stay available to tools whichever wins.

## 1. The record

| Field | Type | Presence | Meaning |
|---|---|---|---|
| `kind` | kind name | always | what went wrong, from the closed set of section 7 |
| `message` | string | always | one sentence, on one line, in the terms of the reference card |
| `span` | span | always | where it went wrong (section 2) |
| `related` | list of related spans | possibly empty | secondary places: the definition of a value, the clause an obligation comes from |
| `obligation` | obligation | by class (below) | the proof obligation that failed or could not be decided |
| `counterexample` | counterexample | by class (below) | concrete values on which the obligation fails |
| `repair` | non-empty list of edits | optional, by class (below) | the fix, when it is certain (section 3) |

The record has no severity: every record is an error, as C2's layout writes it, and `lemma check`
has no warnings. The class of a record is not a field: it is a function of its kind (section 7),
which the R rendering writes out and checks.

The parts:

- **Related span**: `span`, a span, and `label`, a string saying what the place is.
- **Obligation**: `about`, a string naming the construct the obligation is about and what it
  requires (the bound of `f32x4.load(a, i + 12)`: `i + 12 + 4 <= n`); and `assumes`, a list of
  strings, possibly empty, each a clause the obligation assumes: a `req` clause or a bound,
  printed canonically (`docs/grammar.md`, section 5) as an expression, without its keyword
  (`n % 16 == 0`, `k < n`).
- **Counterexample**: `inputs`, a list, possibly empty, of assignments, one per free variable of the
  obligation, each a `name` and a `value`; and `outputs`, optional, the `spec` and `impl` values the
  two sides compute on those inputs. Inputs come in a fixed order: parameters in signature order,
  then loop, sum and quantifier indices in source order. A value is the canonical text of a Lemma
  literal of the variable's type (`12`, `0.5`, `true`), opaque to the record. The notation of a
  region's contents, which the grammar has no literal for, is fixed with the first counterexample
  over a region.
- **Edit**: `span`, a span of the text as submitted, and `replacement`, a string, possibly empty.

Strings are Unicode text. `message`, `label`, `about`, each `assumes` clause, `name` and `value`
contain no line break; a producer that breaks this rule is at fault, and the T rendering escapes
the break rather than breaking its layout (section 6). A `replacement` may contain line breaks.

**Printable ASCII.** The same six fields hold printable ASCII only, U+0020 to U+007E. A code point
outside that range is never written as itself: it is named `U+` and its number in at least four
uppercase hexadecimal digits (`U+00E9`, `U+202E`, `U+E0041`), without backticks, and a byte that is
not UTF-8 is named `byte 0x` and two uppercase hexadecimal digits (`byte 0xFF`). The one exception
is the active C1 variant's symbols, `Σ`, `∈` and `∀` under variant U (`docs/grammar.md`, section
2): they are source text under that variant, so a field quotes them as written and names them as
spellings to use (``write `Σ` ``). Under variant A they are named like any other code point
(`U+03A3`). The models that read a record see no hidden, control or lookalike text through it,
for the reason ADR 0011 rejects such text in comments; and the message is the channel P5 makes the
writer's main input. A producer that breaks this rule is at fault. The parser of the R rendering
does not check it, since the exception depends on the variant, which a record does not carry. The
rule does not apply to `file`, the name the compiler was given, nor to `replacement`, which is
source text: printable ASCII, tab, line breaks and the active variant's symbols.

### Fields by class

| Class | `obligation` | `counterexample` | `repair` |
|---|---|---|---|
| `syntax` | absent | absent | optional |
| `type-or-layout` | absent | absent | optional |
| `effect` | no kind yet | no kind yet | no kind yet |
| `bound-proof` | required | required | absent |
| `equivalence-proof` | required | required | absent |
| `solver-timeout` | required | absent | absent |
| `toolchain-bug` | optional | absent | absent |

A counterexample's `outputs` are required on `equivalence-unproven`, optional on `ens-unproven`,
and absent on every other kind. A solver `unknown` or timeout never carries a counterexample and is
never a success (P5). A proof failure never carries a repair: what fixes it is never certain, and
the contract is the writer's (ADR 0006, "What this does not license").

## 2. Spans and positions

A **position** is three non-negative integers:

- `byte`: the offset in the file's bytes, 0-based;
- `line`: 1-based; lines are split on LF (U+000A) only, so a CR before an LF belongs to its line;
- `column`: 1-based, counting Unicode code points from the start of the line.

A **span** is a `file`, a `start` position and an `end` position. The range is half-open: `start`
is the first byte covered, `end` the position just after the last one, as rustc's `column_end`. An
empty span (`start` equal to `end`) designates a point between two bytes, where a missing token
belongs. `file` is the file's name exactly as the compiler was given it.

What follows from those rules:

- A tab is one column; nothing is expanded.
- A non-ASCII code point is one column and several bytes: on the variant U line
  `  spec Σ(i: u64 ∈ 0..<n) { a[i] * b[i] }`, `Σ` is at column 8 and `(` at column 9, two bytes
  further.
- The position just after the last byte of the file is valid: in a file ending with an LF, it is
  column 1 of the line after the last one.
- On text that is not valid UTF-8 (the `invalid-encoding` case), a column counts the bytes that are
  not UTF-8 continuation bytes (`10xxxxxx`): the code point count wherever the text is valid, and
  still defined where it is not.
- On valid text, a producer's span starts and ends on code point boundaries.

A span is well formed when its `start` is not after its `end`, by byte and by line and column.

## 3. Repairs

A repair is a **non-empty list of edits, applied together, all or none**. Most repairs are one edit;
`bound-in-clause` needs two, one in the signature and one in the clause.

- **Order and separation.** Edits are sorted by file, then by start byte. Two edits of the same file
  are separated: the first one's end byte is strictly less than the second one's start byte. So
  edits never overlap, two insertions never share a position, and an insertion that touches a
  replaced range is written as part of that edit.
- **Not empty.** An edit has a non-empty span or a non-empty replacement.
- **Against the original text.** Every span refers to the text as submitted, before any edit; the
  list is applied from its last edit to its first, so no offset moves under another edit.
- **Certain.** A repair is offered only when exactly one repair makes the text what the writer
  evidently meant (`docs/grammar.md`, section 7). Never a guess.
- **The contract keeps its meaning.** On the contract, the list taken as a whole changes no
  meaning: it respells, adds a missing delimiter, deletes a duplicate, or moves a statement to the
  place where it means the same. An edit of the list may alone change a meaning when the list does
  not: deleting `req k < n` alone would drop a precondition, and inserting `< n` into `k`'s type in
  the same repair keeps it. A repair never adds, removes or alters a `req`, an `ens`, the spec, a
  bound or a permission (ADR 0006).
- **Between records.** When the repairs of several records insert at the same position, which
  happens when several comments move before the same token (`misplaced-comment`,
  `docs/grammar.md`, section 2), their texts are applied in record order (section 4): the earlier
  record's text comes first, so the comments keep their source order (ADR 0011).
- **Offered, not applied.** The compiler never edits the source. The parser's recovery continues as
  if the whole repair had been applied (`docs/grammar.md`, section 7).

Example, `pick.lm`:

```
export fn pick(n: u64, k: u64, a: &f32[n] @a4 ro) -> f32
  eff pure
  req k < n
  spec a[k]
  equiv allow()
  impl {
    return a[k]
  }
```

`req k < n` states a bound, which belongs in `k`'s type (`bound-in-clause`). The repair is two
edits: insert ` < n` at 1:30, just after `k: u64`; replace 3:1-4:1, the whole clause line with its
indentation and its LF, by nothing. The result is `k: u64 < n` and no `req`; the precondition is
the same.

## 4. Order of records

Both renderings write records in one order: by `span.file` (byte order of the name), then
`span.start.byte`, then `span.end.byte`, then the bytes of the record's R rendering. The renderers
sort; the order in which records are produced never shows. Two identical records are both written:
there is no deduplication, and no cascade to remove either, since a producer emits none (P5).

## 5. The R rendering

One compact JSON object per record, each followed by one LF. No records render to the empty string.

**Keys, in this order**, with absent optional fields omitted, never written as `null`:

| Key | Value | Presence |
|---|---|---|
| `kind` | string, the kind name | always |
| `class` | string, the kind's class (section 7) | always |
| `message` | string | always |
| `span` | span object | always |
| `related` | array of `{"span", "label"}` | when not empty |
| `obligation` | `{"about", "assumes"}`, `assumes` an array of strings, written even when empty | when present |
| `counterexample` | `{"inputs", "outputs"}`, `inputs` an array of `{"name", "value"}` written even when empty, `outputs` a `{"spec", "impl"}` object omitted when absent | when present |
| `repair` | array of `{"span", "replacement"}` | when present |

A span object is `{"file", "start", "end"}`, and a position `{"byte", "line", "column"}`, keys in
those orders.

**Bytes.** No whitespace outside strings. Integers in decimal, without sign, leading zero, fraction
or exponent. In strings, `"` is written `\"`, `\` is `\\`, U+0008 `\b`, U+000C `\f`, LF `\n`,
CR `\r`, tab `\t`, any other code point below U+0020 `\u00` and two lowercase hexadecimal digits
(`\u0001`); every other code point, U+007F and non-ASCII included, is its UTF-8 bytes, and `/` is
not escaped. So one record has exactly one rendering.

**Parsing.** The input is lines separated by LF, the last one with or without its LF, each holding
one record; the empty input holds none. Any JSON whitespace between tokens and any key order are
accepted. A line is rejected, with its 1-based line number and a 1-based column (where the JSON
parser locates the error; 1 for a rule on the record as a whole), when it:

- is empty, or is not one JSON object;
- has an unknown key, a duplicate key, a missing required key, a value of the wrong type, or a
  `null`;
- has an empty `related` or `repair` array;
- names an unknown kind, or a `class` that is not its kind's;
- has a position whose `line` or `column` is 0, or a span that is not well formed (section 2);
- has edits that are not sorted, not separated or empty (section 3);
- has a field its kind does not allow, or lacks one its kind requires (section 1).

Parsing the rendering of a list of records gives that list in the order of section 4.

The `pick.lm` record of section 3:

```
{"kind":"bound-in-clause","class":"type-or-layout","message":"the clause `req k < n` states a bound, which belongs in the type of `k`","span":{"file":"pick.lm","start":{"byte":70,"line":3,"column":3},"end":{"byte":79,"line":3,"column":12}},"related":[{"span":{"file":"pick.lm","start":{"byte":23,"line":1,"column":24},"end":{"byte":24,"line":1,"column":25}},"label":"the parameter the bound belongs to"}],"repair":[{"span":{"file":"pick.lm","start":{"byte":29,"line":1,"column":30},"end":{"byte":29,"line":1,"column":30}},"replacement":" < n"},{"span":{"file":"pick.lm","start":{"byte":68,"line":3,"column":1},"end":{"byte":80,"line":4,"column":1}},"replacement":""}]}
```

## 6. The T rendering

The familiar compiler layout, carrying every field of the record but the byte offsets, which line
and column determine in the file. The renderer is given the records and the sources (each a name
and its bytes); a record's snippet is shown when its `span.file` is the name of a source and its
end byte does not exceed that source's length. When two sources share a name, the first is used.

**Per record**, with `W` the number of digits of the span's start line and `_` standing for `W`
spaces:

```
error[<kind>]: <message>
_--> <file>:<line>:<column>
_ |
<line> | <source line>
_ | <pad><carets>
_ = <label>: <value>
```

- **The `-->` line** gives the span's start.
- **The snippet**, the three lines after it, is omitted when it cannot be shown (above). The decoded
  line is the span's start line, without its LF or a final CR, with exactly one character per column
  of section 2. Read from left to right, the bytes of a well-formed UTF-8 encoding of a code point
  (no overlong form, no surrogate, nothing above U+10FFFF) show that code point; any other byte
  shows as one U+FFFD when it is not a continuation byte (`10xxxxxx`), and as nothing when it is,
  since it counts no column. The source line is the decoded line without trailing spaces or tabs;
  when that leaves it empty, the line is `<line> |`. The pad has one character per column before the
  span's start column: a tab when the decoded line's character at that column is a tab, a space
  otherwise. The carets are one `^` per column of the span on its start line, to the line's last
  column (a final CR excluded, trailing spaces included) when the span continues on a later line,
  and at least one, so an empty span shows one caret at its position.
- **Labelled lines**, in this order, each when its field is present:
  - `span ends: <line>:<column>`, when the span ends on a later line than it starts, or when the
    snippet is omitted and the span is not empty;
  - `related: <file>:<line>:<column>-<line>:<column>: <label>`, one per related span, in record
    order;
  - `obligation: <about>`, then `assumes: <clause>`, one per clause, in record order;
  - `counterexample: <name> = <value>, <name> = <value>`, in record order, or
    `counterexample: (no free variables)` when `inputs` is empty;
  - `spec: <value>` and `impl: <value>`, the outputs;
  - `repair: replace <file>:<line>:<column>-<line>:<column> with "<replacement>"`, one per edit, in
    list order; the edits of a record form its one repair and apply together.
- **Escaping.** In every string field but `replacement`, a code point below U+0020 is written as in
  R (`\n`, `\t`, `\u0001`), so a record always keeps its layout; `"` and `\` stay as they are. A
  `replacement` is written between double quotes, escaped exactly as an R string.
- No line ends with a space or a tab. Records are separated by one empty line, the output ends with
  an LF, and no records render to the empty string.

The `pick.lm` record of section 3:

```
error[bound-in-clause]: the clause `req k < n` states a bound, which belongs in the type of `k`
 --> pick.lm:3:3
  |
3 |   req k < n
  |   ^^^^^^^^^
  = related: pick.lm:1:24-1:25: the parameter the bound belongs to
  = repair: replace pick.lm:1:30-1:30 with " < n"
  = repair: replace pick.lm:3:1-4:1 with ""
```

A proof failure, on `examples/dot.lm` with its `req` weakened to `req n % 4 == 0`:

```
error[bound-unproven]: the load `f32x4.load(a, i + 12)` may read past the end of `a`
  --> dot.lm:17:21
   |
17 |       next c3 = fma(f32x4.load(a, i + 12), f32x4.load(b, i + 12), c3)
   |                     ^^^^^^^^^^^^^^^^^^^^^
   = related: dot.lm:4:3-4:17: the clause the obligation assumes
   = obligation: the bound of `f32x4.load(a, i + 12)`: `i + 12 + 4 <= n`
   = assumes: n % 4 == 0
   = counterexample: n = 12, i = 0
```

and its R rendering:

```
{"kind":"bound-unproven","class":"bound-proof","message":"the load `f32x4.load(a, i + 12)` may read past the end of `a`","span":{"file":"dot.lm","start":{"byte":673,"line":17,"column":21},"end":{"byte":694,"line":17,"column":42}},"related":[{"span":{"file":"dot.lm","start":{"byte":159,"line":4,"column":3},"end":{"byte":173,"line":4,"column":17}},"label":"the clause the obligation assumes"}],"obligation":{"about":"the bound of `f32x4.load(a, i + 12)`: `i + 12 + 4 <= n`","assumes":["n % 4 == 0"]},"counterexample":{"inputs":[{"name":"n","value":"12"},{"name":"i","value":"0"}]}}
```

## 7. Kinds and their classes

A kind name is lowercase ASCII words joined by `-`. The set of kinds is closed: a kind exists when
one of the tables listed below names it, each kind in exactly one table. A kind keeps its name and
its class once ADR 0004's toolchain is frozen. Every kind belongs to exactly one class, and every
class is one of ADR 0004's failure causes:

| Class | ADR 0004 cause |
|---|---|
| `syntax` | syntax |
| `type-or-layout` | type or layout |
| `effect` | effect |
| `bound-proof` | bound proof |
| `equivalence-proof` | equivalence proof |
| `solver-timeout` | solver timeout |
| `toolchain-bug` | toolchain bug |

ADR 0004's eighth cause, the wrong spec, has no class and no kind: only the spec tests (P9), the
spec review (P8) and the hidden tests reveal it.

The kinds are listed in four tables, and a test checks that they match the compiler's. Each has
`Kind` as the header of its first column, its kind names in backticks in that column, and no other
table of these three files has that header:

| Table | Class of its kinds |
|---|---|
| [`docs/grammar.md`](grammar.md), section 7, "Syntax diagnostics" | `syntax` |
| [`docs/grammar.md`](grammar.md), section 7, "Kinds of the checker that concern names and bounds" | `type-or-layout` |
| [ADR 0007](adr/0007-export-mapping-and-content-hash.md), "Refusals" | `type-or-layout` |
| This document, "Kinds of the verifier and the toolchain" | its `Class` column |

### Kinds of the verifier and the toolchain

| Kind | Class | Raised by |
|---|---|---|
| `bound-unproven` | `bound-proof` | an obligation that a value stays in a range, refuted by a counterexample: an index or a `load` within its region's extent, an integer operation within its type, a nonzero divisor, a result within its declared bound, the spec's definedness, per-target pointer-width arithmetic (ADR 0008) |
| `equivalence-unproven` | `equivalence-proof` | the impl not equivalent to the spec under the `allow` list, refuted by a counterexample with what both sides compute |
| `ens-unproven` | `equivalence-proof` | an `ens` clause that does not hold of the spec, or of the impl's result under a non-empty `allow` list, refuted by a counterexample |
| `solver-unknown` | `solver-timeout` | the solver answering `unknown`, or reaching its time or resource limit, on an obligation; the message says which |
| `internal-error` | `toolchain-bug` | a failure of the compiler the input does not explain: a broken invariant, an LLVM or Z3 error, a solver model that does not refute the obligation; its span is the construct being processed, or the empty span at the start of the file when none is |

A failed `ens` is counted with the equivalence proofs: an `ens` states what the function computes,
like the spec, and its failure means two statements of that disagree. A bounded result is proven
like an `ens` but states a range, so its failure is a `bound-unproven`: the class follows the shape
of the obligation. ADR 0009 gives the reasons.

Effect kinds are added by the change that implements the effect check, and the kinds of the JIT's
spec-versus-impl differential tests by the change that implements those tests, each with its class
decided there.

### Messages with a fixed shape

Every message holds printable ASCII and names any other code point or byte as section 1 states.
In the shapes below, `<found>` is the token where the diagnostic stands: its spelling in backticks;
a code point outside printable ASCII, other than an active C1 symbol, named as in section 1 without
backticks (`U+03A3` under variant A); or `end of input`. `<code point>` is the code point the
diagnostic is about, named as in section 1.

- `unexpected-token`: `unexpected <found>, expected <list>`; the list gives the spelled tokens in
  backticks, in byte order of their spelling, then the token classes in words (`a name`, `an integer
  literal`, `a type`) in alphabetical order, joined as `x`, `x or y`, `x, y or z`. For example:
  ``unexpected `]`, expected `)` or `,` ``.
- `comment-character`: `a comment holds printable ASCII and tab only; found <first>`, followed by
  ` and <k> more` when the comment holds `k` other rejected code points or bytes, at least one.
  `<first>` is the first of them, named as in section 1 (`U+202E`, `U+E0041`, `byte 0xFF`). For
  example: `a comment holds printable ASCII and tab only; found U+202E and 3 more`.
- `invalid-encoding`: `byte 0x<hh> is not UTF-8`, naming the first byte of the span's run of bytes
  that are not UTF-8; when the run holds `k` other bytes, at least one,
  `byte 0x<hh> and <k> more are not UTF-8`. For example: `byte 0xFF is not UTF-8`,
  `byte 0xC3 and 2 more are not UTF-8`.
- `non-ascii`: `<code point> is not ASCII`. For example: `U+00E9 is not ASCII`.
- `lookalike`, with a repair: ``<code point> looks like `<repair>` ``, the repair being the active
  spelling for a C1 lookalike. For example: ``U+2265 looks like `>=` ``, and
  ``U+2211 looks like `sum` `` under variant A, ``U+2211 looks like `Σ` `` under variant U. For a
  whitespace lookalike, `<code point> is a lookalike of whitespace; use a space`, or `; delete it`
  when the repair deletes it (`U+200B`, `U+FEFF`).
- `lookalike`, without a repair: `<code point> is another symbol, not in the language`
  (`U+2203 is another symbol, not in the language`); `U+2248` outside `equiv`,
  `U+2248 is not an operator of the language`.
- `range-operator` on `…`: ``U+2026 is not a range; write `lo..<hi` ``.
- `c1-other-variant`: ``<found> is the other C1 variant's spelling; write `<active>` ``. For
  example: ``U+03A3 is the other C1 variant's spelling; write `sum` `` under variant A, and
  `` `sum` is the other C1 variant's spelling; write `Σ` `` under variant U.
- `reserved-word`: `<found> is reserved and cannot be used as a name`. For example:
  `` `loop` is reserved and cannot be used as a name ``, and, for `let Σ: u64` under variant A,
  `U+03A3 is reserved and cannot be used as a name`.
- `invalid-number`, for a leading zero:
  `` `<spelling>` is not a valid number: an integer literal has no leading zero ``, with
  `a float literal` in place of `an integer literal` when the spelling holds a `.` or an `e`. For
  example: `` `007` is not a valid number: an integer literal has no leading zero ``.
- `unclosed-delimiter`: `` `<open>` is not closed before <found> ``, where `<open>` is the
  innermost delimiter the token cannot occur inside; when `k` other such delimiters are open, at
  least one, `` `<open>` and <k> more are not closed before <found> ``. At a clause or statement
  keyword those are the open `(` and `[`; at `fn`, `export` or end of input, every open delimiter,
  `{` included. When the record carries the repair, the message ends with ``; insert `<closer>` ``.
  For example: `` `(` is not closed before `spec`; insert `)` `` and
  `` `(` and 1 more are not closed before `spec` ``.
- `assignment`: ``a statement cannot begin with <found>; a value is defined once, by `let`, a
  carry or `next` ``. The word found is a name to Lemma and often a keyword to the writer, so the
  message calls it neither. For example:
  ``a statement cannot begin with `var`; a value is defined once, by `let`, a carry or `next` ``.
- `nesting-limit`: in the impl,
  ``nesting exceeds the limit of 16 levels; bind an inner expression to a name with `let` ``; in
  the signature or a contract clause, which have no `let`,
  `nesting exceeds the limit of 16 levels`. The number is the limit in force (`docs/grammar.md`,
  section 3, "Nesting depth"; ADR 0010).
