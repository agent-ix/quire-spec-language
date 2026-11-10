---
id: SR-202611101240413126931
title: QSL-1826 cyclic MDP expected-reward implementation code-review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; QSL-1826 PR2664; qsl-analyze/src/analyze/exact.rs,
  qsl-analyze/tests/it/cyclic_mdp_reward_certificate.rs, qsl-analyze/tests/it/main.rs,
  qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate/document.rs, qsl-replay/src/probability_certificate/document/mdp.rs,
  qsl-replay/src/probability_certificate/document/writer.rs, qsl-replay/src/probability_certificate/reward.rs,
  tools/qsl-probability-check/src/main.rs, tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs,
  tools/qsl-probability-check/tests/it/main.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
---

## Summary

Independent code-review of frozen PR2664 head 6a13faea3bca7c12c419415bef5e0fc70e7ddd84 against base 14c0d809138ebe041aabfb474c22f296e5be6cc2. Ticket: QSL-1826. Rust lane is folded into code-review.

## Verdict

PASS — no concrete implementation or Rust-rule defect found in the scoped review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable active AssuranceProfile found. No selected exception or whole-repository assurance claim.

## Coverage

All eleven changed Rust files were inspected against FR-201 AC43–46, plus reused reward enumeration, exact action_sum, source checking and candidate/solver callers. No dependency, lockfile, CI workflow, specification or plan edit occurs. Applied code-review with its Rust lane, rust-review, rust-style API/implementation/safety/wire/testing references and repository AGENTS/CLAUDE. No active AssuranceProfile was found by anchored type search. This is runtime-only: spec-review methods do not apply. No optional repository-wide semantic pass or plan was supplied.

The independent checker admits the full original model separately from initial first-B stopped reachability, recomputes each semantic action's sum of per-draw probability times checked R before transition coalescing, and checks the checked model/name/comparison UnitId. Targets, identity, bindings, complete values/rank coverage and closed row/profile encoding remain independently reconstructed. It does not consume a producer's policy or infinity labels. Reused shared action_sum and RewardGraph replace copied arithmetic; shared canonical writer and existing UnitId owner encoding retain one source of truth. Existing acyclic and DTMC eligibility precedes the MDP fallback. Only Unsupported/predicate/cycle conditions retry; actual admission, expansion, accounting, limits and cancellation propagate. A single Meter/storage ledger spans admission retries and original/stopped graphs.

For reward, only B has zero rank; every non-B, including V=0, needs positive rank. MIN requires some tight minimizing action with a positive-probability lower-rank successor; MAX requires such a successor for every nonempty actual action. Non-B terminals fail properness even when Bellman zero holds. In a finite graph, a minimum-rank member excludes every non-B closed class in the chosen tight MIN policy, or in any MAX policy. Thus properness makes the Bellman solution finite and attaining; zero-cost infinite waiting cannot certify a smaller MIN fixed point or finite MAX. The source solver's selected infinity at any positively reachable stopped state causes typed unsupported mixed-infinite coverage rather than omitted rows or fabricated zero. Initial B and post-B query evaluation are correctly stopped; entry cost is paid once.

Independent thermal check: cargo test -p qsl-probability-check --test it cyclic_mdp_reward passed all 5 new tests, exit0, /tmp/qsl1826-mdp-reward-review/process-test.log. The test assertions reject bogus MIN0/.5 and MAX1 specifically as RankBroken with named key, beyond merely checking nonzero process status; valid controls accept. Source cost changes fail equations and threshold misses retain ThresholdMissed. The false-terminal and infinite-trap rows fail properness while omitted rows fail coverage. Exact 2/4 extrema, positive-rank zero, initialB and unit identity controls exercise the real source facade. Coder logs inspected: native2, codec/rank3, combined cyclic process14, old acyclic reward8 pass; three-package all-target/all-feature Clippy -Dwarnings and formatting pass. Actual phase callbacks Request/Deadline are checked for reachability and typed cancellation; scalar/work/storage/state/edge/byte exhaustion returns no acceptance. Representative tests are not exhaustive satisfaction of every enumerated subclause. No branch mutation or unnecessary exhaustive poll sweep was performed. No aggregate, makeci, Kani, cargo-deny or unrelated full-suite gate was run under the explicit lane override.

The earlier source fixture with computed compound reward and a literal compound-dimension threshold fails existing S3 threshold-units-declared admission; root retained that source-admission repair outside this profile increment. Actual simple declared-unit conversion and foreign equal-dimension/equal-scale UnitIds are tested end to end. Compound UnitId encoding is tested only as canonical codec round trip; it is not admitted compound-source qualification. This limitation is not hidden by the removed fixture and does not change source admission. The external ModelTransition private unmetered canonical reader and ordinary interpreter allocations remain outside raw STATE-key accounting scope: no full FR201AC30/all-expansion, whole parent or FR413 facade completion is claimed. The main test exact_probabilistic_item::analyzer_document_budgets_stop_after_solving_without_certification is independently owned by codex1; this branch does not touch that file. Its failing callback assertion predates earlier candidate storage accounting, and this review neither claims to fix nor requalifies that unrelated test.

The full computed matrix was attempted with quire matrix --scope . --format json, emitted zero JSON and was interrupted; stderr retained. No hand-written matrix or coverage percentage replaces it. New profile behavior has owning FR201 AC43–46 and TC1405 trace attributes. No concrete newly unspecified behavior, stub, ignored new test or inflated assertion was found. Repository-wide reverse-gap and qualification assurance remain incomplete.

Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: FR-201-AC-43
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A non-B s chooses actions a or b, each with self/B probabilities 1/2 and
    1/2 and costs R=1 or R=2 on both draws respectively. The new cyclic MDP reward
    profile accepts minimum V(s)=2 under Every AtLeast 2 and maximum V(s)=4 under
    Every AtMost 4, with V(B)=0 and ranks (s,B)=(1,0). Bounds 5/2 and 7/2 respectively
    reject ThresholdMissed. Distinct action costs cannot be averaged despite identical
    successor distributions. Changing entry-B draw R changes the required value; actual
    unit conversion accepts an equivalent bound, while a foreign dimension-equivalent
    UnitId refuses objective mismatch.
- id: FR-201-AC-44
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For WAIT self R=0 versus GO to B R=1, minimum V(s)=1 accepts with rank
    1. Bogus minimum V(s)=0 or 1/2 satisfies Bellman equality through WAIT but rejects
    RankBroken because no tight action progresses. Maximum is infinity: no finite
    native document is emitted, and bogus maximum V(s)=1 rejects RankBroken despite
    Bellman equality. A zero-cost positive-exit retry accepts V(s)=0 with positive
    rank at bound 0; rank 0 rejects. Non-B terminals and closed zero-cost classes
    have infinity and cannot pass vacuous universal progress. Minimum initial safe
    exit cost 1 versus another action to an infinite non'
- id: FR-201-AC-45
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-43/44 documents round-trip canonically through a fresh checker without
    analyzer/provider linkage. Entry-B cost is counted once, initial B has V/rank
    zero, and an otherwise admitted post-B cycle adds no rows or costs. Wrong checked
    reward/model/unit, targets, optimization/profile, identity or binding; changed
    actual draw R; false values/progress; malformed rational/JSON/full-key data; missing/duplicate/out-of-order/extra
    rows and model-only keys prevent acceptance under the precise existing rule/locus.
    Old acyclic and DTMC bytes/verdicts, producer preferences and sealed acceptance
    remain unc
- id: FR-201-AC-46
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic
    exhaustion during original/stopped admission, actual raw/internal/full keys, all-action/draw/B/reward/unit
    reconstruction, exact sums, tightness and universal/existential progress preserve
    the typed cause without accepted truth. Earlier model/negative-reward refusal
    and deferred undefined or unsupported evaluation are not coerced to zero or omitted
    actions. Unsupported shape/fairness/workload/timed/multi-post/refutation/infinity
    forms issue no new-profile proof; zero direct i
bindings:
- test_id: qsl-analyze/tests/it/cyclic_mdp_reward_certificate.rs::native_mixed_infinity_is_unsupported_even_with_finite_initial_minimum
  ac_id: FR-201-AC-44
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_reward_certificate.rs::native_mixed_infinity_is_unsupported_even_with_finite_initial_minimum
  ac_id: FR-201-AC-45
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_reward_certificate.rs::initial_target_stops_query_and_real_reward_phases_preserve_cancellation
  ac_id: FR-201-AC-45
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_reward_certificate.rs::initial_target_stops_query_and_real_reward_phases_preserve_cancellation
  ac_id: FR-201-AC-46
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/mdp.rs::cyclic_mdp_reward_codec_retains_compound_unit_and_large_finite_values
  ac_id: FR-201-AC-45
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::same_distribution_distinct_costs_check_both_extrema_and_thresholds
  ac_id: FR-201-AC-43
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::same_distribution_distinct_costs_check_both_extrema_and_thresholds
  ac_id: FR-201-AC-45
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::zero_wait_cannot_justify_tight_minimum_or_universal_maximum_progress
  ac_id: FR-201-AC-44
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::zero_wait_cannot_justify_tight_minimum_or_universal_maximum_progress
  ac_id: FR-201-AC-45
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::proper_zero_reward_keeps_positive_rank_and_strict_rows_are_closed
  ac_id: FR-201-AC-44
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::proper_zero_reward_keeps_positive_rank_and_strict_rows_are_closed
  ac_id: FR-201-AC-45
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::mixed_infinite_minimum_and_false_terminal_forgery_never_become_finite_proofs
  ac_id: FR-201-AC-44
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::mixed_infinite_minimum_and_false_terminal_forgery_never_become_finite_proofs
  ac_id: FR-201-AC-45
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::checked_comparison_unit_converts_exactly_and_rejects_foreign_unit_identity
  ac_id: FR-201-AC-43
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp_reward.rs::checked_comparison_unit_converts_exactly_and_rejects_foreign_unit_identity
  ac_id: FR-201-AC-45
  trace: correct
```
