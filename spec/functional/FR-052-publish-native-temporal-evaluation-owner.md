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
separate native-temporal request/result document or prescribe a new public API.
FR-043 owns temporal meaning; FR-044 owns activation and captures. The retired
M-6d request/result API criteria remain historical and do not retire these
behaviors.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-9 | For a checked temporal obligation, the Layer-4 evaluation result's truth, settlement and support equal the FR-043 evaluation of the same selected obligation and admitted evidence; no input permits caller-supplied truth, settlement or support to determine that result. | Test |
| FR-052-AC-10 | For each independent activation, capture, anchor, profile/clock, progress, closure, execution and completeness fact present on the route, the result retains that fact distinctly; missing required evidence yields a typed non-truth outcome and never a false/default/closed/complete value. | Test |
| FR-052-AC-11 | Missing, inconsistent, unsupported or over-budget evidence produces the corresponding typed outcome with no partial truth, while an independently settled sibling retains its FR-043 result. | Test |
| FR-052-AC-12 | Where the route accepts a correction relation, a valid correction names its exact direct predecessor and changed evaluation input; self, non-direct, cross-obligation or identity-mismatched predecessors refuse, and the predecessor result remains unchanged. | Test |

## Dependencies

- [FR-043](./FR-043-evaluate-bounded-native-temporal.md) owns temporal evaluation semantics and outcomes.
- [FR-044](./FR-044-activate-temporal-obligations.md) owns activation identity, guards and immutable captures.
- [FR-390](./FR-390-evaluate-checked-protocol-slots.md) owns checked-package protocol-slot evaluation.
- [FR-449](./FR-449-derive-protocol-obligation-activation-bindings.md) owns checked-package activation mapping.
- [FR-051](./FR-051-publish-checked-native-handoffs.md) is historical context for the retired native temporal subject handoff.
