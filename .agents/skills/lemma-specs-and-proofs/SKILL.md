---
name: lemma-specs-and-proofs
description: Use when writing or changing a Lemma contract (a fn signature's types and bounds, spec, req, ens, eff or equiv clauses in a .lm file), when changing crates/lemma-verify, or when a proof is slow, flaky or unknown. Covers checking that the obligations sent to Z3 are the right ones, that a spec is not vacuous, and that "proven" is stable.
---

# Lemma specs and proofs

Lemma's promise is that a function's impl is proven equivalent to its spec before any native
code is emitted ([`docs/design.md`](../../../docs/design.md)). A proof is only as good as the
question asked: the right obligations, a spec that says something, and an answer that does not
depend on luck.

## The contract is not adjusted to fit the impl

A `.lm` function holds its contract and its impl in one file. The contract is the `fn` signature
line (types and their bounds) and the `spec`, `req`, `ens`, `eff` and `equiv` clauses.

- A spec changes because it was wrong, never because the proof is hard. Say which.
- Adding an `allow(...)` to an `equiv` clause (for example `allow(fp_reassoc)`) changes the
  language-level meaning of the function: it is a design decision, see
  [lemma-language-design](../lemma-language-design/SKILL.md).
- A change that touches the impl leaves the contract byte for byte as it was, and the other way
  round: review them as separate changes.

## Writing a spec: list what it must reject

For every new function, write down alongside the spec the **known-wrong outputs** it must reject:
an off-by-one sum, a dropped last element, a wrong sign, a reassociation the `equiv` clause does
not allow. Each becomes a differential test (see [lemma-testing](../lemma-testing/SKILL.md)) and a
check on the spec itself.

## What to check

1. **The obligations are the right ones.** For each construct, write the obligation it should
   generate (bounds of every access, `req` assumed at entry, `ens` proven at every return, loop
   invariants at entry and preserved, spec and impl equivalent under exactly the `allow(...)`
   granted). Dump the query the verifier sends and read it. A missing obligation is a soundness
   hole; an assumption that is too strong (a `req` leaking into an unrelated proof, an axiom that
   makes the context inconsistent) is worse, because everything then proves.
2. **The spec is not vacuous.**
   - `req` is satisfiable: ask Z3 for a model of the preconditions alone. An unsatisfiable `req`
     makes every obligation trivially true.
   - Each known-wrong output, substituted, yields a counterexample. One that passes means the spec
     is too weak.
   - For a small impl, removing any one clause breaks the proof; a clause whose removal still
     proves is dead or unconstrained.
3. **"Proven" is stable.**
   - Budgets are Z3 resource limits (`rlimit`), not wall-clock timeouts: a timeout depends on the
     machine and its load. Report the resource count actually used
     (`(get-info :all-statistics)`) against the budget: under a tenth is comfortable, over half is
     fragile.
   - Rerun each changed obligation with at least five values of `smt.random_seed` and
     `sat.random_seed`. A result that changes with the seed is unstable, whatever today's default
     seed says.
   - Queries carry only the hypotheses their obligation needs, and no quantifier where an
     instantiation is known: quantifiers tie the result to triggers and heuristics, and a growing
     context makes unrelated changes break proofs.
   - Floating point is checked against the IEEE semantics the `equiv` clause names, never against
     real arithmetic silently.
4. **Unknown is not proven.** Any path where `unknown`, a timeout or a solver error is reported as
   success is the most severe soundness hole there is. So is a path that answers "proven" without
   asking Z3.

```sh
cargo nextest run -p lemma-verify
z3 -st rlimit=<n> smt.random_seed=<k> <query>.smt2
```

A result obtained once, with one seed, is a hypothesis.

## Reporting a finding

Give its class (soundness hole, vacuous or weak spec, unstable proof, inefficiency), the file and
line, the query or input that reproduces it, and the resource counts over the seeds you tried.
