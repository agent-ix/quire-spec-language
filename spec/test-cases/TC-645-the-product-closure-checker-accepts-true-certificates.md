---
id: TC-645
title: "The product-closure certificate checker accepts the certificates of true hyper proofs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: verifies
---
# TC-645: The product-closure certificate checker accepts the certificates of true hyper proofs

## Description

Verify that `check_product_closure` accepts the certificate EN-1 writes for
a proved HP-2, HP-2 under copy-swap, HP-3, HP-1 and HP-5 item, using only
core code.

Scope: FR-163-AC-1.

## Test Procedure

Fixtures: ADR-023 §8.1's secure vault with `NonInterference`, with and
without copy-swap; §8.2's secure vault with `Opaque`; FR-179-AC-1's `Det`;
FR-181-AC-1's HP-5 claim over the secure vault.

1. Run each item through EN-1 and take the certificate from its `Holds`.
2. Check each certificate with `check_product_closure` from the recompiled
   package, with the `qsl-analyze` crate not linked.

Tag the tests `#[trace("TC-645", "FR-163-AC-1")]`.

## Expected Results

- Step 1: `Holds` for each; the `NonInterference` certificate has 8
  product states and components `MissingAcceptance` or `Trivial`; the
  copy-swap certificate has 6.
- Step 2: `Accepted` for each of the five certificates.
