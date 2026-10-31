---
id: SR-1933006
title: "Gap analysis of QSL-559 PR #1940: instance-bounded protocol liveness"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-559; qsl-analyze/src/analyze/items.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/model_check/liveness.rs, qsl-analyze/tests/it/protocol_roles.rs; PR #1940 at be07deb2c51d9ea332d81560ef8cabe78bd0cb02"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
---
# SR-1933006: Gap analysis of QSL-559 PR #1940

## Summary

Ticket: QSL-559. This review checks the changed liveness behavior and its FR-209 trace tags against the tracked criteria. The repository-wide computed matrix did not complete: quire matrix --scope . --format json remained running without output for over four minutes and was stopped (signal termination; no exit status from the CLI was produced). Manual checks of the changed criterion bindings were completed from the spec and source.

## Verdict

**CONDITIONAL.** The changed liveness paths have tagged tests for FR-209-AC-5, AC-6, and AC-7. The matrix could not be computed, so this is not a repository-wide matrix verdict. The AC-6/AC-7 test-intent gap is recorded in the code-review artifact SR-1933005.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional static traceability or reverse-gap finding in the manually checked PR scope (placeholder). | - |

## Coverage

- Matrix: attempted with quire matrix --scope . --format json; no output or completion after more than four minutes, so the repository-wide matrix is unavailable for this pass.
- Manual trace check: FR-209-AC-5 is tagged by liveness_over_on_each_settles_instance_bound_after_no_fair_lasso; AC-6 by a_fair_liveness_lasso_may_pass_through_an_instance_limited_state; AC-7 by an_on_each_repeat_self_loop_is_fair_at_the_instance_bound.
- Reverse-gap check: the changed bound propagation, instance-limited state tracking, and settlement path are owned by FR-209's liveness-under-the-bound behavior. No new unowned production behavior or stub was found in the diff.
- Semantic review: skipped; it was not opted into.
- Plan completion: not assessed
