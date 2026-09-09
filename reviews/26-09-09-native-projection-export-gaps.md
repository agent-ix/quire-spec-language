---
id: SR-204
title: "Standalone projection export delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-009-native-workflow; Task-028; FR-029; TM-007"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-007
    type: references
---
## Summary

Task-021–028 are done; standalone Boolean projection export is implemented at
4f2c20f. Broader LC05 producer adoption and deferred assurance remain open.

## Verdict

**CONDITIONAL** — this engineering slice is delivered; broader integration continues.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Actual Quire extraction adoption, wider backend support and deferred activation assurance remain separate work. | Plan-009; IT-003; IT-008 |

## Coverage

Quire reports FR-029 at 3/3 criteria, TM-007 at 13/13 test cases and the global
rollup at 293/301. TC-107 carries real trace attributes on all three tests.
No scoped unbacked row, stub or unowned behavior was found. FR-027 owns shared
source-only intake, FR-009 owns projection semantics, and FR-029 owns export.
Statuses were checked against actual runs because of the known catalog column
mismatch. Existing global metric/integration gaps remain open. Optional semantic
gap review was declined and skipped.
