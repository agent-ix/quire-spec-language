---
id: NFR-012
title: "Bound model normalization and population admission work"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: constrains
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-011
    type: depends_on
---
# NFR-012: Bound model normalization and population admission work

## Statement

If the next normalization or admission charge would exceed a selected
ceiling, then the model binder shall stop with an incomplete result naming
the ceiling's kind, its bound and the denied charge, without performing that
charge or the work it prices.

## Scope

One normalization of a domain package under one
`ModelNormalizationLimitsV1`
([FR-081](../functional/FR-081-preserve-model-correspondence-and-declaration-identity.md)),
and one population binding admission under one `PopulationAdmissionLimitsV1`
([FR-084](../functional/FR-084-admit-closed-populations-and-resolve-lookup.md)).
The ceilings are inclusive unsigned counts. The values below are the
defaults a caller gets from `ModelNormalizationLimits::default()` and
`PopulationAdmissionLimits::default()`. A caller may set each ceiling
independently, above or below its default, and the ceiling is used as given.
A completed effective view records the limits it was normalized under
(`EffectiveView::effective_limits`); an admitted binding records its
`ancestor_steps`.

Normalization charges each phase's work before doing it, in the charge
order of `value-accounting.md`, and stops at the first denied charge. A
ceiling therefore bounds the work done, not only the result:

- Phase 4 prices every redefinition group and charges every phase-4 fact
  and conflict check before it resolves any dominance contest.
- Each effective member's `normalize.hash`, and the view's, is charged from
  a length counted from the preimage's parts, before the preimage is
  encoded or hashed.
- An effective type's identity is hashed in phase 3, before its phase-5
  `normalize.hash`, because phase 4's charge order uses it. Its encoding
  holds only that type's ancestor paths, whose total length the type's
  `normalize.cycle-check` charges have already admitted as work.
- Every fact derived along one ancestor path shares that path, so memory
  grows with the number of facts plus the total ancestor-path length, and
  the total path length is bounded by the work the cycle checks admitted. `ancestor_steps` and
`family_steps` are read, not charged: a walk that would pass one refuses
`resource_exhausted` naming it, unless an earlier charge was denied first.

A meter holds fixed-size state. It counts admitted charges and keeps no
per-charge record, so its memory does not grow with the work it bounds.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Declaration records | At most the selected ceiling (default 100000) per normalization | 100000 records by default | negative-abuse-testing |
| Derivation facts | At most the selected ceiling (default 1600000) per normalization | 1600000 facts by default | negative-abuse-testing |
| Effective declarations | At most the selected ceiling (default 1600000) per normalization | 1600000 declarations by default | negative-abuse-testing |
| Dispatch candidates | At most the selected ceiling (default 1600000) per dispatch link | 1600000 candidates by default | negative-abuse-testing |
| Hashed preimage bytes | At most the selected ceiling (default 268435456 bytes) per normalization | 268435456 bytes by default | negative-abuse-testing |
| Normalization work | At most the selected ceiling (default 16777216 work units) per normalization | 16777216 units by default | negative-abuse-testing |
| Ancestor and family steps | At most the selected ceilings (default 100000 each) per walk | 100000 steps by default | negative-abuse-testing |
| Population members | At most the selected ceiling (default 100000) per admission | 100000 members by default | negative-abuse-testing |
| Admission work | At most the selected ceiling (default 16777216 work units) per admission | 16777216 units by default | negative-abuse-testing |
| Meter memory | Constant in the number of charges | No allocation per charge | inspection-and-test |

## Default derivation

- **Declaration records and population members, 100000.** NFR-007's
  default package-entry ceiling. Each declaration record and each population
  member is at least one entry of the package or population document it is
  read from.
- **Derivation facts, 1600000.** Sixteen facts per record at the record
  ceiling. An effective declaration's derivation holds one qualify fact and
  one inherit fact per ancestor path, so this admits a package at the record
  ceiling whose declarations average fifteen ancestor paths.
- **Effective declarations and dispatch candidates, 1600000.** Every
  effective declaration carries at least one derivation fact, so this
  ceiling does not bind before the fact ceiling. A dispatch candidate pairs
  an effective subtype with a candidate declaration; the same ceiling keeps
  the pairing within the fact budget's scale.
- **Hashed bytes, 268435456.** Sixteen times NFR-007's default package byte
  ceiling. Each effective declaration's preimage is hashed once and again
  inside the view preimage, and the view holds those preimages, so this also
  bounds the view's retained size.
- **Work, 16777216.** NFR-011's default work ceiling. Each record, fact,
  declaration and hash charges one unit, and a cycle check charges its path
  length. The work ceiling binds before the fact ceiling when ancestor paths
  average more than about nine steps.
- **Ancestor and family steps, 100000.** A walk expands each type at most
  once and follows each redefinition at most once, so neither count exceeds
  the package's declaration count, which the record ceiling bounds. Every
  walk uses an explicit stack, so the ceiling is not a stack-depth bound.

Every existing normalization and admission fixture, and every QSL-196 model
benchmark input, completes at these defaults with the same result as with no
limit.

## Verification

TC-434 covers the defaults and the bound on work:

- Each limits type's default equals the values above, and a completed view
  records the defaults, or a caller's limits as given.
- Every normalization fixture has the same outcome at the defaults as
  without a limit.
- A package past `effective_declarations` or `derivation_facts` is refused
  after work bounded by the limit, whatever the package's size, and a
  diamond lattice's exponentially many ancestor paths are walked only as far
  as `derivation_facts` admits.
- Inherited facts share their type's ancestor paths at the defaults.
- A counted limit denied before the walk reaches `ancestor_steps` wins over
  the `AncestorSteps` refusal, and otherwise the walk refuses on
  `ancestor_steps`.
- A production meter and a production admission meter allocate nothing per
  charge.

## Dependencies

- [FR-081](../functional/FR-081-preserve-model-correspondence-and-declaration-identity.md)
  charges normalization under `ModelNormalizationLimitsV1`.
- [FR-084](../functional/FR-084-admit-closed-populations-and-resolve-lookup.md)
  charges admission under `PopulationAdmissionLimitsV1`.
- [NFR-007](NFR-007-bound-native-packages.md) sets the package ceilings the
  record, member and byte defaults derive from.
- [NFR-011](NFR-011-bound-value-checking-work.md) sets the work ceiling the
  work defaults reuse.
