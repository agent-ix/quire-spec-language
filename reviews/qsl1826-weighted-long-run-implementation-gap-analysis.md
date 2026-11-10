---
id: SR-2026111010282992002
title: QSL-1826 weighted long-run implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2657; weighted long-run runtime and CLI; FR201 AC31-34
review_set: subset
---

## Summary

Independent gap-analysis review of PR2657 original frozen cd6bfd7f0aa5a6009f59e9f3f4f3ac4c0961647f, base 479b5638f6e183d9883ad91c7548341b00cb23bc. Ticket QSL-1826. Rust lane is folded into code-review.

## Verdict

**CONDITIONAL — scoped changed behavior has no additional coverage defect; full matrix and all-expansion assurance are unavailable.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable active AssuranceProfile found by anchored local spec type search. No exception used. Direct delegated subset review; no enforced workflow or whole repository acceptance claimed.

## Coverage

Reviewed all 12 changed Rust source/test files at the original frozen head, against merged weighted profile and admission-precedence clarification. No spec, dependency, lock, CI, gate or license change exists in this runtime PR. API/implementation/safety/wire/testing Rust rules and repository AGENTS/CLAUDE/README were inspected. The only defect is the named-wire-payload style violation; no new numerical acceptance, API compatibility, key identity or typed failure defect was found.

Independent mathematics: per-draw rewards remain separate in ModelSystem::weighted_steps until the checker computes w(s)=sum p(e)r(e); it uses the CURRENT-state predicate. One existing iterative SCC/membership/normalization/harmonic engine checks both profiles. Every reachable weighted BSCC must have a positive w state; nonnegative weights and irreducibility imply a positive stationary denominator. The centered equation h=w*(P-rho)+Ph then fixes rho without importing producer stationary/linear algebra. Transported gain equals rho, first canonical bias is zero, every state in a BSCC has that component value, and transient values use the original unweighted harmonic extension. Periodicity does not invalidate these equations. Initial threshold comparison is exact and inclusive; native producer does not turn failed proof bounds into refutations.

Actual checked reward/model/unit identity is rederived; named reward evaluation is factored from reward checking without reusing its acyclic/stopped-target enumeration. Integer representation and exact numeric weight limits are separate; weights may exceed one. Selected weight units govern magnitudes/bias while the ratio and threshold are dimensionless. Terminal stutter has named weight zero even when P holds. ZeroWeightComponent retains the first complete product key in library/CLI diagnostic vocabulary. Existing FR187 model refusals take precedence; deferred object-parameter negatives remain explicit unsupported and undefined rewards/predicates retain nonacceptance.

Wire: separate additive weighted wrapper preserves existing unweighted literals and bytes. Strict typed objectives reject unknown fields; unit is required RawValue so missing and explicit null differ. Shared canonical Writer emits whole documents; unchanged component wire is value/gain/bias. Complete binding-bearing keys, strict rational/member/row ordering, independent predicate/model identity and component coverage are checked. The source facade now admits both strict long-run identities before checked compilation, while final evidence remains independently checked. No analyzer/provider import authorizes checker acceptance.

Resource/lifecycle: raw/current/internal candidate/post/full-product calls use landed caller admission; added magnitude arithmetic receives the same context meter and storage. Actual cancellation polling and typed causes are retained. Unit, draw, SCC, membership and post-membership row phase tests use semantic events rather than fixed poll offsets. Existing external ModelTransition private canonical-reader and ordinary interpreter/model allocation boundary remains explicitly outside raw-state scope; no full FR201AC30/all-expansion memory qualification is claimed.

Evidence accepted from the exact validation manifest/logs, not rerun: 33 long-run/shared callers, 3 weighted fresh processes, 8 reward processes, 4 unweighted long-run processes and 14 codec controls, scoped all-target Clippy/fmt/diff checks pass. The positivity-guard-only counterfactual falsely accepted an otherwise well-formed matching zero-world certificate; production was restored byte-exact before final green runs. No aggregate CI, full workspace, Kani, cargo-deny or unrelated suites were run by the reviewer. Scoped/thermal dispatch supersedes broad default gate commands.

Computed repository matrix was attempted with quire matrix --scope <worktree> --format json; after several minutes it emitted registry duplicate-archetype advisories but zero JSON and was interrupted (130). Full repository traceability/run-evidence assurance is unavailable, not PASS. Manual scoped trace/AC examination is separate: actual #[trace] tags bind weighted oracles and adverse/control paths to TC1402/FR201AC31-34. No matrix was handwritten. Optional repository-wide semantic campaign was not invoked; required scoped correctness/test-intent review was performed. Plan completion: not assessed.

## Examined scope

```yaml
scope:
- id: FR-201-AC-31
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: FR-199-AC-3's Steady availability graph, with up-to-up 1999/2000, down-to-up 9/10, outgoing duration 1 at up and 3 at down, accepts the weighted profile with component value/gain 600/601, both state values 600/601, bias 0 at up and -2000/601 at down in that canonical key order. Both bound sides accept at 600/601; AtMost 999/1000 accepts and AtLeast 999/1000 rejects ThresholdMissed, while AtMost 599/600 rejects. Unweighted value/gain 1800/1801 or bias -2000/1801 fails.
- id: FR-201-AC-32
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A periodic chain alternates P state a and not-P state b; at a two equally probable draws to b have R magnitudes 1 and 3, and b's step to a has R 1. The weighted component accepts rho=gain=2/3 and biases (a,b)=(0,-2/3). Swapping the two a draw weights preserves acceptance; changing 3 to 5 falsifies that certificate. A zero-weight transient s entering this component and an all-P state c with probabilities 1/4 and 3/4, where c has an explicit probability-one self-step of R 1, accepts V(s)=11/12; a globally pooled numerator/denominator substitute 8/9 rejects. A reachable all-zero-weight BSCC, alon
- id: FR-201-AC-33
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-31/32 documents round-trip canonical bytes through a fresh independent checker without analyzer/provider linkage. Wrong checked weight name/model or actual unit, changed predicate set/optimization/profile, foreign binding/model-only keys, malformed rational/JSON/key data and duplicate/missing/out-of-order component/value/bias members prevent acceptance with their specified typed rule/locus. Independently checked equivalent quantity expressions normalize through the actual weight UnitId; another same-dimension descriptor cannot replace it. Gain 0, a numerator payoff in gain, unscaled unweigh
- id: FR-201-AC-34
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and exhaustion/cancellation during actual admission, key construction, graph/draw expansion, predicate/weight/unit evaluation, SCC/membership and exact weighted sums retain the actual typed limit/cancellation cause with no acceptance. Undefined or negative reachable weights, undefined predicates/workload and unsupported reward types preserve typed nonacceptance rather than zero. Unsupported shape/kind/fairness/every-scheduler cases issue no weighted proof. Zero direct over instances preserve UnsupportedBinding/unsupported with no invented evidence. Integer representation and e
- id: FR-199-AC-3
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: examined
  excerpt: "`long-run fraction holds(v.up) weighted by duration >= 0.999` over a variant of `Avail` with `duration` 1 on `tick` and 3 on `repair` weights each position by the step that leaves it, giving `\u03C0_up / (\u03C0_up + 3 \u03C0_down)`, exact. A variant whose only reachable cycle carries `duration` 0 settles `Unsupported(ZeroWeightComponent)`."
- id: TC-1402
  path: spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "Include undefined/negative weights at both recurrent and transient\n   states, absent rewards, unsupported kinds/shapes, and direct zero over\n   instances."
```

## Final evidence and bindings

Final fix head 4fe5ce7e9905f5e9286c1ec6a14094f0e6c4f0bf changes only the developer CLI source-failure representation. Actual 45 affected CLI integration tests pass; one pre-existing exhaustive every-observed-poll cancellation sweep was deliberately skipped after stopping the initial broad run and is NOT reported green. CLI all-target Clippy, fmt and diff checks pass. Original 62 focused mathematical/codec/process controls remain applicable because their production paths are unchanged by this named serialization fix. No reviewer cargo rerun.

```yaml
bindings:
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_availability_checks_ratio_gain_bias_and_strict_objective
  ac_id: FR-201-AC-31
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_availability_checks_ratio_gain_bias_and_strict_objective
  ac_id: FR-201-AC-33
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_availability_checks_ratio_gain_bias_and_strict_objective
  ac_id: FR-199-AC-3
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_draws_and_transients_use_component_fractions_not_pooled_payoffs
  ac_id: FR-201-AC-32
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_draws_and_transients_use_component_fractions_not_pooled_payoffs
  ac_id: FR-201-AC-33
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::reachable_zero_weight_components_remain_typed_unsupported
  ac_id: FR-201-AC-32
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::reachable_zero_weight_components_remain_typed_unsupported
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::weighted_phase_controls_preserve_actual_cancellation_and_limits
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::actual_weight_representation_and_numeric_arithmetic_limits_are_separate
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::global_negative_admission_precedes_proofs_and_zero_binding_invents_nothing
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::admitted_object_parameter_draws_do_not_coerce_negative_weight_to_zero
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::undefined_actual_weights_and_predicates_preserve_typed_nonacceptance
  ac_id: FR-201-AC-34
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::checked_weight_units_and_equivalent_expressions_are_not_display_aliases
  ac_id: FR-201-AC-31
  trace: correct
- test_id: qsl-analyze/tests/it/weighted_long_run_certificate.rs::checked_weight_units_and_equivalent_expressions_are_not_display_aliases
  ac_id: FR-201-AC-33
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_long_run_round_trip_and_members_cross_actual_source_process
  ac_id: FR-201-AC-31
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_long_run_round_trip_and_members_cross_actual_source_process
  ac_id: FR-201-AC-33
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_long_run_strict_scalars_keys_and_objectives_refuse_in_fresh_process
  ac_id: FR-201-AC-33
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_long_run_strict_scalars_keys_and_objectives_refuse_in_fresh_process
  ac_id: FR-201-AC-34
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_zero_component_retains_existing_product_locus_in_fresh_process
  ac_id: FR-201-AC-32
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/roundtrip.rs::weighted_zero_component_retains_existing_product_locus_in_fresh_process
  ac_id: FR-201-AC-34
  trace: correct
```
