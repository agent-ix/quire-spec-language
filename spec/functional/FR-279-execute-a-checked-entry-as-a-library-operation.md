---
id: FR-279
title: "Execute a checked function or clause as a library operation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-279: Execute a checked function or clause as a library operation

## Description

QSL SHALL expose `execute`, which calls a checked function or evaluates a
checked clause through the execution-backend seam (FR-294, ADR-029 OP-1).

FR-100's native-run/1 request for a `1-draft` program is the command encoding
of `execute` with a function selection; FR-109's `run_clause` is `execute`
with a clause selection.

## Inputs

```rust
pub fn execute(
    package: &CheckedPackage,
    request: &ExecuteRequest,
    backends: &SuppliedBackends<'_>,
    limits: &ExecuteLimits,
    cancel: &Cancel,
) -> Result<Evaluation<Value>, CallFailure>;

pub struct ExecuteRequest {
    pub selection: ExecuteSelection,
    pub environment: ObjectEnvironment,
    pub backend: ExecutionBackendChoice,
}

pub enum ExecuteSelection {
    Function { name: QualifiedName, arguments: Arguments },
    Clause { clause: ClauseSelection, input: ClauseInput },
}

pub struct ExecuteLimits {
    pub accounting: ScalarLimits,
    pub prepare: PrepareLimits,
}
```

- `&CheckedPackage`.
- The selection: a function's `QualifiedName` with its `Arguments`, or a
  `ClauseSelection` with its `ClauseInput`.
- The `ObjectEnvironment`.
- The execution backend the caller chose, `ExecutionBackendChoice`, and the
  backends the caller supplies, `SuppliedBackends` (FR-296).
- The limits: the accounting limits (`ScalarLimits`) and the
  `PrepareLimits` (FR-294). `ExecuteLimits::default()` is both defaults.
- `&Cancel`.

## Outputs

`Result<Evaluation<Value>, CallFailure>`: the `Evaluation` the backend's
`call` or `evaluate` returns, or a `CallFailure`: `Input`, `Fault` or
`Cancelled` from the call or from `prepare`; `Limit(PrepareLimitReached)`
(FR-294) when `prepare` reaches a `PrepareLimits` bound; `Unsupported` with the `UnsupportedEntry` when the
chosen backend is not supplied or did not prepare the entry.

## Behavior

- The `execute` operation shall prepare exactly the selected entry with the
  chosen execution backend and call it with the arguments, environment and
  accounting limits.
- If the chosen backend is not supplied, then `execute` shall return
  `CallFailure::Unsupported(UnsupportedEntry { backend, cause: NotSupplied
  })` naming the chosen backend, without calling any backend's `prepare`.
- If the chosen backend's `prepare` lists the entry `unsupported`, then
  `execute` shall return `CallFailure::Unsupported` with that entry's
  `UnsupportedEntry`.
- If `prepare` returns `PrepareFailure::Limit(LimitExceeded)`, then
  `execute` shall return `CallFailure::Limit(PrepareLimitReached)` holding
  that `LimitExceeded`'s setting, configured value and counter reached, and
  no locus.
- If `prepare` returns `PrepareFailure::Cancelled` or `Fault`, then
  `execute` shall return `CallFailure::Cancelled` or `Fault` with the same
  payload.
- The `execute` operation shall return the same `Evaluation` as
  `CheckedPackage::call` (for a function) or `CheckedPackage::evaluate` (for
  a clause) when the chosen backend is the interpreter.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-279-AC-1 | `execute` with the interpreter backend over FR-100-AC-4's unit returns, for `lt(5, 3)`, `flag(1)` and `id(4)`, the `Evaluation` `CheckedPackage::call` returns for the same arguments, and for `id(12)` the same `CallFailure::Input`. | Test (TC-760) |
| FR-279-AC-2 | `execute` with a clause selection over FR-109's ConfigVersion unit returns the `Evaluation` `qsl_replay::spine::run_clause` reports for the same clause and input. | Test (TC-760) |
| FR-279-AC-3 | For each FR-100-AC-1, AC-4 and AC-6 request, the `outcome` member of FR-100's `spine-run-result/1` document equals the rendering of `execute`'s result for the same source, function, arguments and accounting limits. | Test (TC-760) |
| FR-279-AC-4 | `execute` choosing a supplied test backend standing in for `aot` whose `prepare` returns `PrepareFailure::Limit` naming `prepare.code_bytes` with configured value 1 and counter 2 returns `CallFailure::Limit(PrepareLimitReached { setting: PrepareSetting::CodeBytes, bound: 1, actual: 2 })`, whose `setting.name()` is `prepare.code_bytes`, whose category is incomplete (exit 22) and which is built at the path `quire_eval::PrepareLimitReached`; with one whose `prepare` returns `PrepareFailure::Fault`, `CallFailure::Fault` with the same `InternalFault`. | Test (TC-760) |

## Dependencies

- ADR-029 OP-1: the `execute` row.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): the seam.
- [FR-296](FR-296-select-the-execution-backend-in-the-request.md): backend selection.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md), [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the command encodings.
- QSpec FR-300: the `execute` operation.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSpec FR-300 (STD-141): the QSpec half.
