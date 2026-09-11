---
id: SR-285
title: "Gap analysis after the runtime input schema"
type: SpecReview
analysis: gap-analysis
scope: "Plan-009 native workflow; TM-007; Task-036 schema behavior at f4679ef"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-007
    type: references
---
## Summary

All thirteen enumerated Plan-009 tasks are done. Task-036 adds the schema and
four traced Rust tests; its implementation and trace checks pass. Wider LC05
adoption and native-profile reconciliation remain separately open.

## Verdict

**CONDITIONAL** — implementation is ready, with the installed tooling limitation
below retained separately from successful checks on the selected candidate stack.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Matrix status checks require the exact selected CLI/module stack; ordinary installed tooling still has the documented status-column mismatch until #28 adoption. | docs/matrix-status.md; compiler #28 |

## Coverage

Reconciliation used `quire coverage --scope . --json` with both explicit module
roots from docs/matrix-status.md. Clean CLI ff638b9 / engine d3bc2ba and clean
process e6ea515 / ISO a60ee12 revisions were verified before the run. Results:
FR-024 5/5 criteria, TM-007 16/16 test cases, overall 326/330 targets; zero status
lies and zero status-column diagnostics. No new unmatched or untracked schema
test tags appear. Seventeen unrelated declaration/vocabulary diagnostics remain.
The four unbacked targets comprise Manual TC-010, Inspection FR-017-AC-2 and two
StR-001 demonstration criteria; this is not four missing automated tests.

Task inventory: 13/13 done, no unfinished declared prerequisite or contradictory
checkbox. Plan-009 remains active for its stated wider adoption/assurance scope.
Changed-behavior inventory: envelope/value shape, scalar domains, null/vector
preservation and separation from reader admission; all map to FR-024-AC-5.
Untraced changed behaviors: 0; source/test stubs in this change: 0. Existing
production reader and runtime tests still exercise real execution. SR-284
records 357/341 ordinary tests plus three compile-fail doctests per configuration.
Optional semantic gap review was skipped as requested. No plan, code or matrix
was modified by this gap-analysis pass. Neither #27 nor #30 is closed by it.
