---
id: SR-12092
title: "Gap analysis of quire-spec-language PR #942 against QSL-821, FR-341 and FR-342"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ab50faf64693b520352b2dcac6627ccd12f3fc04; PR #942: qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/refinement.rs; FR-341-AC-4, FR-342-AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: reviews
---
# Gap analysis of quire-spec-language PR #942

## Summary

Ticket: QSL-821 (ticket text read as data). Planless, scoped to the PR diff.
Plan completion: not assessed.

QSL-821 asks for three things: `TypeAdmission` carries its `LimitExceeded`, the classifier names the limit for a TypeAdmission refusal, and FR-342-AC-3 holds for that cause. Two tests were added in qsl-replay. `a_check_type_admission_limit_is_incomplete_naming_the_limit` is tagged TC-861/FR-341-AC-4. `a_clause_run_compile_type_admission_limit_names_the_limit` is tagged TC-862/FR-342-AC-3. Both build `CheckCause::TypeAdmission { limit: Some(..) }` by hand and check that the classifier names the limit. This follows the existing `an_assembly_type_limit_is_incomplete_naming_the_limit` precedent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the producer half of the change. `type_admission` (reached through `pending_refusal` and check/mod.rs:1410) fills `limit` from `AssemblyCause::TypeLimit`, and both new tests build the variant by hand. If `type_admission` always set `limit: None`, every test would still pass, and the classifier would again name no limit for a real check-stage type-admission ceiling. That is the defect QSL-821 exists to fix. `EnvironmentFailure::Limit(EnvironmentLimit::new(kind, bound, actual))` is public, so a qsl-semantics unit test of `type_admission`/`pending_refusal` can assert `limit == Some(LimitExceeded::new(Setting::Environment*, bound, actual))`, and a `Refused` failure gives `None`. | qsl-semantics/src/check/mod.rs:672-684, qsl-replay/src/spine/clause/tests/refinement.rs:567-635 |

## Verdict

FIX, because of FND-001.

Clean:

- FR-341-AC-4 / TC-861. The classify_refusal test asserts `incomplete`, the cause's `ReachedLimit { environment.work_units, 1 }` and `limits()`. It also has a negative case: an ill-typed type admission with `limit: None` is `refused` and names no limit.
- FR-342-AC-3 / TC-862. The clause-run test asserts `incomplete` at stage `compile` and `limits()` naming `environment.ancestor_steps` with value 3. The tag cites only the AC the test checks.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | A qsl-semantics unit test, `a_type_admission_ceiling_keeps_its_limit`, asserts `type_admission` and `pending_refusal` keep the `LimitExceeded` of a reached ceiling and that a storage failure keeps none |
