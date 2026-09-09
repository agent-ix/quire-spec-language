---
id: Task-007
title: "Implement and qualify native formal source correspondence"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-035
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-036
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-037
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-038
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-039
    type: verifies
---
# Task-007: Implement and qualify native formal source correspondence

## Scope

Implement reviewed FR-014 without changing source intake, IR types or native
model semantics. Preserve exact identities and reject inconsistent loci.

## Subtasks

- [x] Write the five specified public API tests and observe missing API failure.
- [x] Implement the documented FormalSource API using existing Source lookup.
- [x] Run focused and required regression/style/build checks with one-job limits.
- [x] Complete actual Rust/code review and scoped gap analysis.
- [x] Update matrix and evidence; create a reviewable private PR.

## Deliverables

Rust module, traced integration/property tests, review artifacts, exact local
verification results and private owning-issue handoff.

## Notes

All selected spec reviews passed before this task. The binding trusts caller
assignment of the formal identity and checks byte/coordinate consistency only.
No global source authority or native semantic verdict is implied.

The reviewable deliverable is private PR9. Task completion records implemented,
tested and reviewed behavior; merging that deliverable is the next repository
action under the owner's standing authorization and remains tracked in LC02.
