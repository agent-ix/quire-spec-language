---
id: FR-031
title: "Report each requested assessment capability independently"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When assessing an admitted native package, the dispatcher SHALL report a disposition for every requested clause/capability pair without converting an unsupported request into success or removing an independent supported result.

## Inputs

An admitted package, explicit requested clause/capability pairs, validated runtime inputs and the selected backend's declared support. A pair includes the requested claim, such as finite replay or local projection.

## Outputs

Per-request results or refusals, each bound to its clause, semantic profile, requested claim, backend and prerequisite outcome.

## Behavior

A dependency-closed supported request can execute when another independent request is unsupported. A failed shared prerequisite prevents all of its dependents from executing. Package-level complete success requires every selected required request to complete successfully; partial results retain their limitations. Unsupported method, refused input, resource-incomplete execution and logical violation are distinct outcomes. Backend selection never rewrites the admitted language subject.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | For one admitted protocol, supported finite replay can return its scoped result while unsupported local projection is explicitly reported. | Test (TC-031) |
| FR-031-AC-2 | For a supported state request and unsupported independent temporal lowering, both dispositions appear and package-level complete success is unavailable. | Test (TC-031) |
| FR-031-AC-3 | A failed shared model or observation prerequisite prevents all dependent requests from executing; an independent request keeps its result. | Test (TC-031) |
| FR-031-AC-4 | Replacing the requested backend leaves the admitted semantic subject unchanged and identifies the selected backend in the result. | Test (TC-031) |

## Dependencies

- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Composed architecture](../assurance/AD-001-composed-native-language.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed requirement for the composed profile; it does not amend historical
accepted definitions or establish an implemented capability.
