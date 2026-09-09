---
id: Task-016
title: "Construct complete native packages and static identities"
type: Task
status: not_started
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-078
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-079
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-080
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-082
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-088
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-090
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-091
    type: verifies
---
# Task-016: Construct complete native packages and static identities

## Scope

Implement immutable native package construction and source-bound static identity
from real CheckedPackage inputs under the reviewed contract. Supply the shared
private manifest, feature derivation and package accounting/error primitives
needed by the later reader. This task qualifies producer portions of shared
cases; readback/refusal portions remain pending until Task-017.

## Subtasks

- [ ] Author independent canonical-content/complete-manifest fixtures and public Rust assertions before the producer API; retain the genuine missing-API/failing result.
- [ ] Introduce focused package modules for the typed manifest, bounded traversal/encoding, error/usage records and role-specific public package/reference/identity types.
- [ ] Derive complete source, model, authored clause, occurrence and runtime inventories from actual retained inputs, including empty sources, multiple aliases and unused selected declarations.
- [ ] Derive the exact feature set with exhaustive source/operator/type mapping and cycle-safe model identity visits; do not infer semantics from carrier spelling.
- [ ] Encode the native domain-separated canonical bytes and final artifact, with distinct pass counters, exact integers/Unicode, full projected inventories and excluded display/runtime/projection metadata as specified.
- [ ] Qualify production byte/string/entry/depth limits, unchanged caller inputs, static mutations and repeated construction. Leave impossible coupled maxima explicitly unqualified rather than fabricating passing public paths.
- [ ] Record selected Rust dependency features/licenses; keep production and qualification in Rust. Compile the selected structural schema locally against producer fixtures when its reviewed development dependency is added.
- [ ] Run focused producer tests, actual local code/Rust review and relevant existing regression checks; fix findings before reader work depends on this implementation.

## Deliverables

A constructor-private NativePackage produced only from CheckedPackage, its
exact bytes/ByteDigest/NativePackageIdentity and original checked state;
independent producer vectors and exported-field assertions; source revision,
real red/green evidence and validated review. Partial TCs retain planned status
until all of their producer and reader criteria are backed.

## Notes

Use src/checking.rs, src/linking.rs, src/native_model.rs and existing source/
runtime reference patterns. Do not export the proof graph as executable or
copy the historical audit visitor's different numeric/depth semantics.
The package remains a module of this crate. Review gate: specification
41da6e5, SR-101–108 at 69588ad. No new shared repository or TypeScript helper.
