# Grammar of the prototype subset

This is the grammar of the subset the prototype parses: enough for `dot`, and the base the subset's
extensions build on. It follows every consequence of
[ADR 0006](adr/0006-optimized-for-its-writer.md); the choices it makes beyond that ADR are recorded
in [ADR 0008](adr/0008-prototype-grammar.md), its nesting limit in
[ADR 0010](adr/0010-nesting-limit.md), and the C boundary it maps to in
[ADR 0007](adr/0007-export-mapping-and-content-hash.md). It is provisional: the ADR that records
ADR 0006's calibration freezes it.

The grammar carries both variants of choice C1 (ADR 0006). One is active per run, behind a switch;
the other's spellings are recognized only to be rejected. **The provisional default is variant A**,
ADR 0006's default on a tie, until the calibration settles it.

## 1. The two forms of `dot`

Variant A, the provisional default, and the text of `examples/dot.lm`:

```
// Dot product of two f32 arrays, for 128-bit NEON with four accumulators.
export fn dot(n: u64, a: &f32[n] @a128 ro, b: &f32[n] @a128 ro) -> f32
  eff pure
  req n % 16 == 0
  spec sum(i: u64 in 0..<n) { a[i] * b[i] }
  equiv allow(fp_reassoc, fp_contract)
  impl {
    loop i: u64 in 0..<n step 16 carry(
      c0: f32x4 = f32x4.splat(0.0),
      c1: f32x4 = f32x4.splat(0.0),
      c2: f32x4 = f32x4.splat(0.0),
      c3: f32x4 = f32x4.splat(0.0)
    ) {
      next c0 = fma(f32x4.load(a, i), f32x4.load(b, i), c0)
      next c1 = fma(f32x4.load(a, i + 4), f32x4.load(b, i + 4), c1)
      next c2 = fma(f32x4.load(a, i + 8), f32x4.load(b, i + 8), c2)
      next c3 = fma(f32x4.load(a, i + 12), f32x4.load(b, i + 12), c3)
    }
    return reduce_sum((c0 + c1) + (c2 + c3))
  }
```

Variant U differs in exactly two lines:

```
  spec Σ(i: u64 ∈ 0..<n) { a[i] * b[i] }
    loop i: u64 ∈ 0..<n step 16 carry(
```

Everything before `impl` is the contract (ADR 0006, P8): `lemma` emits it alone, comments removed,
for the spec review. Its C export is derived in ADR 0007 and is exactly ADR 0005's.

## 2. Lexical structure

The lexer takes no feedback from the parser (P7): every token's kind is decided by its own
characters, longest match first.

### Source text

A source file is UTF-8. Invalid UTF-8 is rejected (`invalid-encoding`), and so is a byte-order
mark (`lookalike`, repair: delete it). Code points are compared exactly: no Unicode normalization
is applied.

### Whitespace

Whitespace is U+0020 space, U+0009 tab, U+000A line feed and U+000D carriage return. It separates
tokens and means nothing else (P6). Other Unicode spaces are lookalikes (table in section 2.9).

The whitespace property, tested by property tests: once comments are removed, replacing every run
of whitespace by a single space yields the same AST.

### Comments

`//` starts a comment that runs to the next line feed. It is the only comment form, and the only
construct that ends at a line break. `///` is an ordinary comment. Comments are not part of the
AST, the contract emission, or the content hash; the formatter keeps them (section 5).

### Identifiers

```
IDENT ::= [a-z] [a-z0-9_]*
```

ASCII, lowercase, starting with a letter. A word that matches this pattern and is a keyword or a
reserved type name (below) is that keyword or type name, never an identifier. A word with an
uppercase letter or a leading underscore is rejected (`invalid-identifier`): the C header derives
uppercase constant names from identifiers, and that derivation must be injective (ADR 0007).

### Keywords

Reserved everywhere, none contextual, in every C1 variant:

| Group | Keywords |
|---|---|
| Items and clauses | `export` `fn` `eff` `req` `ens` `spec` `equiv` `allow` `impl` |
| Statements | `let` `loop` `step` `carry` `next` `return` |
| Values | `result` `true` `false` |
| Modes, effects, permissions | `ro` `wo` `pure` `fp_reassoc` `fp_contract` |
| C1, variant A spellings | `sum` `in` `forall` |
| Types | `bool` |

`sum`, `in` and `forall` are reserved under variant U too, so that every arm reserves the same
words and a variant A spelling is always recognized to be rejected.

`requires`, `ensures` and `effects` are not keywords: where a clause is expected they are rejected
with `req`, `ens` and `eff` as repairs (`foreign-keyword`).

### Reserved type names

A word is a type name, not an identifier, when it matches:

```
SCALAR_TYPE ::= ( "u" | "i" ) ( "8" | "16" | "32" | "64" | "128" ) | "f" ( "16" | "32" | "64" )
VECTOR_TYPE ::= SCALAR_TYPE "x" ( "2" | "4" | "8" | "16" | "32" | "64" )
```

The subset's checker accepts `u8` to `u64`, `i8` to `i64`, `f32`, `f64`, `bool`, and vectors of
`f32`, `f64` and the integer types; the other reserved names (`f16`, `u128`, …) are reserved so
that adding them later breaks no program. Words such as `i0`, `i1` or `f2` are identifiers.

### Literals

```
INT   ::= "0" | [1-9] [0-9]*
FLOAT ::= ( "0" | [1-9] [0-9]* ) "." [0-9]+ ( "e" "-"? [0-9]+ )?
```

A literal has no type of its own: it takes the type its context requires (the other operand of a
binary operator, the parameter of the operation it is passed to, the type of the binder it
initializes, the element type of a `splat`, the index type of the loop whose `step` it is). A
literal with no such context, or whose value does not fit its type, is a type error. An integer
literal is never a floating-point value (`0` where `f32` is expected is a type error whose repair
is `0.0`). A float literal denotes its decimal value rounded to the nearest value of its type, ties
to even. Signs are the unary operator, not part of the literal. A `.` continues a number only
when a digit follows it, so `0..<n` is `0`, `..<`, `n`; digits followed by a `.` and then neither
a digit nor a `.` are an `invalid-number` (`5.`). Hexadecimal, digit separators and type suffixes
are not in the subset (section 7 lists their diagnostics).

### Punctuation and operators

```
( ) [ ] { } , : . -> = & @aN ..<
+ - * / %   == != < <= > >=   && || !
```

`@aN` is one token, `ALIGN ::= "@a" [0-9]+`: an alignment of `N` bytes. `..<` is one token.
Longest match also produces tokens that are always errors, so that a habit from another language is
caught where it is written: `;`, `::`, `..`, `...`, `..=`, `++`, `--`, `+=`, `-=`, `*=`, `/=`,
`%=`, `**`, `&` as a binary operator, `|`, `^`, `~`, `<<`, `>>`, `/*`, `#` (section 7).

### The C1 symbols

| Meaning | Variant U | Variant A |
|---|---|---|
| Sum | `Σ`, U+03A3 GREEK CAPITAL LETTER SIGMA | `sum` |
| Range membership | `∈`, U+2208 ELEMENT OF | `in` |
| Universal quantifier | `∀`, U+2200 FOR ALL | `forall` |

The quantifier is not one of ADR 0006's two C1 symbols; it follows C1's outcome (ADR 0008), so one
switch selects all three. `SUM`, `IN` and `FORALL` below stand for the active variant's spelling.
The inactive variant's spelling is rejected with `c1-other-variant`, whose repair is the active
spelling. Non-ASCII text appears only in comments and, under variant U, as these three code points.

### Lookalikes

Rejected with the kind `lookalike`. "Active" means the active variant's spelling. A repair marked
"where expected" is offered only where the parser expects that token; elsewhere the code point is a
`non-ascii` diagnostic without repair.

| Code point | Name | Repair |
|---|---|---|
| U+2211 `∑` | N-ARY SUMMATION | active sum |
| U+01A9 `Ʃ` | LATIN CAPITAL LETTER ESH | active sum |
| U+2140 `⅀` | DOUBLE-STRUCK N-ARY SUMMATION | active sum |
| `𝚺` `𝛴` `𝜮` `𝝨` `𝞢` | the capital sigmas of the Mathematical Alphanumeric Symbols block: bold, italic, bold italic, sans-serif bold, sans-serif bold italic | active sum |
| U+220A `∊` | SMALL ELEMENT OF | active membership |
| U+03F5 `ϵ` | GREEK LUNATE EPSILON SYMBOL | active membership, where expected |
| U+03B5 `ε` | GREEK SMALL LETTER EPSILON | active membership, where expected |
| U+2209 `∉`, U+220B `∋`, U+220C `∌`, U+220D `∍` | NOT AN ELEMENT OF, CONTAINS AS MEMBER, DOES NOT CONTAIN AS MEMBER, SMALL CONTAINS AS MEMBER | none: another meaning |
| U+2C6F `Ɐ` | LATIN CAPITAL LETTER TURNED A | active quantifier |
| U+2203 `∃`, U+2204 `∄` | THERE EXISTS, THERE DOES NOT EXIST | none: another meaning, not in the subset |
| U+2264 `≤`, U+2265 `≥`, U+2260 `≠` | comparisons | `<=`, `>=`, `!=` |
| U+00D7 `×`, U+22C5 `⋅`, U+00B7 `·` | MULTIPLICATION SIGN, DOT OPERATOR, MIDDLE DOT | `*` |
| U+2212 `−` | MINUS SIGN | `-` |
| U+2192 `→` | RIGHTWARDS ARROW | `->` |
| U+2227 `∧`, U+2228 `∨`, U+00AC `¬` | logical and, or, not | `&&`, `\|\|`, `!` |
| U+2026 `…` | HORIZONTAL ELLIPSIS | none: see `range-operator` |
| U+2248 `≈` | ALMOST EQUAL TO | inside `equiv`: see `equiv-relation`; elsewhere none |
| U+00A0, U+2007, U+2009, U+202F | no-break, figure, thin, narrow no-break space | U+0020 |
| U+200B, U+FEFF | ZERO WIDTH SPACE, ZERO WIDTH NO-BREAK SPACE | delete |

Under variant A, `Σ`, `∈` and `∀` themselves are `c1-other-variant`, and the sum,
membership and quantifier lookalikes above are repaired to `sum`, `in` and `forall`.

## 3. Syntax

W3C-style EBNF: `::=` defines, `|` separates alternatives, `?` `*` `+` mean optional, zero or
more, one or more; quoted strings are tokens. `SUM`, `IN` and `FORALL` are the active C1
spellings. The grammar is LL(2): after an identifier, one more token tells an index, a call and a
plain name apart.

```
file          ::= item* EOF
item          ::= "export"? function

function      ::= "fn" IDENT "(" params? ")" "->" sig_type
                  eff_clause req_clause* ens_clause* spec_clause equiv_clause impl_clause
params        ::= param ( "," param )*
param         ::= IDENT ":" ( sig_type | ref_type )
sig_type      ::= value_type | bounded_type
bounded_type  ::= SCALAR_TYPE bound
bound         ::= ( "<" | "<=" ) ( INT | IDENT )
ref_type      ::= "&" element "[" extent "]" ALIGN mode
element       ::= SCALAR_TYPE | "(" bounded_type ")"
extent        ::= INT | IDENT
mode          ::= "ro" | "wo"
value_type    ::= SCALAR_TYPE | VECTOR_TYPE | "bool"

eff_clause    ::= "eff" effect ( "," effect )*
effect        ::= "pure"
req_clause    ::= "req" expr
ens_clause    ::= "ens" expr
spec_clause   ::= "spec" expr
equiv_clause  ::= "equiv" "allow" "(" ( permission ( "," permission )* )? ")"
permission    ::= "fp_reassoc" | "fp_contract"
impl_clause   ::= "impl" block

block         ::= "{" statement* "}"
statement     ::= let_stmt | loop_stmt | next_stmt | return_stmt
let_stmt      ::= "let" binder "=" expr
loop_stmt     ::= "loop" binder IN range "step" INT
                  "carry" "(" carry ( "," carry )* ")" block
carry         ::= binder "=" expr
next_stmt     ::= "next" IDENT "=" expr
return_stmt   ::= "return" expr
binder        ::= IDENT ":" value_type
range         ::= add_expr "..<" add_expr

expr          ::= or_expr
or_expr       ::= and_expr ( "||" and_expr )*
and_expr      ::= cmp_expr ( "&&" cmp_expr )*
cmp_expr      ::= add_expr ( cmp_op add_expr )?
cmp_op        ::= "==" | "!=" | "<" | "<=" | ">" | ">="
add_expr      ::= mul_expr ( ( "+" | "-" ) mul_expr )*
mul_expr      ::= unary ( ( "*" | "/" | "%" ) unary )*
unary         ::= "-" unary | "!" primary | primary
primary       ::= INT | FLOAT | "true" | "false" | "result"
                | IDENT
                | IDENT "[" expr "]"
                | IDENT "(" args? ")"
                | VECTOR_TYPE "." IDENT "(" args? ")"
                | SUM "(" binder IN range ")" "{" expr "}"
                | FORALL "(" binder IN range ")" "{" expr "}"
                | "(" expr ")"
args          ::= expr ( "," expr )*
```

Why no separator is needed: every clause and every statement begins with a keyword, and no
expression can contain one, so an expression ends at the first token that cannot continue it. A
function ends at the `}` that closes its `impl` block.

### Precedence

From tightest to loosest. Operators combine without parentheses only under these precedences,
which every mainstream language shares (P1); everything else requires parentheses.

| Level | Operators | Associativity |
|---|---|---|
| 1 | unary `-`, unary `!` | prefix |
| 2 | `*` `/` `%` | left |
| 3 | `+` `-` | left |
| 4 | `==` `!=` `<` `<=` `>` `>=` | none: `a < b < c` and `a < b == c` are `chained-comparison` |
| 5 | `&&` | left |
| 6 | `\|\|` | left |

Two rules the table does not show:

- `!` applies to a primary, and a `!` expression is never a direct operand of a comparison or an
  arithmetic operator: `!a == b` reads `(!a) == b` in C and `not (a == b)` in Python, so it is
  `ambiguous-not`; write `(!a) == b` or `!(a == b)`.
- Parentheses are kept in the AST as written, never added or removed by the formatter: in
  floating-point code, `(c0 + c1) + (c2 + c3)` and `c0 + c1 + (c2 + c3)` are the same tree, and
  the writer's grouping is what a reader checks against the `allow` list.

### Structure rules the parser checks

These are checked on the syntax alone, before any type is known; each failure has its kind in
section 7.

1. Clauses come in the order of the grammar: `eff`, `req`*, `ens`*, `spec`, `equiv`, `impl`;
   `eff`, `spec`, `equiv` and `impl` exactly once each.
2. `equiv` is followed directly by `allow(...)`, empty for exact equivalence. A permission appears
   at most once. The list is a set: any order is accepted, and the formatter prints it in the
   order of the permission table (`fp_reassoc`, `fp_contract`).
3. An effect appears at most once in `eff`.
4. Lists (`params`, `args`, `carry`, `allow`, `eff`) take no trailing comma.
5. The `impl` block ends with exactly one `return`, its last statement. A loop body holds `let`,
   `loop` and `next` statements, never `return`.
6. `next x` appears only at the top level of a loop body, and `x` is a carry of that loop. Every
   carry of a loop has exactly one `next` in its body.
7. A loop has at least one carry, and its `step` is a positive integer literal.
8. `result` appears only inside an `ens` clause.
9. An index `IDENT "[" expr "]"` applies to a name, and a call `IDENT "(" ... ")"` to a name; a
   member `VECTOR_TYPE "." IDENT` is followed by its arguments.
10. A bound appears only in a signature: on a scalar parameter, on the result, or on the element
    type of a reference. A binder in the impl or in a sum or quantifier takes an unbounded type; a
    loop, sum or quantifier index is already bounded by its range.
11. A bound is `<` or `<=` followed by an integer literal or a name. `>`, `>=`, `==` or an
    arithmetic expression after the type are rejected (section 7).
12. Nesting is at most 16 levels deep, counted as below.

### Nesting depth

The depth is counted on the tokens as written, so that a writer can compute it by reading its own
text, and it does not depend on how the parser is implemented (ADR 0010).

- A **level** is opened by each `(`, `[` and `{`, whatever it belongs to: a parameter list, a
  call's argument list (`fma(`, `f32x4.load(`: one level, its `(`), `carry(`, `allow(`, the binder
  list of a sum or a quantifier, a parenthesized expression or element type, an index, an extent,
  a block, the body of a sum or a quantifier. The level closes at the matching `)`, `]` or `}`.
- A level is also opened by each unary `-` and each `!`. It covers the operand and closes after
  it: `-x` opens one level, `-(x)` two, `!(!c)` three.
- Nothing else opens a level: no binary operator, however long its chain (`a + b + c`,
  `a + b * c`, `x && y && z`); no clause, statement, `loop` or `let`; no recursion of the parser's
  own, such as its precedence levels. Comments hold no tokens.
- The **depth** of a level is the number of levels open once it has opened, itself included. The
  depth is 0 between items. Levels that follow one another do not add up: in
  `(c0 + c1) + (c2 + c3)` both parentheses have the same depth.
- A delimiter that a repair inserts (section 7) opens or closes a level as if it were written,
  since recovery parses as if the repair had been applied; a deleted one does not.

**The limit is 16, provisionally** (ADR 0010). The first token, in source order, that opens a level
of depth 17 is a `nesting-limit` diagnostic (section 7), whose span is that token alone; the parse
of its function ends there.

`dot` (section 1) reaches depth 4, in its loads: `impl {` is 1, the loop body's `{` 2, `fma(` 3,
`f32x4.load(` 4, and the arguments `a` and `i + 4` sit at depth 4. Elsewhere it stays lower: the
signature reaches 2 (`dot(`, then `[n]`), the spec 2 (the sum's body `{`, then `a[`; the sum's `(`
has closed before it), the carries 3 (`impl {`, `carry(`, `f32x4.splat(`), and the `return` 3
(`impl {`, `reduce_sum(`, then either parenthesis).

## 4. Meaning the parser and the checker rely on

This section states what each construct means as far as parsing, naming and typing need. The
meaning of a floating-point sum (its order, start value and empty sum), which `sum` and
`reduce_sum` both depend on, and the obligation of `equiv allow(...)` are decided separately
(ADR 0006, Consequences); nothing here presumes them.

### Functions and parameters

- `fn` defines a function, `export fn` one that is also exported through the C ABI and WebAssembly
  (ADR 0007). A function without `export` is internal to its module: no symbol, no header entry.
- Parameters are read left to right. A reference's extent is an integer literal or the name of an
  **earlier** parameter of unsigned integer type: `n: u64, a: &f32[n] ...`. A size is an ordinary
  parameter, passed at runtime.
- `&T[e] @aN m` is a reference to `e` elements of the scalar type `T`, starting at an address
  aligned to `N` bytes, in mode `m`: `ro` (read only) or `wo` (write only). Alignment and mode are
  always written. `N` is a power of two at least the alignment of `T` (type check).

### Bounds

A bound on a single value is written in its type; `req` and `ens` are for the relations that are
not bounds (`n % 16 == 0`, `result >= a[0]`).

```
export fn pick(n: u64, k: u64 < n, a: &f32[n] @a4 ro) -> f32           // a bounded parameter
export fn find(n: u64, a: &u8[n] @a1 ro) -> u64 <= n                    // a bounded result
export fn gather(n: u64, m: u64, a: &f32[n] @a4 ro, idx: &(u64 < n)[m] @a8 ro) -> f32  // elements
```

(Signatures only; each function continues with its clauses.)

- **Form.** A bounded type is an integer scalar type followed by `<` or `<=` and an integer
  literal or a parameter name: `u64 < n`, `u8 <= 15`. An element type with a bound is
  parenthesized, `&(u8 < m)[n]`, so that the extent's brackets cannot be read as part of the bound.
  Only an upper bound is written: the subset's sizes and indices are unsigned, whose lower bound is
  already 0, and an index is safe by its upper bound. A lower bound (`n > 0`) is a relation, in
  `req`.
- **Names.** A parameter's bound, or its elements' bound, may name only parameters declared before
  it, to its left; the result's bound may name any parameter. The bound has the same type as the
  bounded value: `k: u64 < n` needs `n: u64`, and a literal bound takes the bounded type.
- **Meaning.** A parameter bound is a precondition, as `req k < n` would be. A result bound is a
  postcondition, proven of the impl's result like an `ens`. An element bound is a precondition on
  every element of the region. A value of a bounded type is usable wherever its base type is (the
  bound is forgotten, the bits are the same); a value of the base type goes where the bounded type
  is expected only through a proof that it satisfies the bound (at a `return`, at a call).
- **Storage.** The representation, and the C type at the boundary, are the base type's: `u8 < m` is
  stored and passed as a `u8`, whatever `m` is. A bound never narrows or widens a type.
- **One form.** A `req` or `ens` clause, or a top-level `&&` operand of one, that states what a
  bound would state (`req k < n` where `k` could carry `u64 < n`, `ens result <= n`,
  `req forall(j: u64 in 0..<m) { idx[j] < n }` where `idx` could be `&(u64 < n)[m]`) is rejected
  (`bound-in-clause`) with the bound moved into the type as repair.
- The result type is always written. Vector types may appear in the signature of an internal
  function, never in that of an exported one (ADR 0007).
- In the subset, a function with a `wo` parameter is parsed and its C mapping is defined, but the
  checker rejects it (`not-in-subset`) until the subset gains stores and a spec for written regions.

### Clauses

- `eff pure`: the function has no effect outside its parameters; with `ro` references it still
  reads their memory (`docs/design.md` gives the LLVM lowering).
- `req e`: a precondition, a `bool` expression over the parameters that is not a bound (bounds are
  in the types, above).
- `ens e`: a postcondition, a `bool` expression over the parameters and `result`, the value the
  function returns, that is not a bound on `result`. It is proven of the spec and of the impl's
  result; under a non-empty `allow` list the second is a proof of its own.
- `spec e`: the value the function computes, an expression over the parameters. It names nothing
  the impl defines (P3).
- `equiv allow(p, ...)`: the impl equals the spec up to the relaxations listed; `allow()` is exact
  equality.
- `impl { ... }`: the implementation.

### Statements

- `let x: T = e` defines `x` once (SSA).
- `loop i: T IN lo..<hi step s carry(c: T = init, ...) { body }` runs the body for
  `i = lo, lo + s, lo + 2s, …` while `i < hi`. The range is half-open: `hi` is never taken. These
  indices are meant over mathematical integers: the index never wraps, however close `hi` is to
  the largest value of `T`, and the loop runs at most `ceil((hi - lo) / s)` times (ADR 0008 gives
  the lowering that guarantees it). The index type `T` is an unsigned integer type, `u8` to `u64`,
  as is the index type of a sum or a quantifier; signed indices wait for the subset's extension.
  The step `s` is a literal of type `T` and must fit it: `loop i: u8 in 0..<n step 300` is an
  error. Inside the body, each carry `c` denotes its value at the start of the iteration, `init`
  on the first one. `next c = e` gives its value for the next iteration; `e` and every other
  expression of the body see the start-of-iteration values, whatever the order of the `next`
  statements. After the loop, `c` denotes its value after the last iteration (`init` if the body
  never runs). The index `i` is visible only in the body.
- `return e` gives the function's result.

### Names

Every name is defined once in its scope and never shadows another (P3): parameters are visible in
every clause; a `sum` binder is visible in its body; in the impl, a `let` is visible from its
definition to the end of its block, a loop index in its body, a carry in its loop body and after
its loop until the end of the enclosing block. The spec and the impl are separate scopes, so both
may use `i`. A name may not be one of the builtin operations below (`builtin-name`).

### Expressions and operations

- Arithmetic and comparisons are infix, with the same spelling in the spec and the impl (P2). On
  vector operands they apply lane by lane; scalar and vector operands never mix (no implicit
  `splat`).
- `a[i]` reads element `i` of the reference `a`.
- `SUM(i: T IN lo..<hi) { e }` is the sum of `e` over the range, a primary expression: its body is
  delimited by the braces, so `sum(...) { e } + 1` adds `1` to the sum.
- `FORALL(i: T IN lo..<hi) { c }` is `true` when the `bool` expression `c` holds for every `i` of
  the range (`true` on an empty range), a primary expression of type `bool`, usable in `req`, `ens`
  and `spec`: `req forall(i: u64 in 0..<n) { b[i] != 0 }`. Being over a finite range, it is
  executable (P9).
- In the subset, `/` and `%` apply to unsigned integers and floating-point values; on signed
  integers they wait for a decision on their rounding.

Builtin operations. The spelling fixes the operation; a vector type before `.` names the result
type when the operands do not fix it (P3):

| Operation | Meaning | Spelled as in |
|---|---|---|
| `fma(x, y, z)` | `x * y + z` with a single rounding; lane by lane on vectors | C99 `fma`, IEEE 754 fusedMultiplyAdd, LLVM `llvm.fma`, x86 `_mm_fmadd_ps`: the addend is last |
| `T.splat(x)` | a vector of type `T` with every lane `x` | Rust `f32x4::splat`, WebAssembly `f32x4.splat` |
| `T.load(a, i)` | the vector of type `T` whose lanes are `a[i]`, …, `a[i + lanes - 1]`; no alignment is required of the address | the unaligned loads: x86 `_mm_loadu_ps`, Highway `LoadU`, WebAssembly `v128.load` |
| `reduce_sum(v)` | the sum of the lanes of `v`, in an order this grammar does not fix (below) | Rust `std::simd` `SimdFloat::reduce_sum`, Highway `ReduceSum`; xsimd and AVX-512 spell it `reduce_add` |

`fma` takes the addend last, as C, LLVM and x86 do, not first as NEON's `vfmaq_f32` does.

`load` is not an aligned load: what an aligned load would promise is a separate fact, which codegen
emits only where the proof establishes it (ADR 0008).

The order in which `reduce_sum` adds the lanes of a floating-point vector, and its start value, are
part of the meaning of the floating-point sum, decided with the `sum` and the equivalence
obligation, not here. The references disagree: Rust's `reduce_sum`, Highway's `ReduceSum`, NEON's
`vaddvq_f32` and Java's `reduceLanes` leave it unspecified, Zig's `@reduce(.Add, v)` is sequential,
and LLVM's `llvm.vector.reduce.fadd` is sequential from a start value unless it carries `reassoc`.

## 5. Canonical form

The formatter is the printer of the AST (P2): idempotent, and parsing its output yields the same
AST. `lemma check` accepts any layout; the canonical form is what is stored and shown back.

- **Lines and indentation.** The `fn` line (with `export`) starts at column 0. One clause per line,
  indented two spaces. A block's `{` ends the line that opens it; its statements are one per line,
  two spaces deeper than that line; its `}` is on a line of its own at that line's indentation. So
  in an `impl` block, statements are at four spaces, and in a loop body two deeper than the `loop`.
- **Empty lines and line ends.** One empty line separates two items, and the last item from the
  comments that follow it (below); there is no other empty line: none at the start of the file,
  between clauses, statements or carries, or at the end. The comments attached to an item belong to
  it, so the empty line goes before them, never between them and the `fn` line. Every line ends
  with LF, the last one included: the output ends with exactly one LF, and a file with no item and
  no comment formats to the empty text.
- **Sum and quantifier bodies.** The body of a `sum` or a `forall` is an expression, not a block of
  statements, and stays on its line: `{ e }`, with one space inside each brace, as in dot's
  `sum(i: u64 in 0..<n) { a[i] * b[i] }`. The two rules above for blocks apply to the statement
  blocks of `impl` and `loop` only, and braces are not parentheses or brackets.
- **Carries.** A loop with one carry keeps it on the loop line: `carry(c: T = e) {`. A loop with
  more than one ends its line with `carry(`, puts each carry on its own line two spaces deeper than
  the `loop` line, each but the last followed by a comma, and closes with `) {` on its own line at
  the `loop` line's indentation (section 1).
- **Spaces.** No space between a keyword or a name and the `(` or `[` that follows it: `fn dot(`,
  `allow(`, `carry(`, `sum(`, `forall(`, `fma(`, `f32x4.load(`, `a[i]`, `&(u8 < m)[n]`. One space
  around binary operators (`+`, `*`, `==`, `&&`, …), around `=` and `->`, after `,` and `:`, and
  before every `{` and every `@aN`. No space inside parentheses or brackets, around `.` and `..<`,
  or after a unary operator: `f32x4.splat(0.0)`, `0..<n`, `-x`, `!c`.
- No column alignment anywhere.
- Parentheses as written; permissions in table order.
- A comment is attached to the item, clause or statement it precedes, or, when it follows code on
  the same line, to that line's construct; it is printed in the same place, preceded by one space
  when it trails code. The bullets below settle the places that the canonical form's lines cannot
  keep. Comments printed at the same place keep their source order.
- **Trailing comments.** A comment trails code when it follows, on the same line of the input, the
  last token of a line of the canonical form: `req n % 16 == 0 // why`, a carry and its comma,
  `impl { // why`. It stays at the end of that line.
- **Comments inside a line.** A comment between two tokens that the canonical form prints on the
  same line cannot stay there, whether it trailed code or stood on its own line in the input: it
  leaves that line, and is placed as a comment on its own line just before it (next bullet). In a
  signature written over several lines, `n: u64, // count` moves above the `fn` line, at column 0,
  after the item's other comments; a comment inside a `req` expression moves above the `req` line,
  at two spaces; one inside a single carry, `carry(c: T = e) {`, moves above the `loop` line.
- **Comments on their own line.** A comment on its own line before a line of the canonical form is
  printed on its own line just before it, at its indentation: before an item at column 0, a clause
  at two spaces, a statement or a carry at its own indentation. Before a closing `}`, or before the
  `) {` that closes a carry list printed over several lines, it stays before it, two spaces deeper
  than that line: at the indentation of the block's statements, or of the carries. Comments after
  the last item stay after it, at column 0, separated from it by one empty line.
- **Comment text.** A comment is printed as written, `//` included (`///` stays `///`, and no space
  is added after `//`), minus the spaces, tabs and CR at its end. A comment runs to the line feed
  (section 2), so the CR of a CRLF line end belongs to its text and is dropped with it; the other
  CRs of a CRLF input are whitespace. A CRLF input therefore formats to LF line ends only.

The formatter formats only a text with no syntax diagnostic. When the lexer or the parser reports
any diagnostic of section 7, with a repair or without, the formatter returns those diagnostics and
no text: it never formats around an error. Printing the recovered AST would apply the repairs, which
the toolchain offers and never applies ([`docs/diagnostics.md`](diagnostics.md), section 3), and
would drop every function an unrepaired diagnostic ended. The checker's and the verifier's
diagnostics do not concern it: a text that parses without diagnostic formats, whether or not it
type-checks.

## 6. What the verifier and codegen receive

The obligations each construct generates, the hypotheses they are proven under, and the LLVM facts
it allows, are listed in ADR 0008. In short: every index and every `load` carries a bound
obligation, every integer operation written in the code a no-overflow obligation, every `ens` a
proof, the spec a definedness proof; a loop's own increment carries none, since its lowering cannot
wrap. They are proven under the `req` clauses and bounds, the loop facts, and the reference facts
the C boundary checks (each reference's alignment, its extent fitting the address space,
disjointness from written regions). The only facts the grammar adds are load alignments computed
from proven index facts. A bound adds a precondition (a
parameter or its elements), assumed in the body and checked at the C boundary, or a postcondition
(the result), proven of the spec and the impl.

## 7. Syntax diagnostics

The closed list of kinds the lexer and the parser emit. All belong to ADR 0004's **syntax** class.
A repair is a list of edits applied together, most often one
([`docs/diagnostics.md`](diagnostics.md), section 3). It is offered only when exactly one repair
makes the text what the writer evidently meant; on the contract, only when the repair as a whole
changes no meaning (a spelling, a delimiter, a bound moved to where it means the same). Kinds whose
repair depends on a condition give it. The record these kinds are reported in, its two renderings,
and the kinds of the verifier and the toolchain are in [`docs/diagnostics.md`](diagnostics.md).

**Recovery.** A diagnostic with a repair is reported, and parsing continues as if the repair had
been applied, so independent slips are all reported in one round. The first diagnostic without a
repair ends the parse of its function; parsing resumes at the next `fn` or `export`, which cannot
occur inside a function. So a function yields at most one diagnostic without repair, and none that
only follows from an earlier one (P5).

| Kind | Raised by | Repair |
|---|---|---|
| `invalid-encoding` | bytes that are not UTF-8 | none |
| `invalid-character` | an ASCII character that begins no token (`$`, `` ` ``, `'`, `"`, `?`, `\`, a control character), `@` not followed by `a` and digits | none |
| `non-ascii` | a non-ASCII code point outside a comment that is neither an active C1 symbol nor a lookalike repaired at that position | none |
| `lookalike` | a code point of the lookalike table | per the table |
| `c1-other-variant` | the inactive C1 variant's spelling: `Σ`, `∈` or `∀` under A, `sum`, `in` or `forall` under U | the active spelling |
| `semicolon` | `;` | when it is the first token of its line, replace it, and any `;` directly after it, by `//`; otherwise delete it when the rest of its line is empty or begins with `}` or a keyword; otherwise none |
| `block-comment` | `/*` | `//` and the comment's text, when the comment closes on the same line and nothing follows it; otherwise none |
| `hash-comment` | `#` | `//`, when `#` is the first character of its line and is not followed by `[`; otherwise none |
| `invalid-number` | a malformed literal | `.5` → `0.5`; `5.` → `5.0`; `1e5` → `1.0e5`; `e+5` → `e5`; `1_000` → `1000`; a type suffix (`0.0f`, `16u64`, `1.0_f32`) deleted; none for a leading zero (`007`, octal in C) or a hexadecimal literal |
| `invalid-identifier` | an uppercase letter or a leading `_` in a word | none |
| `reserved-word` | a keyword or reserved type name where a name is expected | none |
| `unsupported-operator` | `++`, `--`, `**`, a compound assignment (`+=` …), `&` as a binary operator, `\|`, `^`, `~`, `<<`, `>>` | none |
| `assignment-in-expression` | `=` inside an expression (`req n % 16 = 0`) | `==` |
| `path-separator` | `::` | `.` |
| `range-operator` | `..`, `...`, `..=`, `…`: inclusive in some languages, exclusive in others | none: the range is `lo..<hi`, and the writer must say which bound was meant |
| `unexpected-token` | any other token the grammar does not allow there; the message lists the expected tokens ([`docs/diagnostics.md`](diagnostics.md), section 7) | none |
| `unclosed-delimiter` | a `(` or `[` still open at a token that cannot occur inside it (a clause or statement keyword, `fn`, `export`, end of input); a `{` still open at `fn`, `export` or end of input | `)` or `]` inserted before that token, when exactly one delimiter is open; none for `{` |
| `unmatched-delimiter` | a closing delimiter with no opener, or the wrong closer | none |
| `trailing-comma` | a comma that ends a list: before `)`, or after the last effect of an `eff` list | delete it |
| `foreign-keyword` | another language's word where a Lemma keyword is expected | `ret` → `return`; `function`, `func`, `fun`, `def` → `fn`; `requires` → `req`; `ensures` → `ens`; `effects` → `eff`; `var`, `val`, `const` before `name:` → `let`; none for `mut`, `pub`, `extern`, `for`, `while`, `if`, `else`, `match`, `break`, `continue` |
| `assignment` | a statement that begins with a name (`c0 = …`) | none: a value is defined once, by `let`, a carry or `next` |
| `clause-order` | clauses out of the grammar's order | the clauses reordered |
| `missing-clause` | no `eff`, `spec`, `equiv` or `impl` | none |
| `duplicate-clause` | a second `eff`, `spec`, `equiv` or `impl` | none |
| `equiv-without-allow` | `equiv` not followed by `allow(` | `allow()` inserted, as ADR 0006 states |
| `equiv-relation` | tokens between `equiv` and `allow` (`equiv spec ≈ impl allow(…)`) | delete them |
| `duplicate-permission` | a permission listed twice | delete the second |
| `duplicate-effect` | an effect listed twice | delete the second |
| `chained-comparison` | two comparison operators in one operand chain | none |
| `ambiguous-not` | a `!` expression as a direct operand of a comparison or arithmetic operator | none |
| `missing-type` | a parameter, `let`, carry, loop index, sum or quantifier binder without `: T` | none: the annotation is the writer's statement, checked against the code (P4) |
| `bound-operator` | `>`, `>=` or `==` after a type in a signature (`n: u64 > 0`) | none: an upper bound is `<` or `<=`, and a lower bound is a `req` |
| `bound-expression` | a bound that is not an integer literal or a name (`k: u64 < n - 1`) | none |
| `bound-on-binder` | a bound on a `let`, carry, loop index, sum or quantifier binder | delete it on an index binder, which its range bounds; none elsewhere |
| `unparenthesized-element-bound` | a bound on a reference's element type without parentheses (`&u8 < m[n]`) | the parentheses added: `&(u8 < m)[n]` |
| `missing-alignment` | a reference without `@aN` | none |
| `missing-mode` | a reference without `ro` or `wo` | none |
| `missing-step` | a loop without `step` | none |
| `invalid-step` | a `step` that is not a positive integer literal | none |
| `missing-carry` | a loop without carries | none |
| `carry-without-next` | a carry with no `next` in its loop body | none |
| `duplicate-next` | a second `next` for the same carry | none |
| `misplaced-next` | `next` outside a loop body, nested deeper than its top level, or naming no carry of that loop | none |
| `missing-return` | an `impl` block that does not end with `return` | none |
| `misplaced-return` | `return` inside a loop body, or followed by a statement | none |
| `result-outside-ens` | `result` outside an `ens` clause | none |
| `misplaced-export` | `export` not followed by `fn` | none |
| `unexpected-end` | input ends inside a function | none |
| `nesting-limit` | a token that opens a level of depth 17, counted as in section 3, "Nesting depth"; the span is that token | none: lowering the depth means binding an inner expression with `let`, under a name the compiler would have to invent |

### Kinds of the checker that concern names and bounds

Raised by the checker, not the parser; they belong to ADR 0004's **type or layout** class and are
listed here because they concern names and bounds the grammar defines. None changes the contract
automatically (ADR 0006): `bound-in-clause` only moves a bound to the place where it means the same.

| Kind | Raised by | Repair |
|---|---|---|
| `unknown-name` | a name defined nowhere in scope | `hsum` → `reduce_sum`; otherwise none (for `ld`, the message names `T.load(a, i)`) |
| `duplicate-name` | a name defined twice in one scope, or shadowing an enclosing one | none |
| `builtin-name` | a definition named `fma` or `reduce_sum` | none |
| `unknown-member` | `T.m` where `m` is neither `splat` nor `load` | none |
| `bound-order` | a parameter's bound, or its elements' bound, naming a parameter declared at or after it, or a name that is not a parameter | none: moving a parameter changes the C signature |
| `bound-type-mismatch` | a bound whose type differs from the bounded value's (`k: u32 < n` with `n: u64`), or a literal bound that does not fit it | none applied: the message names both edits that would make the types agree (the bounded type, or the bound parameter's type), marked as contract changes |
| `bound-on-non-integer` | a bound on a floating-point, `bool` or vector type | none |
| `bound-in-clause` | a `req` or `ens` clause, or a top-level `&&` operand of one, stating a bound the type could carry | the bound moved into the type, and the clause or operand removed |
| `index-not-unsigned` | a loop, sum or quantifier index whose type is not `u8`, `u16`, `u32` or `u64` | for a signed type, the unsigned type of the same width (`i64` → `u64`), when every bound of the range is a non-negative literal or a name of that unsigned type; otherwise none |
| `literal-out-of-range` | a literal whose value does not fit the type its context gives it, a loop's `step` included (`step 300` on a `u8` index) | none |
| `not-in-subset` | a construct the grammar parses and the subset does not implement (a `wo` parameter, a signed `/` or `%`, a reserved type outside the subset) | none |

### Calibration switch

The C1 variant is a switch of the lexer: one variant active per run, the other's spellings
rejected with `c1-other-variant` and its repair, during the calibration and, for the losing
variant, after the freeze. No other choice of this grammar is calibrated. The nesting limit is not a
calibration arm: its value is revisited by the ADR that freezes the grammar, from the number of
`nesting-limit` records in the calibration runs (ADR 0010).
