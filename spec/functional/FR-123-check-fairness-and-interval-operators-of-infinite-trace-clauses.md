---
id: FR-123
title: "Check fairness constraints and interval operators of infinite-trace clauses"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
---
# FR-123: Check fairness constraints and interval operators of infinite-trace clauses

## Description

The S3 TemporalTrace `check` (ADR-014 §5) SHALL admit, on a temporal clause
whose unit selects `quire.temporal.infinite-trace/v1`, fairness constraints
over model operations (ADR-018 FA-1, FA-5, FA-6) and interval operators
nested in any order with unbounded ones (ADR-018 IV-1). It SHALL classify
every admitted clause over a model subject into one property form (ADR-018
TP-1 to TP-4) and record that form beside the clause's requirement record.

## Use case

A verification operator writes a liveness claim about a state model,
`always eventually holds(c.versionNumber = 2)`, and states the fairness the
claim relies on. They write `fair weak attemptUpdate` when any transition of
the operation discharges the premise, and `fair weak each attemptUpdate`
when every receiver and argument vector must be served. They also write a
recovery-stability claim that puts a bounded operator under an unbounded
one. The checker admits both, refuses a constraint it cannot resolve with
its span, and records which engines can settle each claim.

## Inputs

- Parsed temporal clause forms (S2) with their fairness constraint forms and
  temporal operators, each operator with an optional interval form.
- The unit's resolved temporal profile selection (FR-110).
- The admitted domain packages and the unit's model operations (FR-103).

## Outputs

- A checked temporal clause holding its formula and its fairness set:
  `Vec<FairnessConstraint>`, where
  `FairnessConstraint{kind: FairnessKind, operation: DeclarationKey,
  granularity: FairnessGranularity}`, `FairnessKind::Weak`, and
  `FairnessGranularity::{Whole, Each}`.
- Each checked operator holding `Option<TemporalInterval>` (ADR-014 TR-3).
- The clause's `PropertyForm::{ReachableInvariant, BoundedMltl, Safety,
  Liveness}` (TP-1 to TP-4), recorded beside its `Requirements` (ADR-014
  A-3).
- A typed `CheckRefusal` with a span, and no checked clause, on refusal.

## Behavior

### Fairness constraints

- The checker SHALL read a fairness constraint form `fair <kind>
  [<granularity>] <operation>`. The kind is required; the one kind is
  `weak`. A constraint written without a granularity SHALL check as
  `Whole`. A constraint written `whole` or `each` SHALL check as that
  granularity.
- The operation SHALL resolve to one operation of an admitted model
  (FR-103). A name that resolves to no operation SHALL refuse
  `missing_declaration`/`missing-name` at the operation's span.
- A fairness constraint on a clause whose unit selects a bounded profile
  SHALL refuse `unsupported_construct`/`expression-form` at the
  constraint's span.
- The checked fairness set SHALL hold each constraint once, in source
  order. Two constraints with equal kind, operation and granularity SHALL
  check as one.
- The fairness set is part of the clause's checked identity, so two clauses
  that differ only in their fairness sets have different node identities.

### Interval operators under infinite-trace

- Under `quire.temporal.infinite-trace/v1`, each of `eventually`, `always`,
  `until`, `release`, `once`, `historically`, `since` and `triggered` SHALL
  admit a closed interval `[a,b]` with `a <= b`, or none. The checked
  operator SHALL hold `Some(TemporalInterval)` for an interval and `None`
  for none. Interval and unbounded operators SHALL nest in any order.
- An interval `[a,*]` SHALL refuse, and an interval with `a > b` SHALL
  refuse, each as ADR-014 TR-3 states for a bounded profile, at the
  interval's span.
- The interval's key SHALL carry the infinite-trace profile identity
  (QSpec FR-255), so it never equals a bounded-profile interval with the
  same numbers.

### Property form

- The checker SHALL classify a clause over a model subject as follows, in
  order:
  1. `ReachableInvariant` (TP-1): the formula is `always holds(I)` with `I`
     a state predicate.
  2. `BoundedMltl` (TP-2): the unit selects the event-position
     false-extension profile.
  3. `Safety` (TP-3): the formula, in negation normal form, uses only
     atoms, `and`, `or`, `always`, `release`, past operators and interval
     operators (ADR-014 A-4 as amended by ADR-018 IV-4).
  4. `Liveness` (TP-4): every other admitted infinite-trace formula.
- A clause under the fixed-sample or timestamped-event profile over a model
  subject records no property form, and its requirement record carries the
  profile, so negotiation settles it `unsupported`,
  `unsupported-requested-capability` (ADR-018 §1).
- The requirement record of an admitted infinite-trace clause SHALL be
  (`temporal-satisfaction`, `Unbounded{domains}`) as ADR-014 A-3 states,
  and of a TP-2 clause as its bounded profile gives.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-123-AC-1 | Over ADR-018 §6's ConfigVersion example unit, `fair weak attemptUpdate` and `fair weak whole attemptUpdate` check to equal fairness sets with granularity `Whole`; `fair weak each attemptUpdate` checks to granularity `Each`; and a clause with each fairness set has a different node identity from the same clause with the other. | Test (TC-518) |
| FR-123-AC-2 | `fair weak Absent` refuses `missing_declaration`/`missing-name` at `Absent`'s span. A fairness constraint on a clause under `quire.temporal.event-position.false-extension/v1` refuses `unsupported_construct`/`expression-form` at the constraint's span. A constraint repeated twice checks to a set holding it once. | Test (TC-518) |
| FR-123-AC-3 | Under infinite-trace, `always (holds(not s.healthy) implies eventually always[0,2] holds(s.healthy))` checks, with `Some([0,2])` on the inner `always` and `None` on the outer `always` and on `eventually`. `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)` each refuse at the interval's span. The `[0,2]` interval's key differs from the key of `[0,2]` under the event-position profile. | Test (TC-518) |
| FR-123-AC-4 | Property forms: `always holds(c.versionNumber <= 1000)` is `ReachableInvariant`; `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension is `BoundedMltl`; `always (holds(req) implies eventually[0,5] holds(ack))` under infinite-trace is `Safety`; `always eventually holds(c.versionNumber = 2)` and the recovery-stability formula are `Liveness`. Each infinite-trace clause records (`temporal-satisfaction`, `Unbounded`). | Test (TC-518) |

## Dependencies

- ADR-014 §3 TR-3 and §5 A-2 to A-4 (the TemporalTrace check, interval and
  safety fragment), as amended by ADR-018 IV-1 and IV-4.
- ADR-018 §1 (property forms), §4 FA-1, FA-5, FA-6 (fairness), §11 IV-1 to
  IV-4 (interval operators).
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations), [FR-110](FR-110-resolve-header-profile-selections-at-e3.md)
  (profile selection), [FR-057](FR-057-admit-shared-capability-kinds.md)
  (`temporal-satisfaction`).
- QSpec owns the surface grammar of the fairness constraint and the
  admission of intervals under infinite-trace (ADR-018 QS-4, QS-13); the
  spellings here follow ADR-018 §6.
