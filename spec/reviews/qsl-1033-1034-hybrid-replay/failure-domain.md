---
id: SR-1301
title: "Failure-domain analysis of hybrid replay settlement"
type: SpecReview
analysis: failure-domain
scope: "FR-450-AC-3/9, FR-290-AC-19, TC-1221"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---

## Summary

Checked malformed provider traces, replay ineligibility, negative replay and resource failure paths. The spec preserves the submitted counterexample on both parity outcomes and rejects malformed or out-of-scope trace arms before terminal record creation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain gap found in the changed scope. Resource exhaustion and malformed trace behavior remain governed by existing FR-450 and FR-250 rules. | FR-450; FR-290 |
