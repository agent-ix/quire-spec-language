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
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-279: Execute a checked function or clause as a library operation

## Description

QSL SHALL expose `execute`, which calls a checked function or evaluates a
checked clause through the execution-backend seam (FR-294, ADR-029 OP-1).

FR-100's native-run/1 request for a `1-draft` program is the command encoding
of `execute` with a function selection.

### Clause execution and `run_clause`

A clause selection evaluates one checked state clause over the observations
FR-106 admitted for it: `execute` and `PreparedEntries::evaluate` take the
clause's `QualifiedName` and that `AdmittedObservations` value, which are
exactly the inputs of S6a's clause entry
`CheckedPackageEvaluation::evaluate_clause` (FR-107). `execute` does not
compile, select or admit. FR-109's `qsl_replay::spine::run_clause` is the
reference pipeline that does: it compiles the unit, resolves the clause by
name, admits the request's observations by FR-106 and calls `evaluate_clause`
directly, keeping the `ClauseEvaluation`'s decision trail for the separating
witness (FR-268) and the consequent source events. The interpreter's
`evaluate` calls the same `evaluate_clause` and returns its `evaluation`
member. So clause execution has one evaluator, S6a's clause entry, and two
callers; `run_clause` does not route through `execute`. This is the simplest
consistent design for three reasons: the seam takes a `CheckedPackage` and
returns `Evaluation<Value>`, so it can neither run `run_clause`'s source
front end nor carry the decision trail `run_clause` reports; FR-106
admission needs the document provisions, package input and observation
limits, which belong to a request, not to a prepared backend; and
`run_clause` is a replay path, and replay runs only through S6a (ADR-029
EB-3), so `run_clause` takes no chosen backend. EB-3 restricts replay
only: a function call through the spine's function entries
(`qsl_replay::spine::run`, `run_with_backends`) is execution, not replay,
and runs on the call's chosen backend as FR-296 states.

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
    pub backend: ExecutionBackendChoice,
}

pub enum ExecuteSelection {
    Function {
        name: QualifiedName,
        arguments: Vec<Value>,
        environment: ObjectEnvironment,
    },
    Clause {
        clause: QualifiedName,
        input: AdmittedObservations,
    },
}

pub struct ExecuteLimits {
    pub accounting: ScalarLimits,
    pub supplied: SuppliedLimits,
    pub prepare: PrepareLimits,
}
```

- `&CheckedPackage`.
- The selection, one of:
  - a function: its `QualifiedName`, its argument values in parameter order
    (`Vec<Value>`, what `CheckedPackageEvaluation::call` takes) and the
    `ObjectEnvironment` the call reads objects from;
  - a state clause: its `QualifiedName` and the `AdmittedObservations`
    (FR-106) admitted for that clause, which supply every object and value
    the clause reads, so a clause selection has no `ObjectEnvironment`.
- The execution backend the caller chose, `ExecutionBackendChoice`, and the
  backends the caller supplies, `SuppliedBackends` (FR-296).
- The limits: the accounting limits (`quire_exact::ScalarLimits`), the
  supplied-value limits (`SuppliedLimits`, FR-277) and the `PrepareLimits`
  (FR-294). `ScalarLimits` has every member required and no `Default`, so
  `ExecuteLimits` has no `Default` and the caller supplies `accounting`
  explicitly (FR-277 Inputs), for example from FR-255's shared accounting
  table; `SuppliedLimits::default()` and `PrepareLimits::default()` give the
  other two fields' published defaults.
- `&Cancel`.

## Outputs

`Result<Evaluation<Value>, CallFailure>`: the `Evaluation` the backend's
`call` or `evaluate` returns, or a `CallFailure`: `Input`, `Fault` or
`Cancelled` from the call or from `prepare`; `Limit(PrepareLimitReached)`
(FR-294) when `prepare` reaches a `PrepareLimits` bound; `Unsupported` with the `UnsupportedEntry` when the
chosen backend is not supplied or did not prepare the entry.

## Behavior

- The `execute` operation shall prepare exactly the selected entry with the
  chosen execution backend under `limits.prepare`, the one `PrepareLimits`
  that applies to whichever backend is chosen.
- For a function selection, the `execute` operation shall call the prepared
  entry through `PreparedEntries::call` with the arguments, the environment
  and `CallLimits { accounting: limits.accounting, supplied:
  limits.supplied }`.
- For a clause selection, the `execute` operation shall evaluate the
  prepared clause through `PreparedEntries::evaluate` with the admitted
  observations and `limits.accounting`; `limits.supplied` bounds supplied-value
  admission of function arguments only and is not read for a clause, whose
  input FR-106 has already admitted.
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
- When the chosen backend is the interpreter, the `execute` operation shall
  return, for a function, what `CheckedPackageEvaluation::call_supplied`
  returns under a supplied-value run built from `limits.supplied` and the
  caller's `Cancel` (`CheckedPackage::call` at the default `SuppliedLimits`),
  and for a clause, the `evaluation` member of what
  `CheckedPackageEvaluation::evaluate_clause` returns for the same clause,
  observations and accounting limits, or its `CallFailure`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-279-AC-1 | `execute` with the interpreter backend over FR-100-AC-4's unit returns, for `lt(5, 3)`, `flag(1)` and `id(4)`, the `Evaluation` `CheckedPackage::call` returns for the same arguments, and for `id(12)` the same `CallFailure::Input`. | Test (TC-760) |
| FR-279-AC-2 | Over FR-109's ConfigVersion unit, for the FR-109-AC-1 healthy-parent and violating-parent requests, `execute` with the interpreter and a clause selection naming `ParentOrder`, with the `AdmittedObservations` FR-106 admits for it from the request's snapshot, returns `Ok` with the `evaluation` member of `CheckedPackageEvaluation::evaluate_clause` for the same clause, observations and accounting limits; FR-100's outcome mapping of that `Evaluation` is `Completed(Boolean(true))` and `Completed(Boolean(false))` respectively, equal to the `Evaluate` disposition `qsl_replay::spine::run_clause` reports for the same request. | Test (TC-760) |
| FR-279-AC-3 | For each FR-100-AC-1, AC-4 and AC-6 request, the `outcome` member of FR-100's `spine-run-result/1` document equals the rendering of `execute`'s result for the same source, function, arguments and accounting limits. | Test (TC-760) |
| FR-279-AC-4 | `execute` choosing a supplied test backend standing in for `aot` whose `prepare` returns `PrepareFailure::Limit` naming `prepare.code_bytes` with configured value 1 and counter 2 returns `CallFailure::Limit(PrepareLimitReached { setting: PrepareSetting::CodeBytes, bound: 1, actual: 2 })`, whose `setting.name()` is `prepare.code_bytes`, whose category is incomplete (exit 22) and which is built at the path `quire_eval::PrepareLimitReached`; with one whose `prepare` returns `PrepareFailure::Fault`, `CallFailure::Fault` with the same `InternalFault`. | Test (TC-760) |
| FR-279-AC-5 | A `compile_fail` doctest that builds `ExecuteLimits` with `..Default::default()` and no `accounting` fails to compile, and a control that states `accounting` explicitly with `SuppliedLimits::default()` and `PrepareLimits::default()` compiles. Over FR-100-AC-4's unit, `execute` with the interpreter for `id(4)` under `supplied.admission_work_units` 0 returns the same `CallFailure::Supplied` that `CheckedPackageEvaluation::call_supplied` returns under a supplied-value run with those `SuppliedLimits`, while the same request under `SuppliedLimits::default()` returns the `Evaluation` of AC-1. | Test (TC-760) |

## Dependencies

- ADR-029 OP-1: the `execute` row.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): the seam.
- [FR-296](FR-296-select-the-execution-backend-in-the-request.md): backend selection.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the command encoding; [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the reference clause pipeline.
- [FR-106](FR-106-admit-snapshots-and-invocations.md), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md): the admitted clause input and S6a's clause entry.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): the supplied-value limits and explicit supply of the accounting limits.
- QSpec FR-300: the `execute` operation.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSpec FR-300 (STD-141): the QSpec half.
