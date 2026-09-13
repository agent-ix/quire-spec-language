---
id: TM-006
title: "Native verification backend matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-009](../functional/FR-009-lower-qualified-projections.md).
The Boolean, bounded-integer and primitive-state lowering tests pass. TC-094
compiles a codegen-produced proptest strategy and observes exact LLVM 3.1.0
source probes while retaining the reusable reader's explicit refusal. IT-010
compiles the numeric/state oracle and all strategy populations, runs the pinned
Kani contract and replays its concrete counterexample through native execution.
Historical code review is retained in SR-114/115; the completion delta receives
its own PR-time specification, code, Rust and gap reviews.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-009 | FR-009-AC-1 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-2 | TC-093 | ✅ Tested |
| FR-009 | FR-009-AC-3 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-4 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-5 | TC-094 | ✅ Tested |
| FR-009 | FR-009-AC-6 | TC-093 | ✅ Tested |
| FR-009 | FR-009-AC-7 | TC-093 | ✅ Tested |
| FR-033 | FR-033-AC-1 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-2 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-3 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-4 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-5 | TC-111 | ✅ Tested |
| FR-034 | FR-034-AC-1 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-2 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-3 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-4 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-5 | TC-112 | ✅ Tested |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-092 | Complete binding and correspondence | Integration | P1 | FR-009 | ✅ Tested |
| TC-093 | Unsupported forms and bounded work | Integration | P1 | FR-009 | ✅ Tested |
| TC-094 | Actual generated truth and activation through IT-008 | Integration | P1 | FR-009 | ✅ Tested |
| TC-111 | Bounded integer IR and explicit command target | Integration | P1 | FR-033 | ✅ Tested |
| TC-112 | State fields and validated primitive inputs | Integration | P1 | FR-034 | ✅ Tested |
| IT-010-SC-01 | Exact backend/tool/package pins | Integration | P0 | IT-010 | ✅ Tested locally |
| IT-010-SC-02 | Actual native projection and generated artifact identity | Integration | P0 | IT-010, FR-033, FR-034 | ✅ Tested locally |
| IT-010-SC-03 | Native/generated bounded-corpus parity | Integration | P0 | IT-010, FR-034 | ✅ Tested locally |
| IT-010-SC-04 | Executed model-domain strategy populations and rates | Property | P0 | IT-010, FR-034 | ✅ Tested locally |
| IT-010-SC-05 | Kani proof, counterexample decode and native replay | Analysis | P0 | IT-010, FR-034 | ✅ Tested locally |
| IT-010-SC-06 | Object/graph refusal clause and source locus | Integration | P0 | IT-010, FR-034 | ✅ Tested locally |
