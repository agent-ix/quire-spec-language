---
id: TM-004
title: "Native runtime validation and reference evaluation matrix"
type: TestMatrix
---

## Overview

LC03 covers FR-018 input construction, FR-007 validation and FR-008 evaluation
under NFR-006. TC-055–057 are qualified at c8fa41f by SR-096; the other 20 cases
remain planned. TM-001–003 keep their existing scopes and qualification.
US-003 traces these functions to StR-001; IT-006 exercises the real native API.
IT-002 still owns the full compiled-model/backend workflow.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-007 | FR-007-AC-1 | TC-059 | 🚧 Planned |
| FR-007 | FR-007-AC-2 | TC-059 | 🚧 Planned |
| FR-007 | FR-007-AC-3 | TC-058, TC-061 | 🚧 Planned |
| FR-007 | FR-007-AC-4 | TC-062 | 🚧 Planned |
| FR-007 | FR-007-AC-5 | TC-064, TC-077 | 🚧 Planned |
| FR-007 | FR-007-AC-6 | TC-058 | 🚧 Planned |
| FR-007 | FR-007-AC-7 | TC-060 | 🚧 Planned |
| FR-007 | FR-007-AC-8 | TC-059, TC-060 | 🚧 Planned |
| FR-007 | FR-007-AC-9 | TC-059, TC-061 | 🚧 Planned |
| FR-007 | FR-007-AC-10 | TC-061 | 🚧 Planned |
| FR-007 | FR-007-AC-11 | TC-062 | 🚧 Planned |
| FR-007 | FR-007-AC-12 | TC-065 | 🚧 Planned |
| FR-007 | FR-007-AC-13 | TC-064 | 🚧 Planned |
| FR-007 | FR-007-AC-14 | TC-063 | 🚧 Planned |
| FR-007 | FR-007-AC-15 | TC-066 | 🚧 Planned |
| FR-008 | FR-008-AC-1 | TC-067, TC-077 | 🚧 Planned |
| FR-008 | FR-008-AC-2 | TC-067, TC-077 | 🚧 Planned |
| FR-008 | FR-008-AC-3 | TC-068 | 🚧 Planned |
| FR-008 | FR-008-AC-4 | TC-074 | 🚧 Planned |
| FR-008 | FR-008-AC-5 | TC-075 | 🚧 Planned |
| FR-008 | FR-008-AC-6 | TC-061, TC-071 | 🚧 Planned |
| FR-008 | FR-008-AC-7 | TC-073 | 🚧 Planned |
| FR-008 | FR-008-AC-8 | TC-073 | 🚧 Planned |
| FR-008 | FR-008-AC-9 | TC-073 | 🚧 Planned |
| FR-008 | FR-008-AC-10 | TC-073 | 🚧 Planned |
| FR-008 | FR-008-AC-11 | TC-068 | 🚧 Planned |
| FR-008 | FR-008-AC-12 | TC-069 | 🚧 Planned |
| FR-008 | FR-008-AC-13 | TC-070 | 🚧 Planned |
| FR-008 | FR-008-AC-14 | TC-071 | 🚧 Planned |
| FR-008 | FR-008-AC-15 | TC-072 | 🚧 Planned |
| FR-008 | FR-008-AC-16 | TC-074 | 🚧 Planned |
| FR-008 | FR-008-AC-17 | TC-073, TC-075 | 🚧 Planned |
| FR-008 | FR-008-AC-18 | TC-075 | 🚧 Planned |
| FR-008 | FR-008-AC-19 | TC-075 | 🚧 Planned |
| FR-008 | FR-008-AC-20 | TC-076 | 🚧 Planned |
| FR-018 | FR-018-AC-1 | TC-055 | ✅ Passed |
| FR-018 | FR-018-AC-2 | TC-055 | ✅ Passed |
| FR-018 | FR-018-AC-3 | TC-056, TC-077 | ✅ Passed (construction; TC-077 planned) |
| FR-018 | FR-018-AC-4 | TC-056 | ✅ Passed |
| FR-018 | FR-018-AC-5 | TC-055 | ✅ Passed |
| FR-018 | FR-018-AC-6 | TC-057 | ✅ Passed |
| FR-018 | FR-018-AC-7 | TC-055, TC-056 | ✅ Passed |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
| --- | --- | --- | --- |
| NFR-006 | Test: negative-abuse-testing with independent work/count controls | TC-057 construction; TC-065 validation; TC-074 exact cost; TC-075 evaluation; TC-066 retries | 🚧 Partial: M-1..5 passed; M-6..17 planned |
| NFR-005 | Inspection and existing Rust gates | Existing Rust-only policy plus SR-096 source/manifest review and actual local gates | 🚧 Construction passed; later LC03 implementation pending |

### Stakeholder and User Story Coverage

StR-001 → US-003 → FR-007/008/018 → TC-055–077. US-003-EX-1 is
illustrated by TC-067/077; US-003-EX-2 by TC-065/074/075. Illustrative EX IDs
are not minted as invented acceptance criteria. Existing stories and policy
coverage remain in TM-001/002.

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-055 | Preserve flat runtime values | Integration | P1 | FR-018-AC-1, FR-018-AC-2, FR-018-AC-5, FR-018-AC-7 | ✅ Passed |
| TC-056 | Bind exact runtime artifact bytes | Property | P1 | FR-018-AC-3, FR-018-AC-4, FR-018-AC-7 | ✅ Passed |
| TC-057 | Bound runtime artifact construction | Property | P1 | FR-018-AC-6 | ✅ Passed |
| TC-058 | Select exact runtime bindings | Integration | P1 | FR-007-AC-3, FR-007-AC-6 | 🚧 Planned |
| TC-059 | Validate complete population closure | Integration | P1 | FR-007-AC-1, FR-007-AC-2, FR-007-AC-8, FR-007-AC-9 | 🚧 Planned |
| TC-060 | Validate every supplied typed value | Property | P1 | FR-007-AC-7, FR-007-AC-8 | 🚧 Planned |
| TC-061 | Validate recorded invocation captures | Integration | P1 | FR-007-AC-3, FR-007-AC-9, FR-007-AC-10, FR-008-AC-6 | 🚧 Planned |
| TC-062 | Validate operation effects and deltas | Integration | P1 | FR-007-AC-4, FR-007-AC-11 | 🚧 Planned |
| TC-063 | Compare frame storage values | Integration | P1 | FR-007-AC-14 | 🚧 Planned |
| TC-064 | Retain mixed runtime diagnostics | Property | P1 | FR-007-AC-5, FR-007-AC-13 | 🚧 Planned |
| TC-065 | Bound validation and cancellation | Property | P1 | FR-007-AC-12 | 🚧 Planned |
| TC-066 | Repeat immutable runtime validation | Property | P1 | FR-007-AC-15 | 🚧 Planned |
| TC-067 | Execute parent and aggregate predicates | Integration | P1 | FR-008-AC-1, FR-008-AC-2 | 🚧 Planned |
| TC-068 | Check reachability against independent closure | Property | P1 | FR-008-AC-3, FR-008-AC-11 | 🚧 Planned |
| TC-069 | Execute exact signed arithmetic | Integration | P1 | FR-008-AC-12 | 🚧 Planned |
| TC-070 | Preserve sequence order and multiplicity | Property | P1 | FR-008-AC-13 | 🚧 Planned |
| TC-071 | Preserve sharing and observation capture | Integration | P1 | FR-008-AC-6, FR-008-AC-14 | 🚧 Planned |
| TC-072 | Compare native text records and identities | Integration | P1 | FR-008-AC-15 | 🚧 Planned |
| TC-073 | Retain implication event lineage | Integration | P1 | FR-008-AC-7, FR-008-AC-8, FR-008-AC-9, FR-008-AC-10, FR-008-AC-17 | 🚧 Planned |
| TC-074 | Qualify exact reference accounting | Integration | P1 | FR-008-AC-4, FR-008-AC-16 | 🚧 Planned |
| TC-075 | Bound evaluation and immutable retries | Property | P1 | FR-008-AC-5, FR-008-AC-17, FR-008-AC-18, FR-008-AC-19 | 🚧 Planned |
| TC-076 | Refuse unsupported collection construction | Integration | P1 | FR-008-AC-20 | 🚧 Planned |
| TC-077 | Qualify the native reference API workflow | Integration | P1 | FR-007-AC-5, FR-008-AC-1, FR-008-AC-2, FR-018-AC-3 | 🚧 Planned |

## Test Matrix Rules

Every one of the 42 scoped FR criteria has an explicit case. The six coverage
rules are addressed below; this authored mapping is not a claim that the Rust
symbols exist. New tests will use imported bare single-line trace attributes
with their actual TC and criterion IDs.

## Option Permutation Matrix

| Dimension | Admitted combinations and adverse controls | Cases |
| --- | --- | --- |
| Selection | Current invariant; recorded pre/post invocation; foreign clause/model/digest/role | TC-058, TC-061 |
| Values | Every node kind at field/State/parameter/result sites; exact/under/over bounds; option/sequence wrappers | TC-055, TC-060 |
| Population | Missing/incomplete/complete; empty/unique/duplicate keys; complete dangling versus unavailable targets | TC-059 |
| Frame | Empty/explicit field/create/delete permissions; unchanged/changed/missing counterparts | TC-062, TC-063 |
| Observation | Current/pre/post self; parameters pre, result post; lexical/conditional captures and deleted references | TC-061, TC-071 |
| Limits | Independent default/lowered/zero/elevated options, exact and one-below work, partial events/diagnostics | TC-057, TC-065, TC-074, TC-075 |

## Constraint Boundary Tests

TC-057 covers construction bytes/text/nodes/entries/depth. TC-065 covers every
validation inventory/byte/object/work/text/diagnostic dimension, including zero
detail capacity and the separate terminal reason. TC-074/075 cover exact
expression/graph cost, comparisons, Unicode advances, events and active depth.
Hard limits are tested with elevated caller options; coupled ceilings are
reported explicitly, never relabeled as exact successes. All loops use bounded
Rust-generated families, not concurrent stress on the shared desktop.

## Error Paths and State Transitions

Input construction, model/static refusal, runtime invalidity, unavailable input,
resource exhaustion, cancellation and defensive execution-invariant failure
have distinct controls. TC-064 mixes known invalid and unavailable data and
checks deterministic complete diagnostic enumeration. TC-061–063 cover actual
pre/post object deltas and frames; TC-066/075 cover immutable retries after
success/failure. A callback panic propagates and is observed only through a
private test catch_unwind boundary, without claiming a returned runtime report.

## Edge Cases

- Flat arena sharing, unused invalid nodes and bounded destruction: TC-055/057.
- Empty/Unicode keys, equal-valued distinct identities and reference cycles: TC-059/068/072.
- Empty sequence, ordered duplicates and short circuiting: TC-067/070.
- One-or-more-edge closure over all functional graphs with one to three vertices: TC-068.
- Signed extrema, negative remainder and original-source invariant diagnostics: TC-069.
- Captured deleted objects, nested pre and evaluate-once locals: TC-061/071.
- Nested/repeated/grouped implications, CRLF and multibyte operand lineage: TC-073.
- Unsupported collect with hypothetical duplicate outputs: TC-076.

## Integration Test Matrix

IT-006 composes real library components and the pinned Contract IR Rust API;
TC-077 executes its seven success criteria. TC-055/058/059/061–063/067/069/071–074/076
exercise their respective real boundaries. This is an in-process Rust API
integration, so no fictitious service/browser/event/database classification is
introduced. No external producer or network service is required.

## Evidence Strategy

Examples and independent adverse mutations qualify concrete diagnostics and
truth. Generated families qualify repeatability, value dimensions, ordering and
bounded work. A separate Boolean adjacency-matrix closure oracle qualifies
reaches over its declared finite test domain. No Loom target is applicable to
this serial immutable API: it has no shared state, locks or scheduler. No fuzz,
model-checking or mutation-adequacy result is claimed without actually running
that tool. Review the advisor's current output and supplement uncovered methods
by explicit analysis rather than assuming its catalog is exhaustive.

## Coverage Gaps

Twenty-one constructor tests and a role-separation compile-fail doctest qualify
TC-055–057 in SR-096. Population validation, reference execution and IT-006
remain planned. Root CLI syntax/audit/linker/checker results do not qualify
runtime truth. LC02 strict linked-package/projection and FS03 acceptance remain
issue-level gates; B portable outcomes, backend qualification and Quire consumer
integration are separately owned downstream work in the full assignment.
