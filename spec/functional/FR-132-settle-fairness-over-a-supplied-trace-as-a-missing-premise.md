---
id: FR-132
title: "Settle a fairness constraint over a supplied trace as a missing premise"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-017
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-129
    type: depends_on
---
# FR-132: Settle a fairness constraint over a supplied trace as a missing premise

## Description

QSL SHALL settle a temporal clause with any fairness constraint, weak or
strong, that is evaluated over a supplied trace, replayed or monitored with
no model subject behind it, `unsupported` with the new cause
`MissingFairnessPremise{constraint}` (ADR-019 SV-2, AM-6). A supplied trace
carries states and steps and no enabledness, so no fairness constraint can
be checked on it. A clause with an empty fairness set over a supplied trace
evaluates as ADR-014 A-4 states.

## Use case

A verification operator evaluates a liveness clause with a fairness premise
over a trace recorded from a running system. The result says the premise
could not be checked there and names it, rather than evaluating the formula
as though every behaviour were fair.

## Semantic authority and boundary

QSpec owns "missing fairness premise" and its settlement (ADR-019 QS-3;
References). This requirement specifies QSL's cause and where the
TemporalTrace evaluator and replay settle it.

## Inputs

- A checked temporal clause (FR-123, FR-129) and a supplied trace with no
  model subject, at the S6a TemporalTrace evaluator or at FR-098 `replay`.

## Outputs

- `TerminalValue::Unsupported(UnavailabilityCause::MissingFairnessPremise
  { constraint: FairnessConstraint })`: QSpec FR-360 label
  `unsupported`, QSpec FR-243 basis `unavailable`, wire cause
  `unsupported_projection`/`missing-fairness-premise` (QSpec FR-362), O-16
  category unsupported.

## Behavior

- When a clause whose fairness set is not empty is evaluated over a
  supplied trace, the evaluator SHALL return
  `Unsupported(MissingFairnessPremise{constraint})` naming the first
  constraint of the checked fairness set, which holds the constraints in
  source order of first occurrence (FR-123), and SHALL evaluate no formula.
- When the clause's fairness set is empty, the evaluator SHALL evaluate the
  clause over the supplied trace as ADR-014 A-4 states.
- `TerminalValue::category` SHALL map `Unsupported(MissingFairnessPremise
  {..})` to unsupported.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-132-AC-1 | A supplied lasso over the mutex's states (`0 2 0` with steps `acq(2)`, `rel`) and the clause `always eventually holds(m.owner = 1)` with a weak `each` constraint on `acquire` settles `Unsupported(MissingFairnessPremise{Weak, acquire, Each})`, label `unsupported`, basis `unavailable`, category unsupported; with a `strong` constraint it names `{Strong, acquire, Whole}`; with `strong each` on `acquire` followed by `weak whole` on `release` it names the `strong each` constraint. | Test (TC-533) |
| FR-132-AC-2 | The same supplied lasso and clause with an empty fairness set evaluates `false`, a violation for that lasso. | Test (TC-533) |

## Dependencies

- ADR-019 §5 SV-2, §4 BE-4, AM-4 and AM-6; ADR-014 §5 A-4.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal value
  and category map), [FR-129](FR-129-check-strong-fairness-constraints.md)
  (the fairness set).

## References

- QSpec FR-362 (the missing fairness premise): the QSpec half of ADR-019
  QS-3 (Linear STD-132).
