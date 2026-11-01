---
id: SR-1988011
title: "Code review of quire-spec-language PR #2006: QSL-574 finite-horizon solver and settlement"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b404f637929f6c9ab387e65f3749a9826c19a058; PR #2006 against 0ba5e46f8a5f1fc5289689b36fd7c8830a5ac49a; qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/{build,finite_horizon,settle}.rs, qsl-analyze/src/{model_check,zone_check}/**.rs, qsl-analyze/tests/it/{exact_probabilistic,model_settle,prob_world,refinement_liveness,zone_replay}.rs, qsl-replay/src/{exact_probabilistic,lib,outcome,proof_result}.rs, qsl-replay/src/{certificate/closed_form/settle,model_check/**}.rs, qsl-replay/tests/it/refinement_smt.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: references
---
# Code review of PR #2006 (QSL-574)

## Summary

Reviewed the pinned diff at head `b404f637929f6c9ab387e65f3749a9826c19a058` against base `0ba5e46f8a5f1fc5289689b36fd7c8830a5ac49a`. The change adds finite-horizon exact/dyadic backward induction, limit defaults, typed product-build settlement, and qsl-replay settlement plumbing. PR description accurately states that FR-201/202 public checker and replay facades are not implemented and remain gated on PR #1987. I did not run tests; reviewed the coder-reported focused checks only.

Rust review: production changes use typed outcomes and exhaustive matches at the relevant settlement boundaries; exact and interval passes preserve scheduler edge order and outward rounding; the cancellation/build-stop mapping preserves the typed cancellation source. No changed dependency or CI lane was found. Quoin matrix stalled and is recorded in the separate gap-analysis artifact.

## Verdict

CONDITIONAL. One medium traceability finding: the tests tagged to FR-197-AC-1 do not assert the acceptance vectors in that criterion. The QSL-574 PR body accurately limits its scope and explicitly leaves FR-201/202 public facades gated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tests tagged FR-197-AC-1 use a toy 1/3 weighted graph and a two-action 0/1 graph; neither checks the specified §15.4 scheduler values or `send_b` choices. The tag therefore overstates coverage of the acceptance criterion. Add the AC-1 vectors or narrow the trace to the behavior actually asserted. | qsl-analyze/src/exact_probabilistic/finite_horizon.rs:602-650 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `25df1ec7f76f3b260de93bee8ee6041ffd6cf0bd` adds `deliver_backward_induction_matches_the_published_scheduler_vectors`, tagged FR-197-AC-1. It asserts the §15.4 minimum values `24/25` and `4/5`, selects `send_b` at both states, asserts maximum `99/100`, and checks the Even workload values `391/400` and `17/20`. |
