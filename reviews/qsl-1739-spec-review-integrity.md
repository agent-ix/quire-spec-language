---
id: SR-20261105101017
title: "Integrity review — QSL-1739 FR-090 observation contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md; FR-255, FR-397, FR-520; quire-exact 0.1.0 package; src/accounting.rs context"
review_set: subset
---

## Summary

Reviewed the revised FR-090 contract for consistency, traceability, and testability against FR-255, FR-397, FR-520, and the locked Meter implementation. The initial granularity finding is resolved by defining one record per admitted charge and denied/partial behavior; a new testability gap remains because the Meter exposes no complete production charge log and its test-support log is bounded and omits cumulative request amounts.

## Verdict

**CONDITIONAL** — the revised charge ordering is clear, but AC-23 still lacks a complete Meter record source for its equality oracle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-23 does not identify a single authoritative ordered operation-record granularity | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:642 |

FR-397 constrains ordered charge points for IEEE nodes; FR-520 constrains per-binder attribution and aggregation. The AC's phrase “FR-397/FR-520 granularity” does not say how those record types compose for an entire function-body evaluation, so the same run can yield multiple plausible ledgers and still appear conformant.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | AC-23 relies on a complete Meter admission log that the current Meter does not expose | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:646 |

At the locked `quire-exact` revision, production `Meter` stores fixed-size counters and an admission count but no per-charge log (`src/accounting.rs:912-933,1024-1033`). Its test-support `admitted_charges` log is capped at 4096 (`src/accounting.rs:40,1029-1040,1258-1263`), and `AdmittedCharge` omits `work_units` and `result_units` request amounts (`src/accounting.rs:821-847,865-877`). Thus AC-23's exact ledger equality lacks a complete event source and cannot be checked against the Meter for all admissions and all ten request amounts.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The revised FR-090 Description now defines one record per admitted charge in admission order and explicitly defines refused-charge omission and the admitted prefix on partial evaluation. |
| FND-002 | still-open | AC-23 still compares against all Meter admitted charges, but the locked production Meter has no log and its test-only log is truncated and lacks work/result request amounts. |
