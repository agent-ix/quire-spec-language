---
id: TC-893
title: "An SMT proof certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
---
# TC-893: An SMT proof certificate is accepted or rejected by the core checker

## Description

Verify that `check_smt_proof` accepts an Alethe refutation of the query
QSL derives, and rejects a query mismatch, an invalid proof step and a
proof that does not refute.

Scope: FR-314-AC-1 and FR-314-AC-2.

## Test Procedure

1. Check FR-314-AC-1's `BoundedComplete{depth: 4}` certificate over
   `Counter`, then settle the item.
2. Check it with the query unrolled to 3; with one step's premise replaced
   by a later step; with its last step removed; settle each.
3. Settle the same result with no certificate.

Tag the tests `#[trace("TC-893", "FR-314-AC-n")]`.

## Expected Results

- Step 1: accepted; `proved`, success, no certification label.
- Step 2: `QueryMismatch` at `Query`; `ProofStepInvalid` at that step;
  `NotRefutation`; each `inconclusive`, `CertificateRejected`.
- Step 3: `proved`, `Uncertified`.

## Status

🚧 Planned.
