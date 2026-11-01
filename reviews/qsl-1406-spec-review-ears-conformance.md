---
id: SR-202610310143
title: "QSL-1406 EARS conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md"
review_set: subset
---

## Summary

The changed NFR-012 statement was checked with the EARS engine and manually. The engine reported no grammar warning for this file, but the new paragraph contains three separate SHALL obligations in one statement.

## Verdict

**CONDITIONAL** — Split the combined obligations into atomic requirement statements with separately verifiable outcomes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new NFR-012 statement combines three SHALL obligations in one paragraph | spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:23-30 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in fix commit e260c297: linearity, inspection metering and denied-charge behavior now appear as three separate statements. See `after_excerpt` below. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      The unit model binder SHALL keep lookup and selection work in the model-intake
      projection linear in the admitted declaration and field records.

      Before inspecting a package declaration or field record, or a declaration row
      in the normalized view, the unit model binder SHALL charge that inspection to
      the same `ModelNormalizationLimitsV1` work meter used for normalization.

      If a selected-row projection charge is denied, the unit model binder SHALL
      return the existing limit-incomplete result with no partial selected-model
      result.
```
