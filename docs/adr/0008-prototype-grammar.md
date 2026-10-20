# 8. Grammar of the prototype subset

- Status: accepted
- Date: 2026-10-20

## Context

ADR 0006 sets the criteria every surface of Lemma is chosen by (P1 to P9) and lists what they
already decide for the grammar of the prototype subset: `//` comments with `;` rejected, blocks and
clauses delimited by tokens, infix arithmetic under the shared precedences, `fma`, `splat`, `load`
and a horizontal sum named as SIMD APIs name it, types visible at definition, ASCII identifiers and
reserved keywords, `equiv allow(...)` required, both variants of choice C1, and syntax diagnostics
tied to ADR 0004's failure classes. It leaves the grammar to be written, and a later ADR to freeze it
once its calibration has run.

Writing the grammar meets choices those consequences do not settle. Left to the parser, each would
be decided silently in code. The grammar itself is [`docs/grammar.md`](../grammar.md); this ADR
records the choices it makes beyond ADR 0006, each with the alternative and the reason.

## Decision

`docs/grammar.md` is the grammar of the prototype subset, provisional until the ADR that records
ADR 0006's calibration freezes it. Its parser carries both C1 variants behind a switch, **variant A
by default** (ADR 0006's default on a tie) until then. It adds **no choice to the calibration**: each
choice below is settled by the principles, and each added choice would double the calibration's arms.

### Shape of a function

1. **The contract is a prefix of the function.** Clauses come in a fixed order, `eff`, `req`*,
   `ens`*, `spec`, `equiv`, then `impl { ... }`, last. The earlier example put `equiv` after the
   impl. The order chosen is the order in which ADR 0006 lists the contract, it is the shape of
   Dafny and Verus (clauses between the signature and the body), and it makes the contract
   emitted for the spec review (P8) exactly the text before `impl`.
2. **Clauses and statements need no separator.** Each begins with a keyword and no expression
   contains one, so an expression ends where it cannot continue. `;` is no token at all, which is
   what lets ADR 0006 reject it outright. Blocks (`impl`, loop bodies, sum bodies) are braces. The
   function ends with its `impl` block, so it needs no braces of its own.
3. **Sizes are ordinary parameters.** `fn dot(n: u64, a: &f32[n] @a128 ro, ...)`, where a later
   parameter's type names an earlier one, as C99 writes `void f(size_t n, float a[n])`. The earlier
   `fn dot[n:u64] (...)` reads like a compile-time generic parameter in Go, Scala or Rust, while `n`
   is a runtime value: a false friend (P1). It also keeps `[...]` free for the compile-time
   parameters that the tuned accumulator count and vector width need (`docs/design.md`), and the C
   parameters are then the Lemma parameters in order (ADR 0007).
4. **Every binder carries its type**: parameters, `let`, carries, loop indices, sum binders. P3 asks
   that the type of a value be visible where it is defined; one rule for every binder leaves no
   optional annotation (P2), and the compiler checks each against the code (P4).

### Spellings

5. **Half-open ranges are `lo..<hi`.** `0..n` excludes `n` in Rust and Zig and includes it in
   Kotlin, Ruby, Haskell, Ada, Pascal and Bash: a false friend, and in a spec an off-by-one that
   verifies. `..<` is half-open wherever it exists (Swift, Kotlin, Odin). `..`, `...`, `..=` and `…`
   are rejected without repair, since the compiler cannot know which bound was meant.
6. **`return`, not `ret`.** `ret` is the spelling of LLVM IR and assembly; P1 asks that the impl
   read like source. `ret` is rejected with `return` as repair.
7. **`fma(x, y, z)` is `x * y + z`.** C99, IEEE 754's fusedMultiplyAdd, LLVM's `llvm.fma` and x86's
   `_mm_fmadd_ps` take the addend last; NEON's `vfmaq_f32` takes it first, as the earlier example
   did. The frequent order wins (P1). Writing the NEON order is not silent: the equivalence proof
   fails.
8. **An operation whose operands do not fix its result type is prefixed by that type**:
   `f32x4.splat(0.0)`, `f32x4.load(a, i)`. This is WebAssembly's `f32x4.splat` and Rust's
   `f32x4::splat`; `.` is the member separator of most mainstream languages, and `::` is rejected
   with `.` as repair. `load` takes the reference and the index, not an element expression (`a[i]`
   is a value, and using it as an address would be C's `&a[i]` without the `&`). `load` is an
   unaligned load, like x86's `_mm_loadu_ps`, Highway's `LoadU` and WebAssembly's `v128.load`, not
   the aligned `_mm_load_ps` or Highway `Load`: the alignment of an access is a separate fact,
   emitted only where it is proven (below).
9. **The horizontal sum is `reduce_sum(v)`**, as in Rust's `std::simd` (`SimdFloat::reduce_sum`) and
   Highway (`ReduceSum`); x86's AVX-512 `_mm512_reduce_add_ps`, LLVM's `llvm.vector.reduce.fadd` and
   xsimd's `reduce_add` share the `reduce_` stem, and NEON's `vaddvq_f32` is a coined
   abbreviation. `reduce_sum` names the sum the spec writes and matches the lane type names
   `f32x4`, which are Rust's. `hsum` is repaired to `reduce_sum`. Its order and start value are not
   fixed here: they belong to the meaning of the floating-point sum, decided with the equivalence
   obligation. The references disagree, so none can be borrowed: Rust, Highway, NEON and Java
   leave the order unspecified, Zig's `@reduce` is sequential, and `llvm.vector.reduce.fadd` is
   sequential from a start value unless it carries `reassoc`.
10. **A sum's body is in braces**: `sum(i: u64 in 0..<n) { a[i] * b[i] }`. Whether `Σ` binds
    tighter than a following `+` is a convention mathematical texts disagree on; braces delimit it
    by a token (P6) and make the sum a primary expression.
11. **`loop` is kept, not `for`.** Its body defines the next values of its carries instead of
    mutating variables, a fold with no frequent mainstream spelling. `for` would invite mutation
    habits; Rust's `loop` means an unbounded loop, but `loop` followed by an index and a range
    cannot be read that way, and a bare `loop {` is a syntax error, not a silent one.

### Expressions

12. **Precedence is the shared table only**: unary, then `* / %`, then `+ -`, then comparisons, then
    `&&`, then `||`. `&&` over `||` is shared by every mainstream language, so it needs no
    parentheses. Comparisons do not chain (`a < b < c` is a chain in Python, a comparison of a
    boolean in C). `!a == b` is `(!a) == b` in C and `not (a == b)` in Python, so a `!` expression
    is never a direct operand of a comparison or an arithmetic operator. Everything outside the
    table requires parentheses (P1).
13. **Parentheses are kept in the AST**, printed as written. Rejecting redundant ones would reject
    `(c0 + c1) + (c2 + c3)`, the clearest way to write a reduction tree; dropping them would hide
    the writer's grouping, which is what a reader checks a floating-point impl against. The
    content hash therefore covers them (ADR 0007).
14. **The `allow` list is a set.** Any order is accepted and printed in the permission table's order:
    no order is wrong, so none is rejected. A permission listed twice is rejected with a repair.
15. **A literal takes its type from its context**, as in Rust: no suffixes, no implicit conversion
    (`0` is never an `f32`), and a float literal rounds to nearest, ties to even, in its type.

### Lexical rules

16. **Identifiers are lowercase**, `[a-z][a-z0-9_]*`, so that the C constant names derived from them
    (`LINALG_DOT_ERR_NULL_A`) stay distinct (ADR 0007).
17. **Reserved words**: the structural keywords, `result`, `true`, `false`, the modes `ro` and `wo`,
    the effect `pure`, the permissions, `bool`, and C1's `sum`, `in` and `forall` in every variant,
    so that all calibration arms reserve the same words. Type names are reserved by pattern for
    the widths that exist (`u8` to `u128`, `f16` to `f64`, and their vectors), which leaves `i0`,
    `i1` and `f2` free as names. Builtin operations (`fma`, `reduce_sum`) are not keywords; a
    definition may not take their names.
18. **Lookalikes and other languages' habits are rejected by name**, each with its kind and, where
    the intent is certain, its repair: the confusable code points of `Σ`, `∈`, `∀`, comparisons,
    arithmetic and spaces; `requires`, `ensures`, `effects`, `ret`, `function`, `def`; `/*`, `#`;
    `++`, `+=`, `::`. The list is in `docs/grammar.md`.

### Bounds and diagnostics

19. **A bound on a single value is written in its type**, and `req` and `ens` are for the relations
    that are not bounds (`n % 16 == 0`, `result >= a[0]`). Types carry bounds (README), and P3 puts
    a value's type, bound included, where the value is defined. A bound and a clause stating the
    same fact would be two forms (P2), so a clause that only restates a bound the type could carry
    is rejected, with the bound moved into the type as repair (`bound-in-clause`). The form:
    - **Where**: a scalar parameter (`k: u64 < n`), the result (`-> u64 <= n`), the element type of
      a reference, parenthesized (`&(u8 < m)[n]`), since without parentheses `&u8 < m[n]` reads as
      a bound `m[n]`. Not on impl binders: a loop, sum or quantifier index is bounded by its range,
      and a bounded `let` or carry would be a loop invariant, which the subset does not prove.
    - **Operators**: `<` and `<=`, an upper bound only. The subset's sizes and indices are unsigned,
      whose lower bound is already 0, and an index is made safe by its upper bound; a two-sided form
      would be a range type, a construct of its own. A lower bound (`n > 0`) stays a `req`.
    - **Bound expression**: an integer literal or a parameter name, no arithmetic. The C wrapper then
      checks it with one comparison, and no bound expression can itself overflow at the boundary,
      where no proof covers the caller.
    - **Names and types**: a parameter's bound names only parameters to its left (P3: read left to
      right, as extents are); the result's bound may name any parameter. The bound has the bounded
      value's type, with no conversion (`bound-order`, `bound-type-mismatch`). The second carries no
      automatic repair: both possible edits change the contract, so the message names them, marked
      as contract changes, as ADR 0006 requires.
    - **Meaning**: a parameter bound is a precondition, the same obligation as `req k < n`; a result
      bound a postcondition, proven of the spec and of the impl's result like an `ens`; an element
      bound a precondition on every element of a `ro` region.
    - **Storage**: the base type's. A bound never changes a value's width or its C type.
20. **Recovery continues through repairs.** A diagnostic with a repair is reported and the parse
    continues as if it were applied; the first one without a repair ends its function's parse, and
    parsing resumes at the next `fn` or `export`. One round then reports every independent slip
    the compiler can fix with certainty, and never a diagnostic that only follows from another
    (P5).

### Contract keywords and the quantifier

21. **The quantifier follows C1.** `forall(i: T in lo..<hi) { c }` under variant A, `∀(...)` under
    variant U, with the same switch as `sum` and `in`: a contract that writes `sum` and `in` writes
    `forall`, and one that writes `Σ` and `∈` writes `∀`. It is a `bool` primary over a finite
    range, usable in `req`, `ens` and `spec`, and executable (P9). Its body is in braces, like a
    sum's. This extends C1 from two symbols to three without adding an arm to the calibration;
    ADR 0006 carries the amendment.
22. **The contract keywords stay `req`, `ens` and `eff`.** P1 was weighed: `requires` and
    `ensures` are the spelling of Dafny, Verus, ACSL, JML and Why3, and P1 does not use coined
    abbreviations. Against it, the short forms match the language's other keywords (`fn`, `spec`,
    `impl`, `equiv`, `let`). A spelling probe run outside this repository (lemma-bench
    spelling-probe, 2026-10-19: four models, with a reference card and with examples only), a
    research input and not a calibration, found no measurable difference in errors or in tokens
    between `req` and `requires`, `ens` and `ensures`, `eff` and `effects`. So the consistency with
    the other keywords decides. `requires`, `ensures` and `effects` stay rejected with `req`, `ens`
    and `eff` as repairs, so P1's metric keeps counting how often models reach for them.

### What the verifier must prove

For the proof engineer. Every obligation of a function is proven under these hypotheses, and
only these:

- **The reference facts**, for each reference parameter `a: &T[e] @aN m`, which are exactly what
  ADR 0005's checked variant establishes before the body runs, and what the `_unchecked` variant's
  contract states:
  - *alignment* (check 2): the address of `a` is a multiple of `N`;
  - *extent* (check 3): `e * size_of(T)` and `address(a) + e * size_of(T)` are below `2^P`, `P` the
    target's pointer width (64, or 32 on wasm32 and Cortex-M), so every byte offset into the region
    fits a pointer; this hypothesis is target-dependent, like the obligations it discharges;
  - *disjointness* (check 4): every pair of references with one side `wo` is disjoint;
  - *live region* (documented, not checked): `a` designates `e` readable (`ro`) or writable (`wo`)
    elements for the whole call. This one is an assumption of the memory model, not an SMT
    hypothesis.
- **The preconditions**: the `req` clauses (checks 5 and 7), the parameter bounds (check 5) and the
  element bounds (check 7), which make every read `a[i]` of a bounded region satisfy the bound.
- **The floating-point model** of ADR 0003 (check 6).
- **The facts of the enclosing loops, sums and quantifiers**: `lo <= i < hi` and
  `(i - lo) % step == 0`.

A hypothesis outside this list is not assumed, and an obligation that needs one is not provable
(in `dot`, the byte offset `4 * (i + 16)` of the last load fits 64 bits only under the extent
hypothesis: without it, `n = 2^64 - 16` is a counterexample).

The obligations each construct generates:

- `a[i]`: `i < e`, the extent of `a`, in the spec as in the impl.
- A bounded result: the bound holds of the spec and of the value of every `return`, as an `ens`.
- A bound expression: well typed only (a literal or a parameter, nothing to evaluate).
- `T.load(a, i)`: `i + lanes <= e`, stated over mathematical integers so that a wrapped sum cannot
  pass it. "Over mathematical integers" states what is proven, not how it is encoded (open points).
- Integer `+`, `-`, `*`: no overflow or underflow in the operand type. `/` and `%`: a nonzero
  divisor. Overflow is opt-in, and the subset has no way to opt in.
- The spec: defined on every input satisfying `req` (every index in bounds, every integer operation
  in range), since P9 evaluates it.
- Each `ens`: holds with `result` replaced by the spec, and with `result` replaced by the impl's
  result. Under `allow()`, the second follows from the first and the equivalence; under a non-empty
  `allow` list it is a proof of its own, since callers assume `ens` of the value actually returned.
- `equiv allow(...)`: the equivalence under the granted permissions, whose statement is decided
  separately.
- Per target (ADR 0003): vector widths, `@aN` honored, index and extent arithmetic within the
  pointer width, under the extent hypothesis above.
- A loop: no obligation on its implicit increment, because of how it is lowered (below), and none
  on termination.

**Loops cannot wrap, by their lowering.** A loop `loop i: T in lo..<hi step s` means the indices
`lo + k * s` below `hi`, over mathematical integers (`docs/grammar.md`). Lowered naively, as
`i' = i + s` then a test `i' < hi`, it is wrong for a fixed-width index: with `hi = 2^64 - 8` and
`s = 16`, the last index is `2^64 - 16`, `i + 16` wraps to `0`, and the loop runs again over
memory its bound proofs only covered once. Two fixes were possible: an obligation that `i + s`
does not overflow, which every writer would have to discharge with a `req` on `hi`; or a lowering
that cannot wrap. The lowering is chosen, since the meaning already excludes the wrap and it costs
the writer nothing: the loop runs zero times when `lo >= hi`; otherwise, after each iteration it
continues only if `s < hi - i`, a subtraction that cannot underflow since `i < hi`, and only then
computes `i + s`, which is below `hi` and so cannot wrap. Loops therefore terminate after at most
`ceil((hi - lo) / s)` iterations: that, and not the step alone, is the basis of `willreturn`. Codegen
must use this lowering, or another one proven to compute the same indices; the auditor checks it.

The lowering is exact for **unsigned** indices only, so a loop, sum or quantifier index takes an
unsigned integer type, `u8` to `u64` (`index-not-unsigned`). For an `i64` index, the true distance
`hi - i` can exceed `2^63 - 1`; computed in `i64` it wraps negative, and a signed `s < hi - i`
stops the loop while indices below `hi` remain, where the verifier's model keeps going (with
`lo = -2^63`, `hi = 2^63 - 1`, `s = 1`, the loop stops after the first index). Signed indices,
with a comparison that is exact for them, are left to the subset's extension. The step is a
literal of the index type and must fit it (`literal-out-of-range`), since the test compares it
with `hi - i` in that type.

### What codegen may emit, and on what basis

ADR 0006 added no LLVM fact. This grammar adds one, the load alignment below, which rests on
ADR 0005's check 2 (the `@aN` alignment hypothesis); the other facts are ADR 0005's and
`docs/design.md`'s:

- **Load alignment, proven.** `T.load(a, i)` is emitted with the largest power-of-two alignment
  that divides both `N` (from `@aN`) and the byte offset of every index value the proof allows. In
  `dot`, `i` is a multiple of 16 and `a` is `@a128`, so the loads at `i`, `i + 4`, `i + 8` and
  `i + 12` are `align 64`, `16`, `32` and `16`. Without such a proof, the alignment of the element
  type. The fact rests on the alignment hypothesis (`@aN`, check 2).
- **Bounds, on ADR 0005's two bases.** A parameter or element bound becomes an assumption on the
  proven body (`llvm.assume`, or `!range` metadata on a load from a bounded region when the bound
  is a literal), checked on the path through the checked variant (checks 5 and 7) and a contract
  for `_unchecked` callers. A result bound is proven, and becomes a fact for Lemma callers only.
- `fma` lowers to `llvm.fma`, never `llvm.fmuladd`: `fmuladd` lets LLVM choose between one rounding
  and two, and the single rounding the proof assumes holds only with `llvm.fma`. An impl that
  writes `fma` proves equivalent to a spec of separate products and sums only under `fp_contract`.
- `reduce_sum` lowers to `llvm.vector.reduce.fadd`, in the order and from the start value the
  floating-point sum's definition fixes, with the `reassoc` flag only under `fp_reassoc` and only
  where the proof relies on it.
- `willreturn` from the bounded trip count of the loop lowering above.
- A function without `export` has internal linkage in the final binary.

## Consequences

- A parser, a printer and their tests can be written from `docs/grammar.md` alone: tokens, EBNF,
  precedence, structure rules, canonical layout, and the closed list of syntax kinds with their
  repairs and recovery.
- `examples/dot.lm`, the README's snippet and `docs/design.md` change spelling: `fn dot(n: u64, ...)`,
  `0..<n`, `f32x4.load`, `fma` with the addend last, `reduce_sum`, `return`, braces, typed binders,
  and `equiv` before `impl`. The meaning of `dot`'s contract does not change.
- The variable name `sum` is reserved in every arm; a model's habit of naming an accumulator `sum`
  costs a `reserved-word` diagnostic, which P7's metric counts.
- The quantifier rides on C1, so the calibration kernels must use it too: ADR 0006 requires them to
  use every C1 symbol, and a kernel with a precondition or a postcondition over a whole array does.
  Variant U then carries three non-ASCII code points, not two.
- `n > 0` and `n % 16 == 0` stay in `req`; `k < n`, `result <= n` and "every index below `n`" move
  into types. A writer must tell an upper bound on one value from any other relation, and the
  `bound-in-clause` repair teaches the line where a model draws it wrongly.
- The C header gains `ERR_BOUND_<P>` constants for bounded parameters (check 5) and bounded
  elements (check 7), and "Ensures" lines for bounded results (ADR 0007).
- Signed `/` and `%`, stores into `wo` regions and the spec of written regions, element bounds on
  `wo` regions, bounds on impl binders, calls between functions and the content hash's input for
  callees are left to the subset's extension, each by its own decision.
- The ADR that freezes the grammar after calibration supersedes this one.

## Open points

- For the verifier: obligations stated "over mathematical integers" are a statement of meaning,
  not an encoding. Encoded through `bv2nat` and integer arithmetic, `dot`'s load bounds came back
  `unknown` on every seed; zero-extending the fixed-width values to 128 bits proves them well
  within budget. An `unknown` is never a pass (ADR 0006: a kind of its own).
- For the verifier: the hypothesis list is a closed upper bound, not what every query carries; each
  query carries only the hypotheses its obligation uses, and an element bound is assumed at each
  read `a[i]`, not as a quantifier over the region.
- The calibration needs a kernel that uses the quantifier in a `req` or an `ens` (a condition over
  every element of an array), since ADR 0006, as amended, requires its kernels to use every C1
  spelling; none of its five candidates is sure to.
- Floating-point exactness: `equiv allow()` stays exact equality, the default. A correctly rounded
  sum, as a further option, belongs to the decisions on the meaning of the floating-point sum and on
  the equivalence obligation, not to the grammar.
