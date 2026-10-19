# 1. LLVM backend through inkwell, JIT through ORC

- Status: accepted
- Date: 2026-10-19

## Context

Lemma needs native code generation for Apple M1 Pro (AArch64, NEON) and an in-memory JIT for its
development loop: compile, test spec against impl, hot-replace the changed function.

## Decision

- Generate LLVM IR with `inkwell` 0.10, pinned to LLVM 22 (`llvm22-1` feature). inkwell does not
  support LLVM 23 yet.
- Do not use inkwell for the JIT: it has no ORCv2 / LLJIT wrapper, and its LLVM 23 work still lists
  ORC support as unfinished. Load objects through the ORC C API (`llvm-sys`) or a small C++ shim.
- Emit one object per content-hashed function. Hot swap goes through a stub that reads a function
  pointer stored in data pages, so no code page is ever patched (no W^X toggling).
- The JIT and the final binary use the same pinned target configuration (`apple-m1`, never `native`,
  PIC) and the same objects, so tested code is shipped code.

## Consequences

- Upgrading LLVM waits for inkwell support.
- A small amount of FFI (`unsafe`) lives in `lemma-jit`.
- Cranelift is not used: a second backend would break "tested == shipped".
