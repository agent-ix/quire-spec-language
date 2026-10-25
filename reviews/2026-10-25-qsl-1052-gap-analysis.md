---
id: SR-60015
title: "Scoped gap analysis of QSL-1052 stale-object attribute faults"
type: SpecReview
analysis: gap-analysis
scope: "QSL-1052; FR-090-AC-17/22, TC-917, and qsl-eval/tests/it/inherited_attributes.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
---

## Summary

The scoped change links the stale-object fault requirement to TC-917 and to a traced integration test on the public evaluator seam. The `quoin gap-analysis` command is not available in the installed Quoin release; this artifact records a direct scoped comparison and does not claim a full repository audit.

## Verdict

**CONDITIONAL** — the changed requirement/test/code path is covered; the installed gap-analysis command and a repository-wide audit are unavailable in this lane.

## Coverage

- FR-090-AC-22 → TC-917 → `an_attribute_read_of_an_object_absent_from_the_current_state_is_a_fault` (`#[trace("TC-917", "FR-090-AC-17", "FR-090-AC-22")]`). The temporary scoped matrix reports both criteria as `tagged`.
- The test exercises real object environments and public evaluation; it distinguishes current-state absence from replay's unresolved-reference mode.
- No test stub or code path outside the existing attribute fault producer is involved.
- Plan completion: not assessed.
- Optional semantic review was not requested; no repository-wide reverse-gap or stub audit is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found in the scoped QSL-1052 requirement, test, and implementation chain (placeholder). | FR-090-AC-17, FR-090-AC-22, TC-917 |
