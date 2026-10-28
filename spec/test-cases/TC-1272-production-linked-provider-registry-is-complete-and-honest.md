---
id: TC-1272
title: "The production provider snapshot exposes only registered linked engines"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-489
    type: verifies
---
# TC-1272: The production provider snapshot exposes only registered linked engines

## Description

Verify that the FR-345 production snapshot is built from the linked engine
collection, advertises only capabilities of its real providers, and reports
the currently absent refinement-witness providers as absent.

## Test Procedure

1. Build two independent snapshots with
   `RegistrySnapshot::production()`.
2. Query each snapshot for EN-1's `temporal-satisfaction` capability. Query
   EN-4's `probabilistic-satisfaction` capability with the evidence kind in
   EN-4's descriptor, following FR-075-AC-13. Then query for the
   `ValueValidity` and `OperationContract` capabilities used by FR-345's
   witness rows.
3. Construct a test-only factory with an injected descriptor source that
   returns a typed construction error; assert it returns that error and no
   snapshot. Construct another test-only factory with an injected registry
   registration operation that refuses EN-4 after EN-1 is accepted; assert it
   returns the typed refusal and no partial snapshot.
4. Pass each injected-factory error through the CLI error mapper. Assert that
   each produces FR-344 verdict `tool failure`, exit 30, and neither exit 21
   nor a partial production report. Then run the real production snapshot's
   no-provider witness case through the FR-345 report path.

Tag the tests `#[trace("TC-1272", "FR-489-AC-n")]` for the criteria each
assertion verifies.

## Expected Results

- The two snapshots are equal. The registration accessor yields exactly the
  EN-1 `temporal-satisfaction` descriptor and the EN-4
  `probabilistic-satisfaction` descriptor owned by `Engines::qsl()`. For each
  descriptor, the snapshot retains its exact `BackendId`, origin and
  capability set. Candidate queries yield EN-1 only for
  `temporal-satisfaction` and EN-4 only for `probabilistic-satisfaction` when
  supplied the evidence kind in EN-4's descriptor;
  neither descriptor is replaced or supplemented with an ad hoc candidate.
- The `ValueValidity` and `OperationContract` candidate outcomes are empty
  while their provider tickets are incomplete. The no-provider witness result
  remains the FR-345 empty-candidate regression, never `unresolved
  (unsupported)`.
- The injected descriptor-source error and registration refusal are each
  returned as their typed production registry error. The CLI emits FR-344
  tool failure, exit 30, for each and does not return an expected unsupported
  exit or a partial snapshot.
