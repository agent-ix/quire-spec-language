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
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-368
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: references
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
  with its (`temporal-satisfaction`, mode) pairs; for a manifest backend,
  the FR-331 manifest's `fairness_kinds` member as supplied, which may be
  absent.
- A requested item with its checked fairness set (FR-123, FR-129).

## Outputs

- The item's candidate set, each candidate carrying its fairness kinds, or
  FR-075's empty-candidate-set report naming the missing fairness
  capability.
- For a manifest backend whose `fairness_kinds` is defective, a registration
  refusal keyed by backend identity.

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

### Manifest `fairness_kinds` admission

QSpec FR-368 defines `fairness_kinds` as a subset of {`weak`, `strong`} that a
backend advertising `temporal-satisfaction` carries (FR-331), and refuses a
defective one at registration with one cause, `invalid_capability`/
`invalid-fairness-kinds`, keyed by backend identity (QSpec FR-272, FR-368-AC-8
to AC-10). The registry applies that rule:

- If a backend advertises a `temporal-satisfaction` pair and its
  `fairness_kinds` is absent, is not a list, holds an element not byte-equal to
  `weak` or `strong` (including `""`, a non-string element, a case variant such
  as `Weak` and a Rust variant name), or repeats a label, then the registry
  SHALL refuse the registration with `invalid_capability`/
  `invalid-fairness-kinds`, retaining the defect (`absent`, `not-a-list`,
  `unknown-label` or `duplicate-label`) and, for the last three, the received
  value or element.
- When several defects hold, the registry SHALL report the first of `absent`,
  `not-a-list`, `unknown-label`, `duplicate-label`, and within `unknown-label`
  or `duplicate-label` the first offending element in member order (for
  `duplicate-label`, the first element that repeats an earlier element). A
  manifest with an unknown element and a repeat is refused `unknown-label`.
- The registry SHALL admit a present empty list, which advertises no fairness
  kind, so the backend is a candidate only for an item whose fairness set is
  empty.
- The registry SHALL ignore, without validating, a `fairness_kinds` member of a
  backend that advertises no `temporal-satisfaction` pair, as QSpec FR-368
  states.

The registry SHALL judge `fairness_kinds` after every advertised pair and
`domains`, so a manifest with a capability-pair or `domains` defect (FR-057) as
well is refused with that earlier cause. The registry SHALL make each refusal
before registry insertion, keyed by backend identity, so the backend
contributes no candidate and any earlier registration under that identity
stands. The candidate filter above therefore sees only admitted kinds. No
refusal carries a replacement kind.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-134-AC-1 | With EN-1 and a test temporal backend advertising only `Weak` registered, the candidates for an (`temporal-satisfaction`, `unbounded`) item with a weak constraint are EN-1 with fairness kinds `{Weak, Strong}` and the test backend with `{Weak}`; for the same item with a `strong` constraint the only candidate is EN-1. | Test (TC-535) |
| FR-134-AC-2 | With only the `Weak` test backend registered, an item with a `strong` constraint has an empty candidate set, reported as FR-075 reports one and naming the missing `strong` fairness capability; the item settles `Unsupported`, `unsupported-requested-capability`. | Test (TC-535) |
| FR-134-AC-3 | A manifest advertising `temporal-satisfaction` whose `fairness_kinds` is not a list (`null`, a string, an object) is refused `invalid_capability`/`invalid-fairness-kinds` retaining `not-a-list` and the member as supplied; one holding an element not byte-equal to `weak` or `strong` (such as `Weak`, `""`, `eventual` or a non-string) is refused `invalid-fairness-kinds` retaining `unknown-label` and the first such element in member order. Neither is refused as `unknown-kind`, read as a nearest kind or contributes a candidate. | Test (TC-535) |
| FR-134-AC-4 | A manifest advertising `temporal-satisfaction` whose `fairness_kinds` repeats a label (`["weak","weak"]`, or `["weak","strong","weak"]`) and holds no unknown element is refused `invalid-fairness-kinds` retaining `duplicate-label` and the first element that repeats an earlier element; it is not collapsed to a set. A manifest holding each of `weak` and `strong` once is admitted. A manifest with both an unknown element and a repeat is refused with `unknown-label`. | Test (TC-535) |
| FR-134-AC-5 | A manifest advertising a `temporal-satisfaction` pair with no `fairness_kinds` member is refused `invalid_capability`/`invalid-fairness-kinds` retaining `absent`; the same manifest with `fairness_kinds` present and empty is admitted, advertises no fairness kind and is not a candidate for an item with a `weak` or `strong` constraint; a manifest with no `temporal-satisfaction` pair is admitted whether `fairness_kinds` is absent, empty or defective, and its member is not validated. A refusal under AC-3 to AC-5 leaves an earlier registration under that identity standing. | Test (TC-535) |
| FR-134-AC-6 | A manifest with both a capability-pair or `domains` defect (FR-057) and a defective or absent `fairness_kinds` is refused with the pair or `domains` cause, not a fairness cause. | Test (TC-535) |

## Dependencies

- ADR-019 §4 BE-1, §5 SV-1, AM-4.
- [FR-057](FR-057-admit-shared-capability-kinds.md) (registration causes and
  their order, which the fairness causes follow),
  QSpec FR-368 and FR-331 (the `fairness_kinds` member) and FR-272 (the closed
  `invalid_capability` cause row).
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the
  registry and candidate computation),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (EN-1).

## Remaining work

- QSpec ratifies the cause `invalid-fairness-kinds` and FR-368-AC-8 to AC-10
  in quire-specification#313; the refusals here follow that text. The code
  that emits the three earlier causes (`qsl-route/src/lib.rs`,
  `qsl-route/tests/it/fairness_kinds.rs`) is Linear QSL-925.

## References

- QSpec FR-368 (fairness kinds in negotiation): the QSpec half of ADR-019
  QS-4 (Linear STD-132).
