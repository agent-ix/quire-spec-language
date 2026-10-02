---
id: FR-134
title: "Advertise fairness kinds and filter candidates by them"
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
# FR-134: Advertise fairness kinds and filter candidates by them

## Description

The EN-1 provider (FR-126) SHALL advertise, beside its (`temporal-
satisfaction`, `bounded`) and (`temporal-satisfaction`, `unbounded`)
capabilities, the fairness kinds it decides, `weak` and `strong` (ADR-019
SV-1, BE-1). QSL's registry SHALL compute an item's candidate set (FR-075)
from only the backends that advertise every fairness kind the item's
fairness set uses (ADR-019 DS-2).

## Use case

A verification operator requests a claim under strong fairness. Negotiation
sees that EN-1 decides strong fairness and routes the item there; when no
candidate advertises it, the item settles `unsupported` with a warning that
names the strong-fairness capability, and the premise is never dropped or
weakened.

## Semantic authority and boundary

QSpec owns the provider advertisement of fairness kinds, the rule that
negotiation never drops or weakens a fairness constraint, and the warning
(ADR-019 QS-4; References). QSL's registry makes the candidate set, so it
applies the fairness filter (ADR-019 DS-2). This requirement specifies what
EN-1 advertises and how the registry filters.

## Inputs

- Backend registrations (FR-075), each advertising a set of fairness kinds
  with its (`temporal-satisfaction`, mode) pairs.
- A requested item with its checked fairness set (FR-123, FR-129).

## Outputs

- The item's candidate set, each candidate carrying its fairness kinds, or
  FR-075's empty-candidate-set report naming the missing fairness
  capability.

## Behavior

- The EN-1 provider SHALL register advertising the fairness kinds `Weak`
  and `Strong`.
- The registry SHALL carry each registration's fairness kinds unchanged to
  every candidate it computes from that registration.
- The registry SHALL leave out of an item's candidate set every backend
  that does not advertise each fairness kind the item's fairness set uses,
  so a backend without `Strong` is not a candidate for an item with a
  strong constraint.
- When the filter leaves no candidate, the registry SHALL report the empty
  candidate set as FR-075 does, naming the missing fairness capability, and
  the item settles `Unsupported`, `unsupported-requested-capability`
  (ADR-019 SV-1).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-134-AC-1 | With EN-1 and a test temporal backend advertising only `Weak` registered, the candidates for an (`temporal-satisfaction`, `unbounded`) item with a weak constraint are EN-1 with fairness kinds `{Weak, Strong}` and the test backend with `{Weak}`; for the same item with a `strong` constraint the only candidate is EN-1. | Test (TC-535) |
| FR-134-AC-2 | With only the `Weak` test backend registered, an item with a `strong` constraint has an empty candidate set, reported as FR-075 reports one and naming the missing `strong` fairness capability; the item settles `Unsupported`, `unsupported-requested-capability`. | Test (TC-535) |

## Dependencies

- ADR-019 §4 BE-1, §5 SV-1, AM-4.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the
  registry and candidate computation),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (EN-1).

## References

- QSpec FR-368 (fairness kinds in negotiation): the QSpec half of ADR-019
  QS-4 (Linear STD-132).
