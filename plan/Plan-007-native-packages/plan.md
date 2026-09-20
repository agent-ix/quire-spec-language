---
id: Plan-007
title: "Native checked packages and verified reconstruction"
type: Plan
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-007
    type: references
---
# Plan-007: Native checked packages and verified reconstruction

## Requirements Summary

This new LC02 package slice is owned by private language issue #3. Its
specification is 41da6e5eb86bb727fbad5370fd23b38d330bcfea; the actual eight
QUOIN reviews SR-101–108 are committed at
69588ad9888893ae839352661e16df3ec040b0d4. Plan-005's checker and Plan-006's
runtime qualification are done. Existing source/link plans retain their own
requirements; this bundle adds the new package requirements, without
duplicating their completed tasks.

- [x] StR-001: native package contribution qualified; the full backend/Quire objective remains LC04/05.
- [x] FR-019-AC-1..10: complete immutable package and exact static dependencies.
- [x] FR-020-AC-1..11: bounded verified read/rebind, explicit authority and actual compiler reconstruction.
- [x] FR-021-AC-1..6: native source-bound static identity, independent vectors and role separation.
- [x] NFR-007: all five offered/emitted/string/entry/depth metrics and each pass.
- [x] NFR-005: first-party production, assertions, generators and qualification remain Rust.
- [x] IT-007-SC-01..08: actual reconstructed package-to-native-runtime workflow.

The criterion enumeration and authoritative test mapping are in
[TM-005](../../spec/native-packages/tests.md). All fourteen package cases and
all 27 functional criteria now have executed evidence in SR-111/112.
Review approval establishes the specification gate, not implemented packages,
schema qualification, independent consumer acceptance or full LC02 closure.

## Dependency Graph

- FR-015 → FR-016: admitted exact models are required for checked native clauses; both exist.
- FR-016 → FR-021: native identity derives from checked source/model obligations.
- FR-021 → FR-019: final artifact publication includes that static identity.
- FR-008 → FR-019: available native-reference disposition rests on the qualified evaluator.
- FR-019 + FR-016 → FR-020: the reader regenerates the producer manifest through actual checking.
- FR-020 → IT-007: integration executes only after verified readback.

NFR-007 constrains the three package requirements; NFR-005 constrains all
executable work. These edges are acyclic. The shared manifest, feature traversal,
canonical encoder and accounting primitives are explicit first-task
deliverables, so the reader consumes one implementation. They remain private
modules of the compiler. Task frontmatter is authoritative for execution edges.

Existing seams are CheckedPackage/CheckBindings, LinkedPackage, NativeModel,
FormalSource and runtime validate/evaluate. JSON recognition remains Serde-owned;
the existing IR BoundPackage binder is reserved for actual downstream lowering.

## Test Plan

| Cases | Boundary | Entrance criterion | Exit evidence |
| --- | --- | --- | --- |
| TC-078–080 | Package inventory/features | Real admitted model and checked source | Independent complete manifests, aliases/unused declarations, exhaustive feature/cycle controls |
| TC-081–082 | Dispositions and identities | Produced package; reader when applicable | Preserved unlowered clauses, typed roles, representation invariance and ordered-array refusals |
| TC-083–084 | Wire/schema/selection | Correct raw selector and valid unrelated setup | Closed Draft 2020-12 and lexical distinctions, format-first precedence, exact feature selection |
| TC-085–087 | Authority/reconstruction | Independent source/models/authorship | One-axis identity/claim mutations and actual syntax/link/type/definedness failure stages |
| TC-088 | Package/frontend limits | Independently counted pass workloads | Zero/exact/one-below/hard controls, coupled-limit disclosure, unchanged inputs and retries |
| TC-089 | Actual integration | Fresh package reconstructed after original is dropped | Every IT-007 step, healthy/violating/refused/incomplete and operation observations |
| TC-090–091 | Static canonicalization | Independently authored bytes and actual checked fixtures | Exact Unicode/u64/domain vectors, static mutations and excluded-projection/foreign-role refusals |

Tests precede implementation and record a genuine red result at the intended
missing API or assertion. Generated families and independent literal/hash
vectors are mandatory where specified. A failed setup does not establish the
target refusal. Tests use the existing Rust harness and shared trace attributes.
Schema acceptance cannot replace wire/source/constructor validation. Fuzzing
remains an applicable unexecuted recommendation recorded by SR-105; no fuzz,
Loom, mutation-adequacy or benchmark result is fabricated.

## Delivery

### Track A: Critical path, serial

- Task-016 construction and static identity — Done at c195950 / SR-111; independent bytes and complete inventories agree, with measured producer limits. Original estimates and corrective sequence remain in the task/history.
- Task-017 verified reconstruction — Done at eb97a87 / SR-112; strict intake and real compiler replay preserve authority, original bytes, failure stages and selected limits.
- Task-018 integration and qualification — Done at eb97a87 / SR-112/113; reconstructed workflows and local gates pass. The gap audit retains the catalog's metric-target limitation.

A gate failure is fixed before dependent work proceeds; a changed contract
reopens the specification review. There is one active writer and no parallel
Cargo phases or additional agents.

## Parallel Execution Summary

All new tasks run serially: construction → reconstruction → qualification.
Existing independent B/C/TL sessions retain their own claims and resources;
this plan does not schedule their builds or edit their repositories.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-016 | A | FR-019, FR-021, NFR-007, NFR-005 | TC-078–082, TC-088, TC-090–091; producer portions | done |
| Task-017 | A | FR-020, FR-019, FR-021, NFR-007, NFR-005 | TC-081–088, TC-091; reader portions | done |
| Task-018 | A | FR-019, FR-020, FR-021, NFR-007, NFR-005, IT-007, StR-001 | TC-089 | done |

## Coordination Rules

The work is tracked on branch
agent-a/lc02-linked-packages, based on merged runtime 789c636. Keep the private
owning issue and PR concrete with source/review/evidence pins. No public
posting, producer-language invocation or shared-repository change is included.

Use local serial Cargo phases at nice 10, one build job/test thread and existing
offline caches. Hosted workflows remain workflow_dispatch-only; no run is
requested. Read the actual agent-skills code-review and rust-review skills for
implementation reviews, then QUOIN gap-analysis with optional semantic pass off.

A package milestone preserves full original acceptance: independent A/B/C
native-domain/interchange adoption, actual executable lowering and qualified
existing backend with compiled ConfigVersion, then Quire integration. These
remain LC02/FS05, LC04 and LC05 work. An unlowered disposition is retained
evidence of remaining work, not a replacement for the backend.
