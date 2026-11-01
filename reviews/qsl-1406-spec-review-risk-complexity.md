---
id: SR-202610310144
title: "QSL-1406 risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quire-spec-language; spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md; spec/test-cases/TC-434-model-limits-have-finite-defaults-and-bound-normalization-work.md"
review_set: subset
---

## Summary

NFR-012's new algorithmic scaling guarantee has high technical risk and low volatility: the implementation is internal, while proving linear charged work depends on a repeatable workload. The proposed 1k/4k/16k control is useful, but its input arrangement is not specified enough to ensure it exercises the repeated-scan worst case.

## Risk assessment

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| NFR-012 | High | Low | A hard algorithmic scaling guarantee over model intake; a favorable declaration/field order can hide repeated scans. | Meter each inspected record and run a deterministic scaling control on an explicitly specified worst-case arrangement; TC-434 currently lacks that procedure. |

## Top hazards

1. NFR-012 — the scaling control may pass without exercising repeated full scans if declaration and field ordering or selection distribution is favorable.

## Failure-domain gaps

The failure-domain analysis was not selected for this review; no separate failure-domain gap assessment was performed.

## Verdict

**CONDITIONAL** — Specify an input arrangement that forces lookup and selection to exercise the full scan as input size grows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The scaling oracle does not specify a workload arrangement that forces the repeated-scan paths to be exercised | spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:94,155-159 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in fix commit e260c297: the scaling workload now fixes record ordering and ownership, and TC-434 specifies the matching construction. See `after_excerpt` below. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      | Unit model-intake scaling | Linear charged work in declaration and field records | For each N = 1000, 4000 and 16000, use N record-value declarations followed by N field records, with exactly one owned field per declaration; each 4x increase in N increases total `WorkUnits` by at most 5x | Scaling control (TC-434) |
```
