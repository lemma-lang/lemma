# 2. Scope: verified performance-critical code, callable from anywhere

- Status: accepted
- Date: 2026-10-19

## Context

Lemma is Turing-complete and compiles through LLVM, so in principle it can target any domain:
robotics, embedded/IoT, systems software, web. In practice a language is only worth using where its
guarantees pay off, and every feature added for an off-target domain dilutes the core.

AI agents already write glue code (web handlers, UI, scripts) well in existing languages, without
proofs. Lemma adds value where an agent needs a guarantee: code that must be both fast and correct.

## Decision

Lemma's scope is **performance-critical code that must be proven correct**: compute kernels, control
loops, signal and sensor processing, codecs, cryptography, numerical and simulation code.

Lemma is not a general application language. It integrates with other ecosystems instead of
replacing them, through two boundaries:

- **C ABI**: Lemma functions are callable from C, C++, Rust, Python, ROS 2 and embedded firmware.
- **WebAssembly**: Lemma code runs in browsers and in server runtimes (Node, Go, Rust hosts).

Target domains, in priority order:

| Domain | Fit | Requirements it adds |
|---|---|---|
| Robotics, real-time control | Primary | Allocation-free mode, worst-case execution time bounds, ROS 2 interop via C ABI |
| Embedded / IoT | Primary | No standard library, tiny binaries, typed hardware registers, cross-compilation (Cortex-M, RISC-V); the JIT loop runs on the host or in QEMU, never on the device |
| Systems libraries, CLI | Secondary | Strings, allocation, sum types (also required for self-hosting) |
| Web | Via WebAssembly only | Lemma implements the compute engine; UI, HTTP, I/O and database access stay in the host language |

## Consequences

- Features are judged against this scope. A proposal serving only an out-of-scope domain
  (for example DOM bindings, an HTTP framework, an ORM) is rejected or left to host languages.
- The C ABI and WebAssembly exports are first-class deliverables, not afterthoughts.
- Allocation-free operation and bounded execution time are design constraints from the start,
  so that robotics and embedded targets stay reachable.
