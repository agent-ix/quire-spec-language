---
id: SR-16358
title: "Code review of QSL-1013 source timed declarations slice"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-forms/tests/it/time_forms.rs; qsl-semantics/src/check/, qsl-semantics/tests/it/timed_declarations.rs; qsl-package/src/emit.rs and tests; FR-230-AC-1..5, AC-7..8"
review_set: subset
---

## Summary

Reviewed PR #1403 at fba5bbcf1 against its stacked base, including the timed source checker, operation and unit resolution, located refusals, S4 emission guard, and focused tests. The PR claims only the implemented prerequisite slice and records the upstream Clock and package-unit failures.

## Verdict

**PASS for the implemented slice** — no major code defect found. FR-230 AC-1 through AC-5 remain open as the PR states.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence

The focused timed declarations suite passed in the review worktree: 24 passed, 2 ignored. The tests cover source dense and tick declarations, exact positive and invalid periods, operation identity and inherited lookup, source Time units, and duplicate time sources. The package test covers the emission refusal. Two Clock and package-unit integration tests are deliberately ignored and independently documented as failing at FCD I1. The reviewer did not run an aggregate gate; the code lead owns that gate after the stacked base settles.

## Disposition

No findings require a code change in this slice. FR-230 AC-1 through AC-5 remain open for the dependencies recorded in PR #1403.
