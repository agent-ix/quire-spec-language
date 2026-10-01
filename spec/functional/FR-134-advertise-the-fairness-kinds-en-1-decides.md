---
id: FR-134
title: "Advertise the fairness kinds EN-1 decides"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-016
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-134: Advertise the fairness kinds EN-1 decides

## Description

The EN-1 provider (FR-126) SHALL advertise, beside its (`temporal-
satisfaction`, `bounded`) and (`temporal-satisfaction`, `unbounded`)
capabilities, the fairness kinds it decides, `weak` and `strong` (ADR-019
SV-1, BE-1). QSL's registry SHALL carry each registered backend's fairness
kinds to the candidates it computes (FR-075), so negotiation can route an
item with a strong constraint only to a candidate that advertises `strong`.

## Use case

A verification operator requests a claim under strong fairness. Negotiation
sees that EN-1 decides strong fairness and routes the item there; when no
candidate advertises it, the item settles `unsupported` with a warning that
names the strong-fairness capability, and the premise is never dropped.

## Semantic authority and boundary

QSpec owns the provider advertisement of fairness kinds, the rule that
negotiation never drops or weakens a fairness constraint, and the warning
(ADR-019 QS-4; References). CG's negotiation routes on the advertised kinds
(ADR-019 DS-2). This requirement specifies what EN-1 advertises and what
QSL's registry carries.

## Inputs

- Backend registrations (FR-075), each advertising a set of fairness kinds
  with its (`temporal-satisfaction`, mode) pairs.

## Outputs

- `FairnessKind`s on each temporal backend's registration and on each
  candidate the registry computes for it.

## Behavior

- The EN-1 provider SHALL register advertising the fairness kinds `Weak`
  and `Strong`.
- The registry SHALL carry each registration's fairness kinds unchanged to
  every candidate it computes from that registration.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-134-AC-1 | With EN-1 and a test temporal backend advertising only `Weak` registered, the candidates computed for an (`temporal-satisfaction`, `unbounded`) item are EN-1 with fairness kinds `{Weak, Strong}` and the test backend with `{Weak}`. | Test (TC-535) |

## Dependencies

- ADR-019 §4 BE-1, §5 SV-1, AM-4.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the
  registry and candidate computation),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (EN-1).

## References

- QSpec half of ADR-019, carrying ADR-019 QS-4: Linear STD-132.
