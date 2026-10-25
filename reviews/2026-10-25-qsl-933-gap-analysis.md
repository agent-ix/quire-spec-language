---
id: SR-17205
title: "Scoped gap analysis — QSL-933 RA visibility fairness"
type: SpecReview
analysis: gap-analysis
scope: "QSL-933; FR-228-AC-1; FR-227-AC-4; qsl-eval/src/simulation/protocol/fairness.rs; qsl-analyze/tests/it/tso_fairness.rs"
review_set: subset
---

## Summary

The touched-scope matrix and direct source review found runnable tests for both assigned criteria. The RA replay test drives production protocol expansion, resolves stale and mo-last loads into one visibility pair, and confirms replay refuses the unfair lasso.

## Verdict

**PASS** — both in-scope criteria are tagged by tests that exercise the changed behavior, with no reverse requirement gap in this diff.

## Coverage

- FR-228-AC-1: tagged by the RA member wire-form test, the TSO/SC fairness-set test, and the RA lasso replay test.
- FR-227-AC-4: tagged by the RA replay test.
- Matrix input was restricted to FR-228, FR-227, and the touched test files; unrelated criteria were not assessed.
- No `AssuranceProfile` applies in `spec/`.
- Plan completion: not assessed.

## Findings

| ID      | Severity | Summary                      | Refs |
| ------- | -------- | ---------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)    | -    |
