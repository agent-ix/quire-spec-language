---
id: FR-173
title: "Classify hyper and relation clauses into forms and record their requirements"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
---
# FR-173: Classify hyper and relation clauses into forms and record their requirements

## Description

S3 SHALL classify every checked hyper clause over `behaviours` and every
relation over model executions into exactly one form, HP-1 to HP-6 (ADR-023
§3), from its quantifier prefix, body, fairness sets and alignment, and
record the form beside its requirement record. The admitted forms are HP-1,
HP-2, HP-3, HP-5 and HP-6; HP-4 records a form that negotiation settles V-8.
The request writer SHALL add one subject per alias and the deadlock-freedom
item of each subject (ADR-023 HM-6).

## Use case

A verification operator writes a `∀∀` noninterference clause, a `∀∃`
opacity clause, a determinism relation, and a `∃∀` clause. The first three
are admitted and routed to their constructions; the last is reported
`unsupported` with the reason, and no engine runs for it.

## Inputs

- A `CheckedHyperClause` or `CheckedModelRelation` (FR-172), with its
  body's safety classification (ADR-014 A-4 as amended by ADR-018 IV-4, as
  FR-123 classifies `Safety`).

## Outputs

```rust
pub enum HyperForm {
    StepRelation,              // HP-1
    Universal,                 // HP-2
    ForallExistsSafety,        // HP-3
    Other { reason: HyperUnsupported },   // HP-4
    SingleExistential,         // HP-5
    ProjectionAligned,         // HP-6
}
```

- `Requirements{kind: temporal-satisfaction, extent: Unbounded{domains}}`
  with the `HyperForm` beside it; for HP-1 with a code binding, the
  additional record FR-180 states.

## Behavior

- S3 SHALL classify in this order:
  1. `StepRelation` (HP-1): a `relation` over model executions.
  2. `SingleExistential` (HP-5): a prefix of exactly one `exists`, with no
     `align skip`.
  3. `Other` (HP-4): an `exists` before a `forall`; or an `exists` with a
     `forall` or a second `exists` beside it whose body is outside the safety
     fragment or whose fairness set is non-empty; or `align skip` with any
     `exists` or with a body outside the safety fragment.
  4. `ProjectionAligned` (HP-6): every quantifier `forall`, `align skip`
     present, body in the safety fragment.
  5. `Universal` (HP-2): every quantifier `forall`, `n >= 1`, lockstep.
  6. `ForallExistsSafety` (HP-3): prefix `forall^n exists^m`, `n >= 0`,
     `m >= 1`, `n + m >= 2`, body in the safety fragment, every existential
     variable with the empty fairness set.
- `Other` SHALL carry which rule placed it there.
- If an HP-5 clause carries a `match` block, then S3 SHALL refuse
  `unsupported_construct`/`expression-form` at the block (ADR-023 SE-1).
- S3 SHALL record each clause as `Requirements{kind: temporal-satisfaction,
  extent: Unbounded{domains}}` (FR-057, ADR-014 A-3) with its `HyperForm`
  beside it, so it routes on the existing kind and a candidate's arm reads
  the form.
- The EN-1 provider manifest SHALL advertise HP-1, HP-2, HP-3, HP-5 and HP-6
  through FR-075's registration, and no provider SHALL advertise HP-4, so
  negotiation settles HP-4 V-8, `unsupported-requested-capability`, with no
  engine run.
- A `behaviours` clause under a profile other than infinite-trace SHALL
  record its profile and no form, so negotiation settles it V-8 (ADR-023
  HS-7).
- The request writer SHALL supply one model subject (FR-125) per distinct
  alias the item names, and SHALL add the deadlock-freedom item of each
  distinct subject as FR-124 states; a hyper or relation item SHALL report
  no deadlock itself.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-173-AC-1 | ADR-023 §1's `NonInterference` classifies `Universal`; §8.2's `Opaque` `ForallExistsSafety`; FR-171-AC-3's `Det` `StepRelation`; §13's clause with `align skip` `ProjectionAligned`; `exists trace b of V { eventually holds(v.l @ b = 1) }` `SingleExistential`. Each records (`temporal-satisfaction`, `Unbounded`) with its form. | Test (TC-598) |
| FR-173-AC-2 | `Other`, each naming its rule: `exists trace b of V forall trace a of V`; `Opaque` with body `always eventually holds(v.l @ a = v.l @ b)`; `Opaque` with `fair { weak V::Vault::step }` on `b`; §13's clause with an `exists` variable. Each settles V-8 `unsupported-requested-capability` in negotiation with no engine run. | Test (TC-598) |
| FR-173-AC-3 | `SingleExistential` with a `match` block refuses `unsupported_construct`/`expression-form` at the block. `NonInterference` under the event-position profile records no form and settles V-8. | Test (TC-598) |
| FR-173-AC-4 | A request with `SavesPower` supplies subjects for `S` and `N` and carries two deadlock-freedom items; a request with `NonInterference` and `Opaque` over `V` carries one. | Test (TC-598) |

## Dependencies

- ADR-023 §3 HP-1 to HP-6 and the admitted fragment; §2 HM-6; HS-7; SE-1;
  ADR-014 A-3 and A-4 as amended by ADR-018 IV-4.
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (safety classification), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-057](FR-057-admit-shared-capability-kinds.md).
- QSpec owns the forms, the admitted fragment, V-8 for HP-4 and the
  claim-form rows (ADR-023 QS-3, QS-4).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-3, QS-4).
