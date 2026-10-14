---
id: SR-9100
title: "Spec review (base) of quire-spec-language PR #838: FR-056 and FR-255 merge resolutions"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ac65a4a03235a9e04df46b221fecff3c3082ef93; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/tests.md"
review_set: subset
---
# Spec review (base) of quire-spec-language PR #838

## Summary

Ticket: QSL-59. Base spec review of the spec conflict resolutions in merge
2f2b02d50, plus the spec files both sides changed and git merged
automatically. The ref's own spec content was not re-reviewed.

Examined:
- FR-056: the ref's AC-23 (K2 native String retention) keeps its id, with
  verification `Test`. AC-24 to AC-33 are the ref's. main's #773 AC-23
  (complete-V1 String/UUID typing) is now AC-34, with its text unchanged. All
  34 ids are unique. TC-992's row in spec/tests.md points at AC-34, which
  TC-992's procedure exercises (the TC-235 String/UUID typing) (examined)
- FR-255 Behavior 1: the ref's carrying-type table decides the code
  (`stage_limit_exceeded` for a stage's or reader's own limits type;
  `resource_exhausted` for caller budgets, format, intake and library;
  `Outcome::Bounded` for `explore.*`). AC-1 now defers to that table and
  agrees with it. main's AC-3 adds the accounting limits to the name-mapping
  union. The Accounting setting names section, kept from main, says
  Behaviors 1 to 4 do not apply to accounting and an accounting budget settles
  `Incomplete`. Behavior 1 covers only stage and operation limits, so it does
  not contradict AC-1 or AC-3. The Description keeps main's sentence that
  each accounting budget has a setting name (examined)
- FR-100: AC-3 and AC-6 take main's #665 `call.accounting` wording, and AC-9
  takes the ref's `TypeMismatch`/`ill_typed`/exit 20 example (the merge base
  had `AncestorSteps`, so main never changed AC-9). There are no duplicate ids (examined)
- FR-090 (merged automatically): main's #763/#767 rows and the ref's reworded
  `field-slot-unresolved` and `field-option-payload-rejected` rows sit side by
  side with no duplicate or conflicting cause (examined)
- spec/tests.md: no duplicate TC rows. TC-457, TC-993, TC-985 and TC-992 rows
  merged cleanly (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Both spec resolutions are internally consistent, and the AC
renumbering is traced correctly.

Recorded, not reported: low, FR-255-AC-3 requires a builder method for each
accounting field, while the accounting section says Behaviors 1 to 4 (which
include Behavior 4, the builder) do not apply to accounting. This tension
already existed on main (#665) and the merge did not introduce it.
