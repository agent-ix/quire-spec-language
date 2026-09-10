---
id: SR-254
title: "Plan-008 gaps after state-scalar delivery"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-008-native-lowering/; spec/native-lowering/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-008
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-006
    type: references
---
## Summary

Inspected Plan-008 and TM-006 with implementation `b789eed` and the PR readiness
task update. Task-033 is delivered; complete Plan-008 acceptance remains open.

## Verdict

FAIL for complete Plan-008 acceptance because Task-020 is incomplete. The owner's
explicit engineering-delivery direction permits this scoped PR while deferred
assurance remains visible.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Generated activation qualification remains incomplete; tagged TC-094 does not establish a passing run. | Task-020; FR-009-AC-5; IT-008-SC-04 |
| FND-002 | medium | Compiler issue #28 owns the status-column mismatch; authored Tested marks are not engine-verified. | TM-006 |

## Coverage

Reconciliation: actual `quire coverage --scope . --json`, Quire 0.31.0; no grep
fallback. Tasks done: 3/4. FR-034: 5/5 criteria backed; TM-006: 5/5 test-case rows
backed; global trace rollup: 325/329. These are trace counts, not proof or project
completion percentages. No unmatched tags or untracked test symbols occur in
the new test file. TC-112's five tests require actual provenance populations and pass; TC-094
retains its deferred marker. SR-253 records the current full feature lanes.

Reverse discovery inventoried six changed behaviors: target/command selection,
field/pre lowering, alias/read correspondence, exact-context selection,
primitive/provenance materialization and bounded/cancelled failure. All belong
to FR-034. No unowned behavior or source/test stub was found in this slice.
Optional semantic gap review was skipped as requested. Numeric backend and
graph parity remain wider LC04 work, with no new claim of their completion.
