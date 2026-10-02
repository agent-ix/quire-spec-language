---
id: FR-243
title: "Digitize a closed timed subject on the digital-clock route"
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
# FR-243: Digitize a closed timed subject on the digital-clock route

## Description

The request writer SHALL send a timed item to the digital-clock route only
when the request names `exact` evidence for it; the route admits a subject
and claim whose clock constraints, time invariants and timed intervals are
all closed. The request writer SHALL send every other timed item, TT-1
included, to EN-6's zone search (FR-239), which returns `Holds` with a CF-1
certificate (ADR-026 EZ-10).
When an item runs on the digital-clock route, EN-5's digital reading
(ADR-028 TA-1 to TA-3) SHALL scale the constants to integers, explore
integer-valued clocks, give the dense verdict for a TT-1 claim over the
discrete state and for the deadlock-freedom item, report counterexamples
with integer delays, which are valid dense delays, and report proofs as
`HoldsDigitized`, basis `Exhaustive`.

## Use case

A verification operator's model has only non-strict guards and deadlines.
Requested with ordinary evidence, its timed invariant is proved by the zone
engine with a certificate. Requested with `exact` evidence, the same item
runs on the digital-clock route, and the proof says it is exhaustive over
integer clocks, which is exact for closed constraints.

## Inputs

- A timed item, its requested evidence kind, and the timed subject (FR-231).

## Outputs

- `DigitalOutcome::HoldsDigitized`, or `Violated` with a counterexample
  whose delays are integers in scaled units, or an FR-204 unsupported cause.

## Behavior

- When a timed item names `exact` evidence, the request writer SHALL route
  it to EN-5's digital-clock route; otherwise it SHALL route it to EN-6.
  The route SHALL be a function of the requested evidence kind alone.
- When an item runs on the digital-clock route, if any atomic clock constraint in a guard,
  time invariant or timed interval of the claim is strict, then EN-5 SHALL
  settle `Unsupported(StrictClockConstraint{locus})` (ADR-028 TA-2).
- When an item runs on the digital-clock route, EN-5 SHALL scale constants by FR-238's scale
  factor, give each clock the integer values from 0 to one above its largest
  scaled constant, advance time by unit delays, and explore the digital
  subject exhaustively (ADR-028 TA-3).
- A TT-1 item over the discrete state or a deadlock-freedom item that holds
  at every reachable digital state SHALL return `HoldsDigitized`, which
  FR-235 settles `Proved{basis: Exhaustive}` with method
  `digitized-explicit-state`.
- EN-5 SHALL carry each delay of a counterexample on that route as its scaled
  integer divided by the scale factor, an exact rational in the model's
  unit.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-243-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms` (all closed) requested with `exact` evidence runs on the digital-clock route and returns `HoldsDigitized`, and so does the deadlock-freedom item requested the same way. Requested without `exact` evidence, the same two items run on EN-6 and return `Holds` with a certificate (FR-239-AC-1). | Test (TC-698) |
| FR-243-AC-2 | `NoLateReply` with `T = 3 ms` requested with `exact` evidence returns a counterexample with integer delays `0, 3, 0` that replays (FR-237). | Test (TC-698) |
| FR-243-AC-3 | The strict-guard `Rpc` variant requested with `exact` evidence settles `Unsupported(StrictClockConstraint)` naming the guard; ADR-026 §9's retry model requested without `exact` evidence runs on EN-6 and is refuted (FR-239-AC-2). | Test (TC-698) |

## Dependencies

- ADR-026 §8 EZ-10, §8.1 CF-4; ADR-028 TA-1 to TA-3 (EN-5's digital route).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992; J. Ouaknine and J. Worrell, 2003 (ADR-026 References).
