---
id: FR-568
title: "Confine unsafe code in qsl-jit to one module and enforce the module list"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
---
# FR-568: Confine unsafe code in qsl-jit to one module and enforce the module list

## Description

Entering generated machine code needs unsafe code. ADR-035 CG-4 rules that
it lives in exactly one module, `qsl_jit::exec`, under a stated safety
contract. This requirement makes the ruling checkable.

- `qsl-jit` SHALL declare `unsafe_code = "deny"` in its `[lints.rust]`, and
  `#[allow(unsafe_code)]` SHALL appear in `qsl-jit` only on the declaration
  `mod exec`.
- Every other package of the QSL workspace SHALL keep `unsafe_code`
  forbidden: `unsafe_code = "forbid"` in its `[lints.rust]`, directly or
  through `[lints] workspace = true`, or `#![forbid(unsafe_code)]` at each of
  its crate roots.
- `exec` SHALL do only what ADR-035 CG-4's safety contract lists, and the
  safe wrapper `PreparedJit` SHALL uphold the contract's invariants.
- The `extern "C"` trampolines and shims that generated code calls (the
  entries of FR-561's `KernelTable` and the helpers that adapt them) SHALL
  be defined inside `qsl_jit::exec`, each a module symbol fixed at link time:
  no function pointer to a shim is held in writable frame memory. The frame
  the generated code runs over is `EntryFrame`, a typed structure defined in
  `exec` that holds the argument handles, the meter and the call's result and stop
  record, and holds neither a `KernelTable` nor any function pointer;
  cancellation is polled through the meter
  (FR-277). The one entry signature is `extern "C" fn(*const EntryFrame) -> u32`, and no
  opaque handle crosses the boundary of `exec` in its place. No `unsafe` item, and no `extern "C"` function that
  generated code calls, exists outside `exec`.
- `tools/arch-lint` SHALL provide the check `unsafe-confinement`, run as
  `make arch-lint-unsafe` and as part of `make arch-lint` and `make ci`. It
  holds the allowed module list, exactly one entry `(qsl-jit, exec)`, and
  fails, naming the package, file and item, when:
  - a package other than `qsl-jit` does not keep `unsafe_code` forbidden in
    one of those ways;
  - `qsl-jit` does not declare `unsafe_code = "deny"`, or declares
    `forbid` (which no `allow` can lift) or `allow`;
  - an `allow(unsafe_code)`, `expect(unsafe_code)` or `warn(unsafe_code)`
    attribute appears anywhere except on `mod exec` in `qsl-jit`;
  - an `unsafe` block, `unsafe fn`, `unsafe impl` or `unsafe trait` appears
    outside the files of `qsl_jit::exec`, in any workspace package, including
    files a crate reaches through `#[path]` or `include!`;
  - an `extern "C"` function defined outside the files of `qsl_jit::exec`
    is a call target of generated code (it is named in a `KernelTable`).

## Inputs

The workspace manifests and the source trees of its packages.

## Outputs

The `unsafe-confinement` check's pass, or its findings with package, file
and item; `PreparedJit` and its calls.

## Behavior

- The `unsafe-confinement` check shall fail when any workspace package other
  than `qsl-jit` does not keep `unsafe_code` forbidden.
- The `unsafe-confinement` check shall fail when `qsl-jit` does not declare
  `unsafe_code = "deny"`.
- The `unsafe-confinement` check shall fail when an attribute that lowers
  `unsafe_code` appears anywhere but on `mod exec` in `qsl-jit`.
- The `unsafe-confinement` check shall fail when unsafe code appears outside
  the files of `qsl_jit::exec`.
- `PreparedJit` shall convert to the entry signature only addresses it
  recorded at generation for that signature.
- The `EntryFrame` shall hold no function pointer and no `KernelTable`.
- When an entry returns a status outside the closed status type, the
  `PreparedJit` wrapper shall return `Fault`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-568-AC-1 | `make arch-lint-unsafe` passes on the workspace, and its allowed module list is exactly `(qsl-jit, exec)`. Over fixture workspaces it fails, naming package, file and item, for each of: an `unsafe` block in another `qsl-jit` module; an `unsafe fn` in `qsl-eval`; an `unsafe impl` in a file reached through `#[path]`; an `unsafe` block in a file brought in with `include!`; `#[allow(unsafe_code)]` on a second `qsl-jit` module; `qsl-jit` declaring `forbid`; and a package other than `qsl-jit` declaring `deny` instead of `forbid`; an `extern "C"` shim in a `qsl-jit` module other than `exec` that the fixture's table names; a fixture package that forbids `unsafe_code` only by `#![forbid(unsafe_code)]` at its crate root passes. `make arch-lint` and `make ci` list `arch-lint-unsafe` as a prerequisite. | Test (TC-1400) |
| FR-568-AC-2 | A `compile_fail` doctest that keeps a callable entry of a `PreparedJit` past the `PreparedJit`'s drop fails to compile, with a compiling control that calls within the borrow. | Test (TC-1400) |
| FR-568-AC-3 | `PreparedJit` refuses, with `Fault` and without calling, an entry whose recorded signature is not the entry signature (a test hook records one wrongly); a test entry that returns a status outside the closed status type yields `Fault` and no value. | Test (TC-1400) |
| FR-568-AC-4 | The FR-561-AC-3 call-target check and the FR-563 parity gate pass for the Cranelift JIT over the default generated units, with every call entering generated code through `qsl_jit::exec` and every `extern "C"` call target of the generated blocks defined in `qsl_jit::exec`. | Test (TC-1400) |
| FR-568-AC-5 | A `compile_fail` doctest outside `qsl_jit::exec` that names `EntryFrame` or builds one fails to compile, and a doctest that calls through `PreparedJit` compiles; for every entry of the default generated units the Cranelift signature `PreparedJit` recorded at generation is one pointer parameter and a `u32` result, the signature of `extern "C" fn(*const EntryFrame) -> u32`, and a test hook that records a two-parameter signature makes `PreparedJit` refuse with `Fault` as in AC-3. A test inside `exec` over `EntryFrame`'s field types finds no function-pointer or `KernelTable` type (FR-561-AC-4). | Test (TC-1400) |

## Dependencies

- ADR-035 CG-4: the ruling and the safety contract.
- ADR-011 T-12: arch-lint.
- [FR-561](FR-561-call-the-exact-kernel-from-generated-code-through-one-table.md): the call-target check.
- [FR-563](FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md): the parity gate.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): the seam `PreparedJit` implements.

## References

- QSL-12: the Layer 5 program.
