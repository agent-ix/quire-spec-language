---
id: TC-556
title: "The simulation certificate checker accepts a true relation and rejects a false one"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-556: The simulation certificate checker accepts a true relation and rejects a false one

## Description

Verify that the core checker accepts the certificate EN-1 writes for a
proved safety half, rejects each kind of tampered or false certificate,
stops at a limit, and that settlement labels a proof certified, uncertified
or rejected.

Scope: FR-149-AC-1 to FR-149-AC-4, FR-144-AC-7.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter` with and without its `ensure`
row, and its lost-update model.

1. Run the refinement without the `ensure` row and check its certificate.
2. Check the tampered certificates of FR-149-AC-2.
3. Build FR-149-AC-3's certificate for the lost-update model and check it.
4. Check the AC-1 certificate with `max_transitions` 1, a certificate naming
   another refinement node, and the AC-1 certificate twice.
5. Settle `CasRefinesCounter` without and with its `ensure` row, and with
   step 2's first tampered certificate.

Tag the tests `#[trace("TC-556", "<AC id>")]`.

## Expected Results

- Step 1: one position per stored product state; `Accepted`.
- Step 2: rule `PostDiffers`; rule `PostDiffers`; rule `SuccessorsDiffer`;
  each FR-338's `CertificateRejection` with a `SimulationStep` locus.
- Step 3: rule `StepFails` with verdict `AbstractStepRejected{transition:
  inc(c), cause: Postcondition}`.
- Step 4: `Stopped(ResourceExhausted, MaxTransitions)`;
  `stale_dependency`/`content-mismatch`; equal results.
- Step 5: `proved`, `Certified`; `proved`, `Uncertified`; `inconclusive`,
  `CertificateRejected`.
