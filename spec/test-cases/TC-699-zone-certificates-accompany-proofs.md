---
id: TC-699
title: "Every zone-engine proof carries a canonical zone certificate"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: verifies
---
# TC-699: Every zone-engine proof carries a canonical zone certificate

## Description

Verify the reachability, Büchi and divergence parts of zone certificates and their canonical bytes.

Scope: FR-244-AC-1 to FR-244-AC-3.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; FR-240's liveness fixtures; the `Serve` model.

1. Produce `NoLateReply`'s certificate with `T = 4 ms` on the zone search and check it with FR-245.
2. Produce the certificates of FR-240-AC-1 and AC-2's proofs.
3. Produce the time-lock-freedom certificate over `Rpc` twice.

Tag the tests `#[trace("TC-699", "FR-244-AC-n")]`.

## Expected Results

- Step 1: a reachability part only, every coverage holding, the item's identity; accepted.
- Step 2: components respecting the numbering, each with its reason; `fair weak serve` named for the `idle` component.
- Step 3: a divergence path to a quiescent node for every node; byte-equal bytes.
