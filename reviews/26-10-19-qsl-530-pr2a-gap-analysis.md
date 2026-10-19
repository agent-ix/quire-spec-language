---
id: SR-13323
title: "Gap analysis of QSL-530 E9 part 2a formula counterexample replay against FR-128, TC-523, TC-539, FR-127 V-6 (PR #1044)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@499d0a250; spec/functional/FR-128-*.md, FR-127-*.md (V-6), FR-125-*.md (Reading a lasso), FR-131-*.md, spec/decisions/ADR-018-*.md (CX-1..CX-3, SM-4, SM-7, SM-8, UE-5, DL-6), ADR-019-*.md (FS-8, SV-4), spec/test-cases/TC-523, TC-539; qsl-replay/src/model_check/{replay,settle}.rs, qsl-replay/src/execute.rs, qsl-analyze/tests/it/lasso_replay.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-523
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-539
    type: reviews
---
# SR-13323: Gap analysis of QSL-530 E9 part 2a formula counterexample replay against FR-128, TC-523, TC-539, FR-127 V-6 (PR #1044)

## Summary

Ticket: QSL-530 (E9), PR 2a. Planless. Plan completion: not assessed. `quoin matrix --json` (quoin 0.28.3, quire 0.36.2 / engine 0.50.2) was run on a git snapshot of an export of 499d0a250. FR-128-AC-1..AC-6 are all `untagged`. The 7 new tests carry `#[trace("TC-523")]` alone, as do the 3 existing TC-523 tests and the 1 existing TC-539 test, so the matrix credits no FR-128 criterion. That is truthful: no criterion is shown covered that is only partly built. It does mean FR-128 stays at 0 of 6 tagged after this PR. TC-523's procedure asks for `#[trace("TC-523", "FR-128-AC-n")]` once a criterion is fully covered.

What each test asserts, against the AC text. All of these bindings are correct and partial; each doc comment says "part of":

- `a_lasso_reproduces_with_its_formula_false_on_the_unrolled_trace`: AC-1's stutter lasso reproduces with `trace_position` 0, plus a weak-`inc` lasso. AC-1's "lasso under the weak constraint with no granularity" is meant over the ConfigVersion example and is not built. Replaying twice gives equal results (AC-4's last sentence).
- `a_bad_prefix_reproduces_at_its_last_position`: a Counter bad prefix, standing in for AC-1's FR-126-AC-2 bounded prefix.
- `a_lasso_on_which_the_claim_holds_disagrees`: AC-4's `ReplayParity` arm, with a Counter lasso in place of `upd(a)`.
- `a_lasso_that_does_not_close_or_does_not_stutter_at_a_terminal_state_refuses`: three of AC-3's nine rows (the loop does not close, terminal stutter on a non-terminal state, an altered digest).
- `an_unfair_lasso_refuses_naming_the_constraint`: AC-3's unfair row, and its fairness-set row with an empty set in place of `each`.
- `a_counterexample_names_its_clause_by_node_and_occurrence`: AC-4's `clause_node` refusal, with a node that names no clause in place of another clause.
- `a_liveness_violation_settles_refuted_through_replay`: FR-127 V-4 for TP-4.

The PR body's "Not built here" list matches the code. Neither the PR nor the test-file header claims more than is built.

## Verdict

**FAIL at ticket scope (untagged criteria remain), expected for PR 2a.** The PR-relevant gaps are FND-001 (FR-127 V-6's `UnfairLasso` row, which the PR introduces the refusal for but settles by the wrong row) and the coverage deferrals FND-002 to FND-004, which need tracking so QSL-530 is not closed with them open. The spec gaps FND-005 to FND-007 go to the planner as Linear notes. SR-13322 FND-001 (soundness) and FND-002 (a fair safety prefix never refutes) are the code consequences of FND-005 and FND-006.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-127 V-6 settles a `Violated` whose replay refuses `ReplayRefusal::UnfairLasso` as `Inconclusive(ReplayParity)` (ADR-019 SV-4: EN-1 builds only fair lassos, so the refusal is an engine defect). This PR adds `UnfairLasso`, but `settle_model_check` maps it through the generic refusal arm to `Inconclusive(ReplayRefused(invalid_runtime_input))`, and no test covers the settlement. The PR defers it to FR-131's PR (QSL-895). Either map it here, since the variant now exists and the row is one match arm, or record the deferral on QSL-895 explicitly. | qsl-replay/src/model_check/settle.rs:278; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:127 |
| FND-002 | medium | FR-128-AC-3: two of its nine rows are not implemented, not only untested. These are the `interval` refusal (the "For `kind: formula` ... compute the interval" behaviour) and the `trace: "observed"` refusal before recompiling. `TemporalCounterexample` has no `interval` or `trace` member, although ADR-018 CX-2 says it "reads and writes exactly that wire". The `upd(a)`/`attemptUpdate` not-enabled row and the one-snapshot `initial_state` row are not exercised by the new tests either. The deferral needs a ticket. | spec/functional/FR-128-replay-a-model-counterexample.md (Behavior, AC-3); qsl-replay/src/model_check/counterexample.rs:117-133 |
| FND-003 | medium | FR-128-AC-1's ConfigVersion lasso and `Branch` rows (two post-states under one identity, a digest no successor has) and FR-128-AC-4's `upd(a)` lasso over ADR-018 section 6's subject are not built. The Counter tests stand in, tagged TC-523 only. The coder says these need fixtures this crate lacks, but the shipped unit at examples/config-version exists, and SR-13082 FND-001 noted it can be built as a fixture. Name the ticket and PR that carry these rows. | spec/functional/FR-128-replay-a-model-counterexample.md (AC-1, AC-4); spec/test-cases/TC-523-replay-a-model-counterexample.md |
| FND-004 | medium | FR-128-AC-5 and AC-6 (TC-539) are not built for an authored clause. They are also not refused: an authored clause's undefined-atom counterexample now settles `ReplayParity` (SR-13322 FND-003). TC-539 has one test, for the deadlock item's undefined guard, tagged TC-539 alone, which its doc comment correctly says is not AC-5. The deferral needs a ticket. | spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md; qsl-analyze/tests/it/model_settle.rs:459-466 |
| FND-005 | medium | Spec gap: FR-128 gives no replay rule for a bounded-profile finite prefix that neither ends at a terminal state nor covers the formula's horizon from every activation. ADR-018 CX-1 states that shape only as what an engine produces ("so the evaluator decides it exactly"), and FR-128's Behavior evaluates whatever prefix it receives, so a truncated prefix evaluates under false-extension and reproduces. Add the check, with its refusal or `ReplayParity` cause, to FR-128 and an AC-3 row. This is the root of SR-13322 FND-001. | spec/functional/FR-128-replay-a-model-counterexample.md (Behavior, kind: Formula); spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md (CX-1) |
| FND-006 | medium | Spec gap: FR-128 says replay evaluates the formula "by FR-125 under the `over` binding", but FR-125/FR-328 return `MissingFairnessPremise` for a clause with a fairness set on any trace given as observations. That rule is for supplied traces with no enabledness. FR-128 does not say whether a model prefix, which replay re-executes with enabledness, is exempt. Say that a bad prefix of a fair clause is decided on the formula alone (fairness constrains only the infinite continuation), or say what it settles. This is the root of SR-13322 FND-002. | spec/functional/FR-128-replay-a-model-counterexample.md (Behavior); spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md |
| FND-007 | low | Spec gap: `replay_model_trace` now takes a `&dyn ModelLetters`, the seam that reads the `holds` atoms. FR-128's Inputs list only the request and the envelope, and say replay needs no model checker. Name the letters seam in FR-128's Inputs: that it is the one clause evaluator (FR-107), so an auditor's replay reads the same atoms, and who supplies it. | spec/functional/FR-128-replay-a-model-counterexample.md (Inputs); qsl-replay/src/model_check/replay.rs:274-280 |
