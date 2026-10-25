---
name: lemma-perf
description: Use when measuring Lemma performance, writing a benchmark, claiming a speed-up, suspecting a regression, or changing crates/lemma-codegen or crates/lemma-jit. Covers the measurement protocol (build, timer, core placement, statistics, regression rule) and reading emitted IR and assembly to check a codegen claim directly.
---

# Measuring Lemma performance

Lemma exists for performance-critical code, so a claim about speed is a number someone else can
reproduce, with the protocol that produced it. "2x faster" without machine, commit, timer and
sample count is not a result.

## Protocol

Every reported number carries these lines:

1. **Machine**: model, OS, architecture, core counts. On Apple silicon,
   `sysctl hw.perflevel0.physicalcpu hw.perflevel1.physicalcpu` gives performance and efficiency
   cores; on Linux, the CPU model and the governor.
2. **Build**: `--release`, the same target options as the final binary (the JIT and the emitted
   binary must agree), the commit hash, `rustc -V`, LLVM 22.
3. **Timer**: say which. Hardware cycle counters often need root (`kperf` on macOS, `perf` with
   restricted `perf_event_paranoid` on Linux); do not elevate privileges inside a benchmark
   harness. Without them, use `std::time::Instant` (`mach_absolute_time` on macOS, a 24 MHz
   timebase on Apple silicon: about 41.7 ns per tick, so time enough iterations per sample that
   one tick is noise).
4. **Core placement**: on Apple silicon, set the benchmark thread to `QOS_CLASS_USER_INTERACTIVE`
   (`pthread_set_qos_class_self_np`) so it lands on a performance core, and record it. A run that
   may have landed on an efficiency core is not comparable with one that did not. On Linux, pin
   with `taskset` and record the core.
5. **Conditions**: on mains power, nothing else heavy running (no parallel `cargo build`), a
   warm-up before sampling. Record anything unusual.
6. **Statistics**: at least 30 samples per configuration; report the **median** and the
   interquartile range, never the mean alone and never a single run. Run baseline and candidate in
   the same session, interleaved (A B A B…), not one after the other.

## Regression rule

A change is a regression or an improvement only if the medians differ by more than both IQRs and
by more than 2 %, and the difference survives a rerun. Below that, the verdict is "no measurable
change", which is a result.

## Benchmarks

- They live in `benches/` or `crates/<crate>/benches/`, with `[[bench]]` and `harness = false`
  in the crate's `Cargo.toml`. Use `criterion` (`--save-baseline` / `--baseline`), naming
  baselines by branch: `target/criterion` is shared.
- A benchmark measures the workload the issue names. Changing the workload until it shows what
  someone hoped is not measurement.
- Keep the code under measurement and its instrument in separate commits, so a reviewer can see
  that one did not tune the other.

## Read the code, not only the clock

For a kernel such as [`examples/dot.lm`](../../../examples/dot.lm), read the emitted IR and the
assembly (`llc`, `objdump -d`, or the JIT's dump) and check the claim directly: vector width used,
accumulators kept in registers, no unexpected spill, loads aligned as the type says. A missed
vectorization often traces back to a fact the compiler could not prove; see
[lemma-specs-and-proofs](../lemma-specs-and-proofs/SKILL.md).
