---
id: SR-052
title: "scope-boundary review of the formal linker API"
type: SpecReview
analysis: scope-boundary
scope: "FR-013, formal-environment-linking.md, TC-020–024, TC-030–034 and IT-005 linking seam"
review_set: all
evaluated_revision: "c78792a"
review_date: "2026-09-08"
---

## Summary

A owns native linkage and its specific formal binding profile; IR retains its type and canonicalization authority.

## Verdict

PASS for the concrete linking specification and implementation readiness.
The complete state workflow remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved finding in this scoped API specification. | FR-013; IT-005; TC-030–034 |

## Analysis

Allocation: StR-001 and FR-005/013 belong to A's core compiler semantics; FR-002 to A syntax; FR-010 to A cross-cutting diagnostics; FR-006 remains A's subsequent checker. External IR is consumed through FR-013/019 constructors and canonicalization, with IT-005 integration tests required before guarantees. FR-023 remains the later existing binder. Filament schema generation is not in this resolver. B's method/result contracts and C's integration workspaces remain untouched. No production serde decoder, extra crate, second arithmetic checker, model store or executable binder is introduced. New source keeps AGPL-3.0-only and existing dependency grants remain separate.

## Provenance

The actual QUOIN specify/spec-matrix/spec-review skills and retained base plus
seven analyses govern this review. Agent A performed the selected reviews
serially under the no-subagent assignment. Catalog authoring contracts, actual
advisor output and validation were read. The declined optional gap-analysis
semantic comparison is not included. This is not an independent B/C acceptance.

