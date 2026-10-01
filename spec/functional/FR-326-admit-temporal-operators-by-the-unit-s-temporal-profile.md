---
id: FR-326
title: "Admit temporal operators by the unit's temporal profile and record the clause's requirement"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-255
    type: depends_on
---
# FR-326: Admit temporal operators by the unit's temporal profile and record the clause's requirement

## Description

The S3 TemporalTrace `check` SHALL admit each temporal operator of a clause
by the one temporal profile its unit selects (ADR-014 §5 A-2, §3 TR-3), build
each admitted interval as a validated `TemporalInterval`, compute a bounded
formula's horizon (TR-4), and return the clause's requirement record through
`FamilyContract::requirements()` (A-3, FR-062). Under
`quire.temporal.infinite-trace/v1` an operator is unbounded or an interval
operator; FR-123 states the interval-operator and fairness rules under that
profile, and this requirement states the profile identity, the bounded
profiles, the bare-operator rule and the requirement record.

## Use case

An author writes the same predicate under two profiles. Under a bounded
profile every operator needs a closed interval and the clause has a finite
horizon. Under the infinite-trace profile `eventually` alone is admitted and
the clause has no horizon. The checker tells the author, at the operator,
when a form does not mean anything under the selected profile, and records
for negotiation whether the claim is bounded.

## Inputs

- The S2 `TemporalClauseForm` (FR-325).
- The unit's resolved profile selections (FR-110), including its temporal
  profile selection.

## Outputs

- A `CheckedTemporalClause` holding its profile selection, its checked
  formula, each operator with `Option<TemporalInterval>`, and, for a
  bounded-profile clause, its horizon `h: u64`.
- `TemporalInterval{lower: u64, upper: u64}` with one validated constructor,
  and its wire key `IntervalKey{lower, upper, profile, clock_binding}` (F
  `bound`, FR-097), identified by the checked node id of its operator.
- The clause's `Requirements`: kind `temporal-satisfaction` and a
  `ClaimExtent` (FR-097).
- A `CheckRefusal` with a span and no checked clause, on refusal.

## Behavior

### Profile identity

- A temporal clause SHALL be checked under exactly one temporal profile: the
  unit's one temporal profile selection resolved by FR-110 to one of
  `quire.temporal.event-position.false-extension/v1`,
  `quire.temporal.fixed-sample.false-extension/v1`,
  `quire.temporal.timestamped-event.finite-window/v1` and
  `quire.temporal.infinite-trace/v1` (QSpec FR-250).
- When a unit holds a temporal clause and selects no temporal profile, or
  selects two, the check SHALL refuse `unknown_profile`/
  `unsupported-selection` at the clause's span, naming the selections it
  found.
- The infinite-trace profile SHALL read positions with the event-position
  sequence authority and no false-extension closure (QSpec FR-250-AC-6).

### Bounded profiles

- Under a bounded profile, each operator SHALL carry a closed interval
  `[a,b]` with `a <= b`, checked to `Some(TemporalInterval)`.
- When an operator carries no interval under a bounded profile, the check
  SHALL refuse `unsupported_construct`/`expression-form` at the operator's
  span (ADR-014 A-2).
- When an interval is `[a,*]`, under every profile, the check SHALL refuse
  `unsupported_construct`/`expression-form` at the interval's span.
- When an interval has `a > b`, under every profile, the check SHALL refuse
  `ill_typed`/`type-mismatch` at the interval's span, the refusal
  `TemporalInterval`'s constructor gives.
- The check SHALL compute the horizon of a bounded-profile formula as the
  greatest reach of its intervals with checked arithmetic. When the sum
  exceeds `u64`, it SHALL refuse with `LimitExceeded`, limit kind work
  budget (`stage_limit_exceeded`), at the operator whose interval overflowed
  (ADR-014 TR-4).

### Infinite-trace profile

- Under `quire.temporal.infinite-trace/v1`, an operator with no interval
  SHALL be admitted as an unbounded operator holding `None`, and an operator
  with a closed interval SHALL be admitted as FR-123 states.
- An infinite-trace formula SHALL have no horizon.

### Representation

- An interval's key SHALL carry the selected profile identity and clock
  binding (QSpec FR-255), so equal numbers under two profiles give unequal
  keys.
- A bounded-profile operator and an infinite-trace operator SHALL never
  share a checked representation (ADR-014 TR-3).

### Requirement record

- The requirement record of an infinite-trace clause SHALL be
  (`temporal-satisfaction`, `Unbounded{domains}`), with one domain keyed by
  the clause's node and an empty path, of kind infinite-trace formula, which
  no `FiniteBound` can stand for (ADR-014 A-3, §4).
- The requirement record of a bounded-profile clause SHALL be
  (`temporal-satisfaction`, extent by ADR-014 §4 over its `over` parameter
  and bound-variable types).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-326-AC-1 | Under infinite-trace, `eventually holds(c.value = 3)` checks with `None` on `eventually`, no horizon, and the record (`temporal-satisfaction`, `Unbounded`) with one infinite-trace-formula domain keyed by the clause node; under event-position false-extension the same clause refuses `unsupported_construct`/`expression-form` at `eventually`'s span. | Test (TC-836) |
| FR-326-AC-2 | Under event-position false-extension, `always[0,2] eventually[1,3] holds(c.value = 3)` checks with horizon 5, and `eventually[3,*] holds(p)` refuses `unsupported_construct`/`expression-form` at the interval's span; under infinite-trace `eventually[3,*] holds(p)` refuses the same way. `eventually[5,3] holds(p)` refuses `ill_typed`/`type-mismatch` at the interval's span under both profiles. | Test (TC-836) |
| FR-326-AC-3 | Under event-position false-extension, `eventually[0,18446744073709551615] eventually[0,1] holds(p)` refuses `stage_limit_exceeded` with limit kind work budget at the operator whose interval overflowed. | Test (TC-836) |
| FR-326-AC-4 | A unit holding a temporal clause with no temporal profile selection refuses `unknown_profile`/`unsupported-selection` at the clause's span; one selecting both the event-position and the infinite-trace profiles refuses the same way, naming both. | Test (TC-836) |
| FR-326-AC-5 | A bounded-profile clause over `over (c: Counter)` with no unbounded type has record (`temporal-satisfaction`, `Bounded`); the `[0,2]` interval's key under event-position differs from the `[0,2]` key under fixed-sample. | Test (TC-836) |

## Dependencies

- ADR-014 §2 (absent temporal interval), §3 TR-3 and TR-4, §4 (extent rule),
  §5 A-2 and A-3, as amended by ADR-018 IV-1.
- [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (profile
  selection), [FR-062](FR-062-implement-checked-family-contract.md)
  (`requirements()`), [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  (`ClaimExtent`, `IntervalKey`), [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (interval operators and fairness under infinite-trace),
  [FR-325](FR-325-parse-temporal-operators-with-an-optional-interval.md).
- QSpec FR-090-AC-7, FR-250-AC-6, FR-255, FR-091 and FR-092.

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec FR-361 (profiles over a model subject) and FR-367 (interval
  operators under infinite-trace): Linear STD-131 (QS-3, QS-13); Linear
  STD-99.
