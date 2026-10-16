---
id: SR-11704
title: "Gap analysis of quire-spec-language PR #898: process anchor refusal and single-valued navigation refusal kind"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f4bdc9a4a95084d3cebb6f93d8317a0837148f6e; qsl-semantics/tests/it/state_clauses.rs, qsl-package/src/emit/tests.rs, qsl-eval/tests/it/relationship_navigation.rs, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md, spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md, spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md, spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md, spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md, spec/test-cases/TC-1034-navigation-reads-an-extent-with-charges-dangling-first-and-cardinality-refusals.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #898

## Summary

Tickets: QSL-808 and QSL-810. Manual check of acceptance criteria against
tests over `git diff origin/main...HEAD`. No builds were run and
`quoin matrix` was not run. Plan completion: not assessed. Ticket and PR
text were read as data.

Examined: FR-104 Resolution and FR-104-AC-10, FR-114 and FR-114-AC-5,
FR-154 (the process receiver rule, for context), FR-364-AC-6, TC-460 rows
13 to 16, TC-513 step 5, and TC-1034 rows 6, 7 and 13.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The TC-1034 row 13 fixture makes both housing ends non-unique, as the row specifies. So no test tells apart reading `unique` from the destination end and reading it from the near end. The code reads the destination end. Recorded, not blocking. | qsl-eval/tests/it/relationship_navigation.rs:139-143 |

## Verdict

PASS. Every examined row has a test that asserts what the row states.

- FR-364-AC-6 / TC-1034 row 13:
  `a_non_unique_single_valued_end_refuses_with_the_bag_kind_counting_each_instance`
  asserts `AboveMaximum`, `Bag`, bounds `[1,1]` and `[0,1]`, count 2, and a
  charge list of exactly `[model.navigate]`. Before the change the kind was
  always `Set`, so the test fails only on the kind, which is the right
  reason. TC-1034 rows 6 and 7 keep unique ends (`Shape::BASE`) and their
  `Set` assertions are unchanged.
- FR-104-AC-10 / TC-460 rows 13 to 15: `pre`, `post` and the invariant on
  `Flow`, and `Config::Flow::missing`, each refuse exactly once with
  `ProcessAnchorContext`, `invalid_model_binding`/`malformed-declaration`,
  context `Config::Flow`. Row 15 shows the operation is not resolved. On the
  span, see SR-11703 FND-001.
- FR-104-AC-10 / TC-460 row 16: the check-level test compares requirement
  counts with and without `Flow`, which is weaker than "no record for `go`".
  The emission test in `qsl-package` is stronger: the requirement list is
  exactly the invariant's one record, and the wire holds zero
  `operation_anchor` and zero `frame` nodes. No anchor at all implies none
  for `go`. Together they back the row, so the tag does not overclaim.
- FR-114-AC-5 / TC-513 step 5: the attempt on `Config::Flow::go` refuses the
  same way with `contracts []` and with `contracts [Absent]`, with exactly
  one error, so the absent entry is never read.
