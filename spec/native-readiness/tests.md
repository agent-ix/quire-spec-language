---
id: TM-002
title: "Native syntax merge-readiness matrix"
type: TestMatrix
---

## Overview

Plan-002 covers the implemented native syntax/source/CLI boundary and SR-009
findings. Linking, typing, evaluation, shared wire and external producer qualification are outside this plan.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-001 | FR-001-AC-1 | TC-011 |  |
| FR-001 | FR-001-AC-2 | TC-011 |  |
| FR-001 | FR-001-AC-3 | TC-011 |  |
| FR-001 | FR-001-AC-4 | TC-011 |  |
| FR-002 | FR-002-AC-1 | TC-012 |  |
| FR-002 | FR-002-AC-2 | TC-012 |  |
| FR-002 | FR-002-AC-3 | TC-012 |  |
| FR-002 | FR-002-AC-4 | TC-012 |  |
| FR-002 | FR-002-AC-5 | TC-012 |  |
| FR-003 | FR-003-AC-1 | TC-013 |  |
| FR-003 | FR-003-AC-2 | TC-013 |  |
| FR-003 | FR-003-AC-3 | TC-013 |  |
| FR-004 | FR-004-AC-1 | TC-014 |  |
| FR-004 | FR-004-AC-2 | TC-014 |  |
| FR-004 | FR-004-AC-3 | TC-014 |  |
| FR-004 | FR-004-AC-4 | TC-014 |  |
| FR-010 | FR-010-AC-1 | TC-015 |  |
| FR-010 | FR-010-AC-2 | TC-015 |  |
| FR-010 | FR-010-AC-3 | TC-015, TC-017 |  |
| FR-010 | FR-010-AC-4 | TC-015 |  |
| FR-010 | FR-010-AC-5 | TC-015 |  |
| FR-010 | FR-010-AC-7 | TC-015, TC-017 |  |
| FR-010 | FR-010-AC-9 | TC-015, TC-111 |  |
| FR-003 | FR-003-AC-4 | TC-016 |  |
| FR-003 | FR-003-AC-5 | TC-016 |  |
| FR-003 | FR-003-AC-6 | TC-016 |  |
| FR-010 | FR-010-AC-6 | TC-017 |  |
| FR-010 | FR-010-AC-8 | TC-018 |  |
| FR-002 | FR-002-AC-6 | TC-019 |  |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-011 | Exact native source intake | Integration | P1 | FR-001-AC-1, FR-001-AC-2, FR-001-AC-3, FR-001-AC-4 | ✅ |
| TC-012 | Located admitted native grammar | Integration | P1 | FR-002-AC-1, FR-002-AC-2, FR-002-AC-3, FR-002-AC-4, FR-002-AC-5, NFR-001-M-2, NFR-001-M-3, NFR-001-M-4 | ✅ |
| TC-013 | Token-preserving formatting | Unit | P1 | FR-003-AC-1, FR-003-AC-2, FR-003-AC-3 | ✅ |
| TC-014 | Exact extracted source correspondence | Integration | P1 | FR-004-AC-1, FR-004-AC-2, FR-004-AC-3, FR-004-AC-4, NFR-001-M-5 | ✅ |
| TC-015 | Native CLI outcomes and digest | E2E | P1 | FR-010-AC-1, FR-010-AC-2, FR-010-AC-3, FR-010-AC-4, FR-010-AC-5, FR-010-AC-7, FR-010-AC-9 | ✅ |
| TC-016 | Inclusive formatter byte ceilings | Property | P1 | FR-003-AC-4, FR-003-AC-5, FR-003-AC-6, NFR-001-M-1 | 🚧 |
| TC-017 | Native CLI OS argument boundary | E2E | P1 | FR-010-AC-3, FR-010-AC-6, FR-010-AC-7 | ✅ |
| TC-018 | Native diagnostic error interoperability | Integration | P1 | FR-010-AC-8 | ✅ |
| TC-019 | Bounded malformed source corpus | Property | P1 | FR-002-AC-6 | ✅ |

## Six coverage rules

All ACs of FR-001/002/003/004/010 map above. Parse/format commands, missing and
extra arguments, valid/invalid labels, source and output ceilings, malformed
versus unsupported inputs, repeated formatting and source-map binding changes
have explicit cases. Corpus and lowered-ceiling families are deterministic
bounded enumeration; randomized fuzzing is a recommendation, not a claimed run.
No concurrent production state is introduced, so no Loom lane is applicable.

## Evidence boundary

All tests use the actual Rust implementation and canonical ix-trace-rs tags.
The normal suite is local and requires no private standard packet. The separate
LR02 private-packet lane remains a regression check.
