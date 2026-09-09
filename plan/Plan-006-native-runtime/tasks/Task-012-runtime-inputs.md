---
id: Task-012
title: "Construct and qualify exact native runtime artifacts"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-055
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-056
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-057
    type: verifies
---
# Task-012: Construct and qualify exact native runtime artifacts

## Scope

Implement the reviewed flat Snapshot/Invocation input artifacts and exact emitted byte contract in compiler modules. This task owns structural construction only; runtime type/population validity belongs to the next task.

## Subtasks

- [x] Add meaningful Rust public API tests first for all value/root variants, exact independent envelope bytes, stable role-specific refs, invalid indices and immutable errors; record the initial missing-API failure.
- [x] Implement typed flat drafts, immutable artifacts, InputError and bounded deterministic encoding using the existing serde_json/sha2/IR identities.
- [x] Count every node, metadata entry, string byte and output byte before work; compute structural depth without shared-node expansion or recursive owned values.
- [x] Qualify exact/lowered/hard/coupled ceilings, unused nodes, duplicate retention and bounded destruction; preserve raw duplicate vectors for model-aware validation.
- [x] Run the targeted serial tests and relevant format/Clippy checks, update only actually executed matrix rows and retain commands/source revision.

## Deliverables

Public input constructors/references and standard Rust errors; complete emitted bytes/digests; independent structural/budget tests and measured evidence. No external reader, portable authority or runtime-validity claim.

## Notes

Start from the existing SourceIdentity/ByteDigest and native_model artifact pattern, but keep the new payload's explicit schema and stage distinctions. Adding new dependencies, source decoding or changed semantics reopens the spec gate.

Qualified source c8fa41f6e172e58b9406792d1b9f6b6bd85e52cc. SR-096 records
the actual code/Rust review, 21 constructor tests, one role compile-fail doctest,
full local gates and bounded-work limitations. TC-055–057 are passed; Task-013
remains responsible for every model-aware validity/completeness judgment.
