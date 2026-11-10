---
id: SR-202611101414256067252
title: QSL-1826 bounded fair runtime gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2667; qsl-analyze/src/analyze/exact.rs,
  qsl-analyze/tests/it/bounded_fair_certificate.rs, qsl-analyze/tests/it/main.rs,
  qsl-eval/src/simulation/model.rs, qsl-foundation/src/node_wire.rs, qsl-replay/src/lib.rs,
  qsl-replay/src/probability_certificate.rs, qsl-replay/src/probability_certificate/bounded.rs,
  qsl-replay/src/probability_certificate/document.rs, qsl-replay/src/probability_certificate/document/fairness.rs,
  qsl-replay/src/probability_certificate/document/key.rs, qsl-replay/src/probability_certificate/document/mdp.rs,
  qsl-replay/src/probability_certificate/document/writer.rs, qsl-semantics/src/model/state.rs,
  tools/qsl-probability-check/tests/it/bounded_fair.rs, tools/qsl-probability-check/tests/it/main.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
---

## Summary

Independent gap-analysis of frozen PR2667 head 3f9a89c53f1b4ba95c06235f1608aaf53f51a24f against base 15f07c7f1168d1cf98f2607bc6c4e5ed51423810. Ticket: QSL-1826. Rust lane is folded into code-review.

## Verdict

CONDITIONAL — computed repository matrix unavailable; implementation finding FND-001 is recorded by code-review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable active AssuranceProfile found. External qualification boundaries remain explicit.

## Coverage

All sixteen changed Rust files and their reused subject/monitor/action helpers were inspected. Applied code-review with Rust lane, rust-review and rust-style API/implementation/safety/wire/testing references, plus repository AGENTS/CLAUDE. Runtime-only diff: no specification, dependency, lockfile or CI workflow change. No active AssuranceProfile found by anchored type search. No optional whole-repository semantic audit or plan was supplied.

Fairness is checked claim/objective identity, not a seventh product key component. Strict borrowed record decoding denies unknown members; records compare complete shared canonical Writer octets in unsigned lexicographic order, with duplicate/out-of-order denial. Independent checked-set reconstruction resolves each actual declaring MODEL identity through existing metered owner lookup and shared ModelNodeIdentity/foundation projection. Distinct weak/strong and whole/each constraints are retained; the inherited declaring-owner source test rejects a consumer substitution. The backend record is the local profile DTO; structural NodeId ownership and encoding remain shared. No copied schema, owner NodeId recipe, dependency pin or compatibility layer found.

The new fair route reuses qualified bounded monitor context and rank rules, MIN>=/MAX<= numerical operators and actual binding identity. A query-free full original-model admission traversal now establishes the finite Markov/unique-post prerequisite even at horizon zero; it shares caller work/storage with metadata and bounded reconstruction without reading P after its horizon. The actual h0 multi-post counterfeit previously accepted by the coder now rejects. Valid read-only multiple-receiver source has four actions/two receivers/one successor per draw; modifying unresolved multi-post source remains typed unsupported. This is no multi-object solver or fair end-component/policy extension. Canonical empty-fairness legacy bytes and other profile refusals remain.

The shared NodeId text parser has one budgeted implementation; the legacy entry delegates. It admits actual string, stack/path growth and Exact NodeKey::try_new_with reservations, with work before parsing and actual shared projection/encoding hooks. The new full monitor/raw STATE-key path uses existing owner budget hooks and caller ledger. The explicit external ModelTransition private canonical-reader and ordinary interpreter allocation boundaries remain: this review does not establish complete FR201AC30, all-expansion or whole-parent qualification.

Independent thermal command cargo test -p qsl-probability-check --test it bounded_fair passed four new process groups, exit0; log /tmp/qsl1826-bounded-fair-runtime-review/independent-process.log. Coder completed logs were inspected: node-reader10, document18, process11 and native3 pass, all-target/all-feature scoped Clippy and formatting pass. The checked-set guard-only counterfactual actually accepted forged metadata at24/25 and failed the intended oracle; restored source passed. Requested/Deadline callbacks assert reached phases and typed cancellation; byte/state/edge/work/storage/representation/arithmetic ceilings deny acceptance. These are representative controls, not exhaustive assurance of every subclause.

An additional independent source control reproduced FND-001 without changing the branch. A scratch Rust test reused the existing bounded fixture helper and exact dependency fingerprints of the built process suite, generated a valid fair h0 certificate, changed only its source P to 1 mod m.attempts = 0, and called the real fresh-process checker. Output was (2, {status:stopped,limit:null,reason:probability certificate graph stopped: ExpansionRefused}). The branch match at bounded.rs:192-199 confirms Undefined is the wildcard path. Scratch sources, compiler log and reproduction output are retained under /tmp/qsl1826-bounded-fair-runtime-review/. This is typed-cause loss, not accepted false truth.

The full computed matrix was attempted with quire matrix --scope . --format json, emitted zero JSON and was interrupted with SIGINT; stderr retained. No handwritten matrix or coverage percentage substitutes for it. New behavior has FR201AC47-50/TC1406 trace attributes; no other concrete new unspecified behavior, stub, ignored new test or coverage inflation found. Gap-analysis is conditional while repository-wide matrix and qualification remain unavailable. No aggregate, makeci, Kani or unrelated exhaustive poll sweep ran. New owner instruction holds PR2667 unmerged pending terra QSL12 stack; root/coder own full test-binary qualification on exact final head after fixes. Codex1 owns cyclic_reward_certificate.rs and exact_probabilistic_item.rs fixes; neither was edited.

Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: FR-201-AC-47
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: The bounded fair profile admits the exact Every/minimum AtLeast and maximum
    AtMost slice with nonempty checked weak/strong whole/each operation fairness;
    horizon zero and transported 0/1 values work with interior source bounds. The
    fair and unfair bounded fixture both have minimum 24/25, while fair result/objective
    retain the exact checked set. Fairness-empty documents retain existing bytes.
    Unsupported shapes and zero instances emit no new-profile proof.
- id: FR-201-AC-48
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Canonical fairness records reuse the closed shared operation target and
    independently match the resolved checked set, including inherited declaring identities
    and whole/each receiver semantics. Authored reorder/duplicate spellings normalize
    identically; missing, extra, changed-kind/granularity/model/name, duplicate or
    out-of-order records and malformed/foreign targets cannot establish the claim.
    Distinct weak/strong constraints are retained.
- id: FR-201-AC-49
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Independent bounded reconstruction checks exact targets, optimization,
    complete six-part keys, actual monitor context and remaining-horizon ranks without
    producer linkage. Wrong identity/binding/receiver, goal, monitor, horizon, values/ranks,
    coverage or canonical transport cannot be accepted, with existing precise rule/locus.
    Numeric invariance introduces no fair-MEC, policy, seventh key part or new vacuity
    outcome.
- id: FR-201-AC-50
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual Requested/Deadline cancellation and byte/state/edge/work/storage/representation/arithmetic
    exhaustion during checked fairness/source declaration intake, structural identity
    traversal/encoding, canonical writing, raw/full-key admission and independent
    graph/arithmetic/rank checking retain typed causes without accepted truth. Earlier
    source refusals and unsupported/undefined evaluation remain distinct. External
    transition/private-reader and ordinary interpreter allocation boundaries are not
    claimed qualified.
bindings:
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::actual_fairness_stages_preserve_requested_deadline_and_caller_ceilings
  ac_id: FR-201-AC-47
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::actual_fairness_stages_preserve_requested_deadline_and_caller_ceilings
  ac_id: FR-201-AC-48
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::actual_fairness_stages_preserve_requested_deadline_and_caller_ceilings
  ac_id: FR-201-AC-50
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::empty_multiple_bindings_and_refuted_fair_bound_never_emit_new_profile_proofs
  ac_id: FR-201-AC-47
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::empty_multiple_bindings_and_refuted_fair_bound_never_emit_new_profile_proofs
  ac_id: FR-201-AC-49
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::multiple_receivers_require_actual_markov_draws_and_keep_each_identity
  ac_id: FR-201-AC-47
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::multiple_receivers_require_actual_markov_draws_and_keep_each_identity
  ac_id: FR-201-AC-48
  trace: correct
- test_id: qsl-analyze/tests/it/bounded_fair_certificate.rs::multiple_receivers_require_actual_markov_draws_and_keep_each_identity
  ac_id: FR-201-AC-50
  trace: correct
- test_id: qsl-foundation/src/node_wire.rs::text_parser_admits_real_blocks_before_allocating_and_preserves_caller_cancellation
  ac_id: FR-201-AC-50
  trace: correct
- test_id: qsl-replay/src/probability_certificate/document/fairness.rs::shared_structural_metadata_walk_admits_deep_nodes_and_text_before_writing
  ac_id: FR-201-AC-50
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::weak_strong_whole_each_preserve_actual_minimum_and_both_bounded_extrema
  ac_id: FR-201-AC-47
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::weak_strong_whole_each_preserve_actual_minimum_and_both_bounded_extrema
  ac_id: FR-201-AC-49
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::checked_set_normalizes_authored_order_and_duplicates_but_wire_set_is_strict
  ac_id: FR-201-AC-48
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::inherited_operation_uses_declaring_model_identity_and_rejects_consumer_identity
  ac_id: FR-201-AC-48
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::inherited_operation_uses_declaring_model_identity_and_rejects_consumer_identity
  ac_id: FR-201-AC-49
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::zero_horizon_and_legacy_bytes_keep_their_actual_context_and_closed_rows
  ac_id: FR-201-AC-47
  trace: correct
- test_id: tools/qsl-probability-check/tests/it/bounded_fair.rs::zero_horizon_and_legacy_bytes_keep_their_actual_context_and_closed_rows
  ac_id: FR-201-AC-49
  trace: correct
```
