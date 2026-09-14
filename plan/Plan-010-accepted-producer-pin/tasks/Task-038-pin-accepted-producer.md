---
id: Task-038
title: "Pin and qualify the accepted Producer interface"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-114
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-115
    type: verifies
  - target: ix://agent-ix/quire-spec-language/IT-009
    type: verifies
---
# Task-038: Pin and qualify the accepted Producer interface

## Scope

Replace the native consumer's PR-checkpoint revision with the accepted FCD #95
merge and qualify the unchanged direct Producer 1.2 adapter boundary. Refuse all
unsupported or inconsistent inputs exactly as before; do not add a reader,
canonicalizer, approximation or assessment path.

## Subtasks

- [x] Specify and review the exact accepted-revision integration contract.
- [x] Pin the manifest and lock to the full accepted merge identity.
- [x] Update dependency provenance and run the eight direct producer tests.
- [x] Run the complete serial local Rust and fixture gates.
- [x] Complete self `/rust-review` and `/gap-analysis`; resolve all findings.
- [x] Reconcile issue #93 in the PR and prepare owning FS02 tracking for the
  post-merge update.

## Deliverables

- Exact immutable Cargo selection of FCD merge
  `404288282402d60de007295ccbafa960532b955e`.
- Passing direct-adapter, regression, build and fixture evidence.
- Reviewed Plan-010 closure and accurate GitHub tracking.

## Notes

- The existing composed-package architecture and its complete base, scope,
  dependency, evidence, failure, risk, integrity and EARS reviews are the design
  authority. This task does not reopen them.
- FCD #95 / PR #99 is the accepted upstream prerequisite. Completing this task
  unblocks quire-specification FS02 closeout.
