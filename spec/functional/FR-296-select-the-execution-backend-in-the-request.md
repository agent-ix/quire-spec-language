---
id: FR-296
title: "Select the execution backend in the request, with no substitution"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-296: Select the execution backend in the request, with no substitution

## Description

The caller picks the execution backend in the `execute` request (ADR-029
EB-8). The choice is one of the seam's closed set: `interpreter`, `aot` or
`jit`, with `interpreter` as the default. The CLI spells the choice
`--engine interpreter|aot|jit`.

The choice is `ExecutionBackendChoice { Interpreter, Aot, Jit }`, the
`backend` member of `ExecuteRequest` (FR-279), whose `Default` is
`Interpreter`. The binary links the backends it supports and passes them to
`execute` as `SuppliedBackends`:

```rust
pub struct SuppliedBackends<'a> {
    pub aot: Option<&'a dyn SuppliedBackend>,
    pub jit: Option<&'a dyn SuppliedBackend>,
}

pub trait SuppliedBackend {
    fn prepare_entries(
        &self,
        package: &CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Box<dyn PreparedEntries + '_>, PrepareFailure>;
}

impl<B: ExecutionBackend> SuppliedBackend for B { /* boxes B::Prepared */ }
```

`ExecutionBackend` has an associated `Prepared` type, so it is not
dyn-compatible. `SuppliedBackend` is its object-safe form, implemented for
every `ExecutionBackend` by boxing the prepared value. The interpreter is
always available, because `qsl-eval` owns it, so `SuppliedBackends` has no
interpreter member and `SuppliedBackends::default()` supplies the
interpreter alone. A choice whose backend the caller has not supplied
refuses before any stage as unsupported (exit 21), with
`CallFailure::Unsupported` cause `NotSupplied`. A name outside the set refuses as an unknown argument
(exit 20). The chosen backend runs, or the request refuses; another backend
never runs in its place.

## Inputs

The backend choice in the `execute` request, and the backends the caller
supplies.

## Outputs

`execute`'s outcome from the chosen backend, or an unsupported refusal
before any stage.

## Behavior

- The `execute` request shall name its backend from the closed set
  `interpreter`, `aot`, `jit`, and shall default to `interpreter`.
- If the chosen backend is `aot` or `jit` and its `SuppliedBackends` member
  is `None`, then `execute` shall return `CallFailure::Unsupported` with
  `UnsupportedEntry { backend, cause: NotSupplied }`, naming the backend,
  before any stage.
- If the chosen backend lists the entry `unsupported` (FR-294), then
  `execute` shall return `CallFailure::Unsupported` with that entry's
  `UnsupportedEntry`.
- The `execute` operation shall run only the chosen backend.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-296-AC-1 | An `execute` request that names no backend runs `seven` on the interpreter and completes with 7. | Test (TC-781) |
| FR-296-AC-2 | An `execute` request choosing `jit` with `SuppliedBackends::default()` returns `CallFailure::Unsupported(UnsupportedEntry { backend: Jit, cause: NotSupplied })`, category unsupported (exit 21), and no stage runs. | Test (TC-781) |
| FR-296-AC-3 | With a test JIT that lists one entry `unsupported` with cause `Untranslatable`, `execute` of that entry returns `CallFailure::Unsupported` with that `UnsupportedEntry`, category unsupported (exit 21), and the interpreter is never called. | Test (TC-781) |
| FR-296-AC-4 | `ExecutionBackendChoice::default()` is `Interpreter`; a test JIT passed as `SuppliedBackends { aot: None, jit: Some(&jit) }` through the blanket `SuppliedBackend` implementation runs `seven` when `jit` is chosen and is the only backend whose `prepare` is called, and the call completes with 7. | Test (TC-781) |

## Dependencies

- ADR-029 EB-8: backend selection.
- [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md): `execute`.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): the seam.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): exit codes.
- QSpec FR-301: commands never select a fallback target.

## Overlap

The driver repository specifies and tests the `--engine` option against
QSpec FR-306: a name outside the set refuses as an unknown argument
(exit 20), and a backend the binary does not link refuses as unsupported
(exit 21).

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- QSpec FR-306 (STD-141): the QSpec half.
