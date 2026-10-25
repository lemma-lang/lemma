# 10. The JIT: object loading through the ORC C API, and the call contract

- Status: accepted
- Date: 2026-10-25

## Context

ADR 0001 sends code generation through inkwell and the JIT through ORC, because inkwell has no
ORCv2 wrapper. It leaves three things open or unstated: whether ORC is reached through the C API
(`llvm-sys`) or a C++ shim, where the pinned target configuration that the JIT and the final binary
share is defined, and what a caller of JIT-compiled code must guarantee. `docs/design.md` also
described JIT memory on macOS as `MAP_JIT` with `pthread_jit_write_protect_np()`, which no code
used.

The first JIT loads relocatable objects emitted by `lemma-codegen` into an ORC LLJIT and calls
their functions from Rust. Object files carry no types, so nothing at that boundary can check that
a symbol is the function the caller believes it is. This ADR extends ADR 0001 and changes none of
its decisions.

## Decision

### ORC through its C API, no C++ shim

`lemma-jit` calls ORC through `llvm-sys` 221, declared with no features so that it unifies with the
`llvm-sys` inkwell already depends on: one crate, one LLVM link, and the link mode stays inkwell's.
Creating an LLJIT, adding an object, looking up a symbol and disposing of the JIT are all in the C
API (`llvm_sys::orc2::lljit`), so a shim would add a C++ build to every CI image for nothing these
operations need. A shim may come back for what the C API lacks, first a `MAP_JIT` memory manager
(below).

### The object route, not the IR route

The JIT takes the **bytes** of a relocatable object and copies them into an LLVM memory buffer it
owns, which ORC then takes. It never receives an LLVM module: handing a module to ORC would hand it
inkwell's `Context`, and no inkwell value crosses into ORC. The objects are the ones the final
binary links (ADR 0001), emitted by the one route from a module to object bytes,
`lemma_codegen::emit_object`.

### One pinned target, defined once

The pinned configuration of each target lives in `lemma-codegen` as a closed `Target` enum, with
the triple, CPU and features of ADR 0003, and a single constructor of its LLVM target machine:
position-independent code, small code model, highest optimization level. Three variants exist, the
three CI hosts:

| `Target` | Triple | CPU | Features |
|---|---|---|---|
| `AppleAarch64` | `aarch64-apple-darwin` | `apple-m1` | none |
| `X86_64Linux` | `x86_64-unknown-linux-gnu` | `x86-64-v3` | none |
| `Aarch64Linux` | `aarch64-unknown-linux-gnu` | `generic` | `+neon,+fp-armv8` |

The other targets of ADR 0003 are added with their code generation. `emit_object` and the JIT both
build their target machine from this constructor: the JIT's ORC target machine builder is created
from that target machine, never from LLVM's host detection. For objects produced by `emit_object`,
one definition is what makes "tested code is shipped code" structural rather than a convention
between two crates: the JIT runs the code the final binary links.

The guarantee does not extend to bytes from elsewhere. The JIT's target machine is pinned, but an
object's CPU is not visible in its header (below), so an object emitted for another CPU of the
same architecture loads, links and resolves, and may fault when called. Observed: an object using
SME instructions, emitted for `apple-m4`, passed `add_object` and lookup, then stopped on an
illegal instruction (`SIGILL`) on an M1 Pro.

A JIT runs the code it loads, so `Jit::new` refuses a target that is not the host's pinned target
(`Target::host()`, from the running process's architecture and operating system). The pinned CPU
is not compared with the host CPU: running `x86-64-v3` code on a CPU without AVX2 is the caller's
responsibility until that check exists.

### The object header is checked before ORC sees it

Before an object reaches ORC, its header is read: 64-bit little-endian ELF of type `ET_REL` with
machine 62 (x86-64) or 183 (AArch64), or 64-bit Mach-O of file type `MH_OBJECT` with CPU type
arm64. Bytes that are no relocatable object are refused as such; a relocatable object for another
target than the JIT's is refused naming the JIT's target. Without this check, JITLink would link an
object built for a foreign architecture and the first call would run its bytes as host code. The
check reads only the fields that name format, kind and machine: an ELF object's OS ABI, a Mach-O
CPU subtype and the CPU features an object assumes are not checked, and the rest of the file is
left to LLVM.

`add_object` is a safe function, and the header check is not a boundary for untrusted input. Past
the header, the bytes go to LLVM's object parser and to JITLink, which LLVM does not promise to be
memory-safe on hostile input. About 7,000 mutated objects produced no crash, so the risk is
theoretical, not excluded. `add_object` is therefore documented as taking the bytes of
`emit_object` or of a trusted producer. Giving objects a type closes both this gap and the CPU gap
above: `emit_object` returning an `Object` that carries its target, `add_object` accepting only
that, and raw bytes entering through an `unsafe` constructor whose claim is the provenance of the
bytes. That type is a later change.

### The call contract: one unsafe claim, at lookup

The only `unsafe` step of a call is the **signature claim** made when a symbol is looked up
(`unsafe fn Jit::function::<F>`), as `libloading` does for a shared library. The claim is
conditional. Looking up a symbol that no loaded object defines is allowed and returns an error. If
the symbol resolves, the caller claims that it is a function with the C calling convention and
exactly the parameter and return types of `F`, and that running it for any argument values has no
undefined behavior and touches no memory outside its own stack and the objects loaded into the same
JIT. Code that calls host functions, which the JIT resolves (below), is covered by the same claim:
what those calls touch is part of what the caller vouches for.

What the claim makes safe is enforced by types:

- The returned `Function<'jit, F>` borrows the `Jit`, so it cannot outlive the code it points to;
  dropping the JIT while a function is alive does not compile. `Function::call` is safe.
- `F` is a sealed `Signature`: an `extern "C" fn` of zero to four parameters, each one of `i8`,
  `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, returning one of them or `()`.
  Pointers and `usize` are excluded so that a correct claim cannot let the callee reach Rust memory
  through an argument; `bool` is excluded because a returned byte other than 0 or 1 is undefined
  behavior on the Rust side.

Nothing at this boundary can check the claim, since objects carry no types. A signature verified by
Lemma's checker can later back a safe constructor, for functions whose verified contract implies
the claim.

### JIT memory: LLVM's default manager, W^X by `mprotect`

The JIT uses LLVM's default JITLink in-process memory manager. It maps an object's pages
read-write, copies and relocates the code, then makes them read-execute with `mprotect`: a page is
never writable and executable at once. It does not use `MAP_JIT`. This is observed working on
Apple Silicon under macOS 26, in unsigned test binaries. The test suite that exercises it is run by
CI on its three hosts (macOS 15 on Apple Silicon, Linux x86-64 and Linux AArch64); its Linux
results have not been observed yet.

Under the Hardened Runtime, which a signed and notarized `lemma` release runs under, Apple documents
that writable-then-executable memory requires `MAP_JIT` and the `com.apple.security.cs.allow-jit`
entitlement (or the broader `allow-unsigned-executable-memory` entitlement, which weakens the
runtime for the whole process). The default manager is therefore expected to fail there; a signed
release needs a `MAP_JIT` memory manager toggling `pthread_jit_write_protect_np()`, which the C API
cannot provide without a shim. This has not been tested.

Linux has the counterpart of that limit. Under an SELinux policy that denies executable anonymous
memory (`execmem`, denied in some Fedora or RHEL configurations), the step that makes mapped pages
executable is refused, so the default manager is expected to fail there too. This has not been
tested either.

### Observed ORC behavior

These facts were observed on Apple Silicon, unless a fact says otherwise, and are relied on. The
tests that check them are run by CI on its three hosts; their observation on Linux is pending,
since the call tests have not yet run there.

- LLJIT applies the target's symbol prefix at lookup: a function is looked up as `add`, not as
  `_add`, on Mach-O as on ELF.
- An object truncated past its header, or defining a symbol already defined in the same JIT, is
  rejected when it is added, not at the first lookup.
- A call between two functions of one object resolves to the object's own definitions.
- One object can be loaded into two JITs, and is callable from both.
- An absolute symbol defined in module-level assembly (`.globl zero` with `.set zero, 0`) ends in
  a lookup error on both formats, at a different point. On Mach-O, JITLink takes it as a
  definition, and the lookup reaches its address before the JIT reports it resolved to no
  address; on ELF (observed in CI on Linux x86-64 and AArch64, LLVM 22 from apt.llvm.org), the
  link fails with "Missing definitions in module ...: [ zero ]" and the lookup errors before any
  address.
- LLVM's execution session writes link errors to the process's standard error, through its default
  error reporter, besides the error the failing operation returns. A command line whose output is
  structured diagnostics will need to install its own reporter (the C API has
  `LLVMOrcExecutionSessionSetErrorReporter`) or capture that output.

Host symbols are resolved. By default, LLJIT links the process's symbols: it creates a JITDylib
holding a generator for the symbols of the running process and places it in the main JITDylib's
link order, and the C API (`llvm-c/LLJIT.h`) has no setter to turn this off. An object that
references an external symbol, such as the `memcpy` or `memset` LLVM inserts for some loops, but
equally `abs`, `getpid` or any other symbol of the process, is bound to the host's definition at
link time. A direct lookup of a host symbol through the JIT still fails, since lookup searches the
main JITDylib, which holds only the loaded objects. This is the current behavior, not a chosen
boundary: nothing yet keeps JIT code from calling any function of the process.

## Consequences

- `lemma-jit` depends on `lemma-codegen` and on `llvm-sys`; its `unsafe` is the FFI to ORC, the
  transmute from a resolved address to a function pointer, and the signature claim, each block with
  its `// SAFETY:` comment.
- `add_object` takes the JIT mutably and a `Function` borrows it, so no object can be added while a
  function looked up from the same JIT is alive. Hot replacement will revisit this: replacing an
  object behind live function handles is what ADR 0001's stubs are for.
- `Jit` is neither `Send` nor `Sync`. Making it `Send` is a separate decision, against ORC's
  threading rules.
- Lemma code that LLVM lowers to calls to `memcpy`, `memset` or a math library links today,
  through LLJIT's process-symbols JITDylib, and so would code calling any other process symbol.
  Restricting host symbols to a named list (`memcpy`, `memmove`, `memset`, and what lowering adds
  later) so that JIT code cannot bind to arbitrary process symbols is a later change. Since the C
  API cannot remove LLJIT's default, the alternative is to load objects into a bare JITDylib,
  outside the main JITDylib's link order, with a generator limited to that list.
- `add_object` trusts the provenance of its bytes, and the JIT's pinned CPU says nothing about the
  CPU an object was emitted for. Both are closed by an `Object` type, produced by `emit_object`
  with its target and the only input `add_object` accepts, with raw bytes behind an `unsafe`
  constructor.
- For verification: the checker can discharge the signature claim for a Lemma function whose
  parameters and result are scalars, which touches no region (no `ro`, `wo` or region effect), and
  whose contract has no `req` and no bounded parameter. A function with a precondition does not run
  without undefined behavior for any argument values: it is called through its checked variant
  (ADR 0005), or through a claim that carries the precondition, not through this one.
- For codegen: no new LLVM fact. The JIT emits nothing; it loads what `emit_object` produced for the
  same pinned target machine as the final binary, and objects from another producer at the
  caller's word.

Not decided here, each left to a later change: hot replacement through resource trackers and
stubs (ADR 0001), `Send` for `Jit`, checking the host CPU's features against the pinned CPU, the
`Object` type, restricting host symbols to a named list (or a bare JITDylib), a `MAP_JIT` memory
manager for the Hardened Runtime, JIT memory under an SELinux policy that denies `execmem`, and an
error reporter that routes link errors into Lemma's diagnostics.
