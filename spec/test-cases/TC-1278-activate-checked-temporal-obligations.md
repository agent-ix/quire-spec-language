---
id: TC-1278
title: "Activate checked temporal obligations by semantic identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: verifies
---
# TC-1278: Activate checked temporal obligations by semantic identity

## Description

Checked-route successor controls for FR-044. Each case compiles a temporal
declaration through S1-S4 and resolves activation from the checked activation
record plus the independently supplied trace. TC-123 remains the legacy
emitted-artifact procedure and is not relabeled by this successor.

Executable controls are in
`tests/it/checked_temporal_successors.rs`. Expected identities and capture
outcomes come from the authored declaration and supplied trace identities.

## Test Procedure

1. Run `checked_capture_bound_valuations_are_isolated_by_instance_identity`.
   Deliver distinct semantic triggers and require each checked instance's
   capture-bound valuation to remain isolated from its sibling.
2. Run `checked_activation_keeps_siblings_and_capture_identity`. Deliver one
   semantic trigger twice under distinct receipts and require one instance with
   both receipts; change the payload under that identity and require a typed
   contradiction.
3. Run `checked_capture_is_restored_once_and_eviction_is_incomplete`. Require
   the original capture and one evaluation to remain after replay, and an
   evicted capture to make the instance incomplete without reevaluation.
4. Run `checked_capture_failures_are_typed_and_local_to_their_instances`.
   Supply missing, null, wrong-type, stale and anchor-mismatched capture inputs;
   require each exact typed capture failure and preserve the unrelated healthy
   instance in the same evaluation.
5. Run `checked_activation_dispositions_never_become_temporal_truth`. Require
   inactive, unknown-open, missing-evidence, refused-evidence and guard-false
   dispositions to carry no temporal truth and to leave sibling activation
   inspectable.
6. Run `checked_capture_initializer_cannot_read_a_later_capture` and
   `checked_capture_is_restored_once_and_eviction_is_incomplete`. Require
   authored capture order, refusal of a forward capture reference, and exactly
   one initialization across incremental replay and restoration.
7. Run `checked_activation_keeps_siblings_and_capture_identity` and
   `checked_activation_dispositions_never_become_temporal_truth`. Require a
   false guard to create no instance, and an unestablished guard to remain a
   typed incomplete activation rather than becoming false.
8. Run `checked_whole_execution_origins_keep_distinct_identities`. Deliver two
   admitted execution-origin identities and a duplicate receipt; require two
   non-aliasing instances, stable identity across replay, and typed refusal for
   a conflicting payload under one origin identity.
9. Run `checked_capture_is_restored_once_and_eviction_is_incomplete`. Evict a
   retained capture and require an incomplete outcome naming that capture with
   no later initializer evaluation.

## Expected Results

Only the semantic trigger or execution-origin identity keys an instance.
Receipts add provenance and never create a new instance. Captures are immutable
after activation; one failed instance does not erase a healthy sibling.
Inactive, unknown, incomplete and refused outcomes never become temporal truth.
