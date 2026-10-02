---
id: FR-243
title: "Route timed items to the zone engine or the digital-clock route"
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
# FR-243: Route timed items to the zone engine or the digital-clock route

## Description

The request writer SHALL route every non-probabilistic timed item (TT-1 to
TT-4, the deadlock-freedom item and the time-lock-freedom item) to EN-6's
zone search (FR-239), so a timed invariant is proved by `Holds` with a CF-1
certificate (ADR-026 EZ-10). Only a probabilistic claim over a timed
subject that names `exact` evidence takes the digital-clock route, EN-5's
reading of a closed subject through integer-valued clocks (ADR-028 TA-1 to
TA-3, FR-204), whose results settle by FR-203.

## Use case

A verification operator's model has only non-strict guards and deadlines.
Its timed invariant is proved by the zone engine with a certificate. A
probabilistic deadline claim over the same model with `exact` evidence is
decided on the digital-clock route with an exact value.

## Inputs

- A timed item, its capability kind and, for a probabilistic claim, its
  requested evidence kind.

## Outputs

- The routed engine for the item: EN-6 or EN-5's digital-clock route.

## Behavior

- When a timed item's capability kind is `temporal-satisfaction`, the
  request writer SHALL route it to EN-6, whatever its constraints.
- When a `probabilistic-satisfaction` item over a timed subject names
  `exact` evidence, the request writer SHALL route it to EN-5's
  digital-clock route (FR-204); when it names `statistical` evidence, to
  EN-4 (FR-254).
- The route SHALL be a function of the capability kind and the requested
  evidence kind alone.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-243-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms` (all closed) and the deadlock-freedom item over the same subject route to EN-6 and return `Holds` with a certificate (FR-239-AC-1). | Test (TC-698) |
| FR-243-AC-2 | ADR-028 §15.5's `Deadline` over `Retx` with `exact` evidence routes to EN-5's digital-clock route and settles `proved`, `ExactValue{99/100}` (FR-204-AC-1); with `statistical` evidence it routes to EN-4. | Test (TC-698) |
| FR-243-AC-3 | ADR-026 §9's retry model, with strict constraints, routes `always holds(not retried)` to EN-6 and is refuted (FR-239-AC-2); the all-closed `Rpc` routes the same claim kind to EN-6 too. | Test (TC-698) |

## Dependencies

- ADR-026 §8 EZ-10, §8.1 CF-4; ADR-028 TA-1 to TA-3 (EN-5's digital route).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992; J. Ouaknine and J. Worrell, 2003 (ADR-026 References).
