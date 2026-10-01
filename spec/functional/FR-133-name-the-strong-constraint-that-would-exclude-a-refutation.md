---
id: FR-133
title: "Name the strong constraint that would exclude a refuted liveness lasso"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-017
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: depends_on
---
# FR-133: Name the strong constraint that would exclude a refuted liveness lasso

## Description

When replay reproduces the lasso of a refuted liveness (TP-4) item, the
replay executor SHALL add to the item's FR-331 terminal record one warning diagnostic
`fairness.strong-would-exclude` for each operation of the model whose
strong fairness would exclude that lasso and that the clause does not
already constrain that way (ADR-019 SV-6). The diagnostic is outside the
counterexample, its replay identity and the obligation identity, and it
changes no verdict.

## Use case

A verification operator reads a refuted liveness claim. The record says
that strong fairness of `acquire` for each process would exclude the lasso,
and names process 1's acquisition and the loop position where it is first
enabled, so the operator can tell a missing premise from a real defect.

## Semantic authority and boundary

QSpec owns when the hint is emitted, how the suggested constraint is
chosen, its diagnostic code and members, and its place outside the
identities (ADR-019 QS-8; References). This requirement specifies where QSL
computes it and what it writes.

## Inputs

- A reproduced FR-128 replay of a `kind: Formula` lasso for a TP-4 item,
  with the enabled transition identities FR-131 computed at each loop
  state, the loop's steps and the clause's fairness set.

## Outputs

- Zero or more `StrongFairnessHint{suggested: FairnessConstraint,
  identity: Option<ModelTransition>, first_enabled_position: u64}` on the
  item's FR-331 terminal record, each a warning-severity diagnostic with
  code `fairness.strong-would-exclude`.

## Behavior

- After replay settles `reproduced-with-evaluated-witness` for a TP-4
  item's lasso, the replay executor SHALL compute the hints from the
  enabled sets it recomputed at each loop state, for every engine's
  refutation.
- For each operation of the model, in canonical declaration order, the
  executor SHALL:
  - when the operation is enabled at some loop state and no loop step takes
    it, name `{Strong, operation, Whole}`;
  - otherwise, when some transition identity of the operation is enabled at
    a loop state and no loop step takes that identity, name `{Strong,
    operation, Each}` with the first such identity in canonical transition
    order.
- The executor SHALL record with each hint the first loop position at which
  the named operation, or for `Each` the named identity, is enabled.
- When the clause's fairness set already holds the named constraint, the
  executor SHALL emit no hint for that operation.
- `model_check` SHALL write the hints on the item's terminal record beside
  the counterexample, and SHALL leave the counterexample, its replay
  identity, the obligation identity and the verdict unchanged.
- The executor SHALL emit no hint for a safety, bounded or deadlock-freedom
  item, nor for a lasso whose loop is the terminal stutter step, at which
  no operation is enabled.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-133-AC-1 | ADR-019 §6's mutex refuted under `weak each` on `acquire` carries exactly one hint: `{Strong, acquire, Each}`, identity `acq(1)`, first enabled position 0; `release`, taken in the loop, gets none. The refutation under `strong` (whole) on `acquire` carries the same hint. Both records still settle `refuted`, with the same counterexample and obligation identity as when the hint is not computed. | Test (TC-534) |
| FR-133-AC-2 | ADR-018 §6's ConfigVersion claim refuted under weak `attemptUpdate` with no granularity, loop `upd(a)` three times, carries one hint: `{Strong, attemptUpdate, Each}`, identity `upd(b)`, position 0. | Test (TC-534) |
| FR-133-AC-3 | FR-126-AC-3's terminal stutter refutation over `Counter`, its deadlock-freedom refutation and FR-126-AC-2's bounded refutation carry no hint. | Test (TC-534) |

## Dependencies

- ADR-019 §5 SV-6, §9 RU-4, AM-4.
- [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-131](FR-131-replay-checks-strong-fairness-on-a-model-counterexample.md)
  (the enabled sets), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the terminal record).

## References

- QSpec FR-369 (the strong-fairness hint): the QSpec half of ADR-019 QS-8
  (Linear STD-132).
