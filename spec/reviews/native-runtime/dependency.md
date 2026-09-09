---
id: SR-091
title: "Native runtime dependency and ordering review"
type: SpecReview
analysis: dependency
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

The implementation DAG is acyclic: input construction and the landed checked-package API enable validation, which enables reference execution. Broader issue-acceptance gates remain explicitly open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved boundary: remaining LC02 strict package/projection and FS03 acceptance still gate issue closure; landed FR-015/016 suffice for implementing this native runtime API. IT-006 cannot substitute for IT-002. | FR-007; FR-008; IT-006; IT-002 |
| FND-002 | low | No new shared reader dependency: inspected runtime 7caefac contains campaign counters, and B's d577486 reader owns portable references. Neither supplies a domain population validator to wrap. | FR-018; docs/native-runtime-inputs.md |

## Classification

| Requirement | Class | Reason |
| --- | --- | --- |
| StR-001 (existing context) | Feature | Operator trust defines the end-state acceptance |
| FR-018 | Enablement | Exact structural input construction feeds validation |
| FR-007 | Feature | Observable valid/refused/incomplete input judgment |
| FR-008 | Feature | Concrete native truth and actual activation |
| NFR-006 | Enablement constraint | Bounds every stage's work and retained content |

Existing FR-015/016 are qualified enablement inputs, not new tasks in this packet.
Existing NFR-003/005 continue to constrain outcome honesty and Rust execution.

## Dependency graph and topological order

FR-018 → FR-007: validation consumes immutable structurally admitted artifacts.
FR-015/016 → FR-007: exact model roles, authored clause bindings and types are
needed to validate inputs. FR-007 → FR-008: only a successful context can be
evaluated. FR-008 → IT-006 qualification: the actual multi-stage result must
exist before the workflow can execute. NFR-006 constrains each node without a
reverse prerequisite or cyclic semantic dependency.

Suggested sequence is construction, validation, evaluation, then complete
qualification/review. Flat values and checked budget primitives are shared
within compiler modules; no separate crate or new canonicalization authority is
needed. Tasks should gate dependent work on the relevant real positive/adverse
tests. The owner requested one serial Agent A track; logical independence does
not authorize parallel builds or additional agents.

LC02/FS03 acceptance and the eventual B/C handoff remain outside this DAG's
implementation milestone. Their open state is not rewritten by a native true/
false result, and unsupported profile forms remain explicit frontend refusals.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.
