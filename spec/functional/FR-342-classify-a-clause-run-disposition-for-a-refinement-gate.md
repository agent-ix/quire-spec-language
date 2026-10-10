---
id: FR-342
title: "Classify a clause-run disposition into one refinement class"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: depends_on
---
# FR-342: Classify a clause-run disposition into one refinement class

## Description

When the spec-versioning gate (FR-340) has a `ClauseRunReport` from
`run_clause`, it SHALL classify the report's disposition once into a
refinement class and keep the disposition's stage beside it. The
classification reads the typed disposition, never message text (ADR-011
FB-02).

## Inputs

A `ClauseRunReport` (FR-109).

## Outputs

`(RefinementClass, ClauseRunStage)`: one of FR-341's classes or `absent`,
the FR-109 stage (`compile`, `select`, `admit` or `evaluate`), and the
disposition's codes and causes. An `incomplete` class also carries the
limit reached and its value.

## Behavior

The classification SHALL be one exhaustive `match` over
`ClauseDisposition` with no wildcard arm, giving:

| `ClauseDisposition` | Class |
| --- | --- |
| `Compile(r)` | FR-341's class of `r` (`admitted` cannot occur) |
| `UnknownLanguage`, `StalePackage` | `tool failure` (the gate supplies neither an extracted source nor an expected `package_id`) |
| `MissingName` | `absent` |
| `NotAPredicate` | `refused` |
| `Admit(Refused)`, `ArgumentRefusal` | `refused` |
| `Admit(Incomplete)` | `incomplete` |
| `Admit(Fault)`, `EvaluateFault` | `tool failure` |
| `Evaluate(Completed(Boolean(true)))` | `admitted` |
| `Evaluate(Completed(Boolean(false)))`, `Evaluate(Refused)`, `FrameViolation` | `refused` |
| `Evaluate(Refuted)` with cause `UndefinedEvaluation{where, cause}` (an undefined evaluation settles refuted) | `refused` |
| `Evaluate(Completed(Integer))` | `tool failure` (FR-109 selects only `Boolean` functions, so it cannot occur) |
| `Evaluate(Incomplete)` | `incomplete` |

- For an `incomplete` class the classification SHALL carry the limit the
  disposition names (an admission limit, a model limit or FR-100's
  `work_units`) and its value.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-342-CON-1 | The classification matches `ClauseDisposition` with no wildcard arm and reads no message text. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-342-AC-1 | Over FR-109's ConfigVersion fixtures: healthy-parent classifies `admitted` at stage `evaluate`; violating-parent classifies `refused` at `evaluate`; FR-115's forbidden-parent-change frame run classifies `refused` at `evaluate`. | Test (TC-862) |
| FR-342-AC-2 | A `Clause` selection naming `Absent` classifies `absent` at `select`; a `Function` selection of a function returning `Integer` classifies `refused` at `select`; dangling-parent classifies `refused` at `admit`; a `Function` selection with an argument naming an unknown parameter classifies `refused` at `admit`. | Test (TC-862) |
| FR-342-AC-3 | incomplete-population classifies `incomplete` at `admit`, naming its limit; exhausted-work (work budget zero) classifies `incomplete` at `evaluate`, naming `work_units` and the value zero. | Test (TC-862) |
| FR-342-AC-4 | A `StalePackage` and an `UnknownLanguage` disposition, an `Admit(Fault)` and an `EvaluateFault` each classify `tool failure`. | Test (TC-862) |
| FR-342-AC-5 | A run whose compile refuses classifies at stage `compile` with exactly the class FR-341 gives its `CompileRefusal`: `missing-model` (no package supplied) classifies `tool failure` at `compile`. | Test (TC-862) |

## Dependencies

- FR-109 (`ClauseRunReport`, `ClauseDisposition`), FR-115
  (`FrameViolation`), FR-341 (compile classification), FR-100 (`work_units`).

## References

- ADR-017 §2 RF-2 "Run classification".
- Specification ticket QSL-386; implementation ticket QSL-40.
