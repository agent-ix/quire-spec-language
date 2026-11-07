---
id: FR-052
title: "Preserve temporal evaluation ownership on the checked-package route"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-051, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-449, type: references }
---
# FR-052: Preserve temporal evaluation ownership on the checked-package route

## Description

When a checked temporal protocol obligation is evaluated through the Layer-4
checked-package route, the QSL evaluator SHALL remain the sole owner of its
temporal truth, settlement and decision support, with outcomes and failures
consistent with FR-043 and activation inputs consistent with FR-044.

## Behavior

The replacement route SHALL derive temporal truth, settlement and decision
support from the selected checked obligation and admitted evaluation evidence.
It SHALL NOT accept caller-authored truth, settlement or support as evaluation
inputs or treat a leaf valuation as the formula result.

The route SHALL preserve the selected obligation identity, temporal definition
and profile/clock authority, activation state, captures, evaluation anchor,
progress and closure evidence, execution disposition and completeness evidence
where those facts are present in its checked-package evaluation contract.
Independent facts SHALL remain independently inspectable; a missing or
unestablished fact SHALL NOT be converted to false, settled, closed or complete.

If required evidence is missing, inconsistent, unsupported by the selected
profile, or exceeds an applicable work limit, the evaluator SHALL return the
corresponding typed incomplete, refused or unsupported outcome without a
partial temporal truth. It SHALL preserve each independently settled sibling
obligation when another obligation cannot be completed, as required by FR-043.

Where the checked-package route accepts a correction or revised evaluation,
the evaluator SHALL retain the exact direct predecessor and immutable
predecessor result identity, reject a self-edge or non-direct/mismatched
predecessor, and leave prior result content unchanged. This clause applies only
to a correction relation actually carried by that route.

These obligations specify behavior on checked-package protocol evaluation
(FR-390) and its Layer-4 activation mapping (FR-449). They do not require a
separate native-temporal request/result document. The typed request and result
that carry them for quire-protocol are FR-516's; they are Rust values with no
document, schema or QSL-computed content digest (the observation owner's
identifiers on `EvidenceRef` are opaque and echoed).

The QSL-owned native owner-report route (`temporal::evaluate_v2`, its `Report`
and the `Report::to_owner_bytes` / `Report::from_owner_bytes` flat-bytes
carrier) is not part of this requirement. Its binding, support, predecessor and
limit obligations are specified on the spine evidence result by FR-526 ("Result
subject and support", "Predecessor authority" and "Lineage limit"); its byte
carrier is retired below.

FR-043 owns temporal meaning; FR-044 owns activation and captures. The retired
M-6d request/result API criteria remain historical and do not retire these
behaviors.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-9 | For a checked temporal obligation, the Layer-4 evaluation result's truth, settlement and support equal the FR-043 evaluation of the same selected obligation and admitted evidence; no input permits caller-supplied truth, settlement or support to determine that result. | Test |
| FR-052-AC-10 | For each independent activation, capture, anchor, profile/clock, progress, closure, execution and completeness fact present on the route, the result retains that fact distinctly; missing required evidence yields a typed non-truth outcome and never a false/default/closed/complete value. | Test |
| FR-052-AC-11 | Missing, inconsistent, unsupported or over-budget evidence produces the corresponding typed outcome with no partial truth, while an independently settled sibling retains its FR-043 result. | Test |
| FR-052-AC-12 | Where the route accepts a correction relation, a valid correction names its exact direct predecessor and changed evaluation input; self, non-direct, cross-obligation, cross-occurrence (a predecessor of another declaration occurrence of the same checked declaration) or identity-mismatched predecessors refuse, and the predecessor result remains unchanged. This criterion does not authorize bridge predecessor or supersession lineage, which Contract IR owns. | Test |
| FR-052-AC-13 | Where the route carries a progress ledger, a regressing watermark or a conflicting completeness revision under one binding is refused as FR-502 states, leaves the retained progress and every earlier result unchanged, and a foreign clock, clause, package or profile settles nothing for the binding (the spine successor of FR-043-AC-12). | Test (TC-144, TC-1296) |
| FR-052-AC-14 | A checked temporal obligation selected under the timestamped-event finite-window profile is evaluated as FR-501 states and is never refused for its profile. | Test (TC-144, TC-1295) |

## Retired and moved criteria

FR-052-AC-17, AC-18 and AC-21 are retired with the native owner-report flat-bytes
carrier (`Report::to_owner_bytes`, `Report::to_owner_bytes_bounded`,
`Report::from_owner_bytes`, `ReportRefusal`, `Report::predecessor_owner_bytes`
and `Report::validate_predecessor`). AC-17 stated that carrier's encode/decode
round trip and byte determinism, AC-18 its strict-reader refusals, and AC-21 its
flat retained predecessor bytes. The one AC-18 case that is not about bytes, a
binding that differs from the selected subject or occurrence, is already the
spine's: FR-522's `binding-mismatch` and FR-516-AC-32 and AC-33. They exist only for that carrier, which is
deleted; the spine evidence result is a Rust value with no owner byte
representation, so no successor criterion is needed.

FR-052-AC-15, AC-16, AC-19 and AC-20 are moved to the spine evidence result
without narrowing: AC-15 (checked subject, observation and revision references)
to FR-526-AC-20 and AC-21, AC-16 (support positions and evidence rows) to
FR-526-AC-22 and AC-23, AC-19 (predecessor chaining) to FR-526-AC-24 to AC-26,
and AC-20 (predecessor limits) to FR-526-AC-27 and AC-28, on FR-523's existing
`lineage` counter.

## Dependencies

- [FR-043](./FR-043-evaluate-bounded-native-temporal.md) owns temporal evaluation semantics and outcomes.
- [FR-044](./FR-044-activate-temporal-obligations.md) owns activation identity, guards and immutable captures.
- [FR-390](./FR-390-evaluate-checked-protocol-slots.md) owns checked-package protocol-slot evaluation.
- [FR-449](./FR-449-derive-protocol-obligation-activation-bindings.md) owns checked-package activation mapping.
- [FR-526](./FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md) carries the moved subject, support, predecessor and limit criteria.
- [FR-051](./FR-051-publish-checked-native-handoffs.md) is historical context for the retired native temporal subject handoff.
