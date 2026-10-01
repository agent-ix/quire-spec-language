---
id: FR-328
title: "Evaluate an infinite-trace clause three-valued over a finite prefix"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
---
# FR-328: Evaluate an infinite-trace clause three-valued over a finite prefix

## Description

When the TemporalTrace evaluator (FR-327) evaluates a clause under
`quire.temporal.infinite-trace/v1` over a finite trace, it SHALL read the
trace as a prefix of an unknown infinite continuation and return violation
or pending, never a proof (ADR-014 §5 A-4, QSpec FR-161-AC-2). It SHALL
return violation only for a formula in the safety fragment whose prefix is
a bad prefix: every infinite extension of it makes the formula false. The
safety fragment admits interval operators (ADR-018 IV-4). This reading is
sound: it never reports a violation that some extension would not confirm.

## Use case

A runtime monitor watches a service under an infinite-trace claim. When a
request goes unanswered past its deadline, the monitor reports the violation
as soon as the prefix shows it. When the property is liveness, or the
deadline has not passed yet, it reports pending, so a short observation
never reads as a guarantee.

## Inputs

- A `CheckedTemporalClause` under the infinite-trace profile (FR-326,
  FR-123).
- A finite `TemporalTrace` with no loop (FR-327), the `over` binding, and a
  `Meter`.

## Outputs

- `Completed(false)` (violation) with the `TemporalPosition` of the
  activation that the bad prefix falsifies; or `Pending` (O-16
  inconclusive); or `Incomplete` with its charge point.

## Behavior

- The evaluator SHALL treat every position after the finite trace as
  unknown, with no stutter or false-extension; terminal stutter applies only
  to a model behaviour that ends at a terminal state (FR-125, ADR-018 SM-4).
- The safety fragment SHALL be the formulas whose negation normal form uses
  only atoms, negated atoms, `and`, `or`, `always`, `release`, past
  operators and interval operators (ADR-014 A-4 as amended by ADR-018
  IV-4). Whether a clause is in the fragment SHALL be read from its checked
  formula.
- When the clause is in the safety fragment and every infinite extension of
  the prefix makes it false at its activation position, the evaluator SHALL
  return `Completed(false)` at that position.
- In every other case, including a formula outside the fragment whose prefix
  already satisfies it, the evaluator SHALL return `Pending`.
- A past operator, with or without an interval, SHALL read every atomic
  predicate as false before position 0 (ADR-018 IV-2, QSpec FR-092's
  complete-history rule).
- `Pending` SHALL map to O-16 inconclusive and `Completed(false)` to
  violation (QSpec FR-324-AC-2). The evaluator SHALL return neither `proved`
  nor `tested` over a finite prefix.
- The evaluator SHALL charge and exhaust work as FR-327 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-328-AC-1 | Over the `Counter` prefix with `c.value` 0, 1, 2 under infinite-trace: `always holds(c.value <= 1)` is `Completed(false)` at position 0; `always holds(c.value <= 5)` is `Pending`; `eventually holds(c.value = 2)` and `eventually holds(c.value = 9)` are each `Pending`. | Test (TC-838) |
| FR-328-AC-2 | Over the same prefix, `always (holds(c.value = 1) implies eventually[0,1] holds(c.value = 3))` is `Completed(false)` at position 0 (positions 1 and 2 are both present and neither has value 3); `always (holds(c.value = 2) implies eventually[0,1] holds(c.value = 3))` is `Pending` (position 3 is unknown). | Test (TC-838) |
| FR-328-AC-3 | Over the same prefix, `always (holds(c.value = 0) implies once[1,1] holds(c.value = 9))` is `Completed(false)` at position 0, because `once[1,1]` at position 0 reads before position 0, where atoms are false. | Test (TC-838) |
| FR-328-AC-4 | No clause in AC-1 to AC-3 yields `tested` or `proved`; each `Pending` maps to O-16 inconclusive. | Test (TC-838) |

## Dependencies

- ADR-014 §5 A-4; ADR-018 §2 SM-1 and SM-4, §11 IV-2 and IV-4.
- [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md).
- QSpec FR-161-AC-2, FR-324-AC-2, FR-092.

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec half: Linear STD-131 (QS-13).
