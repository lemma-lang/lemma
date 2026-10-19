# 3. Target tiers: what each target is promised

- Status: accepted
- Date: 2026-10-19

## Context

ADR 0002 makes Lemma callable from C and WebAssembly and names robotics and embedded as primary
domains, which means AArch64, x86-64, Cortex-M, RISC-V and Wasm all matter. Supporting a target
fully (native tests, performance tracking, a JIT) costs CI time and hardware, and not every
target needs all of it. Without a written promise per target, "supported" means whatever the last
contributor assumed.

Two different things run on a machine and must not be confused:

- the **host**: where the compiler, the verifier and the JIT development loop run;
- the **target**: where the code Lemma emits runs.

What CI covers today (`.github/workflows/ci.yml`) is hosts only, since no crate emits target code
yet. The repository is public since 2026-10-19, so every host job runs on every push to `main` and
every pull request:

| Host | Jobs |
|---|---|
| Linux x86-64 (Debian trixie container) | lint, test, doc |
| Linux AArch64 (`ubuntu-24.04-arm`) | lint, test, doc |
| macOS AArch64 (`macos-15`, Apple Silicon) | lint, test |

The workflow still conditions the last two on the repository being public: made private again, the
Linux AArch64 job would be skipped and the macOS job would run on `main` only.

No job builds for RISC-V, Cortex-M or Wasm, and no job measures performance: shared, virtualized
CI runners give timings too noisy to compare.

## Decision

### Tiers

| Promise | Tier 1 | Tier 2 | Tier 3 |
|---|---|---|---|
| Code generation supported | yes | yes | yes |
| Emitted code built in CI | every pull request, blocking | every pull request, blocking | every pull request, blocking |
| Test suite, including spec-vs-impl differential tests, run on emitted code | all of it, natively, blocking | all of it, natively or under QEMU, blocking | a smoke subset under an emulator, not blocking |
| JIT development loop | yes, natively | no promise | never on the device (ADR 0002): runs on the host |
| Performance measured | Apple AArch64: yes, on the M1 Pro, a regression blocks the change; x86-64: emitted code inspected, not timed | no: timings under emulation are not recorded | code size and worst-case stack usage reported (Cortex-M); no timing |
| Proofs | identical on every tier (see below) | identical | identical |

Targets and their pinned configuration (ADR 0001: a pinned CPU, never `native`):

| Tier | Target | Triple and CPU | Run in CI by |
|---|---|---|---|
| 1 | Apple AArch64 | `aarch64-apple-darwin`, `apple-m1` | macOS runner; performance on the M1 Pro reference machine (`docs/design.md`) |
| 1 | x86-64 | `x86_64-unknown-linux-gnu`, `x86-64-v3` (AVX2, FMA) | Linux x86-64 runner; no performance reference machine |
| 2 | AArch64 Linux | `aarch64-unknown-linux-gnu`, generic Armv8.0-A (NEON, FMA) | Linux AArch64 runner, natively |
| 2 | RISC-V | `riscv64gc-unknown-linux-gnu`, RV64GC | QEMU user mode in a container |
| 3 | Cortex-M | `thumbv7em-none-eabihf`, `cortex-m4` (FPv4-SP) | QEMU system mode, `mps2-an386` |
| 3 | WebAssembly | `wasm32-unknown-unknown`, `simd128` | wasmtime |

A target changes tier through a new ADR that supersedes this one.

### Proofs do not depend on the tier

The verifier runs on the host, so a lower tier never weakens a proof: it weakens only the evidence
that LLVM's lowering for that target is right. Obligations come in two kinds:

- **Target-independent**, discharged once per function hash: bounds, `req` / `ens`, spec-vs-impl
  equivalence. They are stated on one semantic model, the same for every target.
- **Target-dependent**, discharged for each target a function is built for, on every tier:
  - index and extent arithmetic fits the target's pointer width (32 bits on Cortex-M and wasm32,
    so `n * size_of(T)` must be proven or checked to fit);
  - every vector type `T x N` is within the target's native width (`f32x8` is legal on x86-64-v3,
    not on NEON, Wasm `simd128` or Cortex-M);
  - every operation the impl names exists on the target (Cortex-M4 has single-precision `fma` but
    no double-precision hardware);
  - alignments and cache-level tile sizes respect the target's constants (128-byte lines on
    Apple AArch64, 64-byte on x86-64; an alignment such as `@a128`, in bytes, must be one the
    target's allocators and linker can honor).

The floating-point model of the proofs is IEEE 754 binary32 and binary64, round to nearest even,
subnormals preserved, and a single NaN: NaN payloads are not observable (SMT-LIB floating point has
one NaN, and AArch64, x86-64 and Wasm disagree on payload propagation). This is the default
environment of every target above. A program that changes it (flush-to-zero, another rounding
mode) leaves the model; how the C boundary handles that is ADR 0005.

## Consequences

- Performance is measured on a reference machine, never in CI, and **all performance numbers come
  from the M1 Pro**. x86-64 has no reference machine: its performance promise is limited to
  inspecting the emitted code (expected vector instructions, no spills in the benchmark kernels),
  and no x86-64 timing is published as a Lemma result. Declaring an x86-64 reference machine is a
  new ADR.
- CI must gain cross-compilation jobs for RISC-V, Cortex-M and Wasm, with QEMU and wasmtime, before
  those targets can claim their tier. A target with no CI job has no tier, whatever this table says.
- Cortex-M is tier 3 although embedded is a primary domain (ADR 0002). The tier states what CI
  proves about the emitted code today, not the domain's priority. What embedded needs from the
  language (allocation-free code, no standard library, 32-bit pointers) is enforced on every tier
  by the target-dependent obligations above.
- A kernel written with `f32x8` for x86-64 does not build for NEON targets. Portable kernels choose
  widths per target; how they do so is a design point of `docs/design.md`.
- Proof results are cached by function hash for the target-independent obligations and by
  function hash and target for the target-dependent ones.

## Open questions, deferred

These targets and criteria have no tier yet; each is settled by a new ADR when it is needed.

- The RISC-V vector extension (RVV): which fixed vector length, if any, `T x N` maps to.
- RV32 bare-metal, which ADR 0002 names for embedded but which no tier covers.
- Cortex-M7 (double-precision FPU) and Cortex-M33 as further Cortex-M configurations.
- The criteria for promoting Cortex-M from tier 3 to tier 2 (a stable differential suite under
  QEMU, at least).
