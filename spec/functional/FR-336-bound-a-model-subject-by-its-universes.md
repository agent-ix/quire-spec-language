---
id: FR-336
title: "Make a model subject finite by its universes, apart from proof bounds"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-015
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: depends_on
---
# FR-336: Make a model subject finite by its universes, apart from proof bounds

## Description

A temporal item over a model subject (FR-125) SHALL become finite only
through the subject's universes and the model's own declared domains, never
through a proof bound, a default or a limit (ADR-018 §1 Scope, EN-1
pre-check; ADR-016 §2, EX-1). A universe is part of the subject and of the
item's obligation identity, and a verdict holds for exactly that subject.
The item's requirement record is computed from checked types alone, so an
unbounded declaration keeps an `Unbounded` record whatever universe a
request supplies (ADR-014 §4).

## Use case

An author models accounts with `Population<Account>` and no maximum, and
checks a liveness claim over two accounts, then over three. Each run tells
them which universe its verdict is about. When they forget a universe, or
an operation takes an unranged `Integer`, the checker says which root needs
a bound and how to give one, instead of picking one.

## Inputs

- A model subject (FR-125): the checked package, initial states, universes
  and `ProofBound`s.
- A temporal item over it (FR-123, FR-124) and its requirement record.

## Outputs

- `ModelCheckRefusal::RequiresBound` naming each unbounded root, or an
  explored run (FR-126).
- The item's obligation identity, binding the subject.

## Behavior

- The requirement record of a temporal clause over a population with no
  declared maximum SHALL be `Unbounded` with that population's domain
  (FR-097), independent of the universes any request supplies.
- When a run starts, EN-1 SHALL classify every root of the subject (FR-120
  `domains()` under the subject's universes) before any expansion: a
  population root with a universe is finite with the universe's length, one
  with no universe is unbounded, and a parameter, result or field root is
  finite exactly when its declared type is.
- When any root is unbounded, EN-1 SHALL return
  `ModelCheckRefusal::RequiresBound` naming each unbounded root's
  `WireNodeId`, before exploring any state. The caller answers a population
  root with a universe and any other root by changing the model, for
  example with a `bounded_domain`.
- A `ProofBound` in the subject SHALL NOT make an EN-1 root finite. It
  qualifies the item through its obligation identity (ADR-014 B-4) and is
  read by a bounded-mode backend reached by negotiation.
- The request writer SHALL keep a universe apart from every `ProofBound`:
  a universe changes neither the item's proof bounds nor its requirement
  record.
- The item's obligation identity SHALL bind the subject, its universes
  included (ADR-013 O-09), so a verdict over one universe joins only its own
  request and is no verdict over another universe or over the unbounded
  declaration.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-336-AC-1 | Over ADR-018 §6's ConfigVersion example with `config_history` declared with no maximum, the clause `ReachesTwo` has record (`temporal-satisfaction`, `Unbounded`) with the population's domain both when the request supplies universe `{a, b}` and when it supplies `{a, b, c}`. | Test (TC-846) |
| FR-336-AC-2 | The same subject with no universe for `config_history` returns `RequiresBound` naming the population root and explores no state; with `{a, b}` it explores and returns FR-126-AC-1's outcome. | Test (TC-846) |
| FR-336-AC-3 | A `Counter` variant whose operation `step(n: Integer)` takes an unranged parameter returns `RequiresBound` naming that parameter root, both with no `ProofBound` and with a `ProofBound{IntegerRange{0, 3}}` on the clause's domain; with `n: Int[0, 3]` declared in the model it explores. | Test (TC-846) |
| FR-336-AC-4 | `ReachesTwo` under `fair weak each` over universe `{a, b}` and over `{a, b, c}` gives two obligation identities that differ only in the subject's universe, and each `proved` result joins only its own request index. | Test (TC-846) |

## Dependencies

- ADR-018 §1 (Scope), §2 SM-2, §3 (EN-1 pre-check, amended in place with the
  proof-bound rule); ADR-016 §2 and EX-1; ADR-014 §1 B-4, §4, §8; ADR-013
  O-09.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) ("Bounds"),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (pre-check), [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md),
  [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md).
- QSpec FR-153-AC-9 (a population with no declared maximum is unbounded).

## References

- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: Linear STD-131 (QS-1, QS-9: universes in the subject and the
  obligation identity).
