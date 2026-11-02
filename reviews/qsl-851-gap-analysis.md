---
id: SR-202610310158
title: "Gap analysis — FR-383 protocol recursion groups"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-383-AC-24/25; qsl-package/src/emit/tests/protocol_recursion.rs; qsl-semantics/src/check/lowering/protocol.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

## Summary

Planless gap analysis for the QSL-851 diff found static test bindings for both FR-383-AC-24 and AC-25 and no new unowned production behavior in the changed paths. The AC-25 test's zero and one-unit-short cases accept any error and do not verify the required typed `s3.work_units` limit refusal.

## Verdict

**CONDITIONAL** — one explicit refusal obligation is not asserted by the new test vectors.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The zero and one-unit-short budget cases assert only `is_err()`, so they do not prove exhaustion returns the typed work-budget refusal naming `s3.work_units`. | qsl-package/src/emit/tests/protocol_recursion.rs:367-369 | correct-requirement-no-evidence |

## Coverage

- FR-383-AC-24 is statically tagged by `a_cyclic_predicate_join_protocol_is_grouped_and_reads_back`; it checks the shared group, in-process coordinates, acyclic identity, emitted package, and I2 `Verified` read-back.
- FR-383-AC-25 is statically tagged by `mixed_protocol_cycle_work_is_linear_and_uses_the_exact_budget` and `a_100000_member_function_protocol_cycle_is_budgeted_and_order_invariant`; the latter emits and reads back the 100,000-control mixed cycle under a caller work budget. The source SCC traversal is iterative with an explicit work stack and charges vertices/edges; grouping is keyed once from the completed component.
- `quoin matrix --repo . --json` completed successfully. Both criteria are statically `tagged`; the evidence axis reports `no run evidence` for both. This is distinct from the focused local run in this review and is not a static-coverage gap.
- Changed production code maps to FR-383's cyclic-component behavior. No stub or unrelated unowned behavior was identified in the changed surface.
- Optional semantic review was skipped; no plan was supplied. Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | The pre-fix test already required exactly one refusal and asserted `StageLimitExceeded`, `ResourceExhausted`, `WorkBudget`, and `S3WorkUnits` for both zero and one-unit-short bounds (parent of `7c98b4f2`, lines 428-448). The finding's claim that these cases checked only `is_err()` was incorrect. The fix changes the cardinality selection to find the typed refusal among multiple refusals; it does not newly add the typed assertions. |

### Round 2 recheck

At `33fdc7d7695bdac16640875672772a7581930ac2` on `origin/main` `ce9920e5cba44d6a9fc565ff9c0d97b144601cfb`, FND-001's prior `rejected` disposition remains valid. The current zero and one-unit-short cases still require `StageLimitExceeded`, `ResourceExhausted`, `WorkBudget`, `S3WorkUnits`, and the exact bound; exact measured work succeeds. The focused boundary test passes. No new findings were found. Per the reviewer disposition rule, no duplicate outcome row is added for a terminal prior finding.
