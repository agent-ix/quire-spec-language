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

Verify that `check_smt_proof` accepts Alethe refutations of the queries
FR-315 encodes, both for an inductive proof, and rejects a query mismatch,
an invalid proof step, a proof that does not refute and a
certificate of the wrong shape.

Scope: FR-314-AC-1 and FR-314-AC-2.

## Test Procedure

1. Check FR-314-AC-1's `BoundedComplete{depth: 3}` certificate for
   `always[0,3] holds(c.value <= 3)` and its `Inductive{depth: 1}`
   certificate for `always holds(c.value <= 3)` over `Counter`; settle each.
2. Check the bounded certificate with its query at depth 2; with one
   step's premise replaced by a later step; with its last step removed. Check the inductive certificate
   with an invalid step proof, and a bounded certificate offered for the
   inductive basis. Settle each.
3. Settle the bounded result with no certificate; check and settle the
   bounded certificate with one step's rule replaced by `hole`, then by
   `lia_generic`.

Tag the tests `#[trace("TC-893", "FR-314-AC-n")]`.

## Expected Results

- Step 1: both accepted; `proved`, `Certified`, success.
- Step 2: `QueryMismatch` at `Unrolling`; `ProofStepInvalid` at that step;
  `NotRefutation`; `ProofStepInvalid` at the
  `Step` step; `ShapeMismatch`; each `inconclusive`,
  `CertificateRejected`.
- Step 3: `proved`, `Uncertified`; `Unverifiable` at that step and
  `proved`, `Uncertified`, twice.

