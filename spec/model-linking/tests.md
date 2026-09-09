---
id: TM-003
title: "Native model-linking and static-typing matrix"
type: TestMatrix
---

## Overview

LC02 verification after the owner's internal adoption of specification PR8 at
e897f81. Ten linking cases, TC-020–024 and TC-030–034, now execute through the
public formal linker API. Their eleven FR-005/013 criteria are backed by the
Rust tests in tests/linking.rs. The five FR-006 typing cases remain planned.
IT-005's complete typing/projection qualification and the full workflow remain
open. TM-001/002 retain their existing native/audit evidence.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-005 | FR-005-AC-1 | TC-020 | ✅ Passed |
| FR-005 | FR-005-AC-2 | TC-021 | ✅ Passed |
| FR-005 | FR-005-AC-3 | TC-022 | ✅ Passed |
| FR-005 | FR-005-AC-4 | TC-023 | ✅ Passed |
| FR-005 | FR-005-AC-5 | TC-024 | ✅ Passed |
| FR-006 | FR-006-AC-1 | TC-025 | 🚧 Planned |
| FR-006 | FR-006-AC-2 | TC-026 | 🚧 Planned |
| FR-006 | FR-006-AC-3 | TC-027 | 🚧 Planned |
| FR-006 | FR-006-AC-4 | TC-028 | 🚧 Planned |
| FR-006 | FR-006-AC-5 | TC-029 | 🚧 Planned |
| FR-013 | FR-013-AC-1 | TC-030 | ✅ Passed |
| FR-013 | FR-013-AC-2 | TC-030 | ✅ Passed |
| FR-013 | FR-013-AC-3 | TC-031 | ✅ Passed |
| FR-013 | FR-013-AC-4 | TC-032 | ✅ Passed |
| FR-013 | FR-013-AC-5 | TC-033 | ✅ Passed |
| FR-013 | FR-013-AC-6 | TC-034 | ✅ Passed |
| FR-014 | FR-014-AC-1 | TC-035 | ✅ Passed |
| FR-014 | FR-014-AC-2 | TC-036 | ✅ Passed |
| FR-014 | FR-014-AC-3 | TC-037 | ✅ Passed |
| FR-014 | FR-014-AC-4 | TC-038 | ✅ Passed |
| FR-014 | FR-014-AC-5 | TC-039 | ✅ Passed |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-020 | Exact qualified import | Integration | P1 | FR-005-AC-1 | ✅ Passed |
| TC-021 | Missing selected import | Integration | P1 | FR-005-AC-2 | ✅ Passed |
| TC-022 | Ambiguous exported declaration | Integration | P1 | FR-005-AC-3 | ✅ Passed |
| TC-023 | Stale package closure | Integration | P1 | FR-005-AC-4 | ✅ Passed |
| TC-024 | Failed linkage is atomic | Property | P1 | FR-005-AC-5 | ✅ Passed |
| TC-025 | Unguarded optional unwrap | Integration | P1 | FR-006-AC-1 | 🚧 Planned |
| TC-026 | Presence facts stay with their observation | Integration | P1 | FR-006-AC-2 | 🚧 Planned |
| TC-027 | Guarded bounded addition | Integration | P1 | FR-006-AC-3 | 🚧 Planned |
| TC-028 | Ambiguous scalar inference | Integration | P1 | FR-006-AC-4 | 🚧 Planned |
| TC-029 | Clause roots are Boolean | Integration | P1 | FR-006-AC-5 | 🚧 Planned |
| TC-030 | Exact source and formal artifact binding | Integration | P1 | FR-013-AC-1, FR-013-AC-2 | ✅ Passed |
| TC-031 | Lexical and formal declaration occurrences | Integration | P1 | FR-013-AC-3 | ✅ Passed |
| TC-032 | Unmapped reference and operation forms | Integration | P1 | FR-013-AC-4 | ✅ Passed |
| TC-033 | Native linking resource ceilings | Property | P1 | FR-013-AC-5 | ✅ Passed |
| TC-034 | Ambiguity provenance and atomicity | Property | P1 | FR-013-AC-6 | ✅ Passed |
| TC-035 | Explicit source identity assignment | Integration | P1 | FR-014-AC-1 | ✅ Passed |
| TC-036 | Independent formal coordinate examples | Integration | P1 | FR-014-AC-2 | ✅ Passed |
| TC-037 | Foreign native source requests | Integration | P1 | FR-014-AC-3 | ✅ Passed |
| TC-038 | Inconsistent formal coordinates | Integration | P1 | FR-014-AC-4 | ✅ Passed |
| TC-039 | Bounded generated span correspondence | Property | P1 | FR-014-AC-5 | ✅ Passed |

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
Linker tests use imported single-line ix-trace-rs attributes and real APIs.
Actual Quire reconciliation reports TM-003 10/15 backed, FR-005 5/5 and FR-013
6/6. FR-006 remains 0/5. There are no status lies or untracked symbols. The
known functional-table Status/Coverage Status classifier limitation is retained;
explicit TC statuses and executed logs supply the separate completion evidence.

This packet specifies evidence for LC02. Native linking is implemented; typing,
runtime observations and qualified projection remain incomplete. The accepted IR ADR-0054
removes the earlier prerequisite for a shared Filament model adapter. The
generic lane uses the public formal declaration API; A owns concrete native
projection work for clauses that need additional semantic correspondence.
The full workflow remains IT-002 and the original Agent A assignment.

FR-013 now defines the concrete formal-environment resolution API and TC-030–034
cover its six criteria. TC-020–024 consume that same real API. Canonical byte
selection, explicit self binding, lexical scopes and resource budgets are
defined before code; FR-006's five static-judgment cases remain separate.

## Formal source bridge qualification

FR-014 adds five executed cases to the same LC02 matrix. TC-035–038 exercise
actual pinned IR constructors and independent adverse inputs; TC-039 generates
156 sources and enumerates all valid and invalid offset pairs with a separate
coordinate oracle. Every FR-014 criterion maps to one case. Boundaries include
empty input, EOF, CRLF interior, split scalars, the existing source-byte ceiling,
foreign labels/digests and misleading IR endpoint coordinates. Success after
failure checks immutable request behavior; there is no runtime state transition
or callback/concurrency option in this API. Loom and concurrency fault injection
do not apply to this immutable, single-request bridge. No fuzz result is claimed.
These tests do not discharge FR-006 or the full IT-005 model/checker integration.
