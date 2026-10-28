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
2. Query each snapshot for the capabilities exposed by EN-1 and EN-4, then for
   the `ValueValidity` and `OperationContract` capabilities used by FR-345's
   witness rows.
3. Exercise the production factory's descriptor-construction or registration
   error seam with a descriptor failure and with a refused registration.
4. Inspect the CLI error path for each factory error and run the no-provider
   witness case through the FR-345 report path.

Tag the tests `#[trace("TC-1272", "FR-489-AC-n")]` for the criteria each
assertion verifies.

## Expected Results

- The two snapshots are equal and contain exactly EN-1 and EN-4 from
  `qsl_analyze::Engines::qsl()`; their candidate outcomes match their
  descriptors. Neither snapshot contains an ad hoc candidate.
- The `ValueValidity` and `OperationContract` candidate outcomes are empty
  while their provider tickets are incomplete. The no-provider witness result
  remains the FR-345 empty-candidate regression, never `unresolved
  (unsupported)`.
- A descriptor construction or registration error is returned as the typed
  production registry error. The CLI emits a tool-failure report and does not
  return an expected unsupported exit.
