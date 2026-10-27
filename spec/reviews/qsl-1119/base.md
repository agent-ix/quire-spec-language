---
id: SR-60309
title: "Base review of the QSL-1119 weak-access item clarification"
type: SpecReview
analysis: base
scope: "FR-220, FR-224, TC-669 (QSL-1119)"
review_set: subset
---

## Summary

The FR-224 clarification is concrete and consistent with FR-220's per-reachable-state enabled-footprint rule. FR-224-AC-7 has an authored test procedure, but no source test currently carries its criterion tag; QSL-1119 is tasked to add that evidence.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|----|----------|---------|------|--------------|
| FND-001 | medium | FR-224-AC-7 is not yet bound to an implementation test. Add the required `TC-669` trace-tagged tests for the acquire and relaxed RMW variants under both weak models. | FR-224-AC-7, TC-669 | correct-requirement-no-evidence |
