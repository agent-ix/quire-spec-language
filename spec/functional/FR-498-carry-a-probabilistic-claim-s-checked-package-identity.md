---
id: FR-498
title: "Carry a probabilistic claim's checked-package identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-421
    type: depends_on
---
# FR-498: Carry a probabilistic claim's checked-package identity

## Description

QSpec FR-421 reports its `timed-form-shape{locus}` and `punctual-interval{locus}`
causes against a probabilistic claim's exact `{declaration NodeRef, occurrence}`
resolved through `checked_term_data.probabilistic_claims` (QSpec FR-322-AC-297).
[FR-186](./FR-186-check-probabilistic-claim-forms-at-s3.md)'s
`CheckedProbabilisticClaim` holds only a `name`, a `span` and a normal form,
neither of which is an identity (ADR-013 R-04), so the exact plan cannot name
the locus of its `TimedFormShape` (FR-203, FR-204) and `PunctualInterval`
(FR-235) causes; QSL-1416 records that gap. This requirement owns carrying the claim's
checked-package identity from the producer to the carrier and to those causes.

## Inputs

- A checked probabilistic claim (FR-186) and the `claim` node with
  `semantic_form` `probabilistic_claim` that the emitter produced for its
  declaration, with one declared `claim` occurrence.

## Outputs

- A `checked_term_data.probabilistic_claims` array of QSpec FR-322, and a
  `CheckedProbabilisticClaim` that exposes the identity of its row.

## Behavior

- The emitter SHALL write exactly one `probabilistic_claims` row
  `{declaration, occurrence}` for each probabilistic claim declaration emitted
  as a local `claim` node whose `semantic_form` is `probabilistic_claim`.
  `declaration` is that node's structural `NodeRef` and `occurrence` one of its
  occurrences with role `claim`. Rows are in ascending structural `NodeRef`
  order, then occurrence order. The row repeats no formula, bound or other
  claim body, and no row names another claim form.
- `CheckedProbabilisticClaim` SHALL carry the producer's `NodeRef` and
  occurrence for its own declaration, taken from the same emission decision
  that wrote the row, and SHALL expose them to every consumer that holds the
  claim. The identity SHALL NOT be derived from `name`, `span`, normal form,
  display text or declaration order: two claims with equal names and equal
  spans keep two identities.
- Every probabilistic claim declaration of an emitted package has exactly one
  row (QSpec FR-322-AC-297), so a package SHALL NOT carry a probabilistic claim
  node without its row. A `CheckedProbabilisticClaim` checked without emission
  (no package is produced) has no identity and exposes none. No identity is
  synthesized from its name or span, and no protocol-location row is
  substituted.
- The `TimedFormShape` cause (FR-203, FR-204) and the `PunctualInterval` cause
  (FR-235) SHALL
  carry the claim's `{declaration, occurrence}` and the member path of the
  unsupported form or interval operator within that declaration's body, as QSpec
  FR-421 states its loci. A claim with no identity (checked without emission) SHALL keep the
  refusal that carries the claim's `span` as its locus (FR-186) and SHALL NOT
  report a package locus built from the span.
- Adding rows leaves the semantic graph nodes and node identities equal to those
  emitted for the same package without the carrier.
- QSL emission SHALL NOT claim reader admission; QSpec FR-322 and TC-233 own the
  reader refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-498-AC-1 | A unit with two probabilistic claims that have equal display names and equal source spans emits two `probabilistic_claims` rows with distinct declaration `NodeRef`s, each naming one declared `claim` occurrence, in ascending structural order (QSpec FR-322-AC-297 `PC-01`, `PC-02`); permuting source order emits equal rows. | Test (TC-1288) |
| FR-498-AC-2 | `CheckedProbabilisticClaim` for each of those claims exposes the same `{declaration, occurrence}` as its emitted row, and an equal name and span on the two claims do not make the identities equal. | Test (TC-1288) |
| FR-498-AC-3 | An elapsed-time claim with an unsupported form and a claim with a punctual interval in a non-admitted form settle `Unsupported(TimedFormShape)` and `Unsupported(PunctualInterval)` carrying the claim's `{declaration, occurrence}` and the member path of the unsupported form or interval operator (QSpec FR-421-AC-7); for two claims with equal names and spans the cause names the selected claim only. | Test (TC-1288) |
| FR-498-AC-4 | A package that emits probabilistic claim nodes has exactly one row per claim and none without a node; a claim checked without emission exposes no identity and its cause keeps the span-locus refusal of FR-186, with no row, locus or key synthesized from its name or span. A package with the rows has the same semantic graph nodes and identities as one without. | Test (TC-1288) |

## Dependencies

- **Upstream:** FR-186 defines `CheckedProbabilisticClaim`; FR-203 and FR-204 own
  `TimedFormShape` and FR-235 owns `PunctualInterval`, the causes this wires; QSpec FR-322 (FR-322-AC-297), TC-233 and FR-421 own the
  carrier, its reader checks and the locus.
- **Open:** no QSL requirement states the emission of the `claim` node with
  `semantic_form` `probabilistic_claim` that the row joins (see QSL-1416); this
  requirement assumes that node exists wherever a row is written.
