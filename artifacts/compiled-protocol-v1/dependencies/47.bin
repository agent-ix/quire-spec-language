---
id: FR-055
title: "Activate protocol obligations under exact captured facts"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-051
    type: depends_on
---
## Description

When a protocol trigger is established, the conformance evaluator SHALL create each dependent obligation with its exact workflow, role, effect, predicate, capture and evaluation-anchor identities.

## Inputs

A linked protocol, admitted trigger observations, immutable captures and E/F-owned
clock, progress, closure and completeness bindings.

## Outputs

Triggered, inactive or activation-unknown obligation states with their exact
dependencies; no business action is executed.

## Behavior

The evaluator SHALL keep activation independent from truth. An absent trigger
under complete closed input yields inactive; an unavailable required trigger
yields activation-unknown or incomplete. Captured amounts, identifiers and
anchors SHALL be immutable for that obligation. The evaluator SHALL NOT treat an
inactive or unknown obligation as satisfied or demonstrated.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-055-AC-1 | A failed shipment after payment effect E1 activates the matching refund obligation with E1's captured amount and identity. | Test (TC-055) |
| FR-055-AC-2 | A successful shipment leaves its refund obligation inactive and cannot count it as demonstrated recovery. | Test (TC-055) |
| FR-055-AC-3 | A missing external trigger under open or incomplete input yields activation-unknown or incomplete rather than inactive. | Test (TC-055) |
| FR-055-AC-4 | Substituting O2's trigger, a changed capture or a different evaluation anchor refuses the binding. | Test (TC-055) |

## Dependencies

- [FR-051](./FR-051-preserve-communication-identities.md).
- E-owned temporal anchors and F-owned observation completeness.
