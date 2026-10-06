---
id: SR-1340
title: "Gap analysis of quire-spec-language PR #640: post-state range witness trace (QSL-634)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6a09ff93786ec1607cd08af6ccc79e54cf991c81; PR #640 diff against origin/main; FR-106-AC-12, FR-122-AC-7; trace tags TC-465/FR-106-AC-12 (qsl-semantics/src/model/observation/document.rs tests) and TC-517/FR-122-AC-7 (qsl-replay/src/spine/clause/tests/state_clause_replay.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
---
# Gap analysis of quire-spec-language PR #640

## Summary

Ticket: QSL-634. Reviewed head 6a09ff93. No build was run.

FR-122-AC-7 is fully backed by four TC-517 tests: the wrapping debit (-1,
evaluated `false`, names child/versionNumber/[0,1000]/-1), the 1001
overflow the clause does not read (evaluated `true`, still reproduced), the
out-of-range pre-state refusal (invalid_runtime_input/invalid-value naming
child and versionNumber), and the in-range path with no range violation. The
traces are correct.

FR-106-AC-12 is backed by two unit tests over `admit_scalar`. They check the
sink/no-sink behaviour and exact admission (no clamping), but not the
argument path the AC names (FND-001) or the object/field reporting the AC
claims (FND-002). No test shows `run_clause` or a frame run still refusing
an out-of-range post value (FND-003).

## Verdict

Changes requested on coverage: FND-001 (medium) and FND-003 (medium) are real
gaps. The coder's reason for the substitution (no fixture has an Int-ranged
parameter) does not hold: FR-106-AC-8's own test builds an operation
variant (`probeSub(target: Sub)`) inline, so an `Int[0, 1000]` parameter
can be built the same way.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106-AC-12's argument case is not tested. Both unit tests call `admit_scalar` directly with `None`, so they would still pass if `admit_parameters` started passing a witness sink. That is the regression the AC exists to catch. Fix: admit an invocation or `PreCall` whose operation declares an `Int[0, 1000]` parameter with value `-1`, through `admit_parameters` (or full admission), and assert `invalid_runtime_input`/`invalid-value` naming the parameter. Do it under `PostStateRange::Witness` too, to show witnessing never reaches arguments. | qsl-semantics/src/model/observation/document.rs:1912-1937 |
| FND-002 | low | FR-106-AC-12 says each witnessed value is reported "with its declared range [0, 1000], its object and its field". The TC-465 unit test checks only the range and the value, because `admit_scalar` has no object or field. Object and field are covered only through FR-122-AC-7's replay tests. Either test the object and field through `admit_population_values` under TC-465, or drop them from AC-12 and leave them to FR-122-AC-7. | qsl-semantics/src/model/observation/document.rs:1939-1968 |
| FND-003 | medium | No test pins the `Refuse` default for the post snapshot. Nothing asserts that `run_clause` (default limits) or a frame run (`post_self` false) over the wrapping-debit invocation still refuses `invalid_runtime_input`/`invalid-value`. FR-106 check 6.5 now states both ("The default is `Refuse`, and a `Frame` run's post snapshot always refuses"), but no AC row or test backs them. Add both to FR-106-AC-12 and test them. | qsl-semantics/src/model/observation.rs:1338-1351 |

## Dispositions

Round 1, reviewed at b2c7911504123cbd65bb139209b9736f9cddf675 (fix commits 9869ac332, 78edbf4b3, b2c791150, rebased onto origin/main; compared by content).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 78edbf4b3 |
| FND-002 | fixed | 78edbf4b3 |
| FND-003 | fixed | 9869ac332 |
