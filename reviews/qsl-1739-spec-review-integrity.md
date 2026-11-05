---
id: SR-20261105101017
title: "Integrity review — QSL-1739 FR-090 observation contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md; FR-255, FR-397, FR-520 context"
review_set: subset
---

## Summary

Reviewed FR-090's edited description and AC-23 for consistency, traceability, and testability against the existing accounting contracts. The new criterion is externally observable and testable, but its operation-ledger oracle inherits an ambiguity from the cited granularity references.

## Verdict

**CONDITIONAL** — resolve the operation-record granularity before treating AC-23 as an implementation-ready oracle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-23 does not identify a single authoritative ordered operation-record granularity | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:642 |

FR-397 constrains ordered charge points for IEEE nodes; FR-520 constrains per-binder attribution and aggregation. The AC's phrase “FR-397/FR-520 granularity” does not say how those record types compose for an entire function-body evaluation, so the same run can yield multiple plausible ledgers and still appear conformant.
