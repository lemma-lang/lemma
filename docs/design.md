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
- JIT memory: `MAP_JIT` and `pthread_jit_write_protect_np()`, W^X rule.
- PMU counters: only through private APIs (`kperf`) or Instruments. Otherwise, measure time (`mach_absolute_time`).

### Other targets

Targets come in three tiers ([ADR 0003](adr/0003-targets.md)): Apple AArch64 and x86-64
(`x86-64-v3`, AVX2 and FMA, so `f32x8` is native) in tier 1; AArch64 Linux and RISC-V in tier 2,
tested for correctness only; Cortex-M4F and WebAssembly (`simd128`) in tier 3. Each target has a
pinned CPU, never `native`.

Proofs are the same on every tier. What changes per target is checked per target: index and extent
arithmetic must fit the pointer width (32 bits on Cortex-M and wasm32), vector types must fit the
native width, and every operation the impl names must exist there. All performance numbers come
from the M1 Pro: x86-64 has no performance reference machine.

## Calling Lemma from C and WebAssembly

An exported function is always exported twice ([ADR 0005](adr/0005-c-abi-boundary.md)), under its
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

## Two levels of effects

1. **Compute kernels**: static, proven effects with zero runtime cost. A `pure` function is trivially reversible.
2. **Components**: reversible effects and reactive coeffects (Cordis paradigm). Every acquired resource records its inverse.
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

`allow(fp_reassoc)` permits reordering additions of the same terms. `fma c x y` computes `x*y + c`
with a single rounding, where the spec rounds the product and then the sum: the terms themselves
differ, so reassociation does not cover it. Each relaxation of IEEE semantics is a separate
permission, opt-in by name:

| Permission | Admits | LLVM lowering |
|---|---|---|
| `fp_reassoc` | reordering of additions | `reassoc` flag on the operations the proof relies on |
| `fp_contract` | fusing a multiply and an add into one rounding | `contract` flag, or `llvm.fma` where the impl writes `fma` |

Codegen never emits `fast`, `nnan`, `ninf`, `nsz` or `arcp` unless a permission of that name
exists and is granted. `dot` must read:

```
  equiv spec ≈ impl  allow(fp_reassoc, fp_contract)
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

`willreturn` is emitted only on the basis of a termination proof (every loop bounded, as
`loop i ∈ 0..n` is), `noalias` only on proven or checked disjointness (ADR 0005).

### Alignment is in bytes

`@aN` is an alignment of `N` **bytes**: `@a128` aligns to 128 bytes, the M1 Pro's cache line, so a
tile or a stream of `f32x4` loads never straddles a line. It is not the width of a load (that would
be `@a16`). A C caller allocates such buffers with `aligned_alloc(128, …)` or `posix_memalign`;
the checked export rejects anything less aligned (ADR 0005). On x86-64, whose lines are 64 bytes,
`@a64` is the cache-line alignment.
