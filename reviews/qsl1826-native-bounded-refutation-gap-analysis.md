---
id: SR-20261109172912000056
title: QSL-1826 native bounded refutation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826; PR2623; six changed Rust files
review_set: subset
---

## Summary

Frozen PR quire-spec-language#2623 at 75e3e00c194083b9240d09765d98ba701d65f699, base 5c4dbc058. Ticket: QSL-1826. Read-only review of the six changed Rust files. No spec or plan changes. No applicable AssuranceProfile document found under spec/. Existing review prose mentioning that type is not an applicable profile.

## Verdict

CONDITIONAL: Manual scoped AC/source/test check found no introduced defect. The full computed repository matrix was excluded by the dispatch and is not claimed complete. Engineering merge is permitted by the repository owner instruction separating incomplete assurance from implementation dependencies.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

## Coverage

Reviewed the native facade binding and dispatch, independently reconstructed qualified bounded monitor, actual S3 binding and predicate, canonical model/formula context/debt keys, first-decision stopping, terminal false extension, horizon-zero empty cylinders, exact rejecting/accepting mass inequalities, workload probability derivation, Markov restrictions, full-key deterministic policy consistency, explicit draw/transition cylinders and prefix freedom. Raw model cycles remain finite by actual monitor debt. Source, threshold, horizon, binding, action, draw and post-state mutations cannot bypass reconstruction. Refuted settlement requires the opaque replay token; Bounds, PrecisionBudget, replay disagreement, cancellation and budget failures do not retain truth evidence. Unbounded dispatch and continuation-policy behavior remain separate and unchanged.

Resource review: path count, visited states, represented transitions, canonical retained evidence/storage, rational bit ceilings and metered rational arithmetic have typed failures; model expansion uses the admitted system's expansion/work limits. Candidate re-enumeration is bounded by existing expansion controls; represented-path traversal is bounded by transition limits. Pairwise cylinder checks are quadratic in represented paths, cancel-polled, and bounded by path/transition admission; no exponential graph walk or duplicate unbounded monitor-cache clone was introduced. The final token clones admitted evidence once after checks and polls cancellation again.

Read README, AGENTS, CLAUDE, reviewer, code-review, rust-review and Rust API/implementation/safety/boundary/testing guidance. No CI/configuration change, weakened gate, vendored semantic contract, stub or unjustified public API expansion was found. The monitor/key/transition owners are reused; independent replay does not trust analyzer product labels or probabilities. No guessed FR-413 wire contract is introduced.

Existing final-head evidence inspected: 27 replay tests (/tmp/qsl1826-replay-regressions.log), 13 adapter tests (/tmp/qsl1826-item-regressions.log), 17 document/fresh-process tests (/tmp/qsl1826-document-regressions.log), three-crate all-target Clippy (/tmp/qsl1826-bounded-clippy.log), formatting (/tmp/qsl1826-bounded-fmt.log), and checked head (/tmp/qsl1826-bounded-checked-head.txt). All passed. Reviewer git diff --check passed. Cargo was not rerun after target removal because review found no concrete concern requiring a new build. No make ci, aggregate, Kani, full feature matrix, all-workspace cargo-deny or qualification claim.

The parent document-test expectation now requires retained native evidence and independently invokes replay to compare its token mass; it does not merely accept the new Refuted label.

Plan completion: not assessed

Computed matrix: not run by instruction. Manual scope uses actual #[trace] attributes. Optional whole-repository semantic review was not run. Spec-review and its sub-analyses do not apply because the diff contains no spec/plan files. This is a scoped implementation review, not repository-wide assurance completion.

Changed files examined:

- qsl-analyze/src/analyze/exact.rs
- qsl-analyze/tests/it/bounded_probabilistic_replay.rs
- qsl-analyze/tests/it/main.rs
- qsl-probability-check/tests/it/bounded.rs
- qsl-replay/src/probability_certificate/witness.rs
- qsl-replay/src/probability_certificate/witness/bounded.rs

## Examined scope and bindings

```yaml
scope:
- id: FR-202-AC-12
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A cyclic undelivered model has action slow delivering with probability
    1/2 and otherwise returning undelivered, and fast delivering with probability
    3/4 and otherwise returning undelivered. For origin eventually[0,2] holds(delivered),
    slow at both full choice keys refutes minimum AtLeast 4/5 with the two-failure
    path of mass 1/4 greater than 1/5; fast at both keys refutes maximum AtMost 9/10
    with first-hit paths of masses 3/4 and 3/16, sum 15/16. Equal-weight resolved
    workload has per-step delivery 5/8: all two-failure action/draw cylinders sum
    to 9/64 and refute AtLeast 7/8; all first-hit cyl'
- id: FR-202-AC-13
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-12 initial and one-failure states share model content but carry different
    complete formula keys with remaining contexts 2 and 1. Changing a supplied key
    to a model-only key, null/local-index monitor, another remaining horizon or binding
    prevents a token. Rebinding to a changed source package, horizon, threshold, claim
    occurrence, actual S3 over-parameter node or selected instance prevents reuse
    of the original token/evidence as an accepted refutation of that item. A checked
    source with two possible object values is restricted to one admitted over binding;
    replay uses that object's predicate
- id: FR-202-AC-14
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: At horizon 0, initially true P gives an empty accepting path of mass 1
    refuting AtMost 1/2, while initially false P gives an empty rejecting path refuting
    AtLeast 1/2; opposite-verdict empty paths yield parity. At horizon 2, a terminal
    initially false P closes by false extension with mass 1 and no fabricated stutter
    step, refuting AtLeast 1/2. A live undecided prefix after one failure does not
    refute; after two failures horizon rejection does, even though another model action
    is enabled. An initial true P path extended by an operation cannot reproduce.
- id: FR-202-AC-15
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-12 a missing represented path cannot be counted, equal-threshold
    represented mass yields parity, and a changed action/draw/post-state, duplicated
    draw-cylinder, overlapping path or inconsistent choice at one full key cannot
    produce a token. First-hit masses under slow are only 3/4 and cannot refute AtMost
    4/5. A policy may choose different enabled actions at the same model content with
    different remaining horizons; this is a consistent full-product policy.
- id: FR-202-AC-16
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-12 replay with pre-cancellation, cancellation at its final acceptance
    poll, or each configured expansion/work/storage/arithmetic budget exhausted returns
    its typed incomplete cause with no truth, token or accepted evidence in a refuted
    record. A refused predicate preserves its typed refusal; FR-197 Bounds and PrecisionBudget,
    unsupported bounded until/always/reward/timed/fair inputs, subsystem evidence
    and nonreplayed exact results produce no bounded reproduced token. The native
    entry needs neither S4 emission nor QSpec FR-413 document transport.
bindings:
- test_id: cyclic_model_has_finite_debt_products_and_replays_both_bounds_and_workload
  ac_id: FR-202-AC-12
  trace: correct
- test_id: cyclic_model_has_finite_debt_products_and_replays_both_bounds_and_workload
  ac_id: FR-202-AC-13
  trace: correct
- test_id: full_key_source_and_semantic_cylinder_mutations_never_reproduce
  ac_id: FR-202-AC-13
  trace: correct
- test_id: full_key_source_and_semantic_cylinder_mutations_never_reproduce
  ac_id: FR-202-AC-15
  trace: correct
- test_id: zero_horizon_and_terminal_false_extension_need_no_model_steps
  ac_id: FR-202-AC-14
  trace: correct
- test_id: insufficient_missing_and_equal_mass_are_parity_not_refutations
  ac_id: FR-202-AC-15
  trace: correct
- test_id: cancellation_and_configured_native_limits_never_return_a_token
  ac_id: FR-202-AC-16
  trace: correct
- test_id: final_acceptance_cancellation_has_no_truth_or_evidence
  ac_id: FR-202-AC-16
  trace: correct
- test_id: all_workload_first_hit_cylinders_have_exact_mass_fifty_five_over_sixty_four
  ac_id: FR-202-AC-12
  trace: correct
- test_id: all_workload_first_hit_cylinders_have_exact_mass_fifty_five_over_sixty_four
  ac_id: FR-202-AC-15
  trace: correct
- test_id: adapter_exhaustion_preserves_incomplete_without_accepted_evidence
  ac_id: FR-202-AC-16
  trace: correct
- test_id: a_decided_initial_path_cannot_continue_and_policy_keys_keep_horizons_distinct
  ac_id: FR-202-AC-14
  trace: correct
- test_id: a_decided_initial_path_cannot_continue_and_policy_keys_keep_horizons_distinct
  ac_id: FR-202-AC-15
  trace: correct
- test_id: interval_bounds_and_precision_budget_cannot_be_cast_into_bounded_refutations
  ac_id: FR-202-AC-16
  trace: correct
- test_id: one_admitted_over_binding_uses_the_actual_s3_parameter_node
  ac_id: FR-202-AC-13
  trace: correct
- test_id: an_exact_engine_value_cannot_settle_refuted_after_bounded_replay_disagrees
  ac_id: FR-202-AC-15
  trace: correct
- test_id: an_exact_engine_value_cannot_settle_refuted_after_bounded_replay_disagrees
  ac_id: FR-202-AC-16
  trace: correct
```
