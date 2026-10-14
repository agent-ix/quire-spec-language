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
  holds its own key and one direct-supertype edge with that parent's identity
  per parent, each edge already admitted by a `normalize.fact` and a
  `normalize.cycle-check` charge. No encoding holds an ancestor path longer
  than one edge, so hashed bytes grow with the type count plus the edge count
  (for the 83 to 87 byte keys of TC-1025, about 902 bytes per type and 608
  per edge, counting each preimage once on its own and once inside the view),
  not with the number of ancestor paths.
- A diamond lattice is walked and stored once per edge. The cycle check
  charges one unit per direct edge for an acyclic package; a path-length
  charge applies only to the walk that reports a specialization cycle.
- Memory grows with the retained facts plus the closure the conformance
  walks read; a chain's proper-ancestor closure is quadratic in its length,
  so a representation holds it as compact indices, not as cloned keys.
- `ancestor_steps` and `family_steps` are charged like every other ceiling:
  each `supertypes` edge a walk follows is charged before it is followed
  (FR-082), and each distinct `redefines` edge is charged once, before the
  first walk follows it, however many walks follow it (FR-083), and the first denied charge stops the walk
  `Incomplete`. They count edges, never a depth, and every walk uses an
  explicit stack.

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
| Ancestor steps | At most the selected ceiling (default 16777216 edges) per conformance walk | 16777216 edges by default | negative-abuse-testing |
| Family steps | At most the selected ceiling (default 16777216 edges) per dispatch-family link | 16777216 edges by default | negative-abuse-testing |
| Population members | At most the selected ceiling (default 100000) per admission | 100000 members by default | negative-abuse-testing |
| Admission work | At most the selected ceiling (default 16777216 work units) per admission | 16777216 units by default | negative-abuse-testing |
| Meter memory | Constant in the number of charges | No allocation per charge | inspection-and-test |

## Default derivation

- **Declaration records and population members, 100000.** NFR-007's
  default package-entry ceiling. Each declaration record and each population
  member is at least one entry of the package or population document it is
  read from.
- **Derivation facts, 1600000.** Sixteen facts per record at the record
  ceiling. An effective declaration's derivation holds one qualify fact, one
  inherit fact per direct parent, and its phase-4 facts, so this admits a
  package at the record ceiling whose declarations average fifteen direct
  parents and phase-4 facts. It counts retained facts, never the path-form
  facts of their expansion.
- **Effective declarations and dispatch candidates, 1600000.** Every
  effective declaration carries at least one derivation fact, so this
  ceiling does not bind before the fact ceiling. A dispatch candidate pairs
  an effective subtype with a candidate declaration; the same ceiling keeps
  the pairing within the fact budget's scale.
- **Hashed bytes, 268435456.** Sixteen times NFR-007's default package byte
  ceiling. With about 902 bytes per effective type and 608 per direct edge
  (keys of 83 to 87 bytes), it admits a single-inheritance chain up to the
  100000 record ceiling (99999 edges, 151633328 bytes) and about 426000 direct
  edges over 10001 types. A chain whose every type declares one field has
  (n+1)(n+2)/2 effective members and is refused beyond 527 edges on this
  ceiling and beyond 1786 on the effective-declaration ceiling; those are member counts, which identity
  chaining does not reduce. Each effective declaration's preimage is hashed once and again
  inside the view preimage, and the view holds those preimages, so this also
  bounds the view's retained size.
- **Work, 16777216.** NFR-011's default work ceiling. Each record, fact,
  declaration, hash and direct-edge cycle check charges one unit. The work
  ceiling does not bind before the fact ceiling for an acyclic package.
- **Ancestor and family steps, 16777216 edges.** The work default above,
  NFR-011's S3 work default. Both count edges followed, a unit of work: a
  walk follows each edge of a type's closure at most once, so the default
  admits any package whose closures fit the work default.

Every existing normalization and admission fixture, and every model
benchmark input, completes at these defaults with the same result as with no
limit.

## Verification

TC-434 covers the defaults and the bound on work:

- Each limits type's default equals the values above, and a completed view
  records the defaults, or a caller's limits as given.
- Every normalization fixture has the same outcome at the defaults as
  without a limit.
- A package past `effective_declarations`, `derivation_facts` or
  `hashed_bytes` is refused after work bounded by the limit, whatever the
  package's size.
- A 10000-edge single-inheritance chain completes at the defaults with 20001
  retained facts, 15104845 hashed bytes and 60006 work units (TC-1025), and a diamond ladder with exponentially many
  paths completes at the defaults (TC-1024).
- The first denied charge decides the outcome: a `declaration_records`
  charge denied before the walk exhausts `ancestor_steps` stops on
  `declaration_records`, and otherwise the walk stops `Incomplete` on
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
