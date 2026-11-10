---
id: SR-20261109172912000091
title: QSL-1826 expected-reward implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2639; 20 changed files; FR201 AC19-22
  and FR282 AC16-17
review_set: subset
---

## Summary

Independent early blocked-draft review of frozen d9cf8f849b68b9e26ba2e25aac52763e40d237b1. Ticket QSL-1826.

## Verdict

FAIL / BLOCKED; not mergeable until the actual required owner raw-key API and transitive callback integration lands and is independently rechecked. Available math, S3 and codec review found no additional concrete defect. Passing tests do not establish the missing raw admission bound.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Reward caller-stop coverage does not establish bounded raw ModelSystem key admission or its transitive candidate/post-key callbacks; required owner API adoption is missing. | qsl-replay/src/probability_certificate/reward.rs:96-112 | implementation-bug-despite-evidence |

## Coverage

This is the explicitly requested EARLY BLOCKED-DRAFT engineering review of PR2639, not a readiness approval or whole-repository qualification. Reviewed the 20-file implementation diff against 3d204880374fb422244f408bfda8677ec76f85c9 at frozen d9cf8f849b68b9e26ba2e25aac52763e40d237b1. Main has advanced through repaired-spec merge 36f6078860971baeb1410c79468b6bc0613eae08; no source edits or rebase performed. Read repo AGENTS.md, CLAUDE.md, README, supplied handoff, draft PR body, reviewer/code-review/rust-review/rust-style plus API, implementation, safety, wire and testing references. No applicable AssuranceProfile found. No CI/workflow/manifest/lockfile gate changes in this diff. The two existing SR77/78 edits are custody errata only; no duplicate finding is raised.

Math and admission: traced checked named Integer/bounded-Integer/Quantity identity, actual model-root identity, comparison-unit equality and S3 threshold conversion, target zero and nonnegative values above one. Enumerates original graph with stop_at_target=false, checks original acyclicity and all original terminals targeting B, then constructs the reachable stopped graph, exact heights, complete full keys, independent per-draw costs, cost-plus-successor Bellman equations, min/max/resolved workload and threshold. Missing named reward on an operation contributes zero. Zero-probability draws are consistently omitted. Existing actions/weighted steps preserve draw order for costs and edges; no producer cost labels/action rows or new wire are trusted. Reward refutation remains unsupported. Producer core preflight precedes decide_expected_reward and final builder/checker independently reconstructs the subject. No additional concrete math or objective-identity defect found.

Codec and accounting: examined exact closed reward objective fields, absence/presence distinction for explicit null unit, RawValue transport, iterative canonical unit descriptor parsing, borrowed declared/compound UnitId encoding, scalar text prechecks and reserve-before-growth wrappers. Existing reachability and state-predicate encodings retain their branches. Shared product-key encoder now carries cumulative graph storage; rational slots are charged even for missing-reward zero values and outer cost rows are reserved before retention. These wrapper improvements do not bound the earlier raw ModelSystem key path. S3 conversion uses the real cumulative CheckContext meter and original Cancel; rational_bits sets the integer representation facet before conversion allocation; typed located size/work/cancel causes survive to CLI. Nonreward Integer/bounded-Integer/arithmetic controls remain covered. No extra new wire or local copied upstream contract found.

FND-001 evidence: reward.rs:96-99 calls system.state_key before model_product_key_with_storage admission. model.rs:947-969 constructs encode::model_key(...).0 and uses identity_limits() without caller cancellation or caller working-storage reservation. identity_limits() at model.rs:1601 uses IdentityLimits::default(), whose identity.input_bytes default is 16 MiB, distinct from document I/O's 1 GiB. key.rs:132-158 materializes raw canonical bytes and clones them. actions_with_cancel/weighted_steps_with_cancel do not make their transitive candidate/post-sort raw keys quota-aware: probabilistic.rs:655,698 and frames.rs:1547 still use canonical_state_key. Consequently a caller can exhaust its working quota only after raw tree/scratch/bytes have already grown; a later wrapper refusal is insufficient. Integrate the actual owner bounded/cancellable raw key API and all candidate/post-key callbacks, preserving raw identity-input and graph/document bounds and typed cancellation/size causes. Owner preparation is unavailable at this head: IR spec142 is reviewed but actual Luna merge pending; code143 is stacked and UNBUILT. No waiver, accepted-no-change disposition or full-memory claim is granted.

Evidence inspected and accepted from supplied logs, not rerun: reward7 /tmp/qsl1826-reward-quota-tests.log after final storage correction; codec13 /tmp/qsl1826-key-codec-green.log; S3 2 /tmp/qsl1826-s3-green-current.log; fresh-process32 /tmp/qsl1826-reward-all-fresh-tests.log before later quota-only correction; adapter13/native41 /tmp/qsl1826-reward-adapter-tests2.log and /tmp/qsl1826-reward-native-tests2.log. Fresh-process reward cases assert two-step 3/4 and workload7/2, value-preserving 2/6 swap acceptance, 2/8 or unit-step rejection, target absorption, missing reward zero, checked unit conversion, false-threshold rejection, objective/key/coverage/height tampering and typed stops. S3 representation/work/cancel counterfactual logs each fail the intended assertion; /tmp/qsl1826-key-cumulative-red.log fails old per-key reset. Source restoration was supplied as byte-exact evidence. Four affected-package all-target Clippy with analyze/test-support and warnings denied passes in /tmp/qsl1826-reward-clippy.log; formatter evidence /tmp/qsl1826-reward-fmt.log passes. Feature-unified no-op semantic observer preserves native probes. No unresolved concrete concern warranted cargo reruns.

Qualification limit: full matrix/aggregate/make ci/Kani/timeout/full qualification was explicitly excluded. quire matrix has no subset filter; real trace attributes were manually inspected for the six scoped reward ACs and recorded below. Existing reward FR201 AC19-22 and FR282 AC16-17 are detached from parsed tables in this frozen source; actual text was read manually. Independently cleared spec PR2638 is now actually merged on main at 36f6078860971baeb1410c79468b6bc0613eae08 and repairs tables (FR201 parsed18->26, FR28214->18) without changing reward meaning. Current repaired reward text was compared with the frozen text: AC20 changes SHALL accept/reject to accepts/rejects without changing its meaning; the other five reward rows are byte-identical. Scope excerpts use the current repaired specification. The repaired spec is not integrated into this frozen implementation candidate; new cyclic criteria do not describe this runtime lane. No claim is made that missing rows were previously matrix-validated. No spec/plan behavior changed here, so spec-review subanalyses do not apply. Optional gap semantic campaign not separately run; code-review's required faithfulness and test-intent checks were performed under the dispatch. Plan completion: not assessed.

## Examined scope and bindings

```yaml
scope:
- id: FR-201-AC-19
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For an acyclic two-step source with named integer reward R, initial choices
    a (cost 1, then cost 2 to B) and b (cost 4 directly to B) have exact minimum 3
    and maximum 4. The minimum AtLeast 3 and maximum AtMost 4 documents accept at
    equality; thresholds 4 and 3 respectively reject ThresholdMissed. An equal-weight
    resolved workload has value 7/2 and accepts either bound at 7/2. Values above
    1 are accepted; a probability-style target value 1 rejects.
- id: FR-201-AC-20
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A draw-dependent authored reward of 2 or 6 with equal probability on an
    edge entering B gives value 4, including the entry cost. Swapping costs 2 and
    6 between those equally probable draws preserves the value 4; the same certificate
    accepts after independent recomputation when every equation, target, identity
    and rank remains valid. Changing the costs to 2 and 8 falsifies the certified
    initial value 4 and rejects; substituting a fixed unit-step cost also rejects
    because its value 1 fails that equation. Initial B has value 0 even with an enabled
    positive-cost operation. A reached B absorbs a la
- id: FR-201-AC-21
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A checked quantity reward expressed as 2 ms and 6 ms under equal draws
    gives 4 ms and accepts a source threshold 0.004 s after exact checked-unit normalization.
    Wrong reward name/model, target set, optimization, binding or full key prevents
    acceptance; changing the claimed reward/unit identity with the same numeric magnitudes
    prevents objective acceptance. Equivalent independently rederived costs under
    the actual admitted subject remain valid; source changes alone do not imply a
    staleness rejection. A dimensionally wrong source bound preserves the typing refusal.
    Noncanonical, duplicate, omitt
- id: FR-201-AC-22
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: The checker independently checks analyze's AC-19 to AC-21 exact documents
    with no qsl-analyze/provider dependency or reused solver graph/cost labels. Negative/undefined
    reward evaluation, unsupported profile/kind, exhausted graph/arithmetic/representation
    budgets, pre-cancellation and cancellation during expansion or reward evaluation
    preserve typed nonacceptance without truth values. Existing reachability/v1 and
    state-predicate/v1 canonical bytes/verdicts are unchanged.
- id: FR-282-AC-16
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Analyze emits the real expected-reward proof documents of FR-201-AC-19
    to AC-21. A fresh executable linking only qualified-core crates accepts their
    canonical bytes from the bounded source provision and actual S3 subject/claim
    binding, including random/workload source packages refused by S4. It independently
    rederives reward identity, exact unit normalization, semantic actions/draws/costs,
    original acyclicity and stopped Bellman equations, with no qsl-analyze/provider
    dependency, emitted-package bytes, fabricated replay request or solver graph.
    Source admission failures retain their typed refu
- id: FR-282-AC-17
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: For AC-16, source/document byte ceilings, state/edge/rational/key-representation
    budgets, pre-cancellation and cancellation during independent reward evaluation/graph
    expansion preserve the actual typed failure, named limit/consumption or cancellation
    without acceptance or truth values. Both older probability document profiles retain
    their canonical bytes and verdicts.
bindings:
- test_id: two_step_named_reward_checks_minimum_maximum_and_both_workload_bounds
  ac_id: FR-201-AC-19
  trace: correct
- test_id: two_step_named_reward_checks_minimum_maximum_and_both_workload_bounds
  ac_id: FR-282-AC-16
  trace: correct
- test_id: independent_draw_costs_accept_value_preserving_swap_and_reject_changed_equation
  ac_id: FR-201-AC-20
  trace: correct
- test_id: independent_draw_costs_accept_value_preserving_swap_and_reject_changed_equation
  ac_id: FR-282-AC-16
  trace: correct
- test_id: quantity_reward_uses_checked_exact_conversion_and_retains_unit_identity
  ac_id: FR-201-AC-21
  trace: correct
- test_id: quantity_reward_uses_checked_exact_conversion_and_retains_unit_identity
  ac_id: FR-282-AC-16
  trace: correct
- test_id: source_quantity_normalization_stops_at_its_real_s3_representation_limit
  ac_id: FR-201-AC-22
  trace: correct
- test_id: source_quantity_normalization_stops_at_its_real_s3_representation_limit
  ac_id: FR-282-AC-17
  trace: correct
- test_id: reward_objective_keys_coverage_heights_and_caller_stops_are_checked
  ac_id: FR-201-AC-21
  trace: correct
- test_id: reward_objective_keys_coverage_heights_and_caller_stops_are_checked
  ac_id: FR-201-AC-22
  trace: correct
- test_id: reward_objective_keys_coverage_heights_and_caller_stops_are_checked
  ac_id: FR-282-AC-17
  trace: correct
- test_id: nonreaching_terminals_hidden_raw_cycles_and_negative_rewards_never_accept
  ac_id: FR-201-AC-20
  trace: correct
- test_id: nonreaching_terminals_hidden_raw_cycles_and_negative_rewards_never_accept
  ac_id: FR-201-AC-22
  trace: correct
- test_id: nonreaching_terminals_hidden_raw_cycles_and_negative_rewards_never_accept
  ac_id: FR-282-AC-17
  trace: correct
- test_id: cancellation_inside_qualified_reward_evaluation_stops_and_old_actions_still_work
  ac_id: FR-201-AC-22
  trace: correct
- test_id: cancellation_inside_qualified_reward_evaluation_stops_and_old_actions_still_work
  ac_id: FR-282-AC-17
  trace: correct
- test_id: reward_codec_preserves_large_values_and_closed_objective_without_changing_probability_codec
  ac_id: FR-201-AC-21
  trace: correct
- test_id: reward_codec_preserves_large_values_and_closed_objective_without_changing_probability_codec
  ac_id: FR-201-AC-22
  trace: correct
- test_id: reward_unit_descriptors_use_bounded_input_depth_parsing
  ac_id: FR-201-AC-21
  trace: correct
- test_id: reward_unit_descriptors_use_bounded_input_depth_parsing
  ac_id: FR-201-AC-22
  trace: correct
- test_id: model_keys_share_the_exact_cumulative_graph_storage_ceiling
  ac_id: FR-201-AC-22
  trace: correct
- test_id: model_keys_share_the_exact_cumulative_graph_storage_ceiling
  ac_id: FR-282-AC-17
  trace: correct
- test_id: the_conversion_cap_preserves_nonreward_integer_checking
  ac_id: FR-201-AC-22
  trace: correct
- test_id: actual_quantity_conversion_obeys_representation_work_and_mid_conversion_cancel
  ac_id: FR-201-AC-22
  trace: correct
- test_id: actual_quantity_conversion_obeys_representation_work_and_mid_conversion_cancel
  ac_id: FR-282-AC-17
  trace: correct
```
