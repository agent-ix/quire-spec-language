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
FR-031 / TC-109 extraction behavior also requires that feature; its disabled-build
request-refusal control runs in the minimal suite.
FR-032-AC-4 / TC-110 package replay runs in both configurations; its Markdown
outcome and original/body mapping checks require quire-extraction. Every runtime
case requires report provenance. Missing-model compilation instead requires the
appropriate error provenance, explicitly rather than through a presence guard.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-011 | FR-011-AC-1 | TC-108 |  |
| FR-011 | FR-011-AC-2 | TC-108 |  |
| FR-011 | FR-011-AC-3 | TC-108 |  |
| FR-011 | FR-011-AC-4 | TC-108 |  |
| FR-022 | FR-022-AC-1 | TC-095 |  |
| FR-022 | FR-022-AC-2 | TC-096 |  |
| FR-022 | FR-022-AC-3 | TC-096 |  |
| FR-022 | FR-022-AC-4 | TC-096 |  |
| FR-022 | FR-022-AC-5 | TC-095 |  |
| FR-023 | FR-023-AC-1 | TC-097 |  |
| FR-023 | FR-023-AC-2 | TC-098 |  |
| FR-023 | FR-023-AC-3 | TC-098 |  |
| FR-023 | FR-023-AC-4 | TC-097 |  |
| FR-024 | FR-024-AC-1 | TC-099 |  |
| FR-024 | FR-024-AC-2 | TC-100 |  |
| FR-024 | FR-024-AC-3 | TC-099, TC-100 |  |
| FR-024 | FR-024-AC-4 | TC-099, TC-100 |  |
| FR-024 | FR-024-AC-5 | TC-099, TC-100 |  |
| FR-025 | FR-025-AC-1 | TC-101 |  |
| FR-025 | FR-025-AC-2 | TC-101 |  |
| FR-025 | FR-025-AC-3 | TC-102 |  |
| FR-025 | FR-025-AC-4 | TC-102 |  |
| FR-025 | FR-025-AC-5 | TC-101 |  |
| FR-026 | FR-026-AC-1 | TC-103 |  |
| FR-026 | FR-026-AC-2 | TC-103 |  |
| FR-026 | FR-026-AC-3 | TC-104 |  |
| FR-026 | FR-026-AC-4 | TC-104 |  |
| FR-026 | FR-026-AC-5 | TC-103 |  |
| FR-027 | FR-027-AC-1 | TC-105 |  |
| FR-027 | FR-027-AC-2 | TC-105 |  |
| FR-027 | FR-027-AC-3 | TC-105 |  |
| FR-027 | FR-027-AC-5 | TC-435 |  |
| FR-027 | FR-027-AC-6 | TC-435 |  |
| FR-027 | FR-027-AC-7 | TC-435 |  |
| FR-027 | FR-027-AC-8 | TC-435 |  |
| FR-028 | FR-028-AC-1 | TC-106 |  |
| FR-028 | FR-028-AC-2 | TC-106 |  |
| FR-028 | FR-028-AC-3 | TC-106 |  |
| FR-028 | FR-028-AC-4 | TC-106 |  |
| FR-029 | FR-029-AC-1 | TC-107 |  |
| FR-029 | FR-029-AC-2 | TC-107 |  |
| FR-029 | FR-029-AC-3 | TC-107 |  |
| FR-030 | FR-030-AC-1 | TC-108 |  |
| FR-030 | FR-030-AC-2 | TC-108 |  |
| FR-030 | FR-030-AC-3 | TC-108 |  |
| FR-030 | FR-030-AC-4 | TC-108 |  |
| FR-030 | FR-030-AC-5 | TC-108 |  |
| FR-031 | FR-031-AC-1 | TC-109 |  |
| FR-031 | FR-031-AC-2 | TC-109 |  |
| FR-031 | FR-031-AC-3 | TC-109 |  |
| FR-031 | FR-031-AC-4 | TC-109 |  |
| FR-032 | FR-032-AC-1 | TC-110 |  |
| FR-032 | FR-032-AC-2 | TC-110 |  |
| FR-032 | FR-032-AC-3 | TC-110 |  |
| FR-032 | FR-032-AC-4 | TC-110 |  |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-095 | Mapped parent workflow | Integration | P1 | FR-022 | ✅ |
| TC-096 | Typed mapped refusals | Integration | P1 | FR-022 | ✅ |
| TC-097 | Retained native execution | Integration | P1 | FR-023 | ✅ |
| TC-098 | Native execution stops | Integration | P1 | FR-023 | ✅ |
| TC-099 | Read native inputs | Integration | P1 | FR-024 | ✅ |
| TC-100 | Refuse native input reads | Integration | P1 | FR-024 | ✅ |
| TC-101 | Public model source frontend | Integration | P1 | FR-025 | ✅ |
| TC-102 | Model source refusals | Integration | P1 | FR-025 | ✅ |
| TC-103 | Standalone native execution | Integration | P1 | FR-026 | ✅ |
| TC-104 | Standalone native refusals | Integration | P1 | FR-026 | ✅ |
| TC-105 | Standalone package export | Integration | P1 | FR-027 | ✅ |
| TC-435 | CLI compile routes a program by its declared edition | Integration | P1 | FR-027 | ✅ |
| TC-106 | Selected package execution | Integration | P1 | FR-028, FR-016 | ✅ |
| TC-107 | Standalone projection export | Integration | P1 | FR-029 | ✅ |
| TC-108 | Actual Quire/native workflow | Integration | P1 | FR-030, FR-011 | ✅ |
| TC-109 | Standalone Markdown execution | Integration | P1 | FR-031 | ✅ |
| TC-110 | Concrete ConfigVersion workflow | Integration | P1 | FR-032 | ✅ |
