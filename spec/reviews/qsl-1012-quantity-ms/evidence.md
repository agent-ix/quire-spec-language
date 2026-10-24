---
id: SR-16627
title: "QSL-1012 evidence review"
type: SpecReview
analysis: evidence
scope: "FR-120-AC-16 and FR-189-AC-15 tests and source"
review_set: all
---

## Summary

The scoped matrix binds FR-120-AC-16 to `a_quantity_encodes_its_rational_magnitude_and_unit` and FR-189-AC-15 to the exact quantity integration test and the mismatched-support refusal test. The assertions check the serialized rational magnitude and unit digest, equivalent exact spellings, normalized threshold, measured quantile and the pre-sampling refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The selected obligations have direct tests that exercise the production encoder, assembler and analysis path. | FR-120-AC-16; FR-189-AC-15 |

## Matrix evidence

`quire matrix --scope /tmp/qsl-1012-scope --format json` reports the two selected criteria as `tagged`; the filtered output names the qsl-eval encoder test and both qsl-analyze integration tests. The scope contains only the two linked requirements, their TC documents and the touched test sources.
