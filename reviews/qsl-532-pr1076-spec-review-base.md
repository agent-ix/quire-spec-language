---
id: SR-13380
title: 'Spec review of PR #1076: FR-133 hints name only starved constraints, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@080408b8c; git diff refs/remotes/origin/main...080408b8c (PR #1076, base main 9567ed601): spec/decisions/ADR-019-strong-fairness.md, spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md, spec/test-cases/TC-534-strong-fairness-hint.md; read against FR-131, QSpec FR-369 (quire-specification main) and qsl-replay replay.rs on main and on qsl-532-engine (#1073, open); ticket QSL-532'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-133
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-534
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-019
  type: reviews
---

## Summary

Ticket: QSL-532. PR: quire-spec-language#1076, head 080408b8c, scoped to `git diff refs/remotes/origin/main...080408b8c` (one commit, three files, +15/-8). Spec-only: FR-133 drops 'emit no hint when the clause already holds the named constraint' and states that a hint names only a constraint the lasso starves; ADR-019 SV-6 gets the same rule; new FR-133-AC-5 and TC-534 step 6. Unreachability verified. The hint conditions (FR-133 Behavior, SV-6) are: Whole when the operation is enabled at some loop state and no loop step takes it; otherwise Each for an identity enabled at a loop state and never taken. FR-131/SF-4's strong check is: when the constraint is enabled at some loop state, some loop step takes it. On qsl-532-engine (#1073, open) unfair_constraint builds a strong Whole constraint over every identity of the operation the loop enables or takes, and one strong constraint per such identity for Each; check_loop's strong rule is taken || !enabled_somewhere. So a held strong Whole fails exactly when the Whole hint would fire, and a held strong Each fails exactly when either hint would fire (a Whole-starved operation also starves an identity). In both cases replay returns UnfairLasso before strong_hints runs (strong_hints is called only in the Reproduced arm). The one combination left, held strong Whole with an Each hint (FR-133-AC-1's second sentence: loop takes acq(2), acq(1) enabled at position 0 and never taken), suggests {Strong, acquire, Each}, which differs from the held {Strong, acquire, Whole}: a different granularity, never matched by the old 'same kind and granularity' suppression. Weak held constraints never matched either, since every hint is Strong. QSpec FR-369 (the semantic authority, ADR-019 QS-8) already says 'A reproduced lasso is fair under every constraint the clause carries, so no suggestion names one of them', so QSL now agrees with QSpec. On main the premise is spec-only: replay.rs refuses any strong constraint with ModelTraceRefusal::StrongFairness; strong replay arrives with #1073, and #1073's strong_hints still carries the now-dead `if !clause.fairness().contains(&suggested)` and the doc line 'An operation the clause already constrains that way gets none' (code, outside this diff). FR-133-AC-5 / TC-534 step 6 checked by hand against FR-131-AC-1: lasso 0 -acq(2)-> 2 -rel-> 0 under strong each acquire, acq(1) enabled at position 0 never taken, refuses UnfairLasso{Strong, acquire, Each} with no result; FR-131-AC-3 / FR-127 settles an EN-1 outcome so refused inconclusive ReplayParity (V-6), carrying no hint since it is not refuted. Correct. Stale text: grep of spec/ for suppress, 'already carries', 'already holds', 'already constrain' and every hint/would-exclude mention (ADR-018 AM-4 note, ADR-019 §6 worked example, DS-1, QS-7, QS-8, RU-4, US-017, FR-129, FR-130, FR-131, TC-531, TC-532) finds no remaining suppression wording. Base review: soundness of the removal, wording, behaviour-not-changelog. The new text describes the system (what a hint names and why a held constraint never appears), not the edit. Methods: spec-review/base (this file), spec-review/ears (SR-13381), spec-review/integrity (SR-13382); code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 9567ed601 and none in FR-133, TC-534 or ADR-019; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-133-description | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-starves-bullet | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-AC-1 | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-AC-5 | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| ADR-019-SV-6 | spec/decisions/ADR-019-strong-fairness.md | examined |
| TC-534-description | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| TC-534-step-6 | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| TC-534-step-6-expected | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| FR-131-AC-1 | spec/functional/FR-131-replay-checks-strong-fairness-on-a-model-counterexample.md | context_only |
| QSpec-FR-369-behavior | quire-specification:spec/functional/temporal/FR-369-name-excluding-strong-fairness.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-133-AC-5 joins two claims. The first (FR-131-AC-1's lasso under strong each on acquire is refused UnfairLasso with no result and no hint) restates FR-131-AC-1, already tested in TC-532, and its 'no hint' cannot fail on its own since hints live only in the result that a refusal does not return. The second ('a lasso that replay accepts carries hints only for constraints the clause does not hold') is a universal with no instance, and TC-534 step 6 exercises only the refusal, so a test tagged FR-133-AC-5 does not cover it. Bind the second half to its witness, FR-133-AC-1's strong (whole) refutation whose one hint {Strong, acquire, Each} is not the held {Strong, acquire, Whole}, and add that to step 6 and its expected result; or drop the second half and say AC-5 guards that hints are computed only after FR-131 accepts the lasso. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:103 |
| FND-002 | low | Step 6's expected result says 'UnfairLasso naming the constraint' without naming it. FR-131-AC-1 fixes it as {Strong, acquire, Each}; write that, so the step fails on a refusal that names the wrong constraint. | spec/test-cases/TC-534-strong-fairness-hint.md:54 |

## Verdict

Mergeable after FND-001 is fixed or deferred: the removal is sound (suppression is unreachable given FR-131) and matches QSpec FR-369; one medium (AC-5 compound, second half untested) and one low (step 6 does not name the refused constraint).

## Dispositions

Round 1 checked at head a54220159 (fix commit a54220159 on top of merge ff6a60244 with main 40f8707b1); gates: make check-spec-validation exit 0, 17 warnings, same 17 as main 40f8707b1, none in FR-133, TC-534, ADR-019 or reviews/; tools/check-index-completeness.sh passes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: ADR-019 §6's mutex refuted under `strong` (whole) on `acquire` carries the hint `{Strong, acquire, Each}`, identity `acq(1)`, and no hint for the held `Whole` constraint; FR-131-AC-1's lasso under `strong each` on `acquire` is refused `ReplayRefusal::UnfairLasso` by replay (FR-131) and carries no hint. |
| FND-002 | still-open | Step 6's expected result still reads 'ReplayRefusal::UnfairLasso naming the constraint, no result and no hint' without naming it; FR-131-AC-1 fixes it as {Strong, acquire, Each}. Write 'UnfairLasso{constraint: {Strong, acquire, Each}}'. |
