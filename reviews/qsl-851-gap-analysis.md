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
