---
id: FR-241
title: "Concretize a symbolic counterexample with exact rational delays"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: depends_on
---
# FR-241: Concretize a symbolic counterexample with exact rational delays

## Description

EN-6 SHALL turn every symbolic path or lasso it reports into a concrete
timed counterexample with exact rational delays, by a canonical backward
propagation (ADR-026 EZ-8, CT-4). When a symbolic lasso has no rational
concretization, the engine SHALL try the next accepting cycle and, when
none concretizes, settle `inconclusive`, because a refutation that does not
replay is not admitted.

## Use case

A verification operator receives a counterexample with delays such as
`0`, `3` and `1/2`. Running the engine again gives the same delays, and
each delay is a value replay can check exactly.

## Inputs

- A symbolic path or lasso over the retained graph (FR-239, FR-240).

## Outputs

- A `TemporalCounterexample` over the timed subject (FR-236), or
  `Undecided(LassoNotConcretized)`.

## Behavior

- **Finite paths.** The engine SHALL pick a valuation in the last zone and,
  step by step backwards, a predecessor valuation and a delay. At each step
  it SHALL choose the least admissible delay when the delay set has a
  least element, otherwise the midpoint of a bounded delay set, otherwise
  its lower end plus one unit. Every chosen value SHALL be an exact
  rational.
- **Lassos.** The engine SHALL concretize a symbolic lasso to a timed lasso
  (FR-236) by solving the loop's linear constraints over rational delays,
  with each clock the loop resets taking its value from the loop's own
  delays, and SHALL extend the stem by loop copies until every clock the
  loop never resets is above its largest constant.
- If a loop's constraints have no rational solution, then the engine SHALL
  try the next accepting cycle in canonical order.
- If no accepting cycle concretizes, then the engine SHALL return
  `Undecided(LassoNotConcretized)`.
- **Time-locks.** A time-lock path SHALL end with the delay that reaches
  the chosen time-locked valuation, as `final_delay`.
- The concretization SHALL be a function of the symbolic path.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-241-AC-1 | `NoLateReply` over `Rpc` with `T = 3 ms` concretizes to delays `0`, `3`, `0`: `send` at the least delay, `timeout` at the least delay with `x >= 3`, `reply` at the least delay after it. | Test (TC-696) |
| FR-241-AC-2 | ADR-026 §9's retry counterexample has a heartbeat delay `d1` with `0 < d1 < 1` and a retry delay `d2` with `d1 + d2 > 1` and `d2 < 1`, each an exact rational, and it replays to `reproduced-with-evaluated-witness` (FR-237). Two runs give byte-equal counterexamples. | Test (TC-696) |
| FR-241-AC-3 | FR-240-AC-2's `idle` lasso concretizes with a positive loop delay and with `x` above 1 at loop entry. | Test (TC-696) |
| FR-241-AC-4 | A symbolic lasso constructed with a loop whose delay constraints are `d > 1` and `d < 1` returns, with no other accepting cycle, `Undecided(LassoNotConcretized)`. | Test (TC-696) |

## Dependencies

- ADR-026 §8 EZ-8, §7 CT-2 and CT-4.
- [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-240](FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md).
