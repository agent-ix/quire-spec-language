---
id: SR-202611101213245722622
title: QSL-1826 cyclic MDP eventual implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2663; qsl-analyze/src/analyze/exact.rs,
  qsl-analyze/src/exact_probabilistic/build.rs, qsl-analyze/tests/it/cyclic_mdp_certificate.rs,
  qsl-analyze/tests/it/main.rs, qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate/cyclic.rs,
  qsl-replay/src/probability_certificate/document.rs, qsl-replay/src/probability_certificate/document/long_run.rs,
  qsl-replay/src/probability_certificate/document/mdp.rs, qsl-replay/src/probability_certificate/document/writer.rs,
  tools/qsl-probability-check/tests/it/cyclic_mdp.rs, tools/qsl-probability-check/tests/it/main.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
---

## Summary

Independent gap-analysis of frozen PR2663 head b09706de520379a48e13293999f077a2be55b156 against base f12c6bf363f1410035aeba951411098af4b8858b. Ticket: QSL-1826. Rust review is folded into code-review.

## Verdict

**CONDITIONAL — no concrete new scoped gap found; computed matrix and repository qualification remain unavailable.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable active AssuranceProfile found. No selected exception or whole-repository assurance claim.

## Coverage

Reviewed all twelve changed files and the surrounding shared enumerator, candidate builder, source facade and exact document callers needed to establish behavior. Repository AGENTS.md/CLAUDE.md/README.md, code-review Rust lane, rust-style API/implementation/safety/wire/testing references and gap-analysis were applied. No dependency, lockfile, CI workflow, specification or plan edits occur. No applicable AssuranceProfile was found by anchored frontmatter type search. Spec-review methods do not apply to this runtime-only diff. Optional repository-wide semantic review was not invoked; scoped soundness and oracle checks were explicitly dispatched.

Independent soundness check: exact per-action sums and boundary equations establish Bellman equalities. MIN universal positive-support rank descent to target or offered zero set excludes any positive-valued end component under any scheduler; a tight minimizing policy plus a closed zero-support choice realizes the offered value. MAX requires a tight maximizing action with descending support, giving a proper attaining policy outside zeros, while max Bellman inequalities bound other policies. Zero-valued MAX action supports are all closed. Every positive non-target, including value one, needs positive rank; a zero-probability edge never establishes progress. Offered ranks need not be shortest or producer ranks. The S/T integration and unit controls correctly allow a lower-rank zero successor while positive successors retain equal ranks.

The checker reconstructs all actual semantic action distributions, first admitting the complete original model with no query P, then evaluating P only on the first-P stopped graph. Source-bound identity, checked occurrence and actual binding are independently reconstructed. Target sets, full coverage, ordered unique rows, strict objective/member shapes and exact rational/key encoding are checked. A producer policy/action subset cannot substitute for the source graph. Actual post-target Markov refusals remain authoritative while solely post-target query undefinedness is irrelevant. The new private candidate stop applies only to budgeted Every/Eventually construction; the legacy public builder retains Continue. Existing eligible profile preference, public document literals and legacy canonical bytes remain intact through an additive typed profile wrapper and shared writer.

Only typed predicate/unsupported/cycle failures enter the new fallback. Admission, expansion, identity, accounting, resource limits and cancellation retain typed errors. The caller Meter, cumulative storage ledger and actual Cancel span retries and both model passes. The old adapter now preserves its real UndefinedPredicate cause. MIN/MAX sums, row processing, rank derivation and zero/progress loops poll/meter actual work. No solver/analyzer dependency reaches the checker. Shared action_sum replaces the old equality sum rather than leaving a copied arithmetic rule.

Independent thermal focused execution: cargo test -p qsl-probability-check --test it cyclic passed 13 tests, 40 filtered, exit 0; log /tmp/qsl1826-cyclic-mdp-review/process-test.log. This selection includes four new MDP process tests plus prior cyclic regressions. Inspected coder logs at /tmp/qsl1826-cyclic-mdp-{analyze,long-run,document,clippy,fmt}.log: 18 analyzer cyclic tests, 21 long-run/weighted dispatch tests and 16 document tests passed; three-package all-target Clippy with warnings denied and formatting passed. Source was tested as a5ff940 and cleanly rebased to frozen b09706de520379a48e13293999f077a2be55b156 over f12c6bf, with identical implementation patch. No claim is made that the earlier 52-pass broad run terminated successfully: it was stopped during the unrelated exhaustive poll sweep. New actual phase controls cover Requested and Deadline, including ZeroSupport. No mutation of the frozen branch was performed. No aggregate CI, Kani, workspace-wide or cargo-deny gate was run under the explicit handoff override.

External ModelTransition private canonical reader and ordinary interpreter/model allocations remain outside the owner raw STATE-key guarantee. This review grants no exception and asserts no full FR-201-AC-30, all-expansion storage, complete ticket or whole-repository qualification. Each scoped trace tag records representative assertions, not exhaustive satisfaction of every enumerated subclause. Full repository matrix and reverse-gap assurance remain unavailable: quire matrix --scope . --format json produced zero JSON and was interrupted with SIGINT (130); stderr retained at /tmp/qsl1826-cyclic-mdp-review/matrix.stderr. No hand-written matrix or computed percentage was substituted. Changed behavior maps to the merged FR201 AC39-42 profile; no concrete new unowned behavior or stub was identified.

Plan completion: not assessed

## Examined scope

Scope excerpts are verbatim source substrings capped at 600 characters. Examined means inspected, not exhaustive execution evidence.

```yaml
scope:
- id: FR-201-AC-39
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A live state s has actions a with probabilities (self,target,fail)=(1/2,1/5,3/10)
    and b with (1/2,2/5,1/10). With target value 1 and false-terminal fail value 0,
    the new profile accepts minimum 2/5 under AtLeast 2/5 and maximum 4/5 under AtMost
    4/5, with rank(s)=1 and boundary ranks 0. Minimum AtLeast 1/2 and maximum AtMost
    3/4 reject ThresholdMissed. Changing an otherwise matching actual action probability,
    omitting a source action from the extremum or replacing separate actions by their
    averaged distribution cannot verify the old vector. Separately let S choose A=(1/2
    fail,1/2 T) or B=(1/4 f
- id: FR-201-AC-40
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For wait-to-self versus go-to-target, the actual minimum is 0. A bogus
    minimum v(s)=1/2 satisfies Bellman equality but rejects RankBroken because wait
    supplies no lower-rank successor, even though go does. For wait-to-self versus
    attempt to target/fail with probabilities 4/5 and 1/5, the maximum is 4/5: bogus
    v(s)=9/10 is a Bellman fixed point through wait but rejects RankBroken because
    only the nontight attempt progresses; at v(s)=4/5 the tight attempt and rank 1
    accept under AtMost 19/20. Zero-valued live minimum states require a closed zero-support
    action; maximum zeros require all action s'
- id: FR-201-AC-41
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-39/40 valid documents round-trip canonical bytes through a fresh checker
    without analyzer/provider linkage. Target/non-target-terminal boundaries, stopped
    targets/coverage and complete full-key bindings are independently checked. Wrong
    objective/identity, malformed rational/JSON/key data, duplicate/missing/out-of-order
    values or ranks, extra after-target rows, model-only keys, false Bellman values,
    zero support and rank progress prevent acceptance with the specified rule/locus.
    A valid post-target cycle alone does not add certificate rows. Earlier profile
    bytes/verdicts and eligible produce
- id: FR-201-AC-42
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic
    exhaustion during original/stopped admission, raw/internal/full keys, all-action
    and draw reconstruction, min/max sums, zero-support and progress checks retain
    their real typed cause without an accepted truth. Undefined predicates/workload
    or source refusals are not coerced to zero or dropped actions. Fairness, workload
    selection, rewards, timed/general temporal, pending multi-post, multiple-initial/binding,
    non-exact or refutation requests issue no new-profile proof. Zero di
bindings:
- test_id: qsl-analyze/src/analyze/exact.rs::threshold_rejection_retains_its_rule_and_complete_product_key
  ac_id: FR-201-AC-6
  trace: correct
- test_id: qsl-analyze/src/analyze/exact.rs::threshold_rejection_retains_its_rule_and_complete_product_key
  ac_id: FR-203-AC-3
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits
  ac_id: FR-201-AC-39
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits
  ac_id: FR-201-AC-42
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits
  ac_id: FR-198-AC-5
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::caller_stops_are_observed_in_actual_mdp_passes_and_limits_do_not_accept
  ac_id: FR-201-AC-42
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::fallback_attempts_keep_the_caller_storage_ledger
  ac_id: FR-201-AC-42
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::undefined_query_after_target_does_not_skip_original_markov_admission
  ac_id: FR-201-AC-39
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::undefined_query_after_target_does_not_skip_original_markov_admission
  ac_id: FR-201-AC-42
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters
  ac_id: FR-201-AC-42
  trace: correct
- test_id: qsl-analyze/tests/it/cyclic_mdp_certificate.rs::pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters
  ac_id: FR-201-AC-24
  trace: correct
- test_id: qsl-replay/src/lib.rs::tc_211_refusal_causes_redact_while_typed_accessors_stay_readable
  ac_id: FR-073-AC-3
  trace: correct
- test_id: qsl-replay/src/probability_certificate/cyclic.rs::every_support_scc_and_distance_poll_preserves_typed_cancellation
  ac_id: FR-201-AC-26
  trace: correct
- test_id: qsl-replay/src/probability_certificate/cyclic.rs::every_support_scc_and_distance_poll_preserves_typed_cancellation
  ac_id: FR-282-AC-19
  trace: correct
- test_id: qsl-replay/src/probability_certificate/cyclic.rs::support_passes_preserve_each_exhausted_work_budget
  ac_id: FR-201-AC-26
  trace: correct
- test_id: qsl-replay/src/probability_certificate/cyclic.rs::support_passes_preserve_each_exhausted_work_budget
  ac_id: FR-282-AC-19
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::cyclic_profile_roundtrip_retains_its_explicit_selection
  ac_id: FR-201-AC-25
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::cyclic_profile_roundtrip_retains_its_explicit_selection
  ac_id: FR-282-AC-19
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::reward_codec_preserves_large_values_and_closed_objective_without_changing_probability_codec
  ac_id: FR-201-AC-21
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::reward_codec_preserves_large_values_and_closed_objective_without_changing_probability_codec
  ac_id: FR-201-AC-22
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::reward_unit_descriptors_use_bounded_input_depth_parsing
  ac_id: FR-201-AC-21
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::reward_unit_descriptors_use_bounded_input_depth_parsing
  ac_id: FR-201-AC-22
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::closed_exact_document_preserves_every_component_and_canonical_bytes
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::structural_identity_nesting_uses_the_shared_iterative_codec
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::storage_and_rational_text_are_admitted_before_rows_or_large_integers
  ac_id: FR-201-AC-11
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::storage_and_rational_text_are_admitted_before_rows_or_large_integers
  ac_id: FR-282-AC-12
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::every_row_collection_requires_strict_complete_key_order
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::bellman_arithmetic_charges_shared_work_and_result_storage
  ac_id: FR-201-AC-11
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::malformed_rows_rationals_fields_and_keys_are_not_normalized
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::unsupported_profiles_and_cancelled_or_exhausted_reads_have_no_evidence
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document.rs::unsupported_profiles_and_cancelled_or_exhausted_reads_have_no_evidence
  ac_id: FR-201-AC-11
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/mdp.rs::equal_positive_ranks_progress_through_zero_and_attractor_witnesses_hold
  ac_id: FR-201-AC-40
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/mdp.rs::equal_positive_ranks_progress_through_zero_and_attractor_witnesses_hold
  ac_id: FR-201-AC-41
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/mdp.rs::probability_zero_edges_do_not_progress_and_non_target_one_needs_positive_rank
  ac_id: FR-201-AC-40
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/mdp.rs::probability_zero_edges_do_not_progress_and_non_target_one_needs_positive_rank
  ac_id: FR-201-AC-41
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/writer.rs::bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/writer.rs::bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text
  ac_id: FR-255-AC-1
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/writer.rs::writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output
  ac_id: FR-201-AC-9
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/writer.rs::writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output
  ac_id: FR-255-AC-1
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::separate_retry_actions_roundtrip_both_extrema_and_interior_threshold_misses
  ac_id: FR-201-AC-39
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::separate_retry_actions_roundtrip_both_extrema_and_interior_threshold_misses
  ac_id: FR-201-AC-40
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::separate_retry_actions_roundtrip_both_extrema_and_interior_threshold_misses
  ac_id: FR-201-AC-41
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::bellman_fixed_points_require_universal_min_and_tight_max_progress
  ac_id: FR-201-AC-40
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::bellman_fixed_points_require_universal_min_and_tight_max_progress
  ac_id: FR-201-AC-41
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::strict_rows_boundaries_and_quantitative_source_scope_are_closed
  ac_id: FR-201-AC-39
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::strict_rows_boundaries_and_quantitative_source_scope_are_closed
  ac_id: FR-201-AC-41
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::cyclic_pair_accepts_equal_positive_ranks_with_only_zero_lower_successor
  ac_id: FR-201-AC-39
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::cyclic_pair_accepts_equal_positive_ranks_with_only_zero_lower_successor
  ac_id: FR-201-AC-40
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/cyclic_mdp.rs::cyclic_pair_accepts_equal_positive_ranks_with_only_zero_lower_successor
  ac_id: FR-201-AC-41
  trace: correct
```
