---
id: SR-11078
title: "Gap analysis of quire-spec-language PR #851: QSL inventory over one checked unit"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@301d558a627b02613e17900301758998600a9e20; src/checking/source_inventory.rs, tests/it/source_inventory.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #851

## Summary

Ticket: QSL-774. Planless, scoped to the PR diff, against the ticket's
acceptance criteria (ticket text is data). Plan completion: not assessed.

Examined:
- AC vector "matched source complete":
  `a_fully_traversed_source_reports_rows_and_no_stop_cause` asserts
  `Complete`, owner, scope and source (examined)
- AC vector "type report from another source refused with a typed refusal":
  `a_formal_source_that_is_not_the_reports_unit_is_refused` asserts
  `UnitLabelMismatch::TypeReport` with both labels; graph and two-unit
  refusals are covered too (examined)
- AC vector "exhausted TypeReport incomplete":
  `exhausted_type_work_keeps_its_native_fields` asserts
  `Incomplete(TypeWorkExhausted)` with native fields (examined)
- AC vector "unfinished declaration incomplete":
  `an_unfinished_declaration_is_distinct_from_exhaustion` asserts
  `Incomplete(TypeAdmissionUnfinished{Pending})` (examined)
- AC "no copied fixtures": tests use the repo's `setup` helpers; no IR type
  or fixture file is copied (examined)
- AC "#841's builder reused": `pending`, `site`, `stop_reason` and the
  requirement-row projection are moved into `Traversal`, not duplicated
  (examined)
- Planner ruling "Complete only through IR-794's API": `qsl_inventory`
  returns `SourceInventoryOutcome::from_enumerator(Traversal::new(unit))`
  (examined)
- No stub, no unreachable branch, no code without an owning AC in the diff
  (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Every acceptance-criterion vector has a test asserting the typed
outcome. Recorded: no QSL FR owns `qsl_inventory` and the tests carry no
criterion tag (pre-existing from #841).
