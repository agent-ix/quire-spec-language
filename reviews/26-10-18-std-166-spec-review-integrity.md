---
id: SR-12885
title: "Integrity review of STD-166 relation and hyper NotEmitted wording (PR #980)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #980 (git diff origin/main...HEAD at d5be6676f); FR-362, TC-1026, FR-171, ADR-033 and the statements elsewhere that they must agree with (FR-172, FR-173, FR-361, TC-596..599, TC-1015)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-033
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: reviews
---

## Summary

Ticket: STD-166. Consistency review of the edit against the rest of the
corpus and against PR #971's behaviour. Examined: FR-362 items 8 and 9 and
AC-6, ADR-033 Decision and Consequences, FR-171 Description, FR-172
Description (execution bindings), FR-361-AC-4 and TC-1015, TC-596..599 and
FR-173. TC-596..599 and FR-173 are consistent with #971; FR-172 apart from the
point in FND-001 and its deferred Outputs note (#971 FND-007) is consistent.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | "The `relation` and `hyper` clauses have moved" is broader than #971. #971 refuses a `hyper` clause over a finite trace domain with `HyperCause::FiniteDomain` ("has no S3 check here", `unsupported_construct`) and a `relation` whose binding is not a model operation with `ExecutionNotAnOperation` (`unsupported_construct`). FR-171 lines 24-25 and FR-172 lines 141-143 still say those keep QSpec FR-191's finite reading. Failure: a reader of item 8 or the ADR expects the deleted test's own fixture (`hyper Secret ... over traces in M::Execution bounded 2`) to check and refuse NotEmitted; it refuses `unsupported_construct`. Scope the moved statement to `hyper` over `behaviours` and `relation` over model operations, and state that the finite-domain hyper and other-name relation are refused at S3 (and who owns them), or fix FR-171/FR-172 to match. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:65-68; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:224-227; spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md:24-25; spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:141-143 | wrong-requirement |
| FND-002 | medium | With relation and hyper moved, no unmoved family is left: the removed ADR text named them as "the unmoved constructs", and ADR-033 Context (lines 103-110) says the composed syntax never had them. So item 8's last sentence is vacuous and item 9's and AC-6's "after the last family has moved" condition is now met, yet the composed lane remains on main (`src/linking/composed`, `src/checking/composed`, `parse_native` callers in `qsl-replay/src/spine.rs`) and #971 deletes none of it. Failure: TC-1027's inspection of AC-6's second sentence fails once #971 merges, and ADR-033 Consequences ("disappear ... with the last family's PR") is broken by the PR that moved the last family. Either name the remaining composed-lane retirement as open work under a ticket and amend item 9/AC-6 to say what still holds it, or route the deletion as owed by #971. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:71-79,100; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:242-246,253-254 | wrong-requirement |
| FND-003 | medium | FR-361-AC-4 and TC-1015 step 2 say that under `quire.model.complete/v1` a unit holding only a `hyper` or `relation` clause "is admitted", and under the value profile "the refusal, if any, is FR-110's layer rule". Under #971 both refuse `NotEmitted` under both profiles, and #971's `hyper_clauses_are_ungated_by_the_model_profile` is tagged TC-1015/FR-361-AC-4 while asserting that refusal. Failure: the matrix reports FR-361-AC-4 backed by a test that asserts the opposite of its text. Add the `NotEmitted` exception for these clauses to FR-361-AC-4 and TC-1015 in this PR, since it is the same stale statement. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:233; spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md:18,24 | wrong-requirement |
| FND-004 | low | ADR-033 Context still says in the present tense that S2 "has none for a `relation` or `hyper` clause ... refuses with `NoDispatchEntry`". As context at decision time it is acceptable, but a one-clause "(at the time of this decision; moved by QSL-550)" would stop a reader taking it as current. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:103-110 | wrong-requirement |

## Verdict

Not mergeable until FND-001 to FND-003 are settled; each is a statement a test
or inspection will now contradict. FND-002 may be answered by a sentence plus a
ticket rather than code. TC-596..599, FR-173 and FR-171-AC-1..3 agree with #971.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-362 item 8 (lines 66-77) and the ADR-033 bullet (lines 224-229) now scope "moved" to a `relation` clause and a `hyper` clause over `behaviours`, and say a finite-domain `hyper` and a `relation` over non-operation names keep QSpec FR-191's finite reading, agreeing with FR-171 and FR-172. |
| FND-002 | deferred | Recorded as open work, out of this PR's scope: FR-362 item 9 now names the composed lane paths and says its retirement is not owed by the change that moved relation and hyper, and ADR-033 Consequences says the same. No ticket id. Residual tension for the owner: item 9 and ADR-033 "Composed requirements" still say "the PR that moves the last family retires them", and AC-6's second sentence ("After the last family has moved ... no longer exist") is unchanged; TC-1027 will need that open work tracked before AC-6 can pass. |
| FND-003 | fixed | FR-361-AC-4 now reads "is not refused by the model-profile gate (a `hyper` or `relation` clause is refused only as `NotEmitted`)" and TC-1015 expected result 2 says the same, matching #971's test tagged TC-1015/FR-361-AC-4. |
| FND-004 | fixed | ADR-033 Context now reads "At the time of this decision, S2 had dispatch entries ... It had none for a `relation` or `hyper` clause, which `qsl-cst` parsed and S2 refused with `NoDispatchEntry`." |
| FND-002 | fixed | Round 2 (2777d6486): the last-family wording is gone. FR-362 item 9, AC-6 and the ADR-033 Composed requirements bullet and Consequences now say retiring the composed lane is open work tracked as QSL-868 (Backlog), so no text claims the change that moved relation and hyper retires it and AC-6 no longer asserts the symbols are gone. |
