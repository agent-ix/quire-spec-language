---
id: SR-12150
title: "Gap analysis of quire-spec-language PR #984: FR-132 missing fairness premise"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@53365a40b4bae9489330394feed9fc2da462e448; qsl-eval/tests/it/temporal_trace.rs, qsl-eval/src/temporal/engine.rs, qsl-eval/src/temporal/outcome.rs, qsl-replay/src/proof_result.rs, spec/functional/FR-132-settle-fairness-over-a-supplied-trace-as-a-missing-premise.md, spec/test-cases/TC-533-fairness-over-a-supplied-trace.md, spec/test-cases/TC-848-a-fairness-premise-over-a-supplied-trace-is-missing.md, spec/test-cases/TC-841-replay-a-temporal-counterexample-over-an-observed-trace.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #984

## Summary

Ticket: QSL-533. This is a planless gap analysis scoped to the PR diff and
FR-132. It checks each claimed criterion against a test that would fail if the
behaviour were wrong, checks that each `#[trace]` binding claims only what its
test verifies, and checks the PR's claim that the behaviour already existed.
Plan completion: not assessed. Static review only: no test was run.

Examined:
- FR-132-AC-1 (TC-533 step 1). The evaluator half is verified by
  `a_fairness_constraint_over_a_supplied_lasso_names_the_first_constraint`:
  the right outcome, the first constraint by kind, operation and granularity,
  and O-16 unsupported. The `Unsupported(...)` terminal value, label
  `unsupported` and basis `unavailable` have no test and no code.
- FR-132-AC-2 (TC-533 step 2):
  `an_empty_fairness_set_over_a_supplied_lasso_is_evaluated`. It covers the
  same unit, lasso and clause with an empty set, and expects `Violated` at 0.
  Binding correct.
- FR-132 Behavior and Outputs, against `engine.rs`, `outcome.rs` and
  `proof_result.rs`.
- TC-848 (FR-328-AC-6, FR-329-AC-7). The existing tests on `main` cover the
  same engine branch, including "no position visited". Bindings correct.
  FR-330-AC-6 (step 3) has no `run_clause` temporal path yet; QSL-510 owns it.
- TC-841 step 4 (FR-331-AC-5). No temporal replay or
  `ReplayRefusal::MissingFairnessPremise` exists. It belongs to FR-331, owned
  by QSL-510 (Backlog). The PR body says it is not covered, which is accurate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-132's output is not implemented. FR-132 Outputs and its third Behavior bullet (ADR-019 SV-2: "a new `Unsupported` cause") require `TerminalValue::Unsupported(UnavailabilityCause::MissingFairnessPremise{constraint})`, and require `TerminalValue::category` to map it to unsupported. `UnavailabilityCause` has only `SolverAbsent` and `BackendAbsent`. The PR's claim that "the behaviour already exists" holds only for the qsl-eval `TemporalOutcome` variant. The ticket lists qsl-replay under Touches, and the PR leaves it untouched on the grounds that "no replay code yet" exists. But this cause type is in `qsl-replay/src/proof_result.rs` today and does not depend on temporal replay. | qsl-replay/src/proof_result.rs:184-190, qsl-replay/src/proof_result.rs:344, qsl-replay/src/proof_result.rs:387 |
| FND-002 | high | Coverage inflation. `#[trace("TC-533", "FR-132-AC-1")]` binds the whole criterion, so the matrix shows FR-132-AC-1 covered. But the test cannot fail on the AC's `Unsupported(...)` terminal value, label `unsupported` or basis `unavailable`, and none of them is produced anywhere (FND-001). The PR body states the gap, but the tag still claims it. This is the same pattern as SR-12136 FND-001. The fix is to implement and assert those parts (preferred, with FND-001), or to bind the test to `TC-533` only. | qsl-eval/tests/it/temporal_trace.rs:996 |
| FND-003 | low | FR-132-AC-1 and TC-533 name a mutex (`m.owner`, steps `acq(2)`, `rel`). The tests use the `Counter` model with `acquire`/`release` and `c.level`, and the evaluator's trace input does not carry steps. The AC also does not say how `0 2 0` splits into prefix and loop; the test uses an empty prefix and a three-position loop. The oracle is unaffected, because no state equals 1. The PR body records the translation. | qsl-eval/tests/it/temporal_trace.rs:983-989 |

## Verdict

FAIL on FND-001 and FND-002. The evaluator half of FR-132 is verified with
strong oracles. The tests fail if the premise is not reported, if it names the
wrong constraint (case 3 tells first from last), or if the empty set settles
differently. FR-132-AC-2 is fully verified. But FR-132's own output, the
`UnavailabilityCause::MissingFairnessPremise` terminal cause with its category
map, label and basis, does not exist, and the AC-1 tag claims it. Both are
fixed by adding the variant with its wire spelling, plus a test that
`TerminalValue::Unsupported(MissingFairnessPremise{..})` maps to unsupported
with label `unsupported` and basis `unavailable`. That leaves the AC-1 tag
honest. TC-841 step 4 and FR-330-AC-6 are correctly declared not covered and
belong to QSL-510.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The AC-1 basis clause is asserted only through the new `UnavailabilityCause::basis()`, a `const fn` that returns `SettlementBasis::Unavailable` for every cause. Nothing in production reads it. The wire item carries no basis for `unsupported`, and clause reports take their basis from `spine/clause.rs` `undecided_basis`. So `assert_eq!(cause.basis().as_str(), "unavailable")` cannot fail on any path that emits a basis. The real basis check belongs with QSL-510's temporal `run_clause` report. | qsl-replay/src/proof_result.rs:224-226, qsl-replay/src/proof_result.rs:933 |

## Dispositions

Round 1, reviewed at `5cf839d42c3acb491e3cf646765168450a0d3cb1`. The fix
commit is `5cf839d42`. Static review only; no build was run.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5cf839d42: `UnavailabilityCause::MissingFairnessPremise { constraint: FairnessConstraint }` exists, with wire spelling `missing-fairness-premise` and code `unsupported_projection`. `OutcomeItem::from_terminal` now emits the code. `TerminalValue::category` maps `Unsupported(_)` to unsupported. `Copy` is dropped and `as_str` takes `&self`; the only caller (`outcome.rs:335`) already holds a reference. |
| FND-002 | fixed | 5cf839d42: `a_missing_fairness_premise_settles_unsupported_naming_the_first_constraint` (qsl-replay, `#[trace("TC-533", "FR-132-AC-1")]`) fails if the terminal value is missing (compile), names the wrong constraint (`assert_eq` plus an `assert_ne` against `weak whole release`), has the wrong category, wire `result`, cause `kind` or `code`. With the evaluator test, the AC-1 binding now claims what the tests can fail on, except the basis clause (FND-004). |
| FND-003 | deferred | Low, recorded only: fixture translation (mutex to `Counter`, no steps, prefix/loop split) is declared in the PR body and does not affect the oracle. |
| FND-004 | deferred | Medium, recorded only (major bugs only): the basis clause is backed by a constant with no production reader. Bring it to QSL-510's temporal `run_clause` report, or delete `basis()` there. |
