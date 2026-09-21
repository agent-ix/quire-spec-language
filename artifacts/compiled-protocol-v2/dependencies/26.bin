---
id: FR-032
title: "Preserve selected meanings across language evolution"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When a tool adds a native operator, declaration form, profile or semantic family, it SHALL preserve the meaning and identity rules of previously selected definitions and require an explicit selection for the new meaning.

## Inputs

Historical and new source subjects, exact definition selections, versioned wire/canonicalization contracts and a tool version supporting an expanded feature set.

## Outputs

A historical subject interpreted under its original definition, an explicitly selected new subject, or a located unsupported-selection refusal.

## Behavior

Edition, semantic-profile, wire/canonicalization and backend implementation identities are separate dimensions. A new token or keyword must not change how a historically admitted identifier is interpreted under its old edition. New optional metadata is ignorable only where the selected wire contract explicitly permits it; unknown required meaning refuses. Explicit migration creates a new subject with correspondence to the original and does not relabel historical evidence. Compatibility decisions state which dimension changed.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-032-AC-1 | Adding an operator, declaration and family leaves historical accepted sources with their prior interpretation and semantic identity under unchanged definitions. | Test (TC-032) |
| FR-032-AC-2 | A new spelling conflicting with an old admitted identifier does not reinterpret that identifier under the old edition. | Test (TC-032) |
| FR-032-AC-3 | A reader that lacks the selected new required feature refuses instead of dropping it or evaluating a smaller subset as the whole subject. | Test (TC-032) |
| FR-032-AC-4 | Explicit migration creates a separately identified subject while the original definition bytes, source and result identities remain unchanged. | Test (TC-032) |

## Dependencies

- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Composed architecture](../assurance/AD-001-composed-native-language.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed requirement for the composed profile; it does not amend historical
accepted definitions or establish an implemented capability.
