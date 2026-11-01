---
id: SR-12250
title: "QSL-560 PR #1986 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d370a0e670f7bc978980a82dab11df3dba23ab8f; PR #1986 diff origin/main...HEAD; FR-212 and FR-217"
review_set: subset
relationships:
  - {type: references, target: "ix://agent-ix/quire-spec-language/spec"}
---

## Summary

Ticket: QSL-560. Planless gap analysis. The `quoin review` workflow could not run because this Quoin CLI requires an installed skill path (`quoin review --target gap-analysis` returned “pass --path <installed-quoin-skill-directory>”). The required full `quoin matrix --repo . --json` / `quire matrix --scope . --format json` did not finish during this review window, so I manually checked the FR-212/FR-217 criteria against the changed test bindings and behavior. No plan was supplied.

FR-212-AC-1 and FR-212-AC-2 are not directly bound by trace tags: the matching tests in `qsl-analyze/tests/it/scheduler_fairness.rs` carry `TC-657` but no FR-212 AC id. FR-212-AC-3 and AC-4 have direct tags. FR-217-AC-4 is directly tagged and exercises liveness plus replay with the scheduler constraints. The new compensation fairness test exercises authored and scheduler classes over actual compensation transitions, but does not drive those constraints through liveness filtering or lasso replay.

## Verdict

FAIL. FND-001 is a high-severity untagged-criterion gap under the matrix contract. FND-002 is a medium gap in end-to-end evidence for the newly authored compensation target path. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-212-AC-1 and FR-212-AC-2 have no direct trace-tagged test binding; the apparent tests carry only TC-657 (and FR-217-AC-4 for the latter). | qsl-analyze/tests/it/scheduler_fairness.rs:301-358 |
| FND-002 | medium | The compensation-target test checks resolution, class membership and scheduler membership, but does not test liveness filtering or lasso replay for an authored compensation target. | qsl-analyze/tests/it/scheduler_fairness.rs:732-770 |

## Coverage

| Unit | Role | Evidence |
| --- | --- | --- |
| FR-212-AC-1 | examined | `chatter_carries_three_scheduler_constraints_and_no_fair_counterexample` checks three scheduler constraints, a component proof, and `Proved`; its trace attribute names TC-657 only. |
| FR-212-AC-2 | examined | `scheduling_adversarial_admits_the_lasso_that_starves_a_branch` checks the adversarial lasso and replay; its trace attribute names TC-657, TC-662 and FR-217-AC-4, but not FR-212-AC-2. |
| FR-212-AC-3 | examined | `protocol_targets_resolve_and_classes_hold_their_threads_steps` is tagged FR-212-AC-3 and checks branch/attempt identity, emission, and refusal behavior. |
| FR-212-AC-4 | examined | Test `safety_is_unchanged_by_the_scheduler_opt_out` is tagged FR-212-AC-4. |
| FR-217-AC-4 | examined | The shared lasso test is tagged FR-217-AC-4 and exercises the liveness and replay path for scheduler constraints. |
| FR-212 compensation fairness | context_only | `compensation_fairness_is_authored_and_scheduled_end_to_end` checks typed target and class identity only; it carries TC-657 and no FR criterion id. |

## Plan completion

Plan completion: not assessed.

## Limits

This was a manual trace-tag and acceptance check because the Quoin review workflow is not installed for this CLI session. The full computed matrix did not return before the review was completed. No plan-completeness or reverse-gap claim is made.
