---
id: TM-006
title: "Native verification backend matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-009](../functional/FR-009-lower-qualified-projections.md).
The Boolean, bounded-integer and primitive-state lowering tests pass.
FR-009-AC-5 is qualified by quire-integration's
[TC-094](ix://agent-ix/quire-integration/TC-094), which composes this
compiler's lowering with the code generator; that repository also owns
[IT-008](ix://agent-ix/quire-integration/IT-008) and
[IT-010](ix://agent-ix/quire-integration/IT-010).

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-009 | FR-009-AC-1 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-2 | TC-093 | ✅ Tested |
| FR-009 | FR-009-AC-3 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-4 | TC-092 | ✅ Tested |
| FR-009 | FR-009-AC-5 | — | 🚧 No local test; verified by [quire-integration TC-094](ix://agent-ix/quire-integration/TC-094) |
| FR-009 | FR-009-AC-6 | TC-093 | ✅ Tested |
| FR-009 | FR-009-AC-7 | TC-093 | ✅ Tested |
| FR-033 | FR-033-AC-1 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-2 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-3 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-4 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-5 | TC-111 | ✅ Tested |
| FR-033 | FR-033-AC-6 | TC-912 | 🚧 Planned |
| FR-034 | FR-034-AC-1 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-2 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-3 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-4 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-5 | TC-112 | ✅ Tested |
| FR-034 | FR-034-AC-6 | TC-112 | ✅ Tested |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-092 | Complete binding and correspondence | Integration | P1 | FR-009 | ✅ Tested |
| TC-093 | Unsupported forms and bounded work | Integration | P1 | FR-009 | ✅ Tested |
| TC-111 | Bounded integer IR and explicit command target | Integration | P1 | FR-033 | ✅ Tested |
| TC-112 | State fields and validated primitive inputs | Integration | P1 | FR-034 | ✅ Tested |
