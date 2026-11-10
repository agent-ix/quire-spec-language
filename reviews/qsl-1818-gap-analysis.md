---
id: SR-20261107070951249607
title: "Gap analysis — QSL-1818 place-reference handling slice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2672 changed files, TC-685, FR-230-AC-24, FR-230-AC-62"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: references
---

## Summary

A manual gap check was limited to PR #2672's changed behavior and its TC-685 context. The new test asserts owner-then-selector traversal, and the existing integration case still verifies an explicit unsupported-feature refusal; the diff does not claim the broader clock-place criteria are complete.

## Verdict

**PASS** — no new test-to-code gap is introduced in the reviewed slice.

## Coverage

- `qsl-semantics/tests/it/clock_places.rs::place_reference_children_visit_owner_then_selectors_in_order` is trace-tagged to TC-685 and asserts the owner and each slot selector in order.
- The changed classification and reference-collection arms align with the `PlaceReference` data shape. Lowering continues to reject it as `UnsupportedFeature` because the downstream term representation is not present.
- FR-230-AC-24 and FR-230-AC-62 remain unmet for the broader reference behavior; the PR and test comment do not claim otherwise.
- The `quoin gap-analysis` command is unavailable in the installed CLI. This was a targeted manual check; no whole-repository matrix was run because the review is scoped to the PR diff. Optional intent↔test↔code semantic review was not requested.
- Plan completion: not assessed.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No gaps found (placeholder) | - |

## Disposition pass 1

Re-checked the fix commit `fa8d2d239` and focused test against FR-230's place-identity encoding requirement. The test asserts lowercase hexadecimal output for the owner and canonical node-key text and verifies ordered slot encoding. There were no actionable findings from the original pass and no new test-to-code gap in this fix. No disposition rows or ticket comment were needed.

## Disposition pass 2

Re-checked fix commit `bb4bb3b72` against the final PR diff and the relevant FR-230 place identity behavior. The focused evaluator test still covers the no-slot form and ordered slot encoding; the source guard covers the display-string restriction that exposed this regression. The new decimal encoder handles zero and repeated base-10 division, and out-of-range or negative selectors return an internal invariant fault. No prior actionable findings or new test-to-code gaps were found. No disposition rows or ticket comment were needed.
