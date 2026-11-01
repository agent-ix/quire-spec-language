---
id: SR-1988012
title: "Focused gap analysis of quire-spec-language PR #2006: QSL-574 finite-horizon solver and settlement"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b404f637929f6c9ab387e65f3749a9826c19a058; focused manual traceability review of PR #2006 changed qsl-analyze and qsl-replay code/tests against FR-197 and FR-203; full repository matrix unavailable"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: references
---
# Focused gap analysis of PR #2006 (QSL-574)

## Summary

Ticket: QSL-574. The required `quoin matrix --repo . --json` did not complete: after emitting module warnings it produced no result for 60 seconds and was interrupted. `quire matrix --scope . --format json` behaved the same way. Quoin version was 0.29.1; quire was 0.37.1 (engine 0.50.3). I therefore performed a focused manual trace-tag review of the changed EN-5 tests only; this is not a repository-wide matrix or reverse-gap result. Plan completion: not assessed.

Manual scope: FR-197-AC-1 through AC-4 and FR-203-AC-1 through AC-7, with the changed test/source files. FR-197-AC-3's NoFault test exercises the exact 1000-step rational and dyadic fallback vector. FR-203-AC-2 has a defaults test but its separate straddling/max-states acceptance scenario is not implemented by the added tests; certificate/replay-dependent ACs remain outside this PR's explicitly gated FR-201/202 slice. FR-201/202 facades are accurately described as remaining work in the PR body.

## Verdict

CONDITIONAL. This focused manual pass found an uncovered FR-197-AC-2 criterion in the added tests. The full computed matrix and repository-wide reverse-gap/stub checks are unavailable because the Quoin/Quire matrix processes stalled.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No changed test is tagged to or exercises FR-197-AC-2's §15.2 exact probabilities and workload/scheduler transform agreement. The added finite-horizon tests use a small synthetic product; the integration test covers only FR-197-AC-3. | qsl-analyze/src/exact_probabilistic/finite_horizon.rs:602-650; qsl-analyze/tests/it/exact_probabilistic.rs:115-150 |

## Coverage

Focused manual check only; Quoin matrix unavailable after a 60-second stall. Repository-wide reverse-gap and stub checks were not completed. Plan completion: not assessed.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `25df1ec7f76f3b260de93bee8ee6041ffd6cf0bd` adds `p95_probabilities_match_the_exact_scheduler_transform`, tagged FR-197-AC-2. It checks both exact §15.2 probabilities (`24233/25000`, `4491/5000`) and verifies the XF-2 transformed scheduler margin and verdict agree with each workload result. |
