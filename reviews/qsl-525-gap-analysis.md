---
id: SR-12136
title: "Gap analysis of quire-spec-language PR #960: FR-123, FR-124 AC-1 and FR-382 at S3"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f05318e1b0dac554afaa789bc0fa39ed3f521492; qsl-semantics/tests/it/temporal_fairness.rs, qsl-semantics/tests/it/terminal_declarations.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-eval/tests/it/temporal_trace.rs, spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md, spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md, spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md, spec/test-cases/TC-518-s3-checks-fairness-and-interval-operators-of-infinite-trace-clauses.md, spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md, spec/test-cases/TC-1059-s2-builds-the-terminal-declaration-form.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #960

## Summary

Ticket: QSL-525. This is a planless gap analysis scoped to the PR diff. It checks
each claimed criterion against a test that would fail if the behaviour were
wrong, and checks that each `#[trace]` binding claims only what its test
verifies. Plan completion: not assessed. Static review only: no test was run.
`quire validate` on the two edited spec files passed (warnings only:
duplicate-archetype notices from the installed module set).

Examined:
- FR-123-AC-1 (TC-518). The fairness set half is verified:
  `an_unmarked_granularity_checks_as_whole_and_each_differs` and
  `same_named_operations_of_different_types_stay_distinct`. The node-identity
  half ("a clause with each fairness set has a different node identity") has no
  test.
- FR-123-AC-2 (TC-518): `an_unresolved_or_misplaced_constraint_refuses_and_a_repeated_one_joins_once`.
  It asserts the exact cause, span, code and tag. Binding correct.
- FR-123-AC-3 (TC-518): `intervals_nest_under_unbounded_operators_and_key_by_profile`.
  It covers the intervals in arena order, `[3,Open]`, the event-position open
  refusal, the inverted refusal and the profile-keyed `[0,2]`. Binding
  correct.
- FR-123-AC-4 (TC-518): `clauses_over_a_model_subject_classify_into_their_property_form`.
  The AC's five forms are covered, and so are `not eventually` (Safety),
  `eventually[3,*]` (Liveness) and `always[3,*]` (Safety). The recovery
  formula discriminates the `implies` polarity. Binding correct.
- FR-124-AC-1 (TC-519). The None/When/Any half, the second declaration and the
  `pre` read are verified in `terminal_declarations.rs`. The "pairwise
  different identities" half has no test.
- FR-382-AC-6 (TC-1059 step 6), as amended: `a_terminal_declaration_on_no_model_refuses_at_the_model_alias`.
  Binding correct.
- FR-328-AC-6 and FR-329-AC-7 (TC-848), re-fixtured onto a modelled unit.
  The constraint's resolved operation is asserted. Binding correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Coverage inflation. Both FR-123-AC-1 tests carry `#[trace("TC-518", "FR-123-AC-1")]`, so the matrix shows the whole criterion covered. Neither test asserts that clauses differing only in their fairness set have different node identities, and the PR body itself says that half is not covered. A wrong implementation that drops the fairness set from the identity passes every test bound to this id. | qsl-semantics/tests/it/temporal_fairness.rs:120, qsl-semantics/tests/it/temporal_fairness.rs:157 |
| FND-002 | high | Coverage inflation. Three tests in `terminal_declarations.rs` and one in `assemble/tests.rs` carry `#[trace("TC-519", "FR-124-AC-1")]`, so the matrix shows the whole criterion covered. No test asserts that the None, When and Any packages have pairwise different identities, and FND-001 of SR-12135 shows they do not differ today. The fourth binding (`a_terminal_declaration_using_no_profile_refuses_at_the_alias`) tests an undeclared `using` alias, which FR-124-AC-1 does not state. | qsl-semantics/tests/it/terminal_declarations.rs:52, qsl-semantics/tests/it/terminal_declarations.rs:87, qsl-semantics/tests/it/terminal_declarations.rs:131, qsl-semantics/src/check/assemble/tests.rs:516 |
| FND-003 | medium | FR-124-AC-1 and TC-519 name the `Counter` unit (`value: Int[0,3]`, `inc`, a predicate true exactly when every counter is at 3). The tests use the ConfigVersion fixture with `self.versionNumber = 1000`. The `When` test checks only that the predicate has one parameter named `self`. It does not check `self`'s type or the body's Boolean result. | qsl-semantics/tests/it/terminal_declarations.rs:22, qsl-semantics/tests/it/terminal_declarations.rs:60-70 |
| FND-004 | low | The polarity cases tested are `not eventually` and the `implies` antecedent. No test covers a negated `always`, `until` or `release` (for example `not always holds(p)` is Liveness), and none covers deep input. The pass is iterative and correct by inspection. | qsl-semantics/tests/it/temporal_fairness.rs:330-355 |

## Verdict

FAIL on FND-001 and FND-002. The claimed criteria are verified with strong
oracles: exact causes, spans, codes and forms. But the two AC-1 ids are bound
whole while their identity halves are untested. The fix is to bind only what
is verified, or to add the identity assertions. FR-124's identity half
currently fails outright (SR-12135 FND-001). FR-124-AC-2 and AC-3 are not
claimed, and nothing traces to them.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new `a_checked_terminal_declaration_refuses_not_emitted_at_package_build` carries `#[trace("TC-1059", "FR-382-AC-6")]`, but it checks only code and cause. The amended AC states the refusal is "at the span of the declaration", and no test checks that span. The span comes straight from `TerminalClauseDeclaration::span` and is correct by inspection. | qsl-semantics/tests/it/terminal_declarations.rs:169-187 |

## Dispositions

Round 2, reviewed at `059cb1466e5b46cb36bb0528acaf222d40fa1594`. The fix
commit is `3fd335c7f`; the merge of `origin/main` was not reviewed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3fd335c7f: both tests now carry `#[trace("TC-518")]` only. Their doc comments name the halves they verify, and no test binds FR-123-AC-1. |
| FND-002 | fixed | 3fd335c7f: the three `terminal_declarations.rs` tests now carry `#[trace("TC-519")]` only, and the undeclared-`using` test has no tag. No test binds FR-124-AC-1. |
| FND-003 | fixed | 3fd335c7f: the `When` test now asserts the body's type is `ValueType::Boolean`, and the tests no longer bind FR-124-AC-1, so the ConfigVersion fixture no longer stands in for the AC's Counter unit. |
| FND-004 | deferred | Low, recorded only: no negated `always`/`until`/`release` polarity case. |
| FND-005 | deferred | Medium, recorded only (round 2 is the last round; major bugs only): the FR-382-AC-6 binding does not check the not-emitted span. |
