---
id: SR-164203
title: "Evidence review — QSL-1642 timed population scope"
type: SpecReview
analysis: evidence
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md#FR-230-AC-58; spec/test-cases/TC-1334-refuse-named-populations-in-timed-model-predicates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

FR-230-AC-58 uses `Test`, with TC-1334 specifying negative timed-predicate cases and positive controls. `quoin advise --json` produced no output for five minutes and was interrupted, so the deterministic catalog recommendation could not be confirmed.

## Verdict

**CONDITIONAL** — the authored verification method matches the obligation by manual inspection, but automated evidence-method advice remains unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Re-run `quoin advise` when it can complete and confirm the catalog's method recommendation for FR-230-AC-58. | FR-230-AC-58 |

## Manual Check

The criterion asserts compilation refusals and successful controls, so a focused integration test is the direct evidence. TC-1334 describes those cases. The implementation must tag the source test with FR-230-AC-58; no test run is claimed at the specification stage.
