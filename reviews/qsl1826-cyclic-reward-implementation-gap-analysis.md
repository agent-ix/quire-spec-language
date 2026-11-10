---
id: SR-202611101125113154242
title: QSL-1826 cyclic first-hit expected-reward implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2660; qsl-analyze/src/exact_probabilistic/build.rs, qsl-analyze/tests/it/cyclic_reward_certificate.rs, qsl-analyze/tests/it/main.rs, qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate.rs, qsl-replay/src/probability_certificate/document.rs, qsl-replay/src/probability_certificate/document/writer.rs, qsl-replay/src/probability_certificate/reward.rs, tools/qsl-probability-check/tests/it/cyclic_reward.rs, tools/qsl-probability-check/tests/it/main.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
- target: ix://agent-ix/quire-spec-language/FR-198
  type: references
---

## Summary

Independent gap-analysis of frozen PR2660 head 89ed24a7728f3a313d0b4c3f2aaee9cfbfd4144f against base b3084437feb865b5632926289d09203524cec022. Ticket: QSL-1826. Rust review is folded into code-review; gap-analysis is a separate method.

## Verdict

**CONDITIONAL — no concrete new scoped gap found; repository matrix and full qualification remain unavailable.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable active AssuranceProfile found. Direct delegated subset review; no enforced workflow or whole-repository assurance claim.

## Coverage

Reviewed every changed file and the shared call paths needed to establish behavior. Repository AGENTS.md, CLAUDE.md and README.md, code-review with Rust lane, rust-style API/implementation/safety/wire/testing references and gap-analysis were applied. No specification/plan, dependency, lockfile, workflow or enforced gate changes occur. Spec-review methods are inapplicable to this runtime-only diff; optional repository-wide semantic review was not invoked. Scoped numerical and test-intent review was performed as dispatched.

Independent soundness argument: on the finite initial-reachable first-B stopped graph, B has value and rank zero. Every other state must have positive rank and a positive-probability lower-rank successor, including zero-valued states and false terminals. Following decreasing ranks yields a finite positive-probability path to B from every state, excluding any non-B bottom class. Finite DTMC absorption is therefore almost sure with finite expected time; finite nonnegative checked draw costs give a unique finite reward Bellman solution. The checker verifies that solution and inclusive initial threshold independently. It does not require offered ranks to equal the producer's reverse BFS distances, and does not assign non-reaching reward states a probability-style zero boundary. Closed zero-cost nontermination cannot obtain acceptance merely by satisfying zero Bellman equations.

The full original model pass uses no target predicate and disables query cost collection. Existing model/Markov/workload admission and identity controls remain active beyond B. The subsequent stopped pass classifies B before visiting costs or expanding it: entry reward is counted on the incoming draw, after-B nodes are absent, and solely post-B query predicate failures do not become relevant evidence failures. Actual draw ordering is retained between independently expanded graph distributions and checked reward rows; different draw costs are not lost through state coalescing. Checked reward/model/UnitId identity and selected magnitude normalization are reused rather than copied; values above one and zero thresholds remain reward semantics.

Only typed query-predicate failures and old-profile unsupported/cycle shape may retry the cyclic reward route. Model admission/expansion failures, accounting, storage and cancellation are not retry discriminants. The actual caller Meter, cumulative storage and Cancel survive both attempts and original/stopped/cost passes. Progress/rank loops meter edges; cancellation is polled through actual controls. Candidate stopping is private to the budgeted ExpectedReward build path; legacy public product behavior retains its old mode. The shared canonical writer handles the additive profile with the previously invalid cyclic_dtmc+reward combination; existing public document literals need no new fields. Strict objective/member/rational/key/row/coverage checks are shared rather than duplicated. No new solver dependency reaches the checker.

Evidence read from /tmp/qsl1826-cyclic-reward-evidence.json and actual logs: seven source integration tests, eleven reward process tests (eight legacy plus three cyclic), thirty-three long-run regressions and fourteen codec controls passed. Three-package all-target Clippy with test-support and -D warnings, fmt and diff checks passed. The properness-only counterfactual log shows an otherwise well-formed closed-zero forgery wrongly accepted when that rejection alone was removed; the production tree is clean at the frozen head and final checks postdate restoration. Existing focused evidence was accepted under the thermal brief; no reviewer Cargo rebuild, aggregate CI, workspace/Kani/cargo-deny or unrelated suites were run. The counterfactual was inspected, not independently repeated.

No applicable AssuranceProfile was found by anchored type search under spec. No exception was granted. Existing external ModelTransition private canonical reads and ordinary interpreter/model allocations remain outside the documented raw STATE-key qualification boundary; this is not a full FR-201-AC-30 or all-expansion memory qualification. Scoped positive/negative tests exercise representative subclauses of AC35-38; tags do not establish every enumerated TC1403 variation or exhaustive budget/shape qualification. No entire AC, ticket or repository completion is asserted.

Reconciliation: quire matrix --scope . --format json was attempted against this worktree (quire 0.36.2 engine 0.50.2). It produced registry advisories but zero JSON and was interrupted with SIGINT (130); full repository matrix/run-evidence results and status counts are unavailable. Manual changed-surface trace/AC inspection is recorded separately below; it is not a computed coverage figure. A full repository reverse-gap/stub campaign was not completed; changed production behavior maps to FR201's merged cyclic-reward contract and has no identified new unowned behavior or hollow implementation. No matrix was handwritten.

Plan completion: not assessed

## Examined scope

Scope excerpts are verbatim source substrings capped at 600 characters. Examined means inspected, not complete execution evidence for every subclause.

```yaml
scope:
- id: FR-201-AC-35
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'A DTMC retry state s has self and B-entering draws each of probability 1/2 and R=1 on both. The cyclic reward profile accepts V(s)=2, V(B)=0 and ranks (s,B)=(1,0), at both workload equality bounds. AtMost 3/2 and AtLeast 5/2 reject ThresholdMissed. Changing only entry-B R to 3 accepts V(s)=4; offering 2 rejects InequalityFails. Swapping equally probable draw costs preserves 4, while changing their mean falsifies it. Costs 2 ms on self and 6 ms on entry give 8 ms and accept an equivalent 0.008 s bound through actual checked conversion; declared/compound descriptors retain their actual identity '
- id: FR-201-AC-36
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A zero-cost retry with positive exit probability accepts V(s)=V(B)=0 and ranks (1,0), including either bound at 0. Giving s rank 0 rejects RankBroken despite zero values. A closed non-B zero-cost self-loop or cycle rejects otherwise Bellman-valid finite V=0 evidence as RankBroken; the producer preserves FR-198 infinity without a finite document. Initial B accepts only its zero value/rank even when its original outgoing operation has positive cost. An admitted A-to-B subject with a positive-cost cycle reachable solely after B accepts first-hit evidence with rows only for A and B; adding after-B
- id: FR-201-AC-37
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-35/36 documents round-trip canonically through a fresh checker without analyzer/provider linkage. Changed actual reward name/model/unit, target set, optimization/profile or binding prevents acceptance; dimension-equivalent foreign UnitIds are not aliases. Noncanonical rational/JSON/key data, malformed or duplicate/missing/out-of-order rows, model-only keys, false Bellman values, and absent/wrong progress ranks retain precise rule/locus diagnostics. The old acyclic reward and probability/long-run profile bytes/verdicts remain unchanged; an eligible acyclic producer keeps its existing profile
- id: FR-201-AC-38
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic exhaustion during original/stopped admission, raw/internal/full keys, draw/B/reward/unit evaluation, progress checking and exact sums preserve the actual typed cause with no accepted truth. Earlier model refusals and deferred undefined/negative evaluation are not coerced to zero. Unsupported kind/shape/fairness/every-scheduler, multi-post or refutation/infinity requests emit no finite cyclic reward proof. Direct zero over instances retain UnsupportedBinding/unsupported without a fab
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: examined
  excerpt: "EN-5 SHALL give the value `+\u221E` at every state that reaches `B` with\n  probability below 1 under some scheduler, for a maximum, or under every\n  scheduler, for a minimum, decided by the graph alone; under a workload,\n  at every state outside `Prob1`."
- id: TC-1403
  path: spec/test-cases/TC-1403-check-cyclic-dtmc-expected-reward-certificates.md
  role: context_only
  excerpt: 'The checker

    independently derives the first-B stopped graph, actual draw costs and

    rank progress; producer values and qualitative classifications are untrusted.'
bindings:
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::geometric_rewards_include_entry_once_and_ignore_post_target_cycles
  ac_id: FR-201-AC-35
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::geometric_rewards_include_entry_once_and_ignore_post_target_cycles
  ac_id: FR-201-AC-37
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::geometric_rewards_include_entry_once_and_ignore_post_target_cycles
  ac_id: FR-198-AC-3
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::zero_reward_still_requires_properness_and_initial_target_is_zero
  ac_id: FR-201-AC-36
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::zero_reward_still_requires_properness_and_initial_target_is_zero
  ac_id: FR-201-AC-38
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::predicate_undefined_only_after_target_is_not_evaluated
  ac_id: FR-201-AC-36
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::cyclic_reward_units_normalize_to_selected_comparison_unit
  ac_id: FR-201-AC-35
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::cyclic_reward_units_normalize_to_selected_comparison_unit
  ac_id: FR-201-AC-37
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::actual_cyclic_reward_budget_and_cancellation_causes_remain_typed
  ac_id: FR-201-AC-38
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::phase_observed_stops_use_original_token_in_every_cyclic_reward_pass
  ac_id: FR-201-AC-38
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_reward_certificate.rs::fallback_retains_the_same_cumulative_storage_quota
  ac_id: FR-201-AC-38
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::cyclic_reward_roundtrip_stops_before_post_target_cycles
  ac_id: FR-201-AC-35
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::cyclic_reward_roundtrip_stops_before_post_target_cycles
  ac_id: FR-201-AC-36
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::cyclic_reward_roundtrip_stops_before_post_target_cycles
  ac_id: FR-201-AC-37
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::closed_zero_cost_finite_forgery_rejects_at_actual_key
  ac_id: FR-201-AC-36
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::closed_zero_cost_finite_forgery_rejects_at_actual_key
  ac_id: FR-201-AC-37
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_reward.rs::cyclic_reward_strict_rows_members_and_objective_cannot_alias
  ac_id: FR-201-AC-37
  trace: correct
```
