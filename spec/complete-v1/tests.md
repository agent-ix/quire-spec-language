---
id: TM-010
title: "Complete-V1 lane adoption matrix"
type: TestMatrix
---
# Complete-V1 lane adoption matrix

## Overview

This matrix covers only IT-011's repository-local adoption contract. It does
not copy the central TM-009 product matrix or claim that planned complete-V1
behavior already executes. Plan-013 retains the authoritative 83-row mapping
from each Agent-A capability to one downstream ticket and one central TestCase.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-055 | FR-055-AC-1 | TC-144 | ✅ Rust audit |
| FR-055 | FR-055-AC-2 | TC-144 | ✅ Rust audit |
| FR-055 | FR-055-AC-3 | TC-144 | ✅ Rust audit plus evidence reconciliation |
| FR-055 | FR-055-AC-4 | TC-144 | ✅ Rust audit |
| FR-055 | FR-055-AC-5 | TC-144 | ✅ Rust audit plus changed-path inspection |

### Integration Requirement Coverage

| Integration Req | Success Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| IT-011 | IT-011-SC-01 | TC-144 | ✅ Rust audit plus frozen-reference inspection |
| IT-011 | IT-011-SC-02 | TC-144 | ✅ Rust audit |
| IT-011 | IT-011-SC-03 | TC-144 | ✅ Rust audit |
| IT-011 | IT-011-SC-04 | TC-144 | ✅ Rust audit plus evidence reconciliation |
| IT-011 | IT-011-SC-05 | TC-144 | ✅ Rust audit |
| IT-011 | IT-011-SC-06 | TC-144 | ✅ Rust audit plus changed-path inspection |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-144 | Audit the complete-V1 Agent-A delivery plan | Static | P0 | FR-055-AC-1..FR-055-AC-5 | ✅ Passing locally |

## Six coverage rules

The only selection alternatives are a correct or mutated central allocation;
TC-144 checks every row rather than sampling. Zero, duplicate, omitted,
cross-ticket and out-of-lane rows are rejected. The task state transition is
the fixed serial order from adoption through qualification. Edge cases include
the cross-repository WASM ticket, central reused requirements, external
FR-304/FR-309 ownership, and the intentionally unallocated FR-135 through
FR-139 identifiers. No option permutation or numeric runtime boundary is
introduced by this planning integration.

## Integration Test Matrix

IT-011 integrates immutable repository artifacts rather than services. TC-144
compiles the plan and task documents into a Rust test, while Quire validates the
typed artifact graph. Central TC-180 through TC-198, TC-209/210 and TC-220
through TC-231 remain pending product tests in their owning downstream tasks.

## Coverage Gaps

The 83 product capability rows retain the implementation, verification,
integration and qualification states from the accepted inventory. TM-010 being
green proves the plan's integrity only; it promotes none of those product rows.
