---
id: SR-12286
title: "Gap analysis of quire-spec-language PR #1171: author_clause and SubjectIdentity against FR-336 AC-4 and AC-5 (QSL-889)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@96338902801d6e5a2557b94630025898632d6a96; PR #1171: qsl-analyze/src/model_check/request.rs and qsl-analyze/tests/it/universe_obligation.rs, against FR-336 (AC-1 to AC-5), TC-846 and the QSL-889 ticket scope"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-846
    type: reviews
---
# Gap analysis of quire-spec-language PR #1171

## Summary

Ticket: QSL-889. Planless. Plan completion: not assessed. The ticket text was
read as data: its scope is to build `ModelRequest` from the checked clause
and the `ModelSubject`, decide the subject's identity projection, and tag
FR-336-AC-4 on a test driven by a real subject. It also names "the
explored-universe half of FR-336-AC-2".

Criterion to test to code:

- FR-336-AC-4 → `two_universes_are_two_obligations_and_each_result_joins_its_own_index`,
  tagged `TC-846, FR-336-AC-4`. The test drives a real `ModelSubject` over
  the `Counter` package through `author_clause` and checks the following:
  indices 0 and 1 for `{a, b}` and `{a, b, c}`; dedup to the first index;
  identity equality for equal keys and inequality for different keys; the
  `counters` universe as read back; equal extent, kind and empty proof
  domains for both items; and each `proved` record joining only its own
  index. The `proved` records are built in the test, not produced by an
  EN-1 run, but the AC is about the join, and the join is what the test
  exercises. The tag is honest: the test fails if universes leave the
  identity or if `author_clause` stops reading the subject.
- FR-336-AC-5 → `a_proof_bound_or_an_initial_state_changes_the_identity_and_a_foreign_clause_is_refused`,
  tagged `TC-846, FR-336-AC-5`. A pushed proof bound and a pushed second
  initial state each change the identity. A clause from another package
  (different formula) is refused with `NoRequirementRecord`, and no item is
  written. Each arm can fail.
- FR-336-AC-1 and AC-3 are untouched by this PR and stay tagged elsewhere.
  FR-336-AC-2 stays untagged, honestly: `qsl-analyze/tests/it/model_check.rs:998-1004`
  records that its explored half needs a liveness `Holds`, and EN-1 still
  returns `FormNotDecided` for it.

No stubs, no `todo!`, no tautological assertions. `author_clause` has no
non-test caller. That is consistent with EN-1 (`check_model` has none
either) and is not an unowned-code gap: FR-336's new "EN-1 request path"
bullet owns it.

## Verdict

FIX, through the code-review's FND-001 (SR-12285). The AC set lets that
defect pass: AC-5 lists proof bound and initial state as the
identity-changing members but not the checked package, so every tagged test
passes while two different models share one obligation (FND-001 here).
FND-002 is low: the ticket's AC-2 item needs an owner after this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-336-AC-5's adverse cases cover a proof bound and an initial state, but not a subject that differs only in its checked package. Its foreign-clause arm passes only because the foreign clause's formula differs (`c.value = 3`), so its node id differs. A clause with the same text in a package with different contracts is accepted: the reviewer probe got index 0 for it (SR-12285 FND-001). Add a "differs only in the checked package" arm to AC-5 and to the test. | qsl-analyze/tests/it/universe_obligation.rs:124-168; spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:107 |
| FND-002 | low | The QSL-889 scope names the explored half of FR-336-AC-2. The PR leaves it out and says so ("Not claimed"), with "Remaining work: QSL-889". If this PR closes QSL-889, that half has no ticket. Its blocker is the EN-1 liveness `FormNotDecided`, not `ModelRequest`. Give it a ticket of its own, or keep QSL-889 open for it. | qsl-analyze/tests/it/model_check.rs:998-1004 |

## Dispositions

Round 1, reviewed at 9d08ad686 (fix commit 10ee83f10). The reviewer checked each row against the fix diff and replaced the coder's draft table with these checked rows.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 10ee83f10. FR-336-AC-5 now names the "only in its checked package" arm and the unselected-package arm, and the test asserts both. The package arm is mutation-checked (SR-12285 FND-001 row). |
| FND-002 | deferred | QSL-889 stays open (state In Progress, checked) for the explored half of FR-336-AC-2. Its blocker is EN-1's liveness `FormNotDecided`. Close the ticket only when that half lands or moves to a ticket of its own. |
