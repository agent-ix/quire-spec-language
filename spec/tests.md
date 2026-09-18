---
id: TM-001
title: "Rust fixture audit test matrix"
type: TestMatrix
---

## Overview

Scoped to FR-012/NFR-005/NFR-011 and IT-004. Status: locally verified after
implementation. This matrix does not assert coverage of the earlier
compiler/evaluator scope. US-004/StR-001 are the driving lineage; their full
operational validation remains outside this audit-only plan.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-012 | FR-012-AC-1 | TC-001 | ✅ Passed locally |
| FR-012 | FR-012-AC-2 | TC-002 | ✅ Passed locally |
| FR-012 | FR-012-AC-3 | TC-003 | ✅ Passed locally |
| FR-012 | FR-012-AC-5 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-6 | TC-006 | ✅ Passed locally |
| FR-012 | FR-012-AC-7 | TC-007 | ✅ Passed locally |
| FR-012 | FR-012-AC-8 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-9 | TC-008 | ✅ Passed locally |
| FR-012 | FR-012-AC-10 | TC-009 | ✅ Passed locally |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | Self-contained identity negative controls | Unit | P1 | FR-012-AC-1 | ✅ Passed locally |
| TC-002 | Selected invocation packet audit | Integration | P1 | FR-012-AC-2 | ✅ Passed locally |
| TC-003 | Selected role and source correspondence audit | Integration | P1 | FR-012-AC-3 | ✅ Passed locally |
| TC-005 | Real native rule syntax outcomes | Integration | P1 | FR-012-AC-5, FR-012-AC-8 | ✅ Passed locally |
| TC-006 | Malformed JSON and field types | Property | P1 | FR-012-AC-6 | ✅ Passed locally |
| TC-007 | Fixture root containment | Property | P1 | FR-012-AC-7 | ✅ Passed locally |
| TC-008 | Audit resource ceilings | Property | P1 | FR-012-AC-9 | ✅ Passed locally |
| TC-009 | CLI encoding and Rust-only execution | E2E | P1 | FR-012-AC-10, NFR-005-M-2 | ✅ Passed locally |
| TC-010 | Owned verification language inventory | Manual | P1 | NFR-005-M-1 | ✅ Inspected locally |
| TC-149 | Refuse a malformed or under-specified vendoring pin | Integration | P1 | NFR-011-AC-1 | ✅ Passed locally |
| TC-150 | Vendor exact pinned bytes and verify external digests | Integration | P1 | NFR-011-AC-2 | ✅ Passed locally |
| TC-151 | Idempotent re-vendoring at an unchanged pin | Integration | P1 | NFR-011-AC-3 | ✅ Passed locally |
| TC-152 | Detect drift, missing/stray files offline, and replace a dropped pin wholesale | Integration | P1 | NFR-011-AC-4, NFR-011-AC-5 | ✅ Passed locally |

## Six coverage rules

All nine FR-012 ACs, both NFR-005 metric obligations and all five NFR-011 ACs
are verified above.
Modes are mutually exclusive, not combinable options. Valid/unknown modes,
missing/extra arguments, contained/foreign paths, zero/exact/over resource
limits, malformed fields and identity changes are covered. No asynchronous
state machine is introduced; the identity registry's first binding/repeated
binding/conflicting binding transitions have explicit controls. The generated
input rows require actual generation and retained failures, not a Property
label on one example. The implemented Property cases enumerate bounded JSON,
field, path and budget families deterministically; no randomized campaign or
fuzzing result is claimed. Independent fixture mutations reach real audit code.

## Integration Test Matrix

IT-004 is a command/file boundary, not a service/browser/event/database system.
TC-002, TC-003 and TC-005 execute real filesystem/parser integration, and TC-009
executes the compiled binary. The private standard packet remains an explicit
local lane; the default local suite uses self-contained negative controls. No substitute network/daemon classification is invented.

## Coverage gaps

The preexisting 21 Rust tests and older FR/NFR obligations still need their own
formal TC/evidence remediation. TC-010's inspection is recorded in
[the remediation inventory](../docs/rust-verification-remediation.md); the module
classifies Manual as no_source_symbol. Hosted CI is manual-dispatch only.

## Execution record

The audit unit and default audit integration tests pass locally. All three named
private-packet tests were explicitly executed against specification revision
36293bae7f5bcb7ca3b2389ed166e525dc9dba87 and passed. Quire resolves every audit
test symbol, every FR-012 AC and every executable TC. TC-010 has manual evidence. These counts establish
the selected scope; they are not full compiler or semantic qualification.
