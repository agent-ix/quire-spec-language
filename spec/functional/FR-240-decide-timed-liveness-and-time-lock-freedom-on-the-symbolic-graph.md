---
id: FR-240
title: "Decide timed liveness and time-lock freedom on the symbolic graph"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
---
# FR-240: Decide timed liveness and time-lock freedom on the symbolic graph

## Description

For TT-3 and TT-4 items EN-6 SHALL run ADR-018 EN-1's second phase on the
retained symbolic graph, with time divergence as one more acceptance set
of the generalized Büchi product and each symbolic state split by the
guards of the claim's fairness constraints (ADR-026 EZ-6). For the
time-lock-freedom item it SHALL find local time-locks by DBM operations at
each explored state, and non-local time-locks as symbolic states that reach
no time-divergent target, confirmed by fresh exploration (EZ-7).

## Use case

A verification operator asks whether a controller always eventually
acknowledges, under weak fairness. A cycle that only acknowledges by
taking infinitely many steps in bounded time is not a counterexample; a
fair cycle that lets time pass is. The operator is also told about any
state from which time cannot diverge.

## Inputs

- The retained symbolic graph of FR-239's search over the item's product.

## Outputs

- For a liveness item: a symbolic lasso for FR-241, or `Holds` with the
  certificate FR-244 builds.
- For the time-lock-freedom item: a symbolic path to a time-locked state
  with its trap confirmation, or `Holds` with its certificate.

## Behavior

### Liveness under time divergence

- The engine SHALL build the product with the claim's generalized Büchi
  automaton (FR-242) and add a divergence acceptance set by the non-Zeno
  emptiness construction, so an accepting SCC passes only when it holds a
  cycle with positive total delay.
- Before the SCC phase, the engine SHALL split each symbolic state by the
  guards of the claim's fairness constraints, so each constraint is enabled
  at every valuation of a split state or at none.
- The SCC decomposition, candidate rule and fairness filter SHALL be
  FR-126's, run on the split graph, with enabledness read over time
  (FR-232).
- A passing candidate SHALL give the symbolic lasso FR-241 concretizes; no
  passing candidate SHALL give `Holds`.

### Time-lock search

- At each explored symbolic state, the engine SHALL compute by DBM
  operations the set of valuations at which no positive delay is
  admissible and no transition identity's guard holds. A non-empty set
  SHALL be a local time-lock.
- The engine SHALL mark the symbolic states that reach no quiescent state
  and no time-divergent SCC on the retained graph, concretize a timed state
  in one of them (FR-241), and confirm the trap by exploring that state's
  forward closure afresh from its point zone. A confirmed trap SHALL return
  `Violated` with `kind: TimeLock` (FR-236); an unconfirmed one SHALL
  continue the search with the next marked state in canonical order.
- The first time-lock in canonical order SHALL be reported, local before
  non-local at equal position.
- When no time-lock is found, the item SHALL return `Holds` with its
  certificate.

### Vacuity

- When the initial symbolic state reaches no fair time-divergent cycle and
  no quiescent state, a proof over the subject SHALL return
  `Undecided(NoAdmittedBehaviour)`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-240-AC-1 | Over `Rpc` with `T = 4 ms`, `always (holds(c.phase = Waiting) implies eventually holds(c.phase = Replied))` returns `Holds`. Over a variant where `reply` may also loop on itself with guard `x < 1` and no reset, a lasso of `reply` steps in bounded time is not reported; the result is still `Holds`. | Test (TC-695) |
| FR-240-AC-2 | In the `Serve` model with guard `x >= 1 ms`, `eventually holds(done)` under `fair weak serve` returns `Holds`; with no fairness it returns `Violated` with an `idle` lasso of positive loop delay. | Test (TC-695) |
| FR-240-AC-3 | The strict-guard `Rpc` variant's time-lock-freedom item returns `Violated`, `kind: TimeLock`, stem `send` at 0 and final delay 3; the unmodified `Rpc` returns `Holds`. The `Stall` model returns `Violated`, `kind: TimeLock`, confirmed by fresh exploration. | Test (TC-695) |
| FR-240-AC-4 | `always holds(true)` under the timed profile over `Stall` returns `Undecided(NoAdmittedBehaviour)`. | Test (TC-695) |

## Dependencies

- ADR-026 §8 EZ-6, EZ-7; ADR-018 EN-1 and FA-1 to FA-6; ADR-022 GM-6 and
  GV-2.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- F. Herbreteau, B. Srivathsan and I. Walukiewicz, "Efficient emptiness
  check for timed Büchi automata", 2012 (ADR-026 References).
