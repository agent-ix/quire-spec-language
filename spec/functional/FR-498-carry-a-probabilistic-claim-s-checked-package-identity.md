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
- The producer SHALL create the node identity as part of the same
  producer/emission decision that associates the node with that checked claim.
- The producer SHALL NOT infer or derive that identity from the claim's
  `name`, `span`, normal form, display text or declaration order.
- For each such node, the producer SHALL emit exactly one
  `checked_term_data.probabilistic_claims` row `{declaration, occurrence}`.
  Its `declaration` SHALL be the node's structural `NodeRef`.
- The row's `occurrence` SHALL be the node's declared `claim` occurrence.
- The row, the node and the associated `CheckedProbabilisticClaim` SHALL carry
  the same identity. Rows are in ascending structural `NodeRef` order, then
  occurrence order. The row repeats no formula, bound or other claim body, and
  no row names another claim form.
- The producer SHALL expose that same identity through the associated
  `CheckedProbabilisticClaim` to every consumer that holds it. Two checked
  claims with equal names and equal spans SHALL retain distinct producer-owned
  identities when the producer emits them as distinct claim declarations.
- An emitted package SHALL NOT carry one of these probabilistic claim nodes
  without exactly one corresponding row and checked claim, nor carry a row or
  checked claim without exactly one corresponding node and occurrence. A
  `CheckedProbabilisticClaim` checked without package emission SHALL have no
  package identity and expose none. No identity is synthesized from its name
  or span, and no protocol-location row is substituted.
- The `TimedFormShape` cause (FR-203, FR-204) and the `PunctualInterval` cause
  (FR-235) SHALL
  carry the claim's `{declaration, occurrence}` and the member path of the
  unsupported form or interval operator within that declaration's body, as QSpec
  FR-421 states its loci. A claim with no identity (checked without emission) SHALL keep the
  refusal that carries the claim's `span` as its locus (FR-186) and SHALL NOT
  report a package locus built from the span.
- Adding the `probabilistic_claims` rows SHALL NOT alter the semantic graph
  nodes or their identities: each row names the identity already associated
  with its emitted claim node, and enabling or omitting the sidecar SHALL NOT
  remint that node or any other graph node.
- QSL emission SHALL NOT claim reader admission; QSpec FR-322 and TC-233 own the
  reader refusals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-498-AC-1 | A unit with two checked probabilistic claims that have equal display names and equal source spans emits exactly two local `claim` nodes with `semantic_form` `probabilistic_claim`, each with one declared `claim` occurrence, and exactly two matching `probabilistic_claims` rows with distinct declaration `NodeRef`s in ascending structural order (QSpec FR-322-AC-297 `PC-01`, `PC-02`). Permuting source order preserves the set of producer-owned node and occurrence identities and emits rows in the required structural order. | Test (TC-1288) |
| FR-498-AC-2 | Each emitted claim node, its `probabilistic_claims` row and its associated `CheckedProbabilisticClaim` expose the same `{declaration, occurrence}`; equal names and spans do not make the two claim identities equal, and the producer does not infer identity from name, span, normal form, display text or declaration order. | Test (TC-1288) |
| FR-498-AC-3 | An elapsed-time claim with an unsupported form and a claim with a punctual interval in a non-admitted form settle `Unsupported(TimedFormShape)` and `Unsupported(PunctualInterval)` carrying the claim's `{declaration, occurrence}` and the member path of the unsupported form or interval operator (QSpec FR-421-AC-7); for two claims with equal names and spans the cause names the selected claim only. | Test (TC-1288) |
| FR-498-AC-4 | An emitted package has a one-to-one correspondence among each included `CheckedProbabilisticClaim`, one local `claim`/`probabilistic_claim` node with one declared `claim` occurrence, and one matching sidecar row; it has no unmatched node, row or checked claim. A claim checked without emission exposes no package identity and its cause keeps the span-locus refusal of FR-186, with no row, locus or key synthesized from its name or span. Emitting or omitting the sidecar rows leaves the semantic graph node set and node identities unchanged. | Test (TC-1288) |

## Dependencies

- **Upstream:** FR-186 defines `CheckedProbabilisticClaim`; FR-203 and FR-204 own
  `TimedFormShape` and FR-235 owns `PunctualInterval`, the causes this wires; QSpec FR-322 (FR-322-AC-297), TC-233 and FR-421 own the
  carrier, its reader checks and the locus.
- **Resolved here:** this requirement defines emission of the local
  `claim`/`probabilistic_claim` node and its one-to-one association with the
  checked claim and FR-322 sidecar row. It intentionally specifies no identity
  algorithm; the producer owns creating and carrying the identity as part of
  its emission decision under the constraints above.
