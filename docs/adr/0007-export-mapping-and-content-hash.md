# 7. Export marking, the C signature mapping and the content hash

- Status: accepted
- Date: 2026-10-20

## Context

ADR 0005 fixes the C ABI and WebAssembly boundary: two exports per function, symbol names prefixed
by the module, a status code, seven checks in a fixed order, and a generated header. It shows all of
this on one function, `dot`, and leaves open what the header generator needs to handle any other:

- how a function is marked exported, and whether a function can be left unexported;
- what a module name may be;
- the C type of each Lemma type at the boundary (sizes, `ro` and `wo` references, scalars, bounds,
  vectors), and the place of the result in each variant;
- how the status constants are ordered and named when a function has written regions, bounded
  values or several `req` clauses;
- what the header's "content hash" covers, and with which algorithm. ADR 0006 keys a function by its
  AST and says formatting never changes its hash.

The grammar of the prototype subset ([`docs/grammar.md`](../grammar.md), ADR 0008) is what this ADR
maps from. This ADR extends ADR 0005 and changes none of its decisions.

## Decision

### Marking

A function is exported by the keyword `export` before `fn`:

```
export fn dot(n: u64, a: &f32[n] @a128 ro, b: &f32[n] @a128 ro) -> f32
```

Zig spells a C ABI export the same way, and JavaScript and TypeScript spell a module export so. An
attribute (`#[export]`) costs more tokens and a grammar of its own; marking every function exported
would put every helper into the C namespace and the header.

A function without `export` is internal to its module: it is verified and tested like any other, and
the JIT compiles it, but it has no C or WebAssembly symbol and no header entry, and the final binary
gives it internal linkage. Exporting a function always produces both variants (ADR 0005).

### Module names

The module is the source file's stem (ADR 0005). A module that exports a function must have a stem
that is a Lemma identifier (`[a-z][a-z0-9_]*`); otherwise the module is refused
(`export-module-name`). Its header is `<module>.h`, its symbols `<module>_<function>` and
`<module>_<function>_unchecked`, its constants `<MODULE>_<FUNCTION>_…`, where the uppercase form
replaces each lowercase letter by its uppercase. Identifiers are lowercase (`docs/grammar.md`), so
this form is injective.

### Type mapping

| Lemma, in an exported signature | C | Note |
|---|---|---|
| `u8` `u16` `u32` `u64` | `uint8_t` `uint16_t` `uint32_t` `uint64_t` | exact width, never `size_t` (ADR 0005), also for sizes |
| `i8` `i16` `i32` `i64` | `int8_t` `int16_t` `int32_t` `int64_t` | |
| `f32` `f64` | `float` `double` | IEEE 754 binary32 and binary64 on every target of ADR 0003 |
| a bounded integer `T < b` or `T <= b`, as a parameter, a result or an element | the C type of `T` | the bound is a check or a guarantee (below), never a different type |
| `&T[e] @aN ro` | `const T_c *` | `T_c` the C type of `T` |
| `&T[e] @aN wo` | `T_c *` | |
| a reference that is one side of a pair required disjoint | the same, with `restrict` | ADR 0005: the pairs where one side is written |
| `@aN` | not in the prototype | C has no portable way to state a pointee's alignment; it is check 2 and its comment |
| a vector type, anywhere in the signature, element types included | refused: `export-vector-type` | C ABIs pass `float32x4_t`, `__m128` and `v128` differently, and a JavaScript host has no vector type |
| `bool`, and any type not in this table | refused: `export-unsupported-type` | |

Sizes are ordinary parameters (`n: u64`), so the C parameters are the Lemma parameters, in the same
order.

**Bounds.** A bound on a single value is written in its type (`docs/grammar.md`, ADR 0008):

- a bounded scalar parameter, `k: u64 < n`, is passed as its base type, `uint64_t k`, and its bound
  is checked by check 5 as `ERR_BOUND_<P>` (`LINALG_PICK_ERR_BOUND_K`, comment `k < n`);
- a bounded element type on a `ro` reference, `idx: &(u64 < n)[m] @a8 ro`, is passed as a pointer to
  its base type, `const uint64_t *idx`, and every element's bound is checked by check 7 as
  `ERR_BOUND_<P>`, O(m) reads, as ADR 0005 states;
- a bounded result, `-> u64 <= n`, is returned or written through `result` as its base type, and
  its bound is listed in the header as a guarantee, "Ensures: result <= n", proven, never checked.

The storage width and the C type always come from the base type, never from the bound: `u8 < m` is
a `uint8_t` whatever `m` is, so a header does not change type when a bound changes.

**The result.** For a function with result type `R`:

- the checked variant returns `int32_t` and takes one more parameter, last, `R_c *result`, written
  only on success (ADR 0005); `result` is a Lemma keyword, so no parameter can take that name;
- the unchecked variant returns `R_c`.

A function with no result (when the subset gains one) has a checked variant without `result` and an
unchecked variant returning `void`.

**WebAssembly** exports carry the same names and the same parameters, under the wasm32 C ABI
(pointers as `i32`, `u64` as `i64`).

### Status constants

The constants follow ADR 0005's checks, in its order; within a check, in the order given. Indices
start at 1 and increase by one across the whole list; `<MODULE>_<FUNCTION>_OK` is 0.

| Check | One constant per | Order | Name |
|---|---|---|---|
| 1 null | reference parameter, then `result` | parameter order | `ERR_NULL_<P>` |
| 2 alignment | reference parameter (`@aN`), then `result` (its type's alignment) | parameter order | `ERR_ALIGN_<P>` |
| 3 extent | reference parameter | parameter order | `ERR_EXTENT_<P>` |
| 4 disjointness | unordered pair of reference parameters where one side is `wo` | by first parameter, then second | `ERR_OVERLAP_<P>_<Q>` |
| 5 scalar preconditions | bounded scalar parameter, then `req` clause reading no memory | parameter order, then source order | `ERR_BOUND_<P>`, `ERR_REQ_<k>` |
| 6 floating-point environment | function in which a floating-point type appears (signature, spec or impl) | | `ERR_FP_ENV` |
| 7 memory contents | `ro` reference with a bounded element type, then `req` clause reading memory | parameter order, then source order | `ERR_BOUND_<P>`, `ERR_REQ_<k>` |

Within checks 5 and 7, bounds come before clauses because they belong to the signature, which
precedes the clauses; a parameter is either a scalar or a reference, so `ERR_BOUND_<P>` names one
constant.

`<k>` is the clause's 1-based position among **all** `req` clauses of the function, whichever check
it falls under, so a clause keeps its name when another one moves between checks 5 and 7. The set of
constants does not depend on the target: a check that is free on one target (check 6 on Wasm) keeps
its constant there, and the header is the same for every target.

A function's checks are also the preconditions of its `_unchecked` variant: the header's list of
constants doubles as that variant's contract, with the "Not checked" list.

### Refusals

Raised by the export checker, in ADR 0004's **type or layout** class, none with a repair:

| Kind | Raised by |
|---|---|
| `export-vector-type` | a vector type in an exported signature |
| `export-unsupported-type` | `bool` or another type outside the mapping table in an exported signature |
| `export-c-reserved-name` | a parameter of an exported function named like a C23 or C++23 keyword or alternative token (`int`, `float`, `restrict`, `new`, `class`, `and`, `or`, `not`, …): the header uses Lemma's parameter names |
| `export-symbol-collision` | two names the module's header or symbols would define alike, for instance an exported `dot_unchecked` beside an exported `dot` |
| `export-module-name` | a module with an exported function whose stem is not a Lemma identifier |

### The header

`<module>.h` holds an include guard `LEMMA_<MODULE>_H`, `#include <stdint.h>`, an `extern "C"` block
under `__cplusplus`, and then, for each exported function in source order, ADR 0005's content:

- a comment with the symbol, the spec, the result's bound and each `ens` clause as "Ensures:", and
  the content hash;
- the "Not checked" list;
- the status constants, each with what it checks and its cost;
- both prototypes.

Lemma text in the header (spec, bounds, `ens`, `req`) is the canonical printing of the AST
(`docs/grammar.md`, section 5), in which `*` and `/` are never adjacent, so `*/` never appears in
it, and with the C1 symbols in their ASCII spelling (`sum`, `in`, `forall`), so the header is
ASCII whatever variant the calibration selects.

### The content hash

**Coverage.** A function's content hash covers its definition from the `(` that opens its parameter
list to the `}` that closes its `impl` block: parameters with their names and types, result type,
every clause, the impl. It does not cover:

- comments and layout (ADR 0006: formatting never changes a hash);
- the function's name and the `export` marker. They say where the function is reachable, not what
  it computes or promises: renaming or exporting a function keeps its proofs (cached by hash,
  ADR 0003) and its JIT object (keyed by hash, ADR 0001). The header prints the name beside the
  hash; the export wrappers are separate objects, keyed by the hash and the export.

It covers parameter names, because the header derives constant names from them, and parentheses and
literal spellings as written, because the AST keeps them.

**Input.** The ASCII string `lemma-fn-v1`, then, for each token of the covered range **as the
canonical printer emits it** (`docs/grammar.md`, section 5), in order, one space (U+0020) followed
by the token's spelling: identifiers and literals as written, every other token in its only
spelling, the C1 symbols in their ASCII spelling. Comments are not tokens. The string is encoded in
UTF-8; with the ASCII C1 spelling it is ASCII. The tokens are those of the canonical printing, not
of the source: where the printer normalizes, as it orders an `allow` list, two sources with the
same AST give the same tokens. Since the printer is a function of the AST, every token of the subset
has one spelling once the C1 symbols are mapped, and parentheses and literals are kept in the AST,
this string is a function of the AST alone: formatting never changes it, and neither does the
calibration's choice of C1 variant.

**Algorithm.** SHA-256 (FIPS 180-4) of that string, written `sha256:` followed by the 64 lowercase
hexadecimal digits. SHA-256 is the most widely available standardized hash: any host language can
recompute it to check a header against a source. The `sha256:` and `lemma-fn-v1` tags let a later
ADR change the algorithm or the input without an old hash being mistaken for a new one. The hash
does not depend on the target.

### `dot`, derived

`dot` in `linalg.lm`, as written in `docs/grammar.md`: parameters `n` (`u64` → `uint64_t`), `a` and
`b` (`ro` references to `f32` → `const float *`), no `wo` so no `restrict`, result `f32`. Checks: null
for `a`, `b`, `result`; alignment for `a`, `b` (128 bytes) and `result` (4 bytes, the alignment of
`float`); extent for `a`, `b`; no disjointness pair; no bounded parameter, element or result; `req` 1 reads no
memory, check 5; `f32` appears,
check 6; no check 7. Numbered in that order, the constants are 1 to 10, exactly ADR 0005's:

```c
/* linalg_dot: sum(i: u64 in 0..<n) { a[i] * b[i] }. Hash: sha256:<64 hex digits>.
 * Not checked: a and b each point to n readable floats; no thread writes them during the call. */
#define LINALG_DOT_OK               0
#define LINALG_DOT_ERR_NULL_A       1  /* a != NULL                                  O(1) */
#define LINALG_DOT_ERR_NULL_B       2  /* b != NULL                                  O(1) */
#define LINALG_DOT_ERR_NULL_RESULT  3  /* result != NULL                             O(1) */
#define LINALG_DOT_ERR_ALIGN_A      4  /* a aligned to 128 bytes                     O(1) */
#define LINALG_DOT_ERR_ALIGN_B      5  /* b aligned to 128 bytes                     O(1) */
#define LINALG_DOT_ERR_ALIGN_RESULT 6  /* result aligned to 4 bytes                  O(1) */
#define LINALG_DOT_ERR_EXTENT_A     7  /* n * 4 and a + n * 4 fit the address space  O(1) */
#define LINALG_DOT_ERR_EXTENT_B     8  /* n * 4 and b + n * 4 fit the address space  O(1) */
#define LINALG_DOT_ERR_REQ_1        9  /* n % 16 == 0                                O(1) */
#define LINALG_DOT_ERR_FP_ENV       10 /* FP environment: round to nearest, no flush O(1) */

int32_t linalg_dot(uint64_t n, const float *a, const float *b, float *result);
float   linalg_dot_unchecked(uint64_t n, const float *a, const float *b);
```

The only difference from ADR 0005's example is the spec comment, which is now the canonical
printing of the spec in the grammar's spelling, and the hash, which now has a definition.

## Consequences

- The header generator has a rule for every function the subset accepts, and refuses what it cannot
  map with a named kind rather than emitting a header a C compiler would reject.
- Status indices stay a function of the signature and the `req` list only, so the hash in the
  header changes whenever they do, and also when only the impl changes: the hash says which build
  the header belongs to, not only which contract.
- Renaming or exporting a function re-proves nothing. Changing a parameter's name changes the hash,
  as it changes the header's constant names.
- When calls between Lemma functions arrive, a caller's hash must cover its callees' hashes, or a
  changed callee would leave its callers' hashes unchanged; that input is fixed then, under a new
  input tag.
- Two modules can still produce the same symbol (`a_b` with `c`, `a` with `b_c`); a single module
  cannot detect it, and it waits for the module system.
- `bool` and vectors stay inside Lemma. A kernel that wants a vector interface takes a reference to
  scalars and loads from it.
- The proven body's LLVM facts at the boundary are those of ADR 0005, unchanged: the mapping adds
  `restrict` and `const` to the header as documentation of the contract, never a fact on the
  checked wrapper's parameters.
