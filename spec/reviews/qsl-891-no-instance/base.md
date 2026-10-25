---
id: SR-17204
title: "QSL-891 NoInstance base specification review"
type: SpecReview
analysis: base
scope: "FR-169-AC-16, TC-594 step 6, qsl-analyze state-graph instance results, qsl-replay state-graph settlement"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---

## Summary

The scoped FR-169 NoInstance requirement is specific, linked to TC-594, and has a tagged executable test. No broader repository matrix was run; the review checks the criterion and touched files only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Checklist and coverage

FR-169-AC-16 has one unambiguous ID, names the no-instance outcome, terminal disposition, and never-proved invariant. TC-594 step 6 covers both empty and admitted instancing; `a_claim_with_no_instances_settles_no_instance_without_exploration` carries `FR-169-AC-16` and exercises `check_state_graph` plus `settle_state_graph`. The six coverage checks were applied to this changed criterion: exact identity, test coverage, failure outcomes, boundary cases, responsibility, and testability. No other criterion or source surface was included.

The no-instance marker is distinguished from `NoInitialState`; the requirement states one item record with no evidence, and the test checks the value, label, basis, category, method metadata, and absence of replay.

## Verdict and provenance

PASS for the scoped FR-169-AC-16 change. No applicable AssuranceProfile was found. Matrix reconciliation was limited to the changed criterion and its tagged test; no repository-wide matrix was computed.
