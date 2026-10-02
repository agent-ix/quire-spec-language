---
id: FR-275
title: "Take a typed request, caller limits and a cancellation handle on every lifecycle operation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: references
---
# FR-275: Take a typed request, caller limits and a cancellation handle on every lifecycle operation

## Description

QSL SHALL expose its lifecycle stages as typed library operations with one
call shape (ADR-029 LC-2, OP-1). The QSL-owned operations are `parse`,
`format`, `select`, `check`, `check_fences`, `package`, `execute`,
`analyze`, `monitor`, `replay`, `inspect` and `render`. `format` lives in the core library, layer 1
`qsl_cst::format`; the driver's `format` verb (ADR-029 CB-5) and the
qualification runner call it. The driver-owned operations `lower`, `generate` and
`prove` reach QSL through FR-280.

| Operation | Takes | Returns | Behaviour |
| --- | --- | --- | --- |
| `parse` | source bytes and `SourceIdentity` | `Staged<ParsedSource>` | FR-278 |
| `format` | an admissible `ParsedSource` | `Staged<FormattedSource>`, the formatted bytes | FR-003 |
| `select` | domain package documents by digest, the unit's model selections | `Staged<AdmittedModels>` | FR-278 |
| `check` | `ParsedSource`, `AdmittedModels`, the dependency input, lock evidence | `Staged<CheckedPackage>` | FR-278 |
| `check_fences` | a spec artifact inventory and the module manifests that type it | `Staged<FenceReport>` | FR-355 |
| `package` | `&CheckedPackage` | `Staged<EmittedPackage>` | FR-278 |
| `execute` | `&CheckedPackage`, a function or clause selection, arguments, `ObjectEnvironment`, the execution backend | `Result<Evaluation<Value>, CallFailure>` | FR-279 |
| `analyze` | `&CheckedPackage`, claim items, subject binding, engine settings | `Staged<AnalyzeOutcome>` | FR-281 |
| `monitor` | `&CheckedPackage`, clause selections, a trace document | `Staged<MonitorOutcome>` | FR-283 |
| `replay` | `ReplayRequest` with its byte provision | `Staged<ReplayResult>` | FR-098 |
| `inspect` | a package, outcome or graph, and a view selector | `Staged<View>` | FR-297 |
| `render` | an outcome or `View`, and the source byte provision | `Staged<Rendered>`, the rendered bytes | FR-298 |

Each operation also takes its limits value (FR-277) and `&Cancel` (FR-276).
A failure is `StageFailure<C>` with the operation's own cause type `C`
(ADR-013 T-4).

## Inputs

For each operation: its typed request, which names the source and artifact
identity and the profile it runs under; its limits value; `&Cancel`.

## Outputs

`Result<Staged<T>, StageFailure<C>>`, with `T` the operation's output, except
`execute`, which returns `Result<Evaluation<Value>, CallFailure>` (ADR-011
§2.3). `analyze` returns one FR-331 terminal record per requested item inside
`Staged` (ADR-013 O-24).

## Behavior

- Each QSL lifecycle operation shall take its typed request, its limits value
  and `&Cancel` as its parameters.
- Each operation that consumes an earlier stage's output shall take that
  output's own type (`ParsedSource`, `AdmittedModels`, `CheckedPackage`,
  `EmittedPackage`), so a program that passes another stage's value, raw
  bytes or an unchecked value does not compile.
- Each operation shall return the same outcome for the same request and
  limits whenever its `Cancel` handle is not cancelled.
- Each operation shall return a typed outcome for every input, and an
  internal invariant failure shall return `StageFailure::Fault` (or
  `CallFailure::Fault` for `execute`) rather than a panic across the public
  boundary.
- Each operation that takes an earlier stage's output shall run none of the
  stages that produced it: `check` runs S3 and S4 over its `ParsedSource`
  without re-running S1 or S2, and `package`, `execute`, `analyze` and
  `monitor` run no stage up to S4 over their `CheckedPackage`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-275-AC-1 | Over `tests/fixtures/spine-compile.native`, the chain `parse`, `select` (no domain packages), `check`, `package`, each called with default limits and an uncancelled `Cancel`, returns `Staged` values at every step, `format` over the `ParsedSource` returns the bytes FR-003 specifies for that source, and `package` returns the bytes `qsl_replay::spine::compile` writes for that source. `execute` on the resulting `CheckedPackage` selecting `seven` with no arguments completes with the integer 7. `inspect` with the package view and `render` of that view both return `Staged` values. | Test (TC-755) |
| FR-275-AC-2 | Passing raw source bytes to `check`, a `ParsedSource` to `package`, package bytes to `execute` or `analyze`, and a `ParsedSource` to `monitor`, each fails to compile (`compile_fail` doctests), and each has a compiling control with the correct type. | Test (TC-755) |
| FR-275-AC-3 | Calling each of the twelve operations twice with equal requests and limits returns equal outcomes. | Test (TC-755) |
| FR-275-AC-4 | A property test over arbitrary source bytes, arbitrary trace documents and arbitrary replay request bytes calls `parse`, `check`, `monitor` and `replay`; every call returns `Ok` or a typed `StageFailure`, and no call panics. | Test (TC-756) |
| FR-275-AC-5 | With each stage's work counter exposed through the outcome's accounting, `check` over the `ParsedSource` of FR-275-AC-1 reports zero S1 and S2 work, and `package`, `execute` and `monitor` over its `CheckedPackage` each report zero work for S1 to S4. | Test (TC-755) |

## Dependencies

- ADR-029 LC-2, OP-1: the operation set and the call shape.
- ADR-013 T-4: `Staged<T>` and `StageFailure<C>`.
- ADR-011 §2.3: `execute`'s result type.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): `Cancel`.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): limits.
- [FR-098](FR-098-execute-a-replay-request.md): `replay`'s behaviour.
- QSpec FR-300: the typed lifecycle API, limits and cancellation on every
  request, and wrong-stage input refusal, as the QSpec half in References aligns it.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50): ADR-029.
- QSpec FR-300 (STD-141): the QSpec half.
