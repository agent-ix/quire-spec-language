---
id: SR-7832
title: "QSL-719 evidence review — allInstances incomplete closure"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language@e944de53912ebb41f6d068541520ea7e39180d40; FR-084-AC-2, FR-090-AC-21, TC-227"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
---

## Summary

The declared `Test` verification methods are within the catalog's recommendation for FR-084-AC-2 and FR-090-AC-21. The binders exist on companion #763; the matrix over isolated #767 does not include that companion code and reports both criteria untagged.

## Verdict

**PASS** — no verification-method mismatch; the pending source PR is the implementation evidence provider.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence advice

- FR-084-AC-2: authored `Test`; catalog recommendations include compile-time-check, design-by-contract, and property-based-testing. The authored Test class is represented among the recommendations; no mismatch is reported.
- FR-090-AC-21: authored `Test`; catalog recommends property-based-testing in the Test class; no mismatch is reported.
- `quoin advise --repo . --json` exited 0. Exact companion binders and their trace tags were inspected at `cb74bf33edb7489061148ba6f52e765aba1737c4`; current companion #763 head `2d952659451988b230b05b7d89796ac112e0b5b4` retains them and asserts the additional closure field required by TC-227.
- `quire matrix --scope . --format tsv` exited 0 on exact #767 head `e944de53912ebb41f6d068541520ea7e39180d40`, but reports FR-084-AC-2 and FR-090-AC-21 as `untagged` because the source binders are in the separate, unmerged #763 branch.

