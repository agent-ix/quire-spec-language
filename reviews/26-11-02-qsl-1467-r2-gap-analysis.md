---
id: SR-2026110215392
title: "Gap analysis round 2 — QSL-1467: settle a construction budget stop as FR-126's Stopped"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2118 fix round 0b1d9029b..08b58f4f9; qsl-analyze/src/model_check/check.rs; qsl-replay/src/model_check/settle.rs; qsl-analyze/tests/it/weak_memory.rs; qsl-eval/src/simulation/protocol/access.rs; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md; spec/test-cases/TC-665-access-orderings-and-classification.md"
review_set: subset
---

## Summary

Ticket: QSL-1467. PR: quire-spec-language#2118, head 08b58f4f9. This is a planless gap analysis (plan completion was not assessed). It is scoped to the fix round: FR-220-AC-16, its two new integration tests, the re-tagged unit test, and the code they back. It includes mutation spot checks in a scratch worktree at 08b58f4f9. No full gates were run.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-220-AC-16 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:284 | examined |
| FR-220 new SHALL (budget stop in EN-1 instantiation settles FR-126 Stopped) | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:177-182 | examined |
| a_candidate_bound_reached_while_classifying_is_a_resource_stop | qsl-analyze/tests/it/weak_memory.rs:2750-2775 | examined |
| a_candidate_bound_reached_listing_potential_identities_is_a_resource_stop | qsl-analyze/tests/it/weak_memory.rs:2781-2806 | examined |
| assert_candidate_stop / deadlock_within helpers | qsl-analyze/tests/it/weak_memory.rs:191-265, 2703-2745 | examined |
| a_budget_stop_is_a_stop_and_a_broken_one_a_fault (re-tagged) | qsl-eval/src/simulation/protocol/access.rs:1415-1449 | examined |
| construction_refusal, expansion_stopped | qsl-analyze/src/model_check/check.rs:915-938, 1032-1050 | examined |
| settle_protocol_check warnings arm | qsl-replay/src/model_check/settle.rs:290-311 | examined |
| FR-126 expansion-stop SHALL | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:306-313 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. AC-16 is fully backed.

- Both integration tests assert all of AC-16's outcome. They check the `InitialStop` stop with limit `MaxCandidates` and code/cause `resource_exhausted`/`insufficient-next-charge`. They check the item outcome `Stopped{ResourceExhausted, Some({MaxCandidates, bound})}` with `product_states == 0`. They check the record value `Incomplete(LimitReached{MaxCandidates, bound})`, `limit_reached`, and the setting `expansion.max_candidates`. Both run under `tso` and `ra`. The controls are present too: the default bound builds under both models, 6 with no absent keys builds, and 12 with ten absent keys builds.
- Trace tags are now `#[trace("TC-665", "FR-220-AC-16")]` on all three tests, matching TC-665 step 15. This resolves round-1's trace finding.
- Mutation spot checks, run in a scratch worktree at 08b58f4f9:
  - (a) `construction_refusal`'s `InitialStop` arm disabled (`if false`), so the stop refuses. Both new integration tests FAIL with `the protocol subject checks: Protocol(InitialStop(.. MaxCandidates ..))`. Confirmed.
  - (b) The settle.rs `Err(Some(_)) if Stopped` arm disabled. The `max_candidates`-0 test FAILS at the `record.value` assertion (weak_memory.rs:2734), because under `ra` the FR-226 pre-check stops and the record turns `failed`. The potential-identities test passes, as expected: at bound 6 the Enabled pre-check fits. Confirmed.
- The Potential-mode claim (reverting the Potential `map_err` fails the potential-identities test) was not re-run. It is plausible by construction: with no absent keys at bound 6, Enabled fits, so only the Potential listing of twelve `Cell::set` identities can stop.
