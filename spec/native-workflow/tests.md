---
id: TM-007
title: "Mapped native workflow tests"
type: TestMatrix
---
## Overview

FR-022 is the compiler-side API under LC05. Its programmatic mapped fixtures do
not claim complete Quire producer integration in IT-003.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-022 | FR-022-AC-1 | TC-095 | ✅ Tested |
| FR-022 | FR-022-AC-2 | TC-096 | ✅ Tested |
| FR-022 | FR-022-AC-3 | TC-096 | ✅ Tested |
| FR-022 | FR-022-AC-4 | TC-096 | ✅ Tested |
| FR-022 | FR-022-AC-5 | TC-095 | ✅ Tested |
| FR-023 | FR-023-AC-1 | TC-097 | ✅ Tested |
| FR-023 | FR-023-AC-2 | TC-098 | ✅ Tested |
| FR-023 | FR-023-AC-3 | TC-098 | ✅ Tested |
| FR-023 | FR-023-AC-4 | TC-097 | ✅ Tested |
| FR-024 | FR-024-AC-1 | TC-099 | ✅ Tested |
| FR-024 | FR-024-AC-2 | TC-100 | ✅ Tested |
| FR-024 | FR-024-AC-3 | TC-099, TC-100 | ✅ Tested |
| FR-024 | FR-024-AC-4 | TC-099, TC-100 | ✅ Tested |
| FR-025 | FR-025-AC-1 | TC-101 | ✅ Tested |
| FR-025 | FR-025-AC-2 | TC-101 | ✅ Tested |
| FR-025 | FR-025-AC-3 | TC-102 | ✅ Tested |
| FR-025 | FR-025-AC-4 | TC-102 | ✅ Tested |
| FR-025 | FR-025-AC-5 | TC-101 | ✅ Tested |
| FR-026 | FR-026-AC-1 | TC-103 | ✅ Tested |
| FR-026 | FR-026-AC-2 | TC-103 | ✅ Tested |
| FR-026 | FR-026-AC-3 | TC-104 | ✅ Tested |
| FR-026 | FR-026-AC-4 | TC-104 | ✅ Tested |
| FR-026 | FR-026-AC-5 | TC-103 | ✅ Tested |
| FR-027 | FR-027-AC-1 | TC-105 | ✅ Tested |
| FR-027 | FR-027-AC-2 | TC-105 | ✅ Tested |
| FR-027 | FR-027-AC-3 | TC-105 | ✅ Tested |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-095 | Mapped parent workflow | Integration | P1 | FR-022 | ✅ Tested |
| TC-096 | Typed mapped refusals | Integration | P1 | FR-022 | ✅ Tested |
| TC-097 | Retained native execution | Integration | P1 | FR-023 | ✅ Tested |
| TC-098 | Native execution stops | Integration | P1 | FR-023 | ✅ Tested |
| TC-099 | Read native inputs | Integration | P1 | FR-024 | ✅ Tested |
| TC-100 | Refuse native input reads | Integration | P1 | FR-024 | ✅ Tested |
| TC-101 | Public model source frontend | Integration | P1 | FR-025 | ✅ Tested |
| TC-102 | Model source refusals | Integration | P1 | FR-025 | ✅ Tested |
| TC-103 | Standalone native execution | Integration | P1 | FR-026 | ✅ Tested |
| TC-104 | Standalone native refusals | Integration | P1 | FR-026 | ✅ Tested |
| TC-105 | Standalone package export | Integration | P1 | FR-027 | ✅ Tested |
