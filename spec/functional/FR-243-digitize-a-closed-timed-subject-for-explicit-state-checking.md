---
id: FR-243
title: "Digitize a closed timed subject for explicit-state checking"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
---
# FR-243: Digitize a closed timed subject for explicit-state checking

## Description

When every clock constraint, time invariant and timed interval of a
subject and claim is closed, EN-6 SHALL hand the subject, its constants
scaled to integers, to ADR-018's explicit-state engine over integer-valued
clocks (ADR-026 EZ-10). The verdict is the same as the dense one for an
invariant over the discrete state, the deadlock-freedom item and location
reachability; its counterexamples have integer delays, which are valid
dense delays; and its proofs carry basis `Exhaustive`.

## Use case

A verification operator's model has only non-strict guards and deadlines.
The engine checks it as a finite integer-clock model, and the proof says it
is exhaustive over integer clocks, which is exact for closed constraints.

## Inputs

- A `ZoneCheckRequest` (FR-239).

## Outputs

- `HoldsDigitized`, or a `Violated` counterexample with integer delays in
  scaled units, in `ZoneCheckOutcome`.

## Behavior

- The engine SHALL take the digitization path only when every atomic clock
  constraint in guards, every time invariant and every timed interval of
  the claim uses `<=`, `>=`, `=` or a closed end, and the item is an
  invariant over a data predicate (TT-1, or an ADR-018 TP-1 claim read over
  the timed subject), a deadlock-freedom item or a location
  reachability item.
- When the engine takes that path, it SHALL scale constants by FR-238's scale factor,
  give each clock the integer values from 0 to one above its largest
  scaled constant, advance time by unit delays, and decide the item with
  FR-126's explicit-state engine.
- A proof on that path SHALL return `HoldsDigitized`, which FR-235 settles
  `Proved{basis: Exhaustive}` with method `digitized-explicit-state`.
- A counterexample on that path SHALL carry each delay as its scaled
  integer divided by the scale factor, an exact rational in the model's
  unit.
- When any constraint is strict, or the item is outside the three kinds,
  the engine SHALL take the zone search (FR-239).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-243-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms` (all closed) takes the digitization path and returns `HoldsDigitized`; the deadlock-freedom item over the same subject also returns `HoldsDigitized`. | Test (TC-698) |
| FR-243-AC-2 | `NoLateReply` with `T = 3 ms` on the digitization path returns a counterexample with integer delays `0, 3, 0` that replays (FR-237). | Test (TC-698) |
| FR-243-AC-3 | ADR-026 §9's retry model has strict constraints, so it takes the zone search and is refuted (FR-239-AC-2); the strict-guard `Rpc` variant also takes the zone search. A TT-3 claim over the all-closed `Rpc` takes the zone search and returns `Holds` with a certificate. | Test (TC-698) |

## Dependencies

- ADR-026 §8 EZ-10, §8.1 CF-4.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992; J. Ouaknine and J. Worrell, 2003 (ADR-026 References).
