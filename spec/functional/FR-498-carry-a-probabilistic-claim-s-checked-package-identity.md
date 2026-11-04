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

- A checked probabilistic claim (FR-186).
- The checked-package producer's decision to emit that claim in the local
  checked semantic graph.

## Outputs

- For each emitted checked probabilistic claim, one local `claim` node with
  `semantic_form` `probabilistic_claim`, one declared `claim` occurrence,
  exactly one corresponding `checked_term_data.probabilistic_claims` row of
  QSpec FR-322, and a `CheckedProbabilisticClaim` that exposes that same
  producer-owned identity.

## Behavior

- For each `CheckedProbabilisticClaim` included in an emitted package, the
  producer SHALL emit exactly one local `claim` node whose `semantic_form` is
  `probabilistic_claim`.
- For each such node, the producer SHALL declare exactly one occurrence with
  role `claim`.
- The node's `node_id` SHALL be the QSpec FR-322 structural `NodeRef`
  `{package, path, ordinal}` the producer assigns when it lowers the checked
  claim: `package` is the unit's package, `path` is
  `["claim", "probabilistic_claim", N]` for the claim's declared name `N`, and
  `ordinal` ranks the claims of that package and path by their written
  content, ascending, from 0. A claim's written content is the exact spelling
  of its significant tokens in source order, with no span, whitespace,
  comment or source position.
- Equal names or equal spans SHALL NOT merge two claim declarations: each
  checked claim has its own node, which the `ordinal` distinguishes. Two
  claims with equal names and equal written content are indistinguishable,
  so permuting them yields identical output and any assignment of their
  ordinals between them conforms.
- Moving a claim declaration within its source, reformatting or commenting
  it, or reordering the unit's declarations SHALL leave every claim's
  `{declaration, occurrence}` unchanged.
- The node's body SHALL be an empty aggregate while QSpec defines no
  `probabilistic_claim` body; the producer fabricates no formula or bound.
- For each such node, the producer SHALL emit exactly one
  `checked_term_data.probabilistic_claims` row `{declaration, occurrence}`.
  Its `declaration` SHALL be the node's structural `NodeRef`.
- The row's `occurrence` SHALL be the node's declared `claim` occurrence.
- The row, the node and the associated `CheckedProbabilisticClaim` SHALL carry
  the same identity. Rows are in ascending structural `NodeRef` order, then
  occurrence order. The row repeats no formula, bound or other claim body, and
  no row names another claim form.
- The producer SHALL expose that same identity through the associated
  `CheckedProbabilisticClaim` to every consumer that holds it. No consumer
  selects or joins a claim's identity by its name, span, display text or
  declaration order; it reads the exposed `{declaration, occurrence}`.
- An emitted package SHALL NOT carry one of these probabilistic claim nodes
  without exactly one corresponding row and checked claim, nor carry a row or
  checked claim without exactly one corresponding node and occurrence. No
  protocol-location row is substituted for a claim row.
- The `TimedFormShape` cause (FR-203, FR-204) and the `PunctualInterval` cause
  (FR-235) SHALL
  carry the claim's `{declaration, occurrence}` and the member path of the
  unsupported form or interval operator within that declaration's body, as QSpec
  FR-421 states its loci. A claim with no identity (one checked outside an assembled package) SHALL keep the
  refusal that carries the claim's `span` as its locus (FR-186) and SHALL NOT
  report a package locus built from the span.
- The producer SHALL assign the claim node identities when it lowers the
  checked claims, before it writes any `probabilistic_claims` row: each row names
  the identity already associated with its emitted claim node, and writing
  the rows mints or remints no graph node.
- QSL emission SHALL NOT claim reader admission; QSpec FR-322 and TC-233 own the
  reader refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-498-AC-1 | A unit with two checked probabilistic claims that share a declared name and differ in written content (the QSL counterpart of QSpec FR-322-AC-297 `PC-01` and `PC-02`; one source cannot give two declarations equal spans), and a third claim with another name, emits exactly three local `claim` nodes with `semantic_form` `probabilistic_claim`, each with one declared `claim` occurrence, and exactly three matching `probabilistic_claims` rows with distinct declaration `NodeRef`s in ascending structural order. Every permutation of the three declarations keeps each claim's `{declaration, occurrence}` and emits byte-identical rows. | Test (TC-1288) |
| FR-498-AC-2 | Each emitted claim node, its `probabilistic_claims` row and its associated `CheckedProbabilisticClaim` expose the same `{declaration, occurrence}`. Renaming one claim changes its identity and no other claim's; moving, reformatting or commenting a claim leaves its written content, and so its identity, unchanged; changing a token of it changes its written content. Two claims written identically are two declarations with distinct identities. | Test (TC-1288) |
| FR-498-AC-3 | An elapsed-time claim with an unsupported form and a claim with a punctual interval in a non-admitted form settle `Unsupported(TimedFormShape)` and `Unsupported(PunctualInterval)` carrying the claim's `{declaration, occurrence}` and the member path of the unsupported form or interval operator (QSpec FR-421-AC-7); for two claims with equal names the cause names the selected claim only. | Test (TC-1288) |
| FR-498-AC-4 | An emitted package has a one-to-one correspondence among each included `CheckedProbabilisticClaim`, one local `claim`/`probabilistic_claim` node with one declared `claim` occurrence, and one matching sidecar row; it has no unmatched node, row or checked claim, every row names a node already in the emitted graph with that node's own occurrence, and the strict reader admits the package. | Test (TC-1288) |

## Dependencies

- **Upstream:** FR-186 defines `CheckedProbabilisticClaim`; FR-203 and FR-204 own
  `TimedFormShape` and FR-235 owns `PunctualInterval`, the causes this wires; QSpec FR-322 (FR-322-AC-297), TC-233 and FR-421 own the
  carrier, its reader checks and the locus.
- **Resolved here:** this requirement defines emission of the local
  `claim`/`probabilistic_claim` node, its structural `NodeRef` (path and
  content-ranked ordinal) and its one-to-one association with the checked
  claim and FR-322 sidecar row.
- **Open:** FR-498-AC-3's member path names a member of the claim
  declaration's body, which needs a QSpec `probabilistic_claim` body
  definition; until one exists the node body is empty and AC-3 is not met.
