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
backend advertising `temporal-satisfaction` carries (FR-331), and names no
refusal cause for a manifest that breaks that definition. QSpec FR-272 defines
`unknown-kind` as an FR-290 capability-kind label, so reusing it for a fairness
label would give one cause two meanings, as QSpec avoids for the evidence-kind
set with `unknown-evidence-kind`. QSL therefore refuses a defective
`fairness_kinds` at registration, with three causes under `invalid_capability`
that are QSL-proposed pending a QSpec catalog ruling (Remaining work):

- A label that is not byte-equal to `weak` or `strong` (including `""`, a
  non-string value, a case variant such as `Weak` and a Rust variant name)
  SHALL be refused `invalid_capability`/`unknown-fairness-kind`, naming the
  received bytes. No refusal carries a replacement kind.
- A label that appears more than once SHALL be refused
  `invalid_capability`/`duplicate-fairness-kind`, naming the repeated label.
  The member is a set, so a repeat is an authoring defect, as a repeated
  `domains` kind is; it is never collapsed to one.
- A manifest that advertises a `temporal-satisfaction` pair and omits the
  `fairness_kinds` member SHALL be refused `invalid_capability`/
  `absent-fairness-kinds`, because FR-331 says such a backend carries it. A
  present empty member is admitted and advertises no fairness kind, so the
  backend is a candidate only for an item whose fairness set is empty. A
  manifest that advertises no `temporal-satisfaction` pair needs no member.

A manifest with both an unknown and a repeated label is refused as unknown,
so an unknown label takes precedence over a repeat. Each refusal is keyed by backend identity and made before
registry insertion, so the backend contributes no candidate and leaves any earlier registration
under that identity standing. When a manifest has a capability-pair or
`domains` defect (FR-057) as well, the refusal carries that earlier cause; the
fairness causes apply only to a manifest whose pairs and `domains` are
admitted. The candidate filter above therefore sees only admitted kinds.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-134-AC-1 | With EN-1 and a test temporal backend advertising only `Weak` registered, the candidates for an (`temporal-satisfaction`, `unbounded`) item with a weak constraint are EN-1 with fairness kinds `{Weak, Strong}` and the test backend with `{Weak}`; for the same item with a `strong` constraint the only candidate is EN-1. | Test (TC-535) |
| FR-134-AC-2 | With only the `Weak` test backend registered, an item with a `strong` constraint has an empty candidate set, reported as FR-075 reports one and naming the missing `strong` fairness capability; the item settles `Unsupported`, `unsupported-requested-capability`. | Test (TC-535) |
| FR-134-AC-3 | A manifest advertising `temporal-satisfaction` whose `fairness_kinds` holds a label not byte-equal to `weak` or `strong` (such as `Weak`, `""`, `eventual` or a non-string) is refused `invalid_capability`/`unknown-fairness-kind`, keyed by backend identity and naming the received bytes; it is never refused as `unknown-kind`, never read as a nearest kind, and contributes no candidate. | Test (TC-535) |
| FR-134-AC-4 | A manifest whose `fairness_kinds` repeats a label (`["weak","weak"]`, or `["weak","strong","weak"]`) is refused `invalid_capability`/`duplicate-fairness-kind` naming the repeated label; it is not collapsed to a set. A manifest holding each of `weak` and `strong` once is admitted. | Test (TC-535) |
| FR-134-AC-5 | A manifest advertising a `temporal-satisfaction` pair with no `fairness_kinds` member is refused `invalid_capability`/`absent-fairness-kinds`; the same manifest with `fairness_kinds` present and empty is admitted, advertises no fairness kind and is not a candidate for an item with a `weak` or `strong` constraint; a manifest with no `temporal-satisfaction` pair and no member is admitted. A refusal under AC-3 to AC-5 leaves an earlier registration under that identity standing. | Test (TC-535) |

## Dependencies

- ADR-019 §4 BE-1, §5 SV-1, AM-4.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) (the
  registry and candidate computation),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (EN-1).

## Remaining work

- The causes `unknown-fairness-kind`, `duplicate-fairness-kind` and
  `absent-fairness-kinds` are QSL-proposed. QSpec FR-368 and the FR-272
  `invalid_capability` row do not yet list them; the QSpec follow-up is
  Linear STD-194. Until QSpec rules, QSL keeps them here and a different QSpec
  cause name replaces these three.

## References

- QSpec FR-368 (fairness kinds in negotiation): the QSpec half of ADR-019
  QS-4 (Linear STD-132).
