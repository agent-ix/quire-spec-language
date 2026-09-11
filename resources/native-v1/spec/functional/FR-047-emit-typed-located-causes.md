---
id: FR-047
title: "Classify producer diagnostics from typed causes"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When reporting a native producer failure, the producer SHALL derive its machine-readable classification from a typed cause in its versioned diagnostic catalog rather than diagnostic message text.

## Inputs

A failure at source recognition, identity/linking, type/profile checking, input binding, capability selection or resource-limited execution, with its available source/subject context.

## Outputs

A stable catalogued classification, failure stage and structured cause details with exact available loci; human-readable wording is presentation.

## Behavior

Every emitted code belongs to the selected catalog; no ad-hoc literal or English-message match determines its kind. Cause details distinguish materially different refusals sharing a broader category, including missing versus conflicting definitions, wrong anchor versus missing observation, unknown syntax versus prohibited recognized construct, and unproved definedness versus runtime logical violation. Diagnostic formatting/localization cannot change classification. Wire extension/version behavior follows the existing shared interchange contract; this requirement creates no competing result envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | Changing/localizing human-readable messages leaves the same typed failure's stage, code and cause details unchanged. | Test (TC-047) |
| FR-047-AC-2 | Every emitted machine code resolves to the selected versioned catalog; an uncatalogued code cannot be published as a stable producer outcome. | Test (TC-047) |
| FR-047-AC-3 | Distinct anchor, observation, profile and definedness failures retain structured discriminating causes without requiring message parsing. | Test (TC-047) |
| FR-047-AC-4 | Original source/model/clause loci and identities remain attached where available; diagnostics do not reconstruct a location by searching for a repeated name. | Test (TC-047) |
| FR-047-AC-5 | Per-request resource/unsupported failures keep their actual stage and prerequisites rather than becoming another clause's Boolean result. | Test (TC-047) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/shared-foundation.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.
