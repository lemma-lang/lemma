# 5. The C ABI and WebAssembly boundary: checked by default, unchecked by name

- Status: accepted
- Date: 2026-10-19

## Context

Inside Lemma, every call is proven: a caller discharges the callee's `req`, alignment, bounds and
region disjointness at compile time, so the callee assumes them and the compiler emits LLVM facts
(`noalias`, `align`, `nonnull`, assumptions on scalars) on that basis.

A caller through the C ABI or a WebAssembly export (ADR 0002) is not verified. If it passes a
misaligned pointer, two overlapping regions or an `n` that breaks a `req`, the proofs say nothing
and the facts emitted from them become undefined behavior: a silent wrong result, the failure
Lemma exists to rule out. At the same time, a caller that does guarantee the preconditions (a
real-time loop calling a small kernel at high rate) should not pay for checking them again.

## Decision

### Two exports per function

Every exported function is exported twice, always: there is no option to omit either variant.

- **`<name>`, the checked variant.** It checks every precondition that can be checked at runtime,
  in a fixed documented order, before reading or writing anything. If all pass, it runs the
  proven body. If one fails, it returns a status naming the first failed check and has written
  nothing.
- **`<name>_unchecked`, the raw variant.** The proven body with its natural signature. Its
  preconditions are a contract stated in the header, as in C: violating one is undefined behavior.

Calls between Lemma functions never go through either: they are proven at the call site.

### Symbol names

C has one flat namespace, so every exported symbol carries its module as a prefix:
`<module>_<function>` and `<module>_<function>_unchecked`, with status constants
`<MODULE>_<FUNCTION>_OK` and `<MODULE>_<FUNCTION>_ERR_<CHECK>`, and one header `<module>.h`. Until
Lemma has a module system, the module is the source file's stem. WebAssembly exports use the same
names.

### Failure is a status code, not a trap

The checked variant returns `int32_t`: `0` on success, otherwise the 1-based index of the first
failed check, each index named by a constant in the header. Results go through out-pointers,
written only on success.

A trap was rejected for three reasons. Traps are opt-in in Lemma ("nothing implicit"), and a trap
the caller did not ask for would be the default. A trap kills a C or Python host process, where a
robot controller would rather fall back to a safe state than abort. And a status code costs one
register and one well-predicted branch. The same convention holds for WebAssembly, where the host
reads results from linear memory like any C-compiled module.

There is no trap mode. One can be added later, by a new ADR, if a caller needs it.

### What is checked, and what is only documented

Checked, in this order:

| # | Precondition | How | Cost |
|---|---|---|---|
| 1 | Pointers are non-null | compare to `NULL` | O(1) per pointer |
| 2 | Alignment (`@a128`, in bytes, and the type's own) | low bits of the address | O(1) per pointer |
| 3 | Extents do not overflow: `n * size_of(T)` and `p + extent` fit the pointer width; in Wasm, the range lies inside the current linear memory | arithmetic, `memory.size` | O(1) per region |
| 4 | Regions required disjoint are disjoint (every pair where one side is written) | interval comparison `a_end <= b \|\| b_end <= a` | O(1) per pair |
| 5 | `req` clauses over scalars and lengths (`n % 16 == 0`) | the clause, compiled | O(1) each |
| 6 | The floating-point environment matches the proofs' model (ADR 0003: round to nearest, subnormals preserved), for functions using floating point. A mismatch fails with a status; the wrapper never sets or restores the environment | read FPCR, MXCSR or FPSCR; nothing on Wasm | O(1) |
| 7 | `req` clauses over memory contents (`∀ i < m. idx[i] < n`), and bounded-integer element types in memory | a loop over the region | O(n) reads |

Not checkable, therefore documented in the header under "Not checked", for every function they
apply to:

- that each pointer designates a live buffer of at least its extent: a C pointer carries no size.
  In Wasm, step 3 checks that the range is inside linear memory, not that it is the intended
  buffer;
- that no other thread or interrupt handler writes a region for the duration of the call. The
  memory-content checks of step 7 are only as good as this: a region changed after the check
  breaks the precondition the check passed;
- disjointness of regions whose extent is not a parameter (pointers reached through other memory),
  and disjointness from memory the caller keeps using elsewhere;
- that memory read as a type with invalid bit patterns is initialized, where step 7 does not
  cover it.

The result out-pointer of the checked variant is written once the body has returned, so it need not
be disjoint from the inputs; it is checked for null and alignment only.

### Generated header

`lemma` emits one C header per exported module, regenerated from source and never edited by hand.
For each function it contains:

- both prototypes, with exact-width types (`u64` is `uint64_t`, never an implicit `size_t`),
  `const` on `ro` regions and `restrict` on regions the function requires disjoint, as
  documentation of the contract;
- the spec, as a comment;
- the status constants of the checked variant, each with the check it names and its cost;
- the "Not checked" list;
- the function's content hash, as a comment.

For `dot` (`examples/dot.lm`) in a module `linalg`, header `linalg.h`:

```c
/* linalg_dot: Σ(i<n) a[i]*b[i]. Hash: <content hash>.
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

`a` and `b` are both read-only, so no disjointness check applies to them. `@a128` is an alignment
in bytes: 128, the M1's cache line (`docs/design.md`).

### LLVM facts at the boundary

The checked wrapper carries no fact on its own parameters: they come from an unverified caller. The
facts sit on the proven body (`noalias`, `align`, `nonnull`, `readonly`, assumptions from `req`),
and each one rests on one of two bases, which the auditor checks:

- **checked**: established by checks 1 to 7 on the path through the checked variant;
- **contract**: stated in the header for callers of `_unchecked`, and for the items of the
  "Not checked" list on both paths.

A fact on neither list is not emitted.

## Consequences

- Safe by default: a C, Python, ROS 2 or JavaScript caller that never reads the header gets a
  status, not undefined behavior, for every checkable violation.
- The O(1) checks cost a few instructions and predictable branches per pointer, negligible for any
  kernel over more than a few dozen elements. Very small kernels called at high rate should use
  `_unchecked`, or better, move the loop into Lemma.
- An O(n) check (step 7) adds a full read pass, which can double the memory traffic of a
  memory-bound kernel. The header states it, so the caller chooses with the cost in view.
- Code size: the wrapper adds tens of bytes per function. Each variant is in its own section, so a
  firmware link with section garbage collection drops the one it does not call.
- A caller that sets flush-to-zero for speed, as audio code often does, gets `ERR_FP_ENV` from the
  checked variant: Lemma does not run proven code outside the model its proofs assume, and does
  not touch the caller's environment behind its back. Such a caller restores the default
  environment around the call, or calls `_unchecked` and accepts results the proofs do not cover.
- `@a128` buffers need 128-byte alignment from the caller (`aligned_alloc(128, …)` or
  `posix_memalign`); a plain `malloc` buffer fails check 2.
- A checked export changes its signature (status return, out-pointer). Binding generators for
  Python or JavaScript can hide that; they are host-language tooling, outside Lemma.
- The checked wrapper belongs to the function's object and is hot-replaced with it by the JIT
  (ADR 0001): changing a `req` changes its checks and its status constants together.
- The status indices follow the order of the checks, so they change when the preconditions change;
  the header is regenerated with the function, and its hash tells a caller which version it was
  written against.
