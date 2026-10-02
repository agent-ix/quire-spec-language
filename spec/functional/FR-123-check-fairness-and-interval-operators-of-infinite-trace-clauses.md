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

The S3 TemporalTrace `check` (ADR-014 §5) SHALL check, on a temporal clause
whose unit selects `quire.temporal.infinite-trace/v1`, the fairness
constraints over model operations (ADR-018 FA-1, FA-5, FA-6) and the
interval operators nested in any order with unbounded ones (ADR-018 IV-1)
that the shared grammar parses. It SHALL classify every admitted clause over
a model subject into one property form (ADR-018 TP-1 to TP-4) and record
that form beside the clause's requirement record.

## Use case

A verification operator writes a liveness claim about a state model,
`always eventually holds(c.versionNumber = 2)`, and states the fairness the
claim relies on: one constraint that any transition of `attemptUpdate`
discharges, or one that every receiver and argument vector of it must
discharge. They also write a recovery-stability claim that puts a bounded
operator under an unbounded one. The checker admits both, refuses a
constraint it cannot resolve with its span, and records which engines can
settle each claim.

## Semantic authority and boundary

QSpec owns the surface grammar of a fairness constraint and of an interval
under the infinite-trace profile, the meaning of fairness, and the rule
that the profile admits intervals (ADR-018 QS-4, QS-13; References). This
requirement specifies what S3 checks once the shared grammar has parsed a
form. The spellings in the use case and the acceptance criteria follow
ADR-018 §6 and are illustrative.

## Inputs

- Parsed temporal clause forms (S2) with their fairness constraint forms and
  temporal operators. A fairness constraint form carries a kind, an
  operation name and an optional granularity. A temporal operator carries an
  optional interval form.
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

- The checker SHALL check each parsed fairness constraint form into one
  `FairnessConstraint` carrying the form's kind, its resolved operation and
  its granularity.
- When a constraint form has no granularity, the checker SHALL check it as
  `Whole` (ADR-018 FA-6). When it names one, the checker SHALL check it as
  that granularity.
- The checker SHALL resolve the operation to one operation of an admitted
  model (FR-103). If the name resolves to no operation, then the checker
  SHALL refuse `missing_declaration`/`missing-name` at the operation's span.
- If a fairness constraint sits on a clause whose unit selects a bounded
  profile, then the checker SHALL refuse
  `unsupported_construct`/`expression-form` at the constraint's span.
- The checker SHALL hold each constraint once in the checked fairness set,
  in source order of first occurrence, checking two constraints with equal
  kind, operation and granularity as one.
- The checker SHALL include the fairness set in the clause's checked
  identity, so two clauses that differ only in their fairness sets have
  different node identities.

### Interval operators under infinite-trace

- Under `quire.temporal.infinite-trace/v1`, the checker SHALL admit on each
  of `eventually`, `always`, `until`, `release`, `once`, `historically`,
  `since` and `triggered` a closed interval `[a,b]` with `a <= b`, an
  interval `[a,*]` with an open upper bound, or none (ADR-018 IV-1). The
  checked operator SHALL hold `Some(TemporalInterval)` for an interval, its
  upper bound `Finite(b)` or `Open`, and `None` for none. The checker SHALL admit interval and unbounded
  operators nested in any order.
- If an interval has `a > b`, then the checker SHALL refuse it as ADR-014
  TR-3 states, at the interval's span.
- The checker SHALL key the interval with the infinite-trace profile
  identity (QSpec FR-255), so it never equals a bounded-profile interval
  with the same numbers.

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
- When a clause over a model subject selects the fixed-sample or
  timestamped-event profile, the checker SHALL record no property form and
  SHALL keep the profile in its requirement record, which negotiation
  settles `unsupported`, `unsupported-requested-capability` (ADR-018 §1).
- The checker SHALL record the requirement of an admitted infinite-trace
  clause as (`temporal-satisfaction`, `Unbounded{domains}`), as ADR-014
  A-3 states, and of a TP-2 clause as its bounded profile gives.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-123-AC-1 | Over ADR-018 §6's ConfigVersion example unit, a weak constraint on `attemptUpdate` written with no granularity and one written `whole` check to equal fairness sets with granularity `Whole`; one written `each` checks to granularity `Each`; and a clause with each fairness set has a different node identity from the same clause with the other. | Test (TC-518) |
| FR-123-AC-2 | A weak constraint on `Absent`, which names no operation, refuses `missing_declaration`/`missing-name` at `Absent`'s span. A fairness constraint on a clause under `quire.temporal.event-position.false-extension/v1` refuses `unsupported_construct`/`expression-form` at the constraint's span. A constraint written twice checks to a set holding it once. | Test (TC-518) |
| FR-123-AC-3 | Under infinite-trace, `always (holds(not s.healthy) implies eventually always[0,2] holds(s.healthy))` checks, with `Some([0,2])` on the inner `always` and `None` on the outer `always` and on `eventually`. `eventually[3,*] holds(p)` checks with `Some([3,Open])`, and the same operator under `quire.temporal.event-position.false-extension/v1` refuses at the interval's span; `eventually[5,3] holds(p)` refuses at the interval's span. The `[0,2]` interval's key differs from the key of `[0,2]` under the event-position profile. | Test (TC-518) |
| FR-123-AC-4 | Property forms: `always holds(c.versionNumber <= 1000)` is `ReachableInvariant`; `eventually[0,5] holds(c.versionNumber = 2)` under event-position false-extension is `BoundedMltl`; `always (holds(req) implies eventually[0,5] holds(ack))` under infinite-trace is `Safety`; `always eventually holds(c.versionNumber = 2)` and the recovery-stability formula are `Liveness`. Each infinite-trace clause records (`temporal-satisfaction`, `Unbounded`). A clause over the ConfigVersion subject under the fixed-sample profile records no property form and keeps that profile in its requirement record. | Test (TC-518) |

## Dependencies

- ADR-014 §3 TR-3 and §5 A-2 to A-4 (the TemporalTrace check, interval and
  safety fragment), as amended by ADR-018 IV-1 and IV-4.
- ADR-018 §1 (property forms), §4 FA-1, FA-5, FA-6 (fairness), §11 IV-1 to
  IV-4 (interval operators).
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations), [FR-110](FR-110-resolve-header-profile-selections-at-e3.md)
  (profile selection), [FR-057](FR-057-admit-shared-capability-kinds.md)
  (`temporal-satisfaction`).

## References

- QSpec FR-362 (fairness constraints), FR-367 (interval operators under
  infinite-trace) and FR-370 (the temporal clause body): the QSpec half of
  ADR-018, QS-1 to QS-13 (Linear STD-131).
