---
id: TM-003
title: "Native model-linking and static-typing matrix"
type: TestMatrix
---

## Overview

Planned LC02 verification for the existing FR-005/006 acceptance criteria,
following the owner's internal adoption of specification PR8 at e897f81.
IT-005 names the actual adapter/input qualification still needed. All ten
cases are unexecuted; no binding tag, passed state or completed plan is invented.
TM-001/002 and their completed native/audit evidence remain unchanged.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-005 | FR-005-AC-1 | TC-020 | 🚧 Planned |
| FR-005 | FR-005-AC-2 | TC-021 | 🚧 Planned |
| FR-005 | FR-005-AC-3 | TC-022 | 🚧 Planned |
| FR-005 | FR-005-AC-4 | TC-023 | 🚧 Planned |
| FR-005 | FR-005-AC-5 | TC-024 | 🚧 Planned |
| FR-006 | FR-006-AC-1 | TC-025 | 🚧 Planned |
| FR-006 | FR-006-AC-2 | TC-026 | 🚧 Planned |
| FR-006 | FR-006-AC-3 | TC-027 | 🚧 Planned |
| FR-006 | FR-006-AC-4 | TC-028 | 🚧 Planned |
| FR-006 | FR-006-AC-5 | TC-029 | 🚧 Planned |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-020 | Exact qualified import | Integration | P1 | FR-005-AC-1 | 🚧 Planned |
| TC-021 | Missing selected import | Integration | P1 | FR-005-AC-2 | 🚧 Planned |
| TC-022 | Ambiguous exported declaration | Integration | P1 | FR-005-AC-3 | 🚧 Planned |
| TC-023 | Stale package closure | Integration | P1 | FR-005-AC-4 | 🚧 Planned |
| TC-024 | Failed linkage is atomic | Property | P1 | FR-005-AC-5 | 🚧 Planned |
| TC-025 | Unguarded optional unwrap | Integration | P1 | FR-006-AC-1 | 🚧 Planned |
| TC-026 | Presence facts stay with their observation | Integration | P1 | FR-006-AC-2 | 🚧 Planned |
| TC-027 | Guarded bounded addition | Integration | P1 | FR-006-AC-3 | 🚧 Planned |
| TC-028 | Ambiguous scalar inference | Integration | P1 | FR-006-AC-4 | 🚧 Planned |
| TC-029 | Clause roots are Boolean | Integration | P1 | FR-006-AC-5 | 🚧 Planned |

## Six coverage rules

Every existing FR-005/006 AC has a case. There is one selected native profile;
current/post operation contexts and matching/mismatching observation-qualified
guards are explicit case pairs. Numeric upper-bound equality and the strict
guard edge distinguish safe addition from possible overflow. Missing,
ambiguous, stale, undefined and ill-typed paths are named rather than collapsed
to generic failure. Binding permutations and prior successful calls test the
atomic-result boundary; no runtime state transition is claimed by static typing.

TC-024 uses a bounded generated input family and is Property. The other cases
use selected real adapter/checker integrations and expected judgments. Further
implementation-specific limits, version-feature combinations and adverse
adapter capabilities must be specified when that API exists; this matrix does
not claim complete coverage of a future interface it has not inspected.

## Preconditions and claim limits

The source of truth for preconditions is
[IT-005](../integration/IT-005-qualify-native-model-consumption.md).
The independently authored rule-model hypotheses require their own qualified
realization; the existing ConfigVersion model is not interchangeable with them.
Future tests use imported canonical ix-trace-rs attributes and actual APIs.
Current Quire reports these rows as unbacked, which is correct for this plan.
The installed functional-table Status/Coverage Status conflict remains visible;
all rows are explicitly planned, and no complete status is being hidden.

This packet specifies evidence for LC02. It does not supply native linking,
typing, a shared model adapter, runtime observations or qualified projection.
The full workflow remains IT-002 and the original Agent A assignment.
