---
id: FR-294
title: "Run every execution backend behind one seam that admits checked input only"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-294: Run every execution backend behind one seam that admits checked input only

## Description

One trait in layer 5 (`qsl-eval`, module `execution`) holds every execution
backend (ADR-029 EB-1):

- `ExecutionBackend::prepare(&self, package: &CheckedPackage, entries:
  &[EntrySelection], limits: &PrepareLimits, cancel: &Cancel)` returns
  `Result<Self::Prepared, PrepareFailure>`. An entry the backend cannot run
  settles `Unsupported` for that entry only.
- `PreparedEntries::call(entry, arguments, environment, accounting, cancel)`
  and `PreparedEntries::evaluate(clause, input, accounting, cancel)` each
  return `Result<Evaluation<Value>, CallFailure>`, with the contract of
  `CheckedPackageEvaluation::call` and `evaluate`.
- `PrepareFailure` holds per-entry `Unsupported` causes, `Limit`,
  `Cancelled` and `Fault`.

Three backends implement the seam: the interpreter in `qsl-eval`, which
accepts every entry S6a accepts and runs S6a; AOT in the driver's
`quire-aot`; and JIT in QSL's `qsl-jit` (layer X).

The interpreter is the reference: S6a defines what a checked package means,
and replay runs only through S6a (ADR-029 EB-3, ADR-011 FB-07).

`prepare` takes `&CheckedPackage`, which only S4 constructs (ADR-011 §4), so
every backend admits checked input only, by type (ADR-029 EB-2). AOT's
`generate` step takes the `EmittedPackage` that the same run's `check`
produced, and the IR reader checks it against that run's expected
`package_id` (ADR-011 E5).

## Inputs

As the trait signatures state.

## Outputs

As the trait signatures state.

## Behavior

- Layer 5 shall define `ExecutionBackend` and `PreparedEntries` with the
  signatures the Description states.
- The interpreter backend shall accept every entry S6a accepts, and its
  `call` and `evaluate` shall return what `CheckedPackage::call` and
  `CheckedPackage::evaluate` return for the same inputs.
- When a backend cannot run an entry, `prepare` shall settle that entry
  `Unsupported` with a typed cause and prepare the other selected entries.
- Each backend's `prepare` and each `PreparedEntries` method shall honour
  `&Cancel` as FR-276 states.
- The `replay` facade shall evaluate through S6a for every request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-294-AC-1 | The interpreter backend, prepared over FR-100-AC-4's unit with every function selected, returns for each AC-4 call the `Evaluation` or `CallFailure` that `CheckedPackage::call` returns for the same arguments and accounting limits; for FR-109's ConfigVersion clause, `evaluate` returns what `CheckedPackage::evaluate` returns. | Test (TC-778) |
| FR-294-AC-2 | Passing package bytes, a `ParsedSource` or an unchecked expression to `prepare`, and package bytes read from a file to AOT's `generate` step, each fails to compile (`compile_fail` doctests with compiling controls). | Test (TC-778) |
| FR-294-AC-3 | A test backend that cannot run one of three selected entries returns a prepared value for the other two and a `PrepareFailure` entry cause `Unsupported` for the third; calling the two prepared entries returns their interpreter results. | Test (TC-778) |
| FR-294-AC-4 | `prepare` and `call` with a handle already cancelled return `PrepareFailure::Cancelled` and `CallFailure::Cancelled`. | Test (TC-778) |

## Dependencies

- ADR-029 EB-1 to EB-3: the seam, checked-only input and the reference.
- ADR-011 §4, E5, FB-07: construction, the IR reader and replay.
- ADR-013 T-4: `CallFailure`.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): cancellation.
- [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md): `execute`.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- QSpec FR-306: AOT and JIT execution, as the QSpec half in References aligns it.

## Overlap

The driver repository owns `quire-aot`: lowering, generation with the AOT
target, the toolchain build and the child-process run.

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- QSL-390 (ARCH-50): owner ruling 3, recorded on the ticket.
- QSpec FR-306 (STD-141): the QSpec half.
