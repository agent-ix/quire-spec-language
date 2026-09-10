---
id: TM-007
title: "Mapped native workflow tests"
type: TestMatrix
---
## Overview

FR-022 is the compiler-side API under LC05. Its programmatic mapped fixtures do
not claim complete Quire producer integration in IT-003.
FR-030 / TC-108 exercise the actual pinned Rust extractor and native consumer.
FR-030 and FR-011 rows using TC-108 require `--features quire-extraction` (or
`--all-features`). Both minimal and enabled feature configurations have separate
local test and lint lanes; the hosted workflow remains manual-dispatch only.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-011 | FR-011-AC-1 | TC-108 | ✅ Tested |
| FR-011 | FR-011-AC-2 | TC-108 | ✅ Tested |
| FR-011 | FR-011-AC-3 | TC-108 | ✅ Tested |
| FR-011 | FR-011-AC-4 | TC-108 | ✅ Tested |
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
| FR-028 | FR-028-AC-1 | TC-106 | ✅ Tested |
| FR-028 | FR-028-AC-2 | TC-106 | ✅ Tested |
| FR-028 | FR-028-AC-3 | TC-106 | ✅ Tested |
| FR-028 | FR-028-AC-4 | TC-106 | ✅ Tested |
| FR-029 | FR-029-AC-1 | TC-107 | ✅ Tested |
| FR-029 | FR-029-AC-2 | TC-107 | ✅ Tested |
| FR-029 | FR-029-AC-3 | TC-107 | ✅ Tested |
| FR-030 | FR-030-AC-1 | TC-108 | ✅ Tested |
| FR-030 | FR-030-AC-2 | TC-108 | ✅ Tested |
| FR-030 | FR-030-AC-3 | TC-108 | ✅ Tested |
| FR-030 | FR-030-AC-4 | TC-108 | ✅ Tested |
| FR-030 | FR-030-AC-5 | TC-108 | ✅ Tested |

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
| TC-106 | Selected package execution | Integration | P1 | FR-028 | ✅ Tested |
| TC-107 | Standalone projection export | Integration | P1 | FR-029 | ✅ Tested |
| TC-108 | Actual Quire/native workflow | Integration | P1 | FR-030, FR-011 | ✅ Tested |
