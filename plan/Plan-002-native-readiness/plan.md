---
id: Plan-002
title: "Native syntax merge readiness"
type: Plan
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: references
---

## Requirements Summary

- [x] Reconcile native OS arguments, inclusive formatter budgets and standard diagnostic traits with reviewed requirements.
- [x] Bind all existing native tests plus the new adverse cases to TM-002 and resolve SR-009 findings for the implemented boundary.

## Dependency Graph

Task-003 implements the reviewed native boundary; Task-004 verifies and records
merge readiness after Task-003. LR02 Plan-001 remains done. The IR/model
adapter seam required for future linking work is not yet accepted.

## Test Plan

TM-002 at spec/native-readiness/tests.md defines TC-011–TC-019. Run local
formatter, strict all-feature/all-target Clippy, Rust tests, the explicit LR02
private-packet lane and Quire validation/coverage. No hosted CI dispatch.

## Remaining Work

No implementation or local verification task remains in this plan. Task-003
delivered the reviewed source/tests; Task-004 records actual gates and SR-028.
The final read-only plan audit and authorized private merge follow completion.
LC02 linking/typechecking is tracked as its own separate task.

## Task File Mapping

| Task | Owns | Verifies | Status |
| --- | --- | --- | --- |
| Task-003 | FR-001/002/003/004/010, NFR-001 | TC-011–TC-019 | done |
| Task-004 | FR-001/002/003/004/010, NFR-001 | TC-011–TC-019 | done |

## Coordination

This is LC01 language#2, on existing Agent A PR7; no competing issue or branch.
The owner authorized landing when ready on 2026-09-08. Do not infer shared
semantic contract acceptance or fresh external producer approval from that
instruction. Preserve local-only CI and all historical fixtures.
