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

## Two levels of effects

1. **Compute kernels**: static, proven effects with zero runtime cost. A `pure` function is trivially reversible.
2. **Components**: reversible effects and reactive coeffects (Cordis paradigm). Every acquired resource records its inverse.
   Hot reload cleanly tears down the old version, and dependents are deactivated then reactivated.

Reference: *A Programming Paradigm for Spatiotemporal Composability*, arXiv 2608.25512.
