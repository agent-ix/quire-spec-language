---
id: TM-001
title: "Rust fixture audit test matrix"
type: TestMatrix
---

## Overview

Scoped to FR-012/NFR-005 and IT-004. Status: 🚧 planned before implementation.
This matrix does not assert coverage of the earlier compiler/evaluator scope.
US-004/StR-001 are the driving lineage; their full operational validation remains
outside this audit-only plan.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-012 | FR-012-AC-1 | TC-001 | 🚧 Planned |
| FR-012 | FR-012-AC-2 | TC-002 | 🚧 Planned |
| FR-012 | FR-012-AC-3 | TC-003 | 🚧 Planned |
| FR-012 | FR-012-AC-4 | TC-004 | 🚧 Planned |
| FR-012 | FR-012-AC-5 | TC-005 | 🚧 Planned |
| FR-012 | FR-012-AC-6 | TC-006 | 🚧 Planned |
| FR-012 | FR-012-AC-7 | TC-007 | 🚧 Planned |
| FR-012 | FR-012-AC-8 | TC-004, TC-005 | 🚧 Planned |
| FR-012 | FR-012-AC-9 | TC-008 | 🚧 Planned |
| FR-012 | FR-012-AC-10 | TC-009 | 🚧 Planned |
| FR-012 | FR-012-AC-11 | TC-009 | 🚧 Planned |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | Self-contained identity negative controls | Unit | P1 | FR-012-AC-1 | 🚧 Planned |
| TC-002 | Selected invocation packet audit | Integration | P1 | FR-012-AC-2 | 🚧 Planned |
| TC-003 | Selected role and source correspondence audit | Integration | P1 | FR-012-AC-3 | 🚧 Planned |
| TC-004 | Model checkpoint identity and producer pin | Integration | P1 | FR-012-AC-4, FR-012-AC-8 | 🚧 Planned |
| TC-005 | Real native rule syntax outcomes | Integration | P1 | FR-012-AC-5, FR-012-AC-8 | 🚧 Planned |
| TC-006 | Malformed JSON and field types | Property | P1 | FR-012-AC-6 | 🚧 Planned |
| TC-007 | Fixture root containment | Property | P1 | FR-012-AC-7 | 🚧 Planned |
| TC-008 | Audit resource ceilings | Property | P1 | FR-012-AC-9 | 🚧 Planned |
| TC-009 | CLI encoding and producer refusal | E2E | P1 | FR-012-AC-10, FR-012-AC-11, NFR-005-M-2, NFR-005-M-3 | 🚧 Planned |
| TC-010 | Owned verification language inventory | Manual | P1 | NFR-005-M-1 | 🚧 Planned |

## Six coverage rules

All eleven FR-012 ACs and three NFR-005 metric obligations are planned above.
Modes are mutually exclusive, not combinable options. Valid/unknown modes,
missing/extra arguments, contained/foreign paths, zero/exact/over resource
limits, malformed fields and identity changes are covered. No asynchronous
state machine is introduced; the identity registry's first binding/repeated
binding/conflicting binding transitions have explicit controls. The generated
input rows require actual generation and retained failures, not a Property
label on one example. Execution is still pending.

## Integration Test Matrix

IT-004 is a command/file boundary, not a service/browser/event/database system.
TC-002 through TC-005 execute real filesystem/parser integration, and TC-009
executes the compiled binary. The private standard packet remains an explicit
local lane; default CI uses only repository model fixtures and self-contained
negative controls. No substitute network/daemon classification is invented.

## Coverage gaps

The preexisting 21 Rust tests and older FR/NFR obligations still need their own
formal TC/evidence remediation. TC-010 needs recorded inspection, not a source
tag. Fresh external producer qualification remains gated, outside this plan.
