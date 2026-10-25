---
id: SR-17207
title: "QSL-891 NoInstance evidence review"
type: SpecReview
analysis: evidence
scope: "FR-169-AC-16, TC-594 step 6, qsl-analyze/tests/it/state_graph_settle.rs"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---

## Summary

The new test verifies the concrete NoInstance result and its terminal settlement without a synthetic replay result. Its trace tag binds it to FR-169-AC-16.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence analysis

`a_claim_with_no_instances_settles_no_instance_without_exploration` builds an actual checked claim and subject with an empty universe, runs the state-graph engine, and sends its marker to the production settlement function. The assertion checks `Inconclusive(NoInstance)`, label, basis, category, execution, method metadata, and zero replay calls. The test is deterministic and carries both `TC-594` and `FR-169-AC-16`.

## Verdict

PASS for the scoped evidence obligation. The full Test Matrix was intentionally not run because the requested review scope is the touched requirement and files.
