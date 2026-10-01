---
id: FR-343
title: "Compare a case's prior and superseding classes into one case result"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: traces_to
---
# FR-343: Compare a case's prior and superseding classes into one case result

## Description

When the spec-versioning gate (FR-340) has classified a case's run against
the prior revision (`P`) and against the superseding revision (`S`), each by
FR-342, it SHALL compare the two into one case result: `holds`,
`regression`, `unresolved (unsupported)`, `unresolved (incomplete)`, `not
applicable` or `tool failure`. The comparison is a pure function of the two
classes, their stages and the case's selection kind (`Clause`, `Function` or
`Frame`).

## Inputs

`P` and `S`, each `(RefinementClass, ClauseRunStage)` with its codes; the
case's selection kind.

## Outputs

One case result. A `regression` carries the case's name, both classes and
the superseding disposition's codes.

## Behavior

The comparison SHALL return the result of the first of these rows, in
order, that matches:

| `P` | `S` | Case result |
| --- | --- | --- |
| `tool failure` | any | `tool failure` |
| any | `tool failure` | `tool failure` |
| `incomplete`, at any stage | any | `unresolved (incomplete)` |
| any other class, at stage `compile` or `select` | any | `tool failure` (the prior revision must compile and select; the case is malformed) |
| `admitted` | `admitted` | `holds` |
| `admitted` | `absent`, for a `Clause` or `Frame` selection | `holds` (the constraint was dropped; a frame exists only while a clause or attempt names its operation, FR-105) |
| `admitted` | `refused`, `prohibited`, or `absent` for a `Function` selection | `regression` |
| `admitted` | `unsupported` | `unresolved (unsupported)` |
| `admitted` | `incomplete` | `unresolved (incomplete)` |
| not `admitted` | any | `not applicable` |

- A typed compile refusal of the superseding revision is class `refused`
  for every case of the pair, so every case the prior revision admits SHALL
  be a `regression`.
- An unresolved result SHALL never be promoted to `holds`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-343-AC-1 | For every combination of `P` class and stage, `S` class and stage, and selection kind, the comparison returns the result the table gives; each expected result in the test is a literal, not computed by the gate's code. | Test (TC-863) |
| FR-343-AC-2 | In the seeded pair, whose superseding revision tightens a ConfigVersion clause, the case the prior revision admits and the superseding refuses is a `regression` naming the case, `admitted`, `refused` and the superseding disposition's codes; the pair's violating-parent case, which the prior revision refuses, is `not applicable`. | Test (TC-864) |
| FR-343-AC-3 | A pair whose superseding revision removes the selected clause gives `holds` for its `Clause` case; a pair whose superseding revision removes the selected function gives `regression` for its `Function` case. | Test (TC-864) |
| FR-343-AC-4 | A pair whose superseding revision has a typed compile refusal gives `regression` for every case the prior revision admits and `not applicable` for every case it does not. | Test (TC-864) |
| FR-343-AC-5 | A pair whose superseding compile refuses `unsupported_construct`/`not-yet-implemented` gives `unresolved (unsupported)`; a case whose `accounting` work budget the prior run fits and the superseding run exceeds gives `unresolved (incomplete)`; a case whose prior run exceeds its work budget gives `unresolved (incomplete)`, not `not applicable`. None gives `holds`. | Test (TC-865) |

## Dependencies

- FR-342 (the two classes), FR-105 (a frame exists only while named).

## References

- ADR-017 §2 RF-2 "Comparison" (amended 2026-10-01: a prior class
  `incomplete` at any stage is unresolved), RF-5.
- Specification ticket QSL-386; implementation ticket QSL-40.

## Status

Specified; not yet implemented -- TC-863 to TC-865 planned.
