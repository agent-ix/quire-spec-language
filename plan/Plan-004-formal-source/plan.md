---
id: Plan-004
title: "Native formal source correspondence"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
---
# Plan-004: Native formal source correspondence

## Requirements Summary

- [x] FR-014: explicit immutable correspondence with checked forward/reverse loci.
- [x] NFR-005: Rust implementation and qualification; no new dependencies.

This bounded enablement step serves US-002/StR-001. It does not complete the
broader FR-006 type checker or IT-002 state workflow. QUOIN spec-to-plan follows
the all-analysis review at 257f787 of contract revision 4eb4ef6.

## Dependency Graph

FR-001 → FR-014 supplies immutable source and indexed coordinates; FR-010 →
FR-014 supplies typed source-bound refusals. Both prerequisites are landed.
NFR-005 constrains production and tests. No cycle or external unimplemented
dependency exists. The concrete module consumes the existing Source and actual
IR SourceIdentity/SourceLocation/SourceSpan types.

## Test Plan

Five criteria FR-014-AC-1..5 have respective cases TC-035..39. The integration
cases establish explicit revision assignment, independent coordinate examples,
foreign native inputs and structurally valid false IR loci. The property case
generates all 156 short strings over its declared alphabet, with independent
coordinate scanning and valid/invalid span enumeration. Setup uses real pinned
IR constructors; assertions cover values, identity, error phase and provenance.

Write tests first and record their missing-API failure. Implement the bridge,
then run the focused suite and repository checks sequentially. Use imported
single-line `#[trace(...)]` attributes. The read-only Rust review and dependency
diff check NFR-005; no source-inspection test substitutes for behavior.

## Remaining Work

Track A, serial: Task-007 — small source bridge and qualification. Critical path
is tests → implementation → real constructor/oracle checks → code/gap review.
Gate: all five cases pass and no unreviewed source-correspondence behavior remains.
Failed identity/coordinate checks require correction before downstream use.

There are no parallel tracks or additional agents. Build phases use nice 10,
one Cargo job, one test thread and existing target caches. Check for competing
builds before starting. No hosted workflow dispatch is authorized.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-007 | A | FR-014, NFR-005 | TC-035–039 | done |

## Coordination Rules

Agent A is the sole writer in this isolated compiler worktree. The reviewed
scope and revision were posted to private LC02 before implementation. Actual
code review includes the user's specified agent-skills/rust-review/SKILL.md.
Optional gap semantic comparison remains declined. Private PR/merge updates
record actual evidence; checker/runtime/backend completion stays open.
