---
id: SR-16311
title: "spec-review of FR-430-AC-5 timed refinement liveness half (quire-spec-language PR 1320)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@efe0553fd1; spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md; spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1170
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: references
---

# Spec review of FR-430-AC-5 timed refinement liveness half (quire-spec-language PR 1320)

## Summary

Ticket: STD-351. One spec-review pass over `git diff origin/main...efe0553fd1` (agent-ix/quire-spec-language#1320), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- FR-430-AC-5 vectors derived by hand against FR-143 and ADR-020 RC-1/ADR-026 TD-5: with the invariant `when not self.sent { x <= 0 }` no time can pass while `sent` is false, so the `poll` loop has total delay 0, is abstract-unfair for `send`, and does not refute (`Checked(HoldsReduced{reductions: []})`, the form FR-143 states). Without it the one-`poll` loop admits positive delay and is a time-divergent lasso, failure `Divergence{constraint: send}` because the loop has only stutter steps (ADR-020 RC-1).
- The Outputs edit names the timed lasso (ADR-026 CT-2) as the liveness counterexample; consistent with ADR-026 CT-2.
- No deferral, narrowing or follow-up wording in the diff. POR on a timed subject stays V-8 at negotiation; the PR does not touch that rule.
- No QSpec #403/#405/#409/#419/#408/#413/#418/#407/#410/#417 names are cited. The QSpec half (FR-377-AC-16) is #394, outside this batch.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The violating half of AC-5 states the lasso loop delay only as "positive". AC-2 and AC-4 give concrete delays (1, 1, 0 and loop delay 1), so their tests assert exact lasso bytes; AC-5's test can assert only the sign, and the delay the engine picks is not pinned anywhere in this PR. Fix: state the canonical delay FR-416 chooses for this loop, or say the test asserts the sign only. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md | missing-requirement |

## Verdict

Mergeable: only low findings remain.

Merge-tree against origin/main and every other open QSL head touching FR-430 or TC-1170: clean. Gate output has 26 warning lines, none in the touched files.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, head a3091082. ll` (precondition `not self.sent`, reset `x`), step map `send -> send`, `poll -> stutter`, mapping `sent -> sent`: with the time invariant `when not self.sent { x <= 0 }`, the `poll` loop at `sent = false` is the only cycle on which `send` is enabled and not taken, and its total delay is 0, so it do |
