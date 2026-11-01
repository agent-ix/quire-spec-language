---
id: SR-1988010
title: "Gap analysis — QSL-588 timed statistical sampling PR 1998 rebase"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-253-AC-1..4, FR-254-AC-1..4; qsl-analyze/src/statistical/, qsl-analyze/tests/it/statistical_model.rs, qsl-analyze/tests/it/statistical_witness_replay.rs, qsl-eval/src/simulation/model/timed/, qsl-eval/src/simulation/sampled/timed.rs, qsl-eval/tests/it/timed_subject.rs, qsl-semantics/src/check/probabilistic/claim.rs, qsl-semantics/tests/it/timed_declarations.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Audited the QSL-588 criteria and changed implementation/tests against FR-253, FR-254, TC-708, and TC-709. The scheduler-choice, exact-probability, sampling, and replay paths are covered, but FR-254-AC-3 bundles timed regeneration and exact-route outcomes that have no matching in-scope test oracle.

## Verdict

**CONDITIONAL** — one medium test-evidence gap remains within FR-254-AC-3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-254-AC-3's tagged tests cover elapsed reading, free-delay refusal, and the sample-step budget, but do not assert timed regenerative `max_cycle_steps` settlement or exact-evidence routing for the strict-guard timed subject. | qsl-analyze/tests/it/statistical_model.rs:246, 465, 509; qsl-analyze/tests/it/dense_route.rs:96 |

## Coverage

- FR-253-AC-1: exact conditioned distributions and windows are asserted.
- FR-253-AC-2: path probabilities and tie order are asserted and directly trace-tagged.
- FR-253-AC-3: malformed declarations and unbounded uniform refusal are asserted in timed declaration tests.
- FR-253-AC-4: the free timeout remains `None`, is exposed as a scheduler-choice MDP after the stochastic draw, and sampled use refuses with `NotStochastic`.
- FR-254-AC-1: both exact-discrete and uniform measured outcomes and deterministic same-seed verdicts are asserted.
- FR-254-AC-2: exact rational grid draws, witness replay, and altered-delay rejection are asserted.
- FR-254-AC-3: elapsed timestamp difference, sampled free-delay refusal, and `max_sample_steps` are tested. The criterion's timed regenerative no-return case and strict-guard exact-route case are not asserted by these tests. The existing `dense_route` test checks temporal routes, not statistical exact routing.
- FR-254-AC-4: the local time-lock produces unsupported with no witness.
- Typed fairness integration: the `ProbScheduler::Every` fairness-set round trip test passed in the changed semantics target; the sampler explicitly admits workloads only.
- Trace bindings: all eight FR acceptance criteria have direct tags in tests. `TC-708` binds AC-1/2 in the race test, AC-3 in declaration tests, and AC-4 in the MDP test. `TC-709` binds AC-1/2 in statistical sampling/replay tests, AC-3 in elapsed/free-delay/budget tests, and AC-4 in the local-lock test. A matrix binding alone cannot establish the missing AC-3 sub-oracles.
- Matrix limitation: `quire matrix --scope . --format json` emitted only module warnings and no matrix result before interruption; `quoin matrix --repo . --json` produced no output before interruption. The focused trace/source check above is manual. Repository-wide untagged criteria, stale tags, reverse-gap, and stub counts therefore remain unavailable; this artifact makes no repository-wide completeness claim.
- EN-5 protected-path check: no paths under `qsl-analyze/src/exact_probabilistic/` or exact-certificate/witness implementation changed.
- Plan completion: not assessed.

## Bindings

- `TC-708` → `FR-253-AC-1`, `FR-253-AC-2`: correct and direct.
- `TC-708` → `FR-253-AC-3`: correct.
- `TC-708` → `FR-253-AC-4`: correct and direct.
- `TC-709` → `FR-254-AC-1`, `FR-254-AC-2`: correct and direct.
- `TC-709` → `FR-254-AC-3`: correctly bound, but the test set does not assert all of the criterion's bundled outcomes (FND-001).
- `TC-709` → `FR-254-AC-4`: correct and direct.
