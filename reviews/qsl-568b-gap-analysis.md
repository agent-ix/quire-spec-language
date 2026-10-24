---
id: SR-13596
title: "Gap analysis of quire-spec-language PR #1116: FR-186 S3 typing of probabilistic claims (TC-621)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@20dafcee1dbfa157ea9f9572071e16f6a82d96ea; PR #1116: FR-186-AC-1 to FR-186-AC-4 and the FR-186 Behavior statements against qsl-semantics/src/check/probabilistic/claim.rs and probabilistic.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-replay/src/spine.rs tests, qsl-cst/tests/it/probabilistic.rs; context FR-185, TC-621, ADR-013 O-09, ADR-024 PM-7/PM-8/PF-10"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: references
---
# Gap analysis of quire-spec-language PR #1116

## Summary

Ticket: QSL-568 (E17a, reopened for FR-186's S3 typing). This was a planless
run: no plan bundle was supplied, so "Plan completion: not assessed".

**Matrix.** Ran `quire matrix --scope . --format tsv` (quire 0.36.2, engine
0.50.2) on the head. It exited 0 and took 4 min. All four FR-186 criteria
compute `tagged`:

- AC-1 at `probabilistic_declarations.rs:979`.
- AC-2 at `spine.rs:1637` and `:1662`, and `probabilistic_declarations.rs:1128`,
  `:1205`, `:1903` and `:1998`.
- AC-3 at `qsl-cst/tests/it/probabilistic.rs:236`, `spine.rs:1607`, and
  `probabilistic_declarations.rs:1437`, `:1514`, `:1557` and `:1614`.
- AC-4 at `probabilistic_declarations.rs:1763`, `:1937`, `:1976` and `:2133`.

FR-392-AC-5, which hands over to FR-186-AC-2 to AC-4, is also tagged.

**Per AC.** The tests were read against the AC text and mutation-tested in
SR-13595: 21 of 25 hand mutants were killed.

- **AC-1.** All four ADR-024 §7 claims are read back with their forms, bounds,
  windows, `Workload(Steady)` and confidences. `P95` accumulates a quantity
  reward `latency` in place of `duration`, because the fixture types
  `duration` as an integer. The test's doc states the substitution, and the
  form, the window and `AtMost(5 ms)` are as the AC states.
- **AC-2.**
  - `0.005 s` is recorded as 5 ms, and `5 m` is refused `ill_typed`/
    `unit-mismatch` naming `Length` and `Time`. Mutants M01 and M20 are
    killed.
  - `>= 1` and `>= 0` are refused `qualitative-threshold`, and the CLI
    message names the TP-2 claim and the possible property.
  - `quantile-over-fraction` is refused.
- **AC-3.**
  - The unbounded event is refused, and the windowed event's horizon is 110
    (mutants M14, M19 and M24 killed).
  - A trace subject is refused, and the message names window aggregates
    (M13 killed).
  - `foreign-workload` names the workload and its model (M03 killed). The
    workload of the subject's own model checks.
  - `every scheduler fair ...` carries its fairness set (M15 and M16 killed).
  - The S1 test refuses `under Steady fair weak` `invalid_syntax` at `fair`.
- **AC-4.**
  - `invalid-confidence` is refused (M04 killed), and both `partial-confidence`
    cases are refused (M05 and M06 killed).
  - `decimal(1, 4294967295)` is refused with a limit refusal, and
    `confidence: None` checks.
  - The identities differ across alpha, workload, profile, fairness and bound
    direction (M07, M08, M10, M23 and M25 killed).

**Reverse gap.** These are code without an owning requirement statement:

- Seven new causes, and the `probabilistic-claim` S4 feature name
  (FND-003).
- The "Kind" classification, which has no observable output (FND-001).

**Stubs.** None found. The remaining `not-yet-implemented` refusals are the
FR-195 and timed constructs, each refused at the construct and tested.

## Verdict

CONDITIONAL. Every FR-186 criterion is tagged, and its checks are exercised
by tests that a mutant makes fail. One medium finding (FND-005) is a conflict
between FR-186's identity statement and ADR-013 O-09, which needs a spec
ruling. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-186's first "Kind and subject" statement has no observable implementation. "S3 SHALL classify a `probabilistic` claim with the QSpec FR-290 kind `probabilistic-satisfaction`." `CheckedProbabilisticClaim` has no kind member. The kind appears only as the tag bytes `probabilistic-satisfaction` inside the identity, and no test asserts it. Fix: record the kind (a constant accessor or a field), with a one-line assertion in the AC-1 test, or reword the statement so that the claim type implies the kind. | qsl-semantics/src/check/probabilistic/claim.rs:1324-1335; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:105-106 |
| FND-002 | low | A test is bound to AC-2 for a rule AC-2 does not state. `a_comparison_threshold_has_the_type_of_its_measure` is tagged `FR-186-AC-2`, but it verifies the "Events and measures" rule that a comparison's threshold has its measure's type (a natural number for `steps`, `(0, 1)` for `fraction`). AC-2 is about unit conversion, `qualitative-threshold` on probability thresholds, and `quantile-over-fraction`. No AC states the comparison-typing rule at all, so that statement is verified only through a mis-bound tag. Fix: add the comparison rule to an AC (for example AC-2: "`steps ... >= rational(1, 2)` is refused `ill_typed`; a fraction bound of `rational(1, 1)` is refused"), or retag the test to the statement's own criterion once one exists. | qsl-semantics/tests/it/probabilistic_declarations.rs:1205; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:138-140 |
| FND-003 | low | Behaviour without an owning requirement statement. These refusals have no FR-186 statement: `UnresolvedSubject`, `UnresolvedProfile`, `UnresolvedWorkload` and `UnresolvedReward` (`missing-name`), `AmbiguousReward` (`ambiguous-name`), and `NotTemporalProfile`. Neither does the `trace-subject` cause name (FR-186 says only "refuse it with a diagnostic that names window aggregates"), the cause names `unbounded-event` and `not-temporal-profile`, or the S4 feature name `probabilistic-claim`. They are tested, but no spec owns them. Fix: the spec follow-up of SR-13595 FND-008, adding the statements and extending STD-188. | qsl-semantics/src/check/probabilistic.rs:314-433; qsl-semantics/src/check/probabilistic/model.rs:323 |
| FND-004 | low | Two diagnostic statements are verified only on cause fields, never on the rendered text. FR-186 says the unbounded-event refusal is "a diagnostic stating that a probabilistic event needs a finite window", and AC-3 says the `foreign-workload` refusal is "naming `Throttle` and `Service`". The tests assert the cause variant and its fields. The `check_message` arms that render the text (spine.rs:1068-1084) are not asserted. Fix: two message assertions beside the existing trace-subject CLI test. | qsl-replay/src/spine.rs:1068-1084; qsl-semantics/tests/it/probabilistic_declarations.rs:1514-1556 |
| FND-005 | medium | FR-186's identity statement conflicts with ADR-013 O-09 and R-04 (spec-level drift). FR-186 says that "the obligation identity SHALL bind the subject, the profile, the form, the scheduler ..., the `over` binding and ... the confidence parameters". O-09 defines the obligation identity as CG's digest over the clause node id, the occurrence key, the kind and the arguments, which must include the occurrence key. The implementation follows FR-186, and its test asserts that two declarations of equal content share one identity. The spec owner must rule which is authoritative before the code can be judged conformant. The full recommendation is in SR-13595 FND-001. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:173-176; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:266-276; qsl-semantics/tests/it/probabilistic_declarations.rs:1883-1896 |

## Coverage

- Criteria in scope: 4 (FR-186-AC-1 to AC-4). Tagged: 4. Untagged: 0.
  Ignored-only: 0.
- FR-186 Behavior SHALL statements examined: 26. Of those, 1 has no observable
  implementation (FND-001) and 1 is verified only through a mis-bound tag
  (FND-002).
- Reverse gaps: 7 causes and 1 feature name with no owning statement
  (FND-003).
- Stubs: none. The 4 remaining `not-yet-implemented` constructs are refused
  and tested.
- Mutation: 25 hand mutants, 21 killed and 4 survived (SR-13595 FND-005 and
  FND-006).
- Plan completion: not assessed (planless run; no plan supplied).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | FR-186-AC-5 and FR-186-AC-6 are untagged. Spec PR #1117 (on main, d2e3c128b) added them, but the branch does not contain it. `quire matrix` on the clean merge of HEAD with origin/main (`git merge-tree` tree dbcb8519) computes both `untagged`. The tests that assert them exist, but carry only `#[trace("TC-621")]`: `a_comparison_threshold_has_the_type_of_its_measure`, `an_over_type_and_a_threshold_have_their_required_shape`, `an_event_horizon_past_u64_is_refused_at_its_operator` and `qsl-package` `a_checked_claim_refuses_emission` for AC-5; and `a_claim_names_a_model_a_temporal_profile_and_a_workload` (missing names), `a_reward_a_claim_names_is_one_the_model_declares`, `every_scheduler_carries_its_fairness_set` (`Server::nothing`) and `a_reward_of_two_value_types_is_ambiguous` for AC-6. Fix: merge origin/main into the branch, then tag those tests `FR-186-AC-5` and `FR-186-AC-6`. | qsl-semantics/tests/it/probabilistic_declarations.rs:1205; qsl-semantics/tests/it/probabilistic_declarations.rs:1282; qsl-semantics/tests/it/probabilistic_declarations.rs:1367; qsl-semantics/tests/it/probabilistic_declarations.rs:2012; qsl-semantics/tests/it/probabilistic_declarations.rs:2035; qsl-package/src/emit/tests/probabilistic.rs:55 |
| FND-007 | medium | Three clauses of the merged FR-186-AC-4 have no test. The ACs are "a fairness set with its constraints reordered", "a reward declared on its operations in another order ... keep one normal form", and "Two declarations of equal content have equal normal forms". The first two are shown by surviving mutants: removing `fairness.sort()` survives (N1), and removing `operations.sort()` in `reward_bytes` survives (N2). The third is gone because the first round's twin-declaration assertion (claims `C` and `D`) was deleted. The code sorts both, so behaviour is likely correct, but the AC is unverified. Fix: in `the_normal_form_binds_what_the_claim_asserts`, add `every scheduler fair weak Server::attempt fair strong Server::reset` against the reversed order; add a `latency` reward declared on `request` then `attempt` against the reverse order; and restore the two-declaration equality. | qsl-semantics/tests/it/probabilistic_declarations.rs:1769-1980; qsl-semantics/src/check/probabilistic/claim.rs:1336; qsl-semantics/src/check/probabilistic/claim.rs:1458 |
| FND-008 | low | Two AC clauses are asserted more weakly than their text. AC-5: `steps ... >= rational(1, 2)` and `>= 5 ms` must be refused `ill_typed`/`type-mismatch`, but the test asserts only `Code::IllTyped` (probabilistic_declarations.rs:1263-1273), so a change of cause would pass. AC-4: "one that differs only in its `over` type" is tested as an `over` present against one absent (`over(OVER) != over("")`), not as another object type. The fixture has one object type, `Server`. Fix: assert `Some("type-mismatch")`, and add a second object type to the fixture, or record the substitution in the test's doc. | qsl-semantics/tests/it/probabilistic_declarations.rs:1263-1273; qsl-semantics/tests/it/probabilistic_declarations.rs:1966-1979 |

## Dispositions

Round 1, reviewed at `ef714e5855d617ed3262a8a6d25dc659d05ef5cf`, against FR-186
and TC-621 as spec PR #1117 left them on main (d2e3c128b). The branch does not
yet contain #1117; see FND-006.

The matrix was run on the clean merge tree (HEAD with origin/main):
- AC-1 to AC-4 are `tagged`.
- AC-5 and AC-6 are `untagged`.

Gates on the head are all green; see SR-13595's Dispositions.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | spec #1117 removed the "classify ... `probabilistic-satisfaction`" statement from "Kind and subject", so the statement has nothing left to implement; confirmed on main d2e3c128b |
| FND-002 | fixed | 96d0012d9 retagged `a_comparison_threshold_has_the_type_of_its_measure` to bare `TC-621`, and #1117 states the rule as FR-186-AC-5; the missing AC-5 tag is FND-006 |
| FND-003 | fixed | spec #1117 adds the missing-name statement (five names), the one-value-type reward rule, the causes and `probabilistic-claim`; `over-not-object-type` is SR-13595 FND-013 |
| FND-004 | fixed | 96d0012d9: `the_event_and_workload_diagnostics_read_as_the_requirement_states_them` asserts "needs a finite window" and `Throttle` and `Service` in the rendered text |
| FND-005 | fixed | spec #1117: FR-186 "Normal form" rules that S3's record is not an identity (R-04), and that O-09's identity stays CG's over the S4 node and its occurrence key |

Round 2, reviewed at `20a7023c0556c145e71c9bb46e3f029d21b914e8`. Spec #1117 is
now in the branch, through the merge of main at 47ac2d4fd.

`quire matrix` on the head computes FR-186-AC-1 to AC-6 as tagged.
FR-185-AC-1, AC-2 and AC-8 and FR-311-AC-3 are untagged, but they were
untagged on the round-0 base too, and this PR does not touch them.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 20a7023c0: AC-5 tags on `a_comparison_threshold_has_the_type_of_its_measure`, `an_event_horizon_past_u64_is_refused_at_its_operator`, `an_over_type_and_a_threshold_have_their_required_shape` and `a_checked_claim_refuses_emission`; AC-6 tags on the missing-name, reward and fairness tests; the matrix shows both tagged |
| FND-007 | fixed | 20a7023c0: `the_normal_form_ignores_declaration_order_and_binds_the_over_type` covers reversed fairness, reversed reward declarations and two equal-content declarations (mutants N1 and N2 killed) |
| FND-008 | fixed | 20a7023c0: the `steps` refusals assert `(IllTyped, "type-mismatch", threshold)`; the fixture gained the object type `Client`, and `over("Server") != over("Client")` is tested |
