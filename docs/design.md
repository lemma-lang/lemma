# Lemma design

## Why not go below assembly

Raw machine code tokenizes poorly, has no redundancy, and a one-bit error in it is silent.
Above all, it throws away the intent (aliasing, bounds, purity) the optimizer needs.
Performance gains come from what can be proven, not from syntactic closeness to the silicon.

## The CPU as a typed resource

Memory can be written to directly. The CPU cannot: it executes what is in memory, and its microcode is signed and inaccessible.
The CPU is not programmed, it is **driven**. Lemma therefore makes every lever explicit in types and effects.

| Resource | What can be controlled | In Lemma |
|---|---|---|
| Registers | Allocation, pressure | SSA vector values (`f32x4`). LLVM's allocator decides; cost feedback reports spills |
| Vector units | Width, number of pipelines | `T x N` types bounded by the target's native width. Multiple accumulators to fill the pipelines |
| Caches | Tile size, prefetching, non-temporal stores | Tiles typed by level (`@L1`, `@L2`), sizes checked against the target |
| Cores | Placement, parallelism | `cores(perf: k)` coeffect, analogous to a memory region |
| PMU counters | Cycles, cache misses, branch mispredictions | Cost feedback from the JIT daemon |

### Reference target: Apple M1 Pro

- 8 performance cores, 2 efficiency cores.
- 128-bit NEON, so `f32x4` is the native width. No SVE or SME.
- 128 KB L1d per P-core, 12 MB L2 shared per cluster.
- **128-byte** cache lines (not 64 as on x86): alignments and tiles account for this.
- macOS offers no strict core affinity: a P-core is requested through QoS (`QOS_CLASS_USER_INTERACTIVE`), without guarantee.
- JIT memory: LLVM's default JITLink memory manager maps code read-write, then read-execute by `mprotect`, never both at once (W^X). It does not use `MAP_JIT`, so it runs in an unsigned binary but not under the Hardened Runtime of a signed release, which needs a `MAP_JIT` memory manager with `pthread_jit_write_protect_np()` ([ADR 0010](adr/0010-jit-object-loading-and-call-contract.md)).
- PMU counters: only through private APIs (`kperf`) or Instruments. Otherwise, measure time (`mach_absolute_time`).

### Other targets

Targets come in three tiers ([ADR 0003](adr/0003-targets.md)): Apple AArch64 and x86-64
(`x86-64-v3`, AVX2 and FMA, so `f32x8` is native) in tier 1; AArch64 Linux and RISC-V in tier 2,
tested for correctness only; Cortex-M4F and WebAssembly (`simd128`) in tier 3. Each target has a
pinned CPU, never `native`, defined once in `lemma-codegen` and used both for the final binary and
by the JIT, which loads the same objects. The JIT runs only for the host's pinned target, and
refuses an object whose header names another architecture or format. An object's CPU is not in
its header, so the JIT relies on its objects coming from `emit_object` (or a trusted producer):
one emitted for another CPU of the same architecture loads and may fault when called. Objects
loaded by the JIT can call any symbol of the host process, as LLJIT links them by default
([ADR 0010](adr/0010-jit-object-loading-and-call-contract.md)).

Proofs are the same on every tier. What changes per target is checked per target: index and extent
arithmetic must fit the pointer width (32 bits on Cortex-M and wasm32), vector types must fit the
native width, and every operation the impl names must exist there. All performance numbers come
from the M1 Pro: x86-64 has no performance reference machine.

## Calling Lemma from C and WebAssembly

A function is exported by writing `export fn`; a plain `fn` is internal to its module, with no
symbol and no header entry ([ADR 0007](adr/0007-export-mapping-and-content-hash.md)). An exported
function is always exported twice ([ADR 0005](adr/0005-c-abi-boundary.md)), under its
module's prefix (`<module>_<function>`; the module is the source file's stem until Lemma has a
module system). The checked variant tests every runtime-checkable precondition (null, alignment,
extent overflow, disjointness of written regions, `req`, floating-point environment) and returns a
status, its result going through an out-pointer; there is no trap mode. The `_unchecked` variant is
the proven body, its preconditions a documented contract. The compiler generates the C header,
listing each check with its cost and what cannot be checked. For `dot` in a module `linalg`:

```c
int32_t linalg_dot(uint64_t n, const float *a, const float *b, float *result);  /* 0 = ok */
float   linalg_dot_unchecked(uint64_t n, const float *a, const float *b);
```

The C parameters are the Lemma parameters in order, sizes included, with exact-width integer
types, `const` on `ro` references and `restrict` on references required disjoint. Vector types and
`bool` never cross the boundary. A bounded value keeps its base type's C type: a bounded parameter
(`k: u64 < n`) is checked by the checked variant, so are a region's bounded elements
(`&(u8 < m)[n]`), and a bounded result (`-> u64 <= n`) is a guarantee stated in the header. Each
function in the header carries its content hash, SHA-256 of its canonical tokens from the
parameter list to the end of its impl, name, `export` and comments excluded (ADR 0007).

## Who each surface is for

Lemma is made by AI for AI ([ADR 0006](adr/0006-optimized-for-its-writer.md)): every
surface, the contract included, is optimized for the models that write and read it and for the
compiler that answers them. That means familiar spellings without false friends, one canonical
form per construct, no meaning carried by layout, redundancy only where the compiler checks it,
and diagnostics that are structured records (kind, span, failed obligation, Z3 counterexample as
concrete values, a repair when one is certain) rendered as text only afterwards (see
[Diagnostics](#diagnostics) below).

A proof shows that the impl matches the spec, never that the spec matches the intent: a wrong spec
that verifies is a silent bug. Two mechanisms catch it. The contract (the signature, `eff`, `req`,
`ens`, `spec` and `equiv`) is reviewed against the task by a model that did not write it, which
sees the task and the contract, not the impl. And every spec is executable: the compiler evaluates
it on concrete inputs with the proofs' semantics, so it is tested against known answers and edge
cases, not only proven against the impl. The contract is never repaired automatically, and no
permission is ever suggested.

The equivalence clause names no relation: its sides are always the spec and the impl, and its
content is the list of granted relaxations, always written, empty for exact equivalence:

```
  equiv allow(fp_reassoc, fp_contract)
  equiv allow()
```

## Grammar

The grammar of the prototype subset is [docs/grammar.md](grammar.md): tokens, EBNF, precedence,
canonical layout and the closed list of syntax diagnostics, with the choices it makes recorded in
[ADR 0008](adr/0008-prototype-grammar.md). It is provisional until ADR 0006's calibration freezes
it. Whether the sum, range membership and quantifier are spelled `sum`, `in` and `forall`
(variant A) or `Σ`, `∈` and `∀` (variant U) is settled by that calibration (ASCII on a
tie); the grammar carries both, the parser defaults to A until then, and the examples in this
document are written in A.

`examples/dot.lm`, in variant A:

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

Under variant U, the spec reads `spec Σ(i: u64 ∈ 0..<n) { a[i] * b[i] }` and the loop
`loop i: u64 ∈ 0..<n step 16 carry(`; nothing else differs.

What a reader needs to read it:

- The contract comes first, in a fixed order (`eff`, `req`, `ens`, `spec`, `equiv`), and the
  `impl` block last; everything before `impl` is what the spec reviewer sees.
- A size is an ordinary parameter that later types name (`n: u64, a: &f32[n] ...`). A reference
  always states its alignment in bytes and its mode.
- A bound on a single value is in its type: `k: u64 < n` for a parameter, `-> u64 <= n` for the
  result, `&(u8 < m)[n]` for a region's elements, naming only parameters to its left. `req` and
  `ens` hold the other relations (`n % 16 == 0` in `dot`, which has no bound). Quantified
  conditions are written `forall(i: u64 in 0..<n) { ... }`.
- Every binder carries its type. Ranges are half-open and spelled `lo..<hi`.
- No separators: each clause and statement starts with a keyword; blocks are braces; layout means
  nothing. Comments are `//`.
- Arithmetic is infix in the spec and the impl alike, lane by lane on vectors. `fma(x, y, z)` is
  `x * y + z` with one rounding, the addend last as in C. `T.splat` and `T.load(a, i)` name the
  vector type they produce; `reduce_sum` adds a vector's lanes.
- A loop's carries are its state: inside the body a carry is its value at the start of the
  iteration, `next` gives the following one, and after the loop the name holds the final value.

## Diagnostics

`lemma check` answers with records, whose format, two renderings and closed list of kinds are in
[docs/diagnostics.md](diagnostics.md) ([ADR 0009](adr/0009-diagnostic-records.md)). A record has a
kind, which fixes its ADR 0004 failure class; a span with byte offsets, 1-based lines and 1-based
columns counted in code points; related spans; a one-line message; for a proof, the obligation
and a counterexample; and, when the fix is certain, a repair: a list of edits applied together,
which on the contract changes no meaning. In the text rendering, for a `req` that states a bound
the parameter's type should carry:

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

The two edits are one repair: `k: u64` becomes `k: u64 < n` and the clause goes, so the
precondition is unchanged. A solver `unknown` or timeout is a kind of its own (`solver-unknown`),
never a failure with a counterexample nor a success.

## Two levels of effects

1. **Compute kernels**: static, proven effects with zero runtime cost. A `pure` function is trivially reversible.
2. **Components** (future direction, not designed: no ADR, outside the prototype, and not yet
   covered by [ADR 0002](adr/0002-scope.md)): reversible effects and reactive coeffects (Cordis paradigm). Every acquired resource records its inverse.
   Hot reload cleanly tears down the old version, and dependents are deactivated then reactivated.

Reference: *A Programming Paradigm for Spatiotemporal Composability*, arXiv 2608.25512.

## Design points raised by `examples/dot.lm`

The example now grants `fp_contract` and states its alignment in bytes. It still hard-codes the
accumulator count and the vector width, which wait for the tuned-parameter decision below.

### The accumulator count is a tuned parameter, not a constant

`dot` hard-codes four accumulators. The number that keeps the FMA units
busy depends on the FMA latency and issue width of the core and on the ratio of loads to FMAs in
the loop: four is a guess, not a derivation, and it is wrong on another core. The same holds for
the vector width on a target where `f32x8` is native. Both should be compile-time parameters of the
impl, chosen per target by measurement (the JIT's cost feedback), with the equivalence proven for
every value the search may pick. `req n % 16 == 0` then becomes a requirement on the product of
width and count, or disappears behind a tail loop. This is an open point: nothing about it is
decided yet, and it needs its own ADR (syntax, and whether the proof covers every value or each
instantiation) before it is implemented.

### `fma` needs `allow(fp_contract)`

`allow(fp_reassoc)` permits reordering additions of the same terms. `fma(x, y, c)` computes
`x * y + c` with a single rounding, where the spec rounds the product and then the sum: the terms
themselves differ, so reassociation does not cover it. Each relaxation of IEEE semantics is a
separate permission, opt-in by name:

| Permission | Admits | LLVM lowering |
|---|---|---|
| `fp_reassoc` | reordering of additions | `reassoc` flag on the operations the proof relies on |
| `fp_contract` | fusing a multiply and an add into one rounding | `contract` flag, or `llvm.fma` where the impl writes `fma` |

Codegen never emits `fast`, `nnan`, `ninf`, `nsz` or `arcp` unless a permission of that name
exists and is granted. `dot` must read:

```
  equiv allow(fp_reassoc, fp_contract)
```

### `pure` with reference parameters is not `memory(none)`

`eff pure` says the function has no effect on the world outside its parameters; its result still
depends on the memory its `ro` references point to. Lowering it to LLVM `memory(none)` would be a
miscompilation: LLVM would treat two calls with the same pointers as equal across a store to the
array, and reading memory inside a `memory(none)` function is undefined behavior. The lowering is:

| Lemma | LLVM |
|---|---|
| `pure`, no reference parameters | `memory(none)` |
| `pure`, only `ro` references | `memory(argmem: read)`; each pointer `readonly`, `captures(none)` |
| `ro` and `wo` references | `memory(argmem: readwrite)`; `ro` pointers `readonly`, `wo` pointers `writeonly`, `noalias` from the proven disjointness |

`willreturn` is emitted only on the basis of the bounded trip count of the loop lowering
([ADR 0008](adr/0008-prototype-grammar.md)): a loop such as `loop i: u64 in 0..<n step 16` runs at
most `ceil(n / 16)` times and its index never wraps. `noalias` is emitted only on proven or checked
disjointness (ADR 0005).

### Alignment is in bytes

`@aN` is an alignment of `N` **bytes**: `@a128` aligns to 128 bytes, the M1 Pro's cache line, so a
tile or a stream of `f32x4` loads never straddles a line. It is not the width of a load (that would
be `@a16`). A C caller allocates such buffers with `aligned_alloc(128, …)` or `posix_memalign`;
the checked export rejects anything less aligned (ADR 0005). On x86-64, whose lines are 64 bytes,
`@a64` is the cache-line alignment.
