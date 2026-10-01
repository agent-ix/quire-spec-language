---
id: FR-200
title: "Decide every-scheduler claims over fair schedulers"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: depends_on
---
# FR-200: Decide every-scheduler claims over fair schedulers

## Description

When a claim `under every scheduler` states a fairness set, EN-5 SHALL
decide it over the schedulers under which every constraint of the set holds
with probability 1 (ADR-028 FS-1, FS-2). It SHALL compute the maximal fair
end components of the product by refinement (FS-3, FS-4), and reduce each
form to a computation of FR-197, FR-198 or FR-199 on a transformed objective
(FS-5 to FS-8). Its witness scheduler SHALL be memoryless and randomized,
and fair (FS-9).

## Use case

A protocol designer checks that a randomized consensus protocol terminates
with probability at least 0.99 against an adversary that schedules
processes as it likes but never starves one forever. Over every scheduler
the claim is refuted by an adversary that starves a process; over every
strongly fair scheduler it is proved.

## Inputs

- FR-196's product and the claim's fairness set (FR-195), each constraint
  weak or strong, `whole` or `each` (FR-123).

## Outputs

```rust
pub struct FairEndComponent { pub states: Vec<ProductStateId>, pub actions: Vec<(ProductStateId, ScheduledIdentity)> }
pub fn maximal_fair_end_components(product: &ProbProduct, fairness: &[FairnessConstraint]) -> Vec<FairEndComponent>;
pub enum SchedulerChoice { Deterministic(ActionChoice), Randomized(Vec<(ActionChoice, Rational)>) }
```

The value, interval or `+∞` per initial state and binding, as FR-197,
FR-198 and FR-199 return them, with a policy of `SchedulerChoice`s.

## Behavior

### Fair end components

- A constraint SHALL be enabled at a model state when one of its scheduled
  identities is an action there, and taken by a product edge whose action
  belongs to it. Intermediate states, the terminal stutter step and delay
  moves SHALL belong to no constraint.
- `maximal_fair_end_components` SHALL decompose the product into MECs; in
  each, drop the component when a weak constraint is enabled at every one of
  its states and none of its actions belongs to the constraint, and remove
  the states where a strong constraint is enabled when none of its actions
  belongs to that constraint; decompose what remains into MECs; and repeat
  until no component changes. The result SHALL be in canonical state order.

### Forms

- For a bounded event or measure, EN-5 SHALL compute FR-197's minimum or
  maximum unchanged, and record the fairness set in the result.
- For `Reach` with a supremum (`eventually` under `<= θ`, or `always` under
  `>= θ` after the exchange), EN-5 SHALL compute FR-198's maximum. Its
  witness SHALL take FR-198's policy at states of positive value and the
  uniform choice over the enabled actions at every other state.
- For `Reach` with an infimum, EN-5 SHALL compute `U`, the union of the
  states of the maximal fair end components of the product restricted to
  non-target states (for `until`, to states where `A` holds and `B` does
  not), and the value `1 − max Pr(not B until U)` by FR-198, with states
  where neither `A` nor `B` holds counted as failing. Its witness SHALL take
  FR-198's policy for that maximum at states of positive value, the uniform
  choice over each fair end component's actions in `U`, and the uniform
  choice over the enabled actions elsewhere.
- For `ExpectedReward`, the minimum SHALL be FR-198's. For the maximum, EN-5
  SHALL give `+∞` where the fair infimum of reaching `B` is below 1, and
  where, among states whose fair infimum is 1, an end component holding an
  action of positive reward is reachable; elsewhere it SHALL collapse
  zero-reward end components and run FR-198.
- For `LongRunFraction`, EN-5 SHALL give each maximal fair end component
  FR-199's optimum over its actions and combine with every end component
  that holds no fair end component collapsed, so every scheduler in the
  range ends in a fair end component. For a refutation it SHALL build a
  randomized witness that takes the actions the optimum leaves out with
  probability `2^-k`, starting at `k = 1` and raising `k` until the induced
  chain's exact value is past the threshold.

### Witness fairness

- Every witness for a claim with a fairness set SHALL be fair: each bottom
  strongly connected component of the chain it induces SHALL meet the fair
  end-component condition. EN-5 SHALL check this before emitting the
  witness.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-200-AC-1 | §15.6: with `fair strong Proc::flip`, `maximal_fair_end_components` returns none, `U` is empty and the infimum of `Pr(eventually holds(p.done))` is 1; with `fair weak Proc::flip` the same; with no fairness set the minimum is 0 (FR-198-AC-1). | Test (TC-635) |
| FR-200-AC-2 | `Coin2`, a variant of `Coin` with a field `stuck`, an operation `stall` that sets it, and `flip` and `wait` both requiring `not self.stuck`, so at the `stuck` state only `stall` is enabled: with `fair strong Proc::flip`, `{stuck}` with `stall` is a fair end component, `U = {stuck}`, and the infimum is `1 − max Pr(not done until stuck) = 0`; its witness takes `stall` at the live state, and its induced chain's bottom component `{stuck}` meets the fairness condition. | Test (TC-635) |
| FR-200-AC-3 | `Avail2` (FR-199-AC-2) with `fair strong Avail2::repair_fast`: `{up, down}` with all three actions is a fair end component and the infimum of the long-run availability is `1000/1001`. The claim `>= 0.9995` is refuted by a randomized witness with `k = 1`, `repair_fast` and `repair_slow` each at `1/2`, whose induced chain has the exact value `1400/1401`. | Test (TC-635) |
| FR-200-AC-4 | §15.4's `Deliver` with `fair weak Msg::send_a` gives the same minimum `24/25` as without fairness, and the result records the fairness set. | Test (TC-635) |

## Dependencies

- ADR-028 §3a FS-1 to FS-9, SCH-4, RU-3; ADR-018 FA-1; ADR-019 (strong
  fairness).
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-196](FR-196-build-the-probabilistic-product.md),
  [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md),
  [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md),
  [FR-199](FR-199-decide-long-run-fractions-by-bottom-components.md).

## References

- QSpec half, which owns fair schedulers, fair end components and the
  randomized witness wire: QSpec FR-406, FR-411 and FR-413 (Linear
  STD-137).
- Owning ticket: Linear QSL-371.
