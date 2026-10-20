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

Scope: FR-149-AC-1 to FR-149-AC-11, FR-144-AC-7.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter` with and without its `ensure`
row, and its lost-update model; the light ring fixture `RingIsQueue`
(FR-139's) built unmetered, for a meter sweep of its mapping rows; `HeavyEnq`
(FR-141-AC-9); a model whose concrete contract costs more than B;
`CasRefinesCounter` without its `ensure` row with `peek` and `begin` of the
second thread never enabled, for a terminal chain; a concrete model with the
precondition `6 mod (2 - self.value) >= 0`, for `BadState`; and a concrete model
whose state constraint has a refused family, for an unnamed expansion stop.

1. Run the refinement without the `ensure` row and check its certificate.
2. Check the tampered certificates of FR-149-AC-2.
3. Build FR-149-AC-3's certificate for the lost-update model and check it.
4. Check the AC-1 certificate with `max_transitions` 1, a certificate naming
   another refinement node, and the AC-1 certificate twice.
5. Settle `CasRefinesCounter` without and with its `ensure` row, and with
   step 2's first tampered certificate.
6. Check the unmetered `RingIsQueue` certificate under a meter that runs out in
   the initial states' mapping rows, then under a larger one that runs out in a
   step's mapping row; check the concrete-contract model's certificate under B;
   build a `HeavyEnq` certificate under a larger meter, then check it under B
   (FR-149-AC-5).
7. Check the terminal-state fixture's certificate, with and without its stutter
   edge (FR-149-AC-6).
8. Check the malformed certificates and the refused-contract position of
   FR-149-AC-7.
9. Serialize the rejections of AC-3 and AC-2, and one verdict of each other
   kind (FR-149-AC-8).
10. Settle items whose certificate names another `concrete_package` and another
    `obligation` (FR-149-AC-9).
11. Cancel a check between positions, and force an expansion stop that names no
    limit (FR-149-AC-10).
12. Check the `RegisterHistory` certificate whose positions differ only in
    history, then the same certificate with one position's history key
    replaced by another's (FR-149-AC-11).

Tag the tests `#[trace("TC-556", "<AC id>")]`.

## Expected Results

- Step 1: one position per stored product state; `Accepted`.
- Step 2: rule `PostDiffers`; rule `PostDiffers`; rule `SuccessorsDiffer`;
  each FR-338's `CertificateRejection` with a `SimulationStep` locus.
- Step 3: rule `StepFails` with verdict `AbstractStepRejected{transition:
  inc(c), cause: Postcondition}`.
- Step 4: `Stopped(LimitReached{MaxTransitions, 1})`;
  `stale_dependency`/`content-mismatch`; equal results.
- Step 5: `proved`, `Certified`; `proved`, `Uncertified`; `inconclusive`,
  `CertificateRejected`.
- Step 6: `Stopped(LimitReached{EvaluationMeter, B})` for each of the four, never `Rejected`.
- Step 7: `Accepted`; rule `SuccessorsDiffer`.
- Step 8: rule `Malformed` at the positions AC-7 names, three times; rule `BadState`.
- Step 9: the locus of FR-149-AC-8; `null` verdict for `PostDiffers`; each kind's members.
- Step 10: `stale_dependency`/`content-mismatch` twice; the checker is not called.
- Step 11: `Stopped(Cancelled{source})`; `Stopped(ResourceExhausted)`; each settles V-7.
- Step 12: `Accepted`; then `Rejected`.
