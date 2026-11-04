---
id: FR-053
title: "Preserve opaque semantic-trigger identity on the checked-package route"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-449, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: references }
  - { target: ix://agent-ix/quire-specification/FR-093, type: references }
  - { target: ix://agent-ix/quire-specification/FR-230, type: references }
  - { target: ix://agent-ix/quire-specification/FR-479, type: references }
---
# FR-053: Preserve opaque semantic-trigger identity on the checked-package route

## Description

When the Layer-4 checked-package route evaluates a temporal obligation
activated by a semantic trigger, the evaluator SHALL preserve the exact opaque
trigger identity selected by the observation owner as the identity of that
activation, in accordance with FR-044 and the FR-449 activation mapping.

## Behavior

The semantic-trigger identity SHALL remain distinct from timestamps, receipt
identities, display text, payloads, capture values, static binding coordinates,
declaration positions and table order. The route SHALL NOT derive or substitute
the trigger identity from any of those values. It SHALL preserve distinct
identities as distinct instances even when their other values are equal, and
the evaluator SHALL associate a repeated delivery of the same identity with the existing
instance, subject to FR-044's contradiction rule.

The observation owner selects the identity. QSL SHALL treat it as opaque and
preserve it byte-for-byte wherever the checked-package route carries it; QSL
does not authenticate observation-owner evidence or construct a QProtocol
obligation binding. An absent, empty, malformed, stale, mismatched or
cross-version identity representation SHALL produce the route's typed
refusal/incomplete outcome without an aliased instance or temporal truth.
Version-specific checked-package readers SHALL NOT translate an identity from
another contract version into the selected version.

These obligations apply when the replacement route accepts semantic-trigger
identity. They do not add a trigger representation to routes that do not carry
one and do not prescribe a concrete API or encoding. FR-449 supplies the
checked-package activation-map row; QProtocol joins that row to its admitted
observation evidence under QSpec FR-479.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-053-AC-7 | Distinct opaque trigger identities remain distinct through checked-package activation and evaluation, including identities with equal receipts, payloads, captures or anchors; repeated delivery of one identity does not create a second instance. | Test |
| FR-053-AC-8 | Changing any trigger identity byte changes the selected activation/instance identity or is refused; no receipt, payload, capture, anchor, guard or static coordinate can stand in for the trigger identity. A trigger delivery carries only the opaque identity bytes, its receipt, anchor, payload, guard and capture facts (FR-524); a timestamp, display text and declaration position are not members of a delivery, so they cannot influence identity. | Test |
| FR-053-AC-9 | An empty trigger identity is refused at construction (FR-526); a delivery whose identity is absent creates no instance and no aliased or partial activation (FR-524); a stale or mismatched identity (a correction naming another trigger's result, or a redelivery whose payload differs from the retained one) is refused (FR-526, FR-524). The identity is opaque non-empty bytes with one version, so a malformed or cross-version identity cannot occur and no translation exists. No outcome carries a substituted identity or truth. | Test |

## Dependencies

- [FR-044](./FR-044-activate-temporal-obligations.md) owns trigger-keyed activation identity and duplicate-delivery behavior.
- [FR-449](./FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md) owns the checked-package control-to-temporal activation map.
- [FR-390](./FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md) owns checked-package protocol evaluation.
- QSpec FR-093 and FR-230 establish semantic-trigger and obligation identity; QSpec FR-479 owns the QProtocol binding and its cross-owner checks.
