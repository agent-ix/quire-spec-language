---
id: SR-16366
title: "STD-392 failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "FR-187, FR-189 and TC-624 quantity-measure path"
review_set: all
---

## Summary

The quantity path has an explicit trust boundary, unit identity, exact-value rule and bounded quantile ordering. The sampled callback's generator mismatch and undefined outcomes already have defined dispositions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found in the changed scope. | - |

## Checks

- **Extension point:** `BehaviourSampler` failures are refused as `GeneratorMismatch`; sample evaluation that is undefined ends the test with its recorded locus and cause.
- **Identity:** the checked reward type `Quantity<u>` identifies the unit. EN-4 keeps its exact magnitude in that unit; it does not infer a unit from the numeric value.
- **Purity:** FR-120 keeps described effects as data; the added requirement changes no callback or side-effect behavior.
- **Termination:** each quantile is selected from a bounded counted sample set; finite values sort before `Censored`.
