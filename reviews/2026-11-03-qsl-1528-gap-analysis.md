---
id: SR-2026110300002
title: "Gap analysis — QSL-1528 number module move"
type: SpecReview
analysis: gap-analysis
scope: "spec/, src/, tests/; detailed reverse-gap inspection of the QSL-1528 diff"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: references
---

## Summary

Planless repository audit using the computed matrix and a reverse-gap check of the QSL-1528 diff. The four FR-511 criteria are tagged; this repository still has broad pre-existing trace gaps, while the changed codec surface maps to FR-511.

## Verdict

**FAIL** — the repository matrix reports 1,204 untagged criteria and 8 criteria covered only by ignored tests; those gaps are outside this ticket's diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository matrix reports 1,204 untagged criteria and 8 ignored-only criteria. FR-511-AC-1..4 are tagged by live tests. | 1,204 untagged criteria; 8 ignored-only criteria |

## Coverage

- Reconciliation: quoin matrix (quoin 0.29.1, quire 0.36.2).
- Plan completion: not assessed.
- Criteria: tagged 2,742 · untagged 1,204 · tagged-by-ignored-test 8 · method-without-symbol 94.
- Evidence: no run evidence for all 4,048 criteria; the evidence store has no bindings.
- QSL-1528 criteria: FR-511-AC-1..4 are tagged; the focused 10-test protocol-number filter passed, but the matrix evidence store has no bound run evidence.
- Reverse-gap inspection: the changed codec API and preserved numeric behavior map to FR-511; 0 untraced behaviors, 0 source stubs, and 0 test stubs were found in the QSL-1528 diff.
- Semantic review: skipped (not requested).
