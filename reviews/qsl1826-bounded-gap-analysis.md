---
id: SR-20261109172912000034
title: QSL-1826 bounded gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826; 27 changed Rust files; PR2619 aed5ef1
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
---

## Summary

Frozen PR quire-spec-language#2619 at aed5ef1c3b2a61177b7aef8f77fef13d8b4b2b0d; base 837ba93a8c26581ffd8ee691b8d8f057b432be43. Ticket: QSL-1826. No source edits. This review covers the bounded implementation slice, not completion of all QSL-1826 assurance obligations.

## Verdict

CONDITIONAL: no concrete introduced defect found. Full computed matrix unavailable/previously slow; dispatch explicitly excludes rerun. Manual scoped AC-to-real-test/source review is recorded below, without a repository-wide coverage claim. Engineering merge is permitted by the AGENTS owner update separating incomplete assurance from implementation dependencies.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

## Examined scope

```yaml
scope:
- id: FR-201-AC-12
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "ADR-028 \xA715.4's bounded Deliver at horizon 2 gives accepted exact minimum\
    \ AtLeast at 24/25, maximum AtMost at 99/100 and Even workload at 391/400; after\
    \ one loss the minimum is 4/5 and workload 17/20. Each threshold beyond the certified\
    \ side fails. Existing unbounded profile documents retain their verdict and canonical\
    \ bytes."
- id: FR-201-AC-13
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A model that stays undelivered with probability 1/2 and delivers with probability
    1/2 per step has bounded horizon-2 probability 3/4 despite its raw self-cycle.
    The initial and one-loss visits share the model key but have distinct full formula-bearing
    keys and ranks 2 and 1. Horizon 0 accepts iff P holds at position 0. A terminal
    undelivered state closes to rejection by false extension without enumerating stutter
    positions.
- id: FR-201-AC-14
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For AC-13's valid certificate, replacing a running monitor with a local
    StateId, null, another remaining horizon, or another binding cannot certify the
    claim. Changing rank 2 to 3 while retaining valid values fails RankBroken at that
    full key. An exact document fabricated from a dyadic interval is never emitted;
    unsupported lower/upper documents and precision stops produce no accepted exact
    certificate.
- id: FR-282-AC-13
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: A real FR-197 Exact bounded event-position certificate for FR-201-AC-13
    is written by analyze and accepted from its canonical document bytes in a fresh
    qualified-core-only process using the source provision and actual subject/claim
    binding. The process independently reconstructs formula-monitor keys and terminal
    closure through qsl-eval, without qsl-analyze or provider linkage. Changing a
    running monitor horizon, binding or bounded rank prevents acceptance.
- id: FR-338-AC-4
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: The canonical octet vectors have exactly the specified state bodies and
    full `K_C` octets; reversing subset/window insertion order leaves bytes equal,
    distinct occurrence paths and differing live facts leave bytes unequal, and no
    graph enumeration index enters the key.
- id: FR-338-AC-5
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: examined
  excerpt: 'Each listed admission negative refuses before the key is used for membership:
    a waiting `each-buchi` state carrying the well-formed shift window of its checked
    `eventually[1,*]` root, a `buchi` state under an unguarded `on each` context,
    an `each-buchi` state under an origin or `each-when` context, and a TP-4 key under
    a timed profile are refused; a same-shaped state under a foreign item, profile
    or activation is not admitted by the original context.'
- id: FR-196-AC-6
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: examined
  excerpt: Every worked vector produces the specified canonical component and full
    P octets; changing any present component changes identity where specified, and
    reversing discovery order changes none. Certificate values, ranks, policy, bias
    and rejection retain the same complete representation.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: The checker SHALL independently construct the model/monitor product using
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: Certificate values, ranks, policy, bias and rejection retain the same complete
    representation.
```

## Coverage

Inspected separate replay enumeration and Bellman/ranking checks; complete parsed model/formula membership; actual S3 parameter and model reference type; explicit instance versus ordinary value/reference context; qualified origin/event-position shape; real debt ranks and counter-free verdicts; position zero; raw cyclic finite product; terminal false-extension closure; producer FhExact admission; Bounds/PrecisionBudget refusal; cancellation and failure paths. Generic automata retain viability search: the shortcut belongs to the translation owner and checks the precise bounded eventuality root. No introduced duplication, vendoring, source stubs, CI changes or weakened gate found. Shared semantic primitives are normative owners, while graph/arithmetic implementations remain independent as FR-201 requires.

Evidence inspected without duplicate cargo reruns: fresh bounded 7/7 (/tmp/qsl1826-bounded-fresh-final.log), parent fresh regression 10/10 (/tmp/qsl1826-bounded-parent-fresh-regression.log), EN-1 bounded closure 7/7 (/tmp/qsl1826-bounded-en1-closure.log), owner monitor 6/6 (/tmp/qsl1826-bounded-owner-tests-2.log), ordinary monitor controls 7/7 and safety parity 1/1. Final five-crate all-target Clippy log passed. Reviewer git diff --check passed. No full CI, Kani, whole matrix or all-workspace cargo-deny claim. No applicable AssuranceProfile found under spec/. README, AGENTS, CLAUDE and Rust API/implementation/safety/boundary/testing review rules were read.

Plan completion: not assessed

Computed matrix: not run by instruction. Manual trace index uses actual #[trace] attributes, not test names. Optional whole-repository semantic review was not run. No spec/plan change occurs, so spec-review and its sub-analyses do not apply.

Changed files examined:

- qsl-analyze/src/analyze/exact.rs
- qsl-analyze/src/exact_probabilistic.rs
- qsl-analyze/src/exact_probabilistic/build.rs
- qsl-analyze/src/exact_probabilistic/plan.rs
- qsl-analyze/tests/it/bounded_closure.rs
- qsl-analyze/tests/it/closure_certificate.rs
- qsl-eval/src/automaton/bounded.rs
- qsl-eval/src/automaton/key.rs
- qsl-eval/src/automaton/mod.rs
- qsl-eval/src/automaton/monitor.rs
- qsl-eval/src/automaton/qualified_bounded.rs
- qsl-eval/src/automaton/translate.rs
- qsl-eval/tests/it/main.rs
- qsl-eval/tests/it/qualified_bounded_monitor.rs
- qsl-probability-check/tests/it/bounded.rs
- qsl-probability-check/tests/it/main.rs
- qsl-probability-check/tests/it/roundtrip.rs
- qsl-replay/src/lib.rs
- qsl-replay/src/model_check/item.rs
- qsl-replay/src/probability_certificate.rs
- qsl-replay/src/probability_certificate/bounded.rs
- qsl-replay/src/probability_certificate/document.rs
- qsl-replay/tests/it/vault_world.rs
- qsl-semantics/src/check/lowering/probabilistic.rs
- qsl-semantics/src/check/mod.rs
- qsl-semantics/src/check/probabilistic/claim.rs
- qsl-semantics/src/check/probabilistic/model.rs

## Inspected trace bindings

```yaml
bindings:
- test_id: bounded_claim_over_is_a_real_s3_parameter_with_its_actual_model_type
  ac_id: FR-201-AC-14
  trace: correct
- test_id: bounded_deliver_exact_minimum_maximum_and_workload_cross_the_process_boundary
  ac_id: FR-201-AC-12
  trace: correct
- test_id: bounded_deliver_exact_minimum_maximum_and_workload_cross_the_process_boundary
  ac_id: FR-282-AC-13
  trace: correct
- test_id: bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks
  ac_id: FR-201-AC-13
  trace: correct
- test_id: bounded_raw_cycle_has_distinct_real_horizons_and_exact_ranks
  ac_id: FR-282-AC-13
  trace: correct
- test_id: bounded_zero_horizon_reads_position_zero_and_terminal_closes_by_false_extension
  ac_id: FR-201-AC-13
  trace: correct
- test_id: bounded_zero_horizon_reads_position_zero_and_terminal_closes_by_false_extension
  ac_id: FR-282-AC-13
  trace: correct
- test_id: bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence
  ac_id: FR-201-AC-13
  trace: correct
- test_id: bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence
  ac_id: FR-201-AC-14
  trace: correct
- test_id: bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence
  ac_id: FR-282-AC-12
  trace: correct
- test_id: bounded_graph_and_document_budget_stops_retain_limits_without_truth_or_evidence
  ac_id: FR-282-AC-13
  trace: correct
- test_id: bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim
  ac_id: FR-201-AC-14
  trace: correct
- test_id: bounded_altered_horizon_binding_and_rank_never_certify_the_original_claim
  ac_id: FR-282-AC-13
  trace: correct
- test_id: bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document
  ac_id: FR-201-AC-14
  trace: correct
- test_id: bounded_wrong_source_and_nonexact_rows_never_produce_an_accepted_document
  ac_id: FR-282-AC-13
  trace: correct
- test_id: qualified_bounded_monitor_keys_follow_real_debt_and_ignore_allocation_order
  ac_id: FR-201-AC-13
  trace: correct
- test_id: qualified_bounded_monitor_keys_follow_real_debt_and_ignore_allocation_order
  ac_id: FR-338-AC-4
  trace: correct
- test_id: qualified_bounded_monitor_terminal_false_extension_and_zero_horizon
  ac_id: FR-201-AC-13
  trace: correct
- test_id: qualified_bounded_monitor_preserves_context_and_refuses_other_forms
  ac_id: FR-201-AC-14
  trace: correct
- test_id: qualified_bounded_monitor_preserves_context_and_refuses_other_forms
  ac_id: FR-338-AC-5
  trace: correct
- test_id: qualified_bounded_monitor_retains_the_producers_reached_handles
  ac_id: FR-201-AC-13
  trace: correct
- test_id: qualified_bounded_monitor_retains_the_producers_reached_handles
  ac_id: FR-201-AC-14
  trace: correct
- test_id: qualified_bounded_monitor_closes_a_full_u64_horizon_without_stutter_states
  ac_id: FR-201-AC-13
  trace: correct
- test_id: over_instance_keys_and_ordinary_reference_arguments_have_distinct_context_forms
  ac_id: FR-338-AC-4
  trace: correct
- test_id: over_instance_keys_and_ordinary_reference_arguments_have_distinct_context_forms
  ac_id: FR-201-AC-14
  trace: correct
```
