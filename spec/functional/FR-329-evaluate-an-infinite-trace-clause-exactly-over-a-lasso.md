---
id: FR-329
title: "Evaluate an infinite-trace clause exactly over a fair lasso"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
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
# FR-329: Evaluate an infinite-trace clause exactly over a fair lasso

## Description

When the TemporalTrace evaluator (FR-327) evaluates a clause under
`quire.temporal.infinite-trace/v1` over a lasso, it SHALL first check the
lasso against the clause's fairness set, refuse a lasso that is not an
admitted trace, and evaluate a fair lasso exactly (ADR-014 §5 A-4). It SHALL
read positions past the represented lasso by ADR-018 SM-8, including past
operators and interval operators (IV-2). This lasso evaluation is the
semantics that the model checker's refutations are replayed through
(ADR-018 SM-1, SM-7).

## Use case

An engineer has a lasso-shaped counterexample from a tool, or a recorded
run that settles into a loop, and an infinite-trace claim with a
"once ... before" condition. The evaluator decides the claim on that lasso
exactly, including a past condition that looks back into the prefix, and
refuses a lasso that the claim's fairness premise excludes, so an unfair
loop is never reported as a violation.

## Inputs

- A `CheckedTemporalClause` under infinite-trace (FR-326, FR-123), with its
  fairness set.
- A lasso, one of:
  - `Lasso::Observed{prefix: Vec<Observation>, loop: Vec<Observation>}`:
    FR-106-admitted observations;
  - `Lasso::Model{subject, initial, prefix: Vec<ModelStep>, loop:
    Vec<ModelStep>}`: a model subject (FR-125), the index of an initial
    state, and steps as FR-128 states them (transition identity and
    post-state digest).
- The `over` binding and a `Meter`.

## Outputs

- `Completed(true)` (`tested`, evidence for this lasso only);
  `Completed(false)` (violation) with the `TemporalPosition` of the first
  activation that evaluated false; `Undefined` (violation) as FR-327
  states, reading every unrolled position; or `Incomplete` with its charge
  point.
- A refusal `invalid_runtime_input`/`invalid-value`, with no truth value,
  for a malformed lasso, or a `Lasso::Model` that is unfair.
- `MissingFairnessPremise{constraint}` (O-16 unsupported), with no truth
  value, for a `Lasso::Observed` and a non-empty fairness set.

## Behavior

### Lasso shape

- A lasso SHALL be a prefix of `n >= 0` positions and a loop of `L >= 1`
  positions; position `n + L` re-enters position `n`. When `L = 0`, the
  evaluator SHALL refuse `invalid_runtime_input`/`invalid-value`.
- For `Lasso::Model`, the evaluator SHALL reconstruct each position by
  re-executing its step through FR-120's `ModelSystem` with FR-101 `replay`
  from the named initial state, as FR-128 does, and SHALL refuse
  `invalid_runtime_input`/`invalid-value` when the loop's last post-state is
  not the loop's entry state or a step's transition is not enabled.

### Fairness

- For a `Lasso::Model`, the evaluator SHALL decide whether the lasso is
  fair for each constraint of the clause's fairness set, of either kind, by
  QSpec FR-362's lasso rule, reading a constraint as taken at a loop step
  whose transition identity belongs to it (ADR-018 FA-1) and as enabled at
  a loop position where ADR-018 FA-2 holds, computed through FR-120's
  `ModelSystem`.
- When the lasso is unfair for any constraint of the clause's fairness set,
  the evaluator SHALL refuse `invalid_runtime_input`/`invalid-value`, naming
  the constraint.
- When the clause's fairness set is non-empty and the lasso is
  `Lasso::Observed`, the evaluator SHALL return
  `MissingFairnessPremise{constraint}` naming the first constraint of the
  checked fairness set, and SHALL evaluate no formula: observations carry
  no transition identity or enabledness, so the fairness premise is missing
  (QSpec FR-362, wire cause `unsupported_projection`/
  `missing-fairness-premise`). It maps to O-16 unsupported.

### Exact evaluation (ADR-018 SM-8)

- State reads SHALL wrap: position `p >= n` reads the observation of
  position `n + ((p - n) mod L)`.
- The evaluator SHALL compute each subformula's past reach `R`: 0 for an
  atom; the largest operand reach for a Boolean or future operator; `R + b`
  for a past interval operator with upper bound `b`, taking the larger
  operand reach for `since[a,b]` and `triggered[a,b]`; the largest operand
  reach plus `L` for an unbounded `once`, `historically`, `since` or
  `triggered`; and the largest operand reach plus `a + L` for a past
  `[a,*]` operator.
- It SHALL unroll the loop `m = ceil(R(φ) / L) + 1` times, evaluate every
  subformula over the `n + m × L` unrolled positions with past operators
  reading back to position 0 and atoms false before it, and read a
  subformula at a position past the unrolled positions at the last unrolled
  loop copy congruent to it modulo `L`.
- Future interval operators SHALL have QSpec FR-091's offset meaning over
  the infinite lasso; every future position exists, so no closed-boundary
  rule applies (ADR-018 IV-2).
- Work SHALL be charged per (node, position) visit and exhausted as FR-327
  states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-329-AC-1 | On the observed lasso with an empty prefix and loop `a.versionNumber` 0, 1, 2 (ADR-018 §6): `always eventually (holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2))` is `Completed(true)`; `always eventually holds(a.versionNumber = 2)` is `Completed(true)`; `eventually always holds(a.versionNumber = 0)` is `Completed(false)` at position 0. | Test (TC-839) |
| FR-329-AC-2 | On the observed lasso with prefix `a.versionNumber` 5 and loop 0, 1, 2: `always (holds(a.versionNumber = 0) implies once holds(a.versionNumber = 5))` is `Completed(true)`, although no loop position holds 5; `always (holds(a.versionNumber = 0) implies once[1,1] holds(a.versionNumber = 2))` is `Completed(false)` at position 0, because position 1 reads position 0. | Test (TC-839) |
| FR-329-AC-3 | A lasso with an empty loop refuses `invalid_runtime_input`/`invalid-value`. A `Lasso::Model` whose last loop post-state differs from its entry state refuses the same way. | Test (TC-839) |
| FR-329-AC-4 | On ADR-018 §6's three-step `upd(a)` lasso as `Lasso::Model`, `always eventually holds(b.versionNumber = 2)` under `fair weak attemptUpdate` is `Completed(false)` at position 0; under `fair weak each attemptUpdate` it refuses `invalid_runtime_input`/`invalid-value` naming the constraint (`upd(b)` is enabled throughout the loop and never taken). | Test (TC-839) |
| FR-329-AC-5 | AC-1's first clause with a meter one unit short of its visit count returns `Incomplete` and no truth value. | Test (TC-839) |
| FR-329-AC-6 | On the observed `Counter` lasso with an empty prefix and loop `c.value` 0, 1, 2, `always eventually holds(2 / (2 - c.value) = 2)` returns `Undefined{where: 2, cause: division-by-zero}`, category violation. | Test (TC-847) |
| FR-329-AC-7 | The same clause with `fair weak attemptUpdate` over the lasso as `Lasso::Observed` returns `MissingFairnessPremise` naming `fair weak whole attemptUpdate`, O-16 unsupported, and evaluates no position; with an empty fairness set over the same observed lasso it is `Completed(false)` at position 0. | Test (TC-848) |

## Dependencies

- ADR-014 §5 A-4 (amended in place with the fairness reading of an observed
  lasso); ADR-018 §2 SM-1, SM-7, SM-8; §4 FA-1 to FA-3; §11 IV-2.
- [FR-327](FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md).
- QSpec FR-161-AC-4 and AC-5 (lasso meaning; fairness filters traces
  before evaluation), FR-091, FR-362 (the missing fairness premise).

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec FR-362 (fairness, the missing fairness premise) and FR-367
  (interval operators): Linear STD-131 (QS-4, QS-10 (a), QS-13).
