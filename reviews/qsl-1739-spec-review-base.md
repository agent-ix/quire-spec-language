---
id: SR-20261105101015
title: "Base specification review — QSL-1739 FR-090 observation contract"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md; FR-255, FR-397, FR-520; quire-exact 0.1.0 package; src/accounting.rs context"
review_set: subset
---

## Summary

Reviewed the revised FR-090 contract against FR-255, FR-397, FR-520, and the locked Meter implementation. The initial granularity finding is resolved by defining one record per admitted charge and denied/partial behavior; the criterion still treats Meter admissions as a complete record source, although the locked Meter has no production log and its test-support log is truncated and omits work/result requests.

## Verdict

**CONDITIONAL** — the record semantics are now ordered, but the AC's complete Meter-record oracle is unsupported by the current Meter interface and test-support records.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-23's cited granularity does not define one ordered operation-record contract | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:642 |

FR-397 specifies charge points and their order for IEEE evaluator nodes, while FR-520 specifies a per-binder charge log and how those totals combine. Neither defines the ordered record shape/granularity for every checked function-body operation. Consequently, two implementations could expose different ledger entries yet both claim to follow the cited requirements, leaving the operation-by-operation oracle underspecified.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | AC-23 relies on a complete Meter admission log that the current Meter does not expose | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:646 |

At the locked `quire-exact` revision, the production `Meter` stores fixed-size counters and an admission count, but no per-charge log (`src/accounting.rs:912-933,1024-1033`). Its test-support log retains only the first 4096 admissions (`src/accounting.rs:40,1029-1040,1258-1263`), and `AdmittedCharge` omits `work_units` and `result_units` requests (`src/accounting.rs:821-847,865-877`). Therefore AC-23's exact all-admissions/per-record request comparison has no complete Meter data source or test oracle for long evaluations or cumulative request amounts. The spec needs to identify the production capture contract and a complete verification source, or scope the equality to a defined supported record set.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The revised FR-090 Description now defines one record per admitted charge in admission order and explicitly defines refused-charge omission and the admitted prefix on partial evaluation. |
| FND-002 | still-open | The new wording continues to require exact equality with all Meter admitted charges, but the locked production Meter has no log; the test-only log is truncated at 4096 and lacks work/result request amounts. |
