# 4. Prototype experiment: success criteria fixed before it runs

- Status: accepted; thresholds confirmed as written and frozen
- Date: 2026-10-19
- Amended: 2026-10-19, one sentence added under "Runs" on calibration runs, approved by the maintainer;
  see ADR 0006

## Context

Lemma rests on a bet: that an AI model writes fast, correct kernels more reliably in a language
where the spec is separate from the impl and their equivalence is proven, than in a mainstream
language with SIMD intrinsics, or in an existing verified language. That bet can be tested before
the full compiler exists, with a prototype:

- a minimal Lemma subset: parser, Z3 proofs of bounds and of spec-vs-impl equivalence, LLVM
  lowering, JIT;
- seven kernels: dot product, saxpy, prefix sum, FIR filter, small matrix multiply, one SHA-256
  round, and a bounds-trap kernel (a gather whose indices come from the caller and can be out of
  range);
- one model writes each kernel three times: in Lemma, in Rust with SIMD (`core::arch` NEON
  intrinsics), and in Verus.

The criteria must be fixed before the first run. Set afterwards, they would be fitted to the data.

Two claims often made about Lemma are **hypotheses this experiment tests, not design goals**: that
it costs fewer tokens to reach a correct kernel, and that it is intuitive for a language model. The
design goals stay those of the README and ADR 0002: proven correctness and performance. If the
hypotheses are refuted, Lemma stops claiming them; the project does not stop for that alone.

## Decision

### Protocol

- **Model.** The primary model is Claude Opus 5.5 (`claude-opus-5-5`), with the same sampling
  settings for every run. Its version is frozen for the whole experiment, like the toolchain: a
  model update mid-experiment voids the runs made after it. Each run starts from a fresh context.
  A second model family may be run with the same protocol if the budget allows; its results are
  reported and gate nothing.
- **Prompt.** The same template for the three languages: the task in natural language, the
  preconditions (for example `n % 16 == 0`), the target (Apple M1 Pro, 128-bit NEON), the request
  for a vectorized implementation, and a **reference card** for the language. Lemma's card is
  needed because no model has seen Lemma; Rust and Verus get a card of the same size, so that the
  comparison is not between documentation and none. The cards are frozen before the first run and
  **contain no solution to any of the kernels**: `examples/dot.lm` is left out, and since dot
  appears in the README the model may still have seen it, so dot's results are reported apart from
  the aggregate as well as within it.
- **Who writes the spec.** The model writes the Lemma spec and the Verus spec, as a real user
  would. A wrong spec that verifies is a silent bug, not a success.
- **Rounds.** One attempt, then at most 3 rounds of feedback, so at most 4 submissions. Feedback
  is the raw output of the toolchain and of a small **visible test** (2 or 3 cases, the same for
  the three languages), with no human hint. A run ends at the first submission accepted by the
  toolchain and the visible test, or after the fourth.
- **Toolchain freeze.** The prototype is frozen before the first run. Only crashes and soundness
  bugs are fixed during the experiment; each fix is logged, and the runs it affected are re-run and
  reported with the count of re-runs.
- **Runs.** 10 independent runs per kernel per language: 70 per language, 210 in total.
  A pilot of 2 runs per kernel debugs the harness and is discarded.
  Runs made under ADR 0006's calibration are not runs of this experiment: they use none of its
  kernels, are made before the toolchain and the reference cards are frozen, are discarded, and
  gate nothing.

Why 10: a pooled rate over 70 runs has a 95 % confidence interval of about ±12 points at 50 %,
narrow enough for the gates below. Per kernel, 10 runs only detect gross failures (±30 points),
so per-kernel thresholds are loose. Languages are compared kernel by kernel (paired), with a sign
test across kernels. For silent bugs, 0 observed in about 60 accepted runs bounds the true rate
below 5 % at 95 % confidence (rule of three), which is why the silent-bug gate sits at 5 %.

### What is measured, and how

- **First-try success**: the first submission is correct.
- **Success within 3 rounds**: a correct submission within the 4 allowed.
- **Correct** means accepted by the language's toolchain (Lemma: verified; Verus: verified; Rust:
  compiled), passing the visible test, **and** passing the hidden test set.
- **Silent bug**: a submission accepted by the toolchain and the visible test that fails the
  hidden test set. Rate = silent bugs / accepted submissions.
- **Hidden test set**, the same for the three languages, written and checked before the runs:
  - a human-written scalar reference in plain Rust, computing floating-point results in f64;
  - seeded random inputs at three sizes (L1-resident, L2-resident, beyond the 12 MB L2), plus
    edge cases: the smallest legal `n`, zeros, infinities, NaN, subnormals, maximal integer values;
  - known-answer vectors for SHA-256 (FIPS 180-4 examples);
  - for the bounds-trap kernel, out-of-range indices: the correct outcome is a rejection (ADR 0005
    status for Lemma, a panic for safe Rust, a failed precondition for Verus); a silent
    out-of-bounds read, caught by a guard page or AddressSanitizer, is a silent bug;
  - integer and SHA-256 results compared bit for bit; floating-point results within the classic
    error bound of a reassociated sum, `|r - ref| <= γ(n) · Σ|terms|` with `γ(n) = n·u / (1 - n·u)`
    and `u` the unit roundoff, NaN compared as NaN.
- **Tokens to a correct kernel**: input plus output tokens over all submissions of a run, counted
  with the model's tokenizer, over successful runs only; reported twice, with and without the
  reference card.
- **Runtime**: on the M1 Pro, on a performance core requested through
  `QOS_CLASS_USER_INTERACTIVE`, timed with `mach_absolute_time`, median of at least 31 repetitions
  after warm-up, at the three sizes. Lemma is timed through its `_unchecked` export (ADR 0005), and
  its checked export separately, to report the boundary cost. Rust is built with
  `-C target-cpu=apple-m1` at `opt-level=3`; the LLVM versions of both compilers are recorded.
  Per kernel and language, the figure is the **median over correct runs** (what a typical run
  produces), with the best run as a secondary figure.
- **Accelerate**, where an equivalent exists: `vDSP_dotpr` (dot), `cblas_saxpy` (saxpy),
  `vDSP_conv` (FIR), `cblas_sgemm` (matrix multiply). `cblas_sgemm` may run on the AMX
  coprocessor, which Lemma cannot target, so it is reported, never gated. Prefix sum, the SHA-256
  round and the bounds-trap kernel have no exact Accelerate equivalent and are not compared.

Verus has no verified SIMD, so its kernels are scalar: Verus is the baseline for **verification
effort** (rounds, tokens, silent bugs), Rust with SIMD the baseline for **runtime** and silent bugs.

### Outcomes

Every failed submission is classified by cause (syntax, type or layout, effect, bound proof,
equivalence proof, solver timeout, toolchain bug, wrong spec), so that whatever the outcome, it
names the constructs at fault.

**GO** (the design holds; build the real compiler on it), when all of these hold:

| Gate | Threshold |
|---|---|
| Lemma success within 3 rounds | ≥ 80 % pooled, no kernel below 50 % |
| Versus Verus | Lemma's success within 3 rounds ≥ Verus's |
| Versus Rust with SIMD | Lemma's success within 3 rounds ≥ Rust's minus 10 points |
| Lemma silent bugs | ≤ 5 % of accepted submissions, and at most a third of Rust's rate |
| Silent bugs caused by the toolchain | none left once the logged fixes are applied |
| Lemma runtime versus Rust with SIMD (median over correct runs) | geometric mean of the ratios ≤ 1.10, no kernel above 1.5 |
| Lemma runtime versus Accelerate (dot, saxpy, FIR, L1- and L2-resident) | ≤ 1.25 |

**STOP and rethink** (the bet failed as designed; revisit the language before writing more of it),
when any of these holds:

| Trigger | Threshold |
|---|---|
| Lemma success within 3 rounds | < 60 % pooled, or below Verus's |
| Lemma silent bugs | not lower than Rust's rate: the proofs gave no observed protection |
| Lemma runtime versus Rust with SIMD | geometric mean of the ratios > 1.5 |
| Soundness | a silent bug from the verifier that cannot be fixed without changing the semantics |

**CONTINUE WITH CHANGES** is everything between the two: the failure classification names the
constructs to change, they are changed through ADRs, and the experiment is re-run. The re-run adds
**three new kernels** not used while changing the language, and the gates apply to those three as
well as to the original seven: a language tuned to seven kernels has learned those seven.

**The two hypotheses** are judged apart and gate nothing:

| Hypothesis | Supported if | Refuted if |
|---|---|---|
| Fewer tokens to a correct kernel | Lemma's median, without the card, ≤ half of Verus's and ≤ 1.5 × Rust's | Lemma's median > Verus's |
| Intuitive for a language model | Lemma's first-try success ≥ Rust's minus 10 points | Lemma's first-try success < half of Rust's |

First-try success has no gate of its own: the model has never seen Lemma, and the rounds of
feedback are part of the intended loop.

## Consequences

- No threshold changes once the first run has started. A threshold found wrong is changed by a new
  ADR that supersedes this one, written and accepted **before** any re-run, with its reason.
- The outcome, the summary tables and the location of the raw transcripts and measurements are
  recorded in a new ADR citing this one. A STOP is recorded like a GO.
- The prototype must implement ADR 0005's checked export for the bounds-trap kernel, and the
  floating-point permissions of `docs/design.md` (`fp_reassoc`, `fp_contract`), since a dot product
  with `fma` cannot be proven equivalent without the second.
- The harness (reference implementations, hidden tests, error bounds, timing) is written and
  reviewed before the runs, and is the experiment's largest piece of trusted code.
- The decision rests on one model. The optional second family says whether the result
  generalizes; it informs the outcome ADR without changing the verdict.
