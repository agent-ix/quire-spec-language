---
id: TM-006
title: "Native Boolean lowering matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-009](../functional/FR-009-lower-qualified-projections.md).
Five lowering tests and all eight generated truth and activation assignments
pass. TC-094 compiles a codegen-produced proptest strategy and observes exact
LLVM 3.1.0 source probes while retaining the reusable reader's explicit refusal.
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
