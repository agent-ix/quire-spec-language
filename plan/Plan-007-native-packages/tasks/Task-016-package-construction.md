---
id: Task-016
title: "Construct complete native packages and static identities"
type: Task
status: done
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

- [x] Retain the genuine pre-API failure and independently authored canonical-content/complete-manifest fixtures and public Rust assertions. Corrected positive fixtures follow the first producer under the reviewed 2c6b9b8/1c3aa50 correction; preserve that actual sequence.
- [x] Introduce focused package modules for the typed manifest, bounded traversal/encoding, error/usage records and role-specific public package/reference/identity types.
- [x] Derive complete source, model, authored clause, occurrence and runtime inventories from actual retained inputs, including the minimum admitted unit, multiple aliases and unused selected declarations. Empty source inventories are adverse parser inputs, not successful setup.
- [x] Derive the exact feature set with exhaustive source/operator/type mapping and cycle-safe model identity visits; do not infer semantics from carrier spelling.
- [x] Encode the native domain-separated canonical bytes and final artifact, with distinct pass counters, exact integers/Unicode, full projected inventories and excluded display/runtime/projection metadata as specified.
- [x] Qualify production byte/string/entry/depth limits, unchanged caller inputs, static mutations and repeated construction. Leave impossible coupled maxima explicitly unqualified rather than fabricating passing public paths.
- [x] Record selected Rust dependency features/licenses; keep production and qualification in Rust. Compile the selected structural schema locally against producer fixtures when its reviewed development dependency is added.
- [x] Run focused producer tests, actual local code/Rust review and relevant existing regression checks; fix findings before reader work depends on this implementation.

## Deliverables

A constructor-private NativePackage produced only from CheckedPackage, its
exact bytes/ByteDigest/NativePackageIdentity and original checked state;
independent producer vectors and exported-field assertions; source revision,
real red/green evidence and validated review. Partial TCs retain planned status
until all of their producer and reader criteria are backed.

## Notes

The initial producer increment passes 16 public tests plus three private
encoding controls. It includes independently composed minimal/Unicode/u64
expectations, exact multi-owner invariant/pre/post records, all operator and
builtin feature mappings, unused selected declarations and finite cycles,
independently counted pass limits and admissible source/authored mutations.
Original API-red, fixture-setup failures and the new-code vocabulary regression
remain recorded in reviews/data/native-packages/.

Fixed minimal/control/multiple-clause source, canonical content, complete
manifest and digest files now pass public producer comparisons and a private
canonical-pass byte comparison. The Rust maintenance author uses the independent
recipe and requires a fresh candidate directory. The reviewed Draft 2020-12
development feature now compiles the structural schema and passes fixed/real
invariant/pre/post manifests plus missing/unknown/type/selector controls.
These follow the initial producer; the original pre-API and failed-setup
history is retained, not retroactively marked as successful prior fixtures.

Source c195950 completes lexical/parameter/result/transitive correspondence,
static-dependency mutations, runtime-independent bytes/identity and type-role
controls. SR-111 records the passing producer gate: 25 public package tests,
five private package controls, 232 ordinary regression tests, three compile-fail
doctests, three selected private audits and the actual required local checks.
TC-078/079/080/090 are qualified; shared reader cases stay planned. Task-017
may now consume the producer and existing private manifest/encoding primitives.
The complete regression is green after updating the explicit code vocabulary
count from 26 to 29 for the three reviewed package codes. Trace inspection also
requires recording the installed tool's multiline-marker and NFR metric-binding
limitations; no shared module is changed to hide them.

Use src/checking.rs, src/linking.rs, src/native_model.rs and existing source/
runtime reference patterns. Do not export the proof graph as executable or
copy the historical audit visitor's different numeric/depth semantics.
The package remains a module of this crate. Review gate: specification
41da6e5, SR-101–108 at 69588ad. No new shared repository or TypeScript helper.
