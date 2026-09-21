---
id: FR-055
title: "Govern the complete-V1 Agent-A delivery lane"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/IT-011
    type: references
  - target: ix://agent-ix/quire-specification/Task-010
    type: depends_on
  - target: ix://agent-ix/quire-specification/TM-009
    type: references
---
# FR-055: Govern the complete-V1 Agent-A delivery lane

## Description

When the accepted complete-V1 campaign gate opens, the QSL repository shall
maintain one executable delivery plan that assigns every Agent-A capability to
its exact implementation ticket, central TestCase and qualification owner.

## Inputs

- QSpec complete-V1 semantic baseline `d49bbdc4a97ae0dca27b4f1eec2826446520fc28` (the QSpec PR #75 merge, amending the QSpec #68 complete contracts after PRs #73 and #74 over the PR #64 baseline).
- The accepted central capability inventory, delivery-ticket manifest and TM-009.
- Current QSL default-branch code, tests, matrices, issues and retained L1–L6 evidence.

## Outputs

- One typed QSL plan bundle with an explicit dependency graph and task ownership.
- One auditable allocation row for every Agent-A capability.
- An evidence reconciliation that preserves complete, partial and missing states independently.

## Behavior

- The plan shall retain each central capability, requirement and TestCase identity without redefining its meaning.
- The plan shall assign exactly one primary implementation ticket to each Agent-A capability.
- The plan shall assign one existing central TestCase and QSL #123 qualification ownership to each Agent-A capability.
- The plan shall preserve prior evidence while leaving every missing obligation scheduled.
- The plan shall preserve the ordered QSL/WASM issue chain and every external producer or consumer boundary.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-055-AC-1 | Plan-013 contains exactly the 83 accepted Agent-A capability identities and introduces no local semantic replacement. | Test (TC-144) |
| FR-055-AC-2 | Every capability row has exactly one permitted primary ticket, one concrete central TestCase and QSL #123 as qualification owner. | Test (TC-144) |
| FR-055-AC-3 | The plan records 18 complete, 36 partial and 29 missing implementation rows; 16 complete, 38 partial and 29 missing verification rows; and 83 missing qualification rows without promotion. | Test (TC-144) |
| FR-055-AC-4 | Nine typed tasks preserve the serial QSL #116–123 and WASM #6 chain while naming the external model, temporal, protocol, IR and integration prerequisites. | Test (TC-144) |
| FR-055-AC-5 | The lane retains Rust-only executable work, `publish = false`, local serial gates, `target-codex-backends`, manual-only hosted CI and immutable `resources/native-v1` and tags. | Test (TC-144) |

## Dependencies

The central QSpec gate is merged at
`5a5f0d5f7598fccafcc2a4ddb84c2241a617274e`. [IT-011](../integration/IT-011-adopt-complete-v1-baseline.md)
checks that Plan-013 integrates the frozen central contract without assuming
that a green planning audit is product qualification.
