---
id: IT-011
title: "Adopt the frozen complete-V1 contracts and delivery lane"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-055
    type: verifies
  - target: ix://agent-ix/quire-specification/Task-010
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-003
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/AD-006
    type: references
  - target: ix://agent-ix/quire-specification/AD-008
    type: references
  - target: ix://agent-ix/quire-specification/FR-131
    type: references
  - target: ix://agent-ix/quire-specification/FR-140
    type: references
  - target: ix://agent-ix/quire-specification/FR-150
    type: references
  - target: ix://agent-ix/quire-specification/FR-180
    type: references
  - target: ix://agent-ix/quire-specification/FR-300
    type: references
  - target: ix://agent-ix/quire-specification/FR-339
    type: references
  - target: ix://agent-ix/quire-specification/TM-009
    type: references
---
# IT-011: Adopt the frozen complete-V1 contracts and delivery lane

## Objective

Verify that QSL adopts, without redefining, the accepted complete-V1 architecture,
requirements, interfaces, capability inventory and test allocation from
`quire-specification` revision `8d0fbad`. The adoption must preserve existing
QSL evidence while exposing every missing or partial capability as scheduled
work rather than a deferral or an inferred completion.

## Target Integration

The integration boundary is the accepted QSpec complete-V1 contract at
`8d0fbad` and the QSL repository-local Plan-013 delivery graph. QSpec owns
AD-003, AD-005, AD-006, AD-008, the applicable FR-131 through FR-153,
FR-180/181, FR-300 through FR-309 and FR-339 contracts, interfaces I01 through
I05 and I17 through I19, and TM-009. QSL owns its implementation and local
evidence; this artifact does not copy or amend the central semantics.

## Preconditions

QSpec Plan-008 Task-010 is complete and QSpec PR #64 is merged at
`5a5f0d5f7598fccafcc2a4ddb84c2241a617274e`. The reviewed semantic baseline is
commit `8d0fbad`. QSL issue #116 and downstream issues #117 through #123, plus
`quire-wasm` #6, are open. QSL main is the evidence baseline for reconciliation.

## Inputs

- QSpec's 176-row `docs/v1-capability-inventory.md`, TM-009 and accepted
  implementation-ticket manifest.
- The 83 rows allocated to Agent-A, including `V1-TOOL-009` in `quire-wasm`.
- Central TC-180 through TC-198, TC-209/210, TC-220 through TC-231 and the
  existing QSL Rust tests and matrices that preserve bounded L1 through L6 work.
- The live GitHub dependency chain QSL #116 through #123 and WASM #6.

## Test Procedure

1. Select QSpec `8d0fbad` and compare Plan-013's normative references with the
   accepted architecture, requirement, interface and matrix set.
   - IT-011-SC-01: every reference resolves to the frozen baseline or its merged
     container, and no local artifact changes the referenced meaning.
2. Parse the Plan-013 atomic allocation table and compare it with the accepted
   Agent-A lane in the central ticket manifest.
   - IT-011-SC-02: exactly 83 unique capability IDs appear, each with exactly one
     primary implementation ticket and QSL #123 as qualification owner.
3. Inspect each allocation and the nine task artifacts.
   - IT-011-SC-03: every capability has one concrete central TC, every task has
     `references` and `verifies` edges, and the primary ticket counts are 18,
     16, 26, 15, 3, 4 and 1 for QSL #117 through #122 and WASM #6 respectively.
4. Compare the central inventory state with current QSL matrices and Rust trace
   tags without promoting a partial capability.
   - IT-011-SC-04: the adoption records 18 complete, 36 partial and 29 missing
     implementation rows; 16 complete, 38 partial and 29 missing direct-
     verification rows; all 83 qualification rows remain missing.
5. Inspect the task dependency graph and all cross-lane prerequisites.
   - IT-011-SC-05: the repository-local tasks preserve the ordered issue chain,
     name external producer/consumer gates, and contain no feature-deferral path.
6. Inspect the delivery constraints and changed paths.
   - IT-011-SC-06: implementation and executable verification remain Rust-only,
     `publish = false`, local/serial, use `target-codex-backends`, dispatch no
     hosted CI, and do not modify `resources/native-v1` or existing tags.

## Expected Results

All six success criteria hold. Plan-013 is a repository-local execution map for
the central contract, not a competing specification. Existing L1 through L6
evidence remains attributable, all incomplete rows retain their true state, and
downstream work cannot claim a smaller semantic profile as complete V1.

## Metadata

- Priority: P0
- Target Integration: QSpec complete-V1 contract to QSL/WASM delivery plan
- Automation: Automated Rust structural audit plus local inspection

## Dependencies

**Upstream:** QSpec #63 / Plan-008 Task-010 and merged PR #64. **Downstream:**
QSL #117 through #123 and `quire-wasm` #6 in the exact order recorded by
Plan-013; cross-lane producer and consumer pins must be merged before use.

## Notes

QSpec assigns no FR-135 through FR-139 artifacts in the frozen revision. The
issue's inclusive `FR-131–153` shorthand therefore means the existing artifacts
FR-131 through FR-134 and FR-140 through FR-153; this adoption does not invent
missing identifiers. FR-304/TC-224 and FR-309/TC-229 remain central contracts
but their primary capability rows belong to Agent-C. QSL consumes their
versioned interfaces and does not claim their implementation.

## Traceability

TC-144 verifies [FR-055](../functional/FR-055-govern-complete-v1-lane.md) and
checks the local allocation and serial task graph. The central TestCase
artifacts remain the normative behavior definitions and become executable
evidence only in their owning downstream tickets.
