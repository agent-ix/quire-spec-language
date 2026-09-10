---
id: SR-249
title: "evidence review of state-scalar projection"
type: SpecReview
analysis: evidence
scope: "spec/functional/FR-034-project-state-scalars.md; TC-112; TM-006"
review_set: all
---
## Summary

Reviewed implementation/spec revision `b789eed`
at PR readiness, using the owner's selected all-set. No applicable AssuranceProfile.

The previous `quoin advise --json` attempt failed to detect the installed Quire 0.31.0 version, so no deterministic recommendation was produced. Inspected the installed catalog's integration-testing, negative-abuse-testing and inspection methods. By author judgment, AC-1/2/3/5 use Rust integration tests across real native/IR/command boundaries; AC-4 uses adverse/boundary cases in the same harness. The authored Test cells reflect that choice. No property, fuzz, mutation or generated-numeric evidence is claimed.

TC-112 now requires the two field reads and the two named direct inputs before
checking their provenance; empty filtered populations cannot satisfy the test.
Full feature-lane execution evidence is recorded in SR-253.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor unavailable; the five method selections are recorded author judgment, with runnable integration/adverse evidence. | FR-034-AC-1; FR-034-AC-2; FR-034-AC-3; FR-034-AC-4; FR-034-AC-5 |
