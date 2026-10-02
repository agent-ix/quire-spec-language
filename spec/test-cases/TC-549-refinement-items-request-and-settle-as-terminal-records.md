---
id: TC-549
title: "Refinement items request one temporal-satisfaction record and settle as one terminal record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-549: Refinement items request one temporal-satisfaction record and settle as one terminal record

## Description

Verify the refinement's requirement record and TP-5 form, its routing, the
concrete deadlock-freedom item beside it, the settlement of each outcome and
half, and the subject binding of the obligation identity.

Scope: FR-144-AC-1 to FR-144-AC-5.

## Test Procedure

Fixtures: `CasRefinesCounter`, its divergence and lost-update variants
(ADR-020 §8); FR-142-AC-4's `writes` variant; FR-142-AC-6's `only`
variant of `RingIsQueue`; FR-143-AC-4's hidden-field refinement.

1. Write the request for `CasRefinesCounter` with and without its `ensure`
   row, and with `terminal any` on `Impl::Counter`.
2. Settle `CasRefinesCounter` and the divergence variant.
3. Settle the `writes` variant, the `only` variant after its replay, and
   the hidden-field refinement with its `ensure` row.
4. Settle the lost-update refinement with its recorded failure moved to
   position 3, and with an `initial` index of 1.
5. Request `CasRefinesCounter` with abstract initial `value` 0 and with
   `value` 1.

Tag the tests `#[trace("TC-549", "FR-144-AC-n")]`.

## Expected Results

- Step 1: one `temporal-satisfaction` record, `Refinement{liveness: true}`,
  then `false`, routed to the explicit-state provider; a concrete deadlock-freedom item and no abstract one; none with `terminal any`.
- Step 2: `proved`, `closed-scope`, both halves `Proved{Exhaustive}`;
  `refuted`, `decisive-counterexample`, safety `Proved{Exhaustive}`,
  liveness `Refuted`.
- Step 3: `inconclusive`, `Inconclusive(MappingUndetermined)`; `refuted`,
  `decisive-counterexample`, cause `UndefinedEvaluation`;
  `unsupported`, `Unsupported(unsupported-requested-capability)`, safety
  `Proved{Exhaustive}` on the record.
- Step 4: `Inconclusive(ReplayParity)`; `Inconclusive(ReplayRefused)`.
- Step 5: different obligation identities; the second settles `refuted` with
  `InitialNotAbstract`.
