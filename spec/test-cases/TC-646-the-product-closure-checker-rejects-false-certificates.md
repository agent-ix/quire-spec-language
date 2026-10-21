---
id: TC-646
title: "The product-closure certificate checker rejects tampered and false certificates and stops at a limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: verifies
---
# TC-646: The product-closure certificate checker rejects tampered and false certificates and stops at a limit

## Description

Verify that `check_product_closure` rejects each kind of tampered or false
certificate with the first failing rule and its member, never accepts a
certificate for a false claim, and stops at a caller limit.

Scope: FR-163-AC-2, FR-163-AC-3, FR-163-AC-4, FR-163-AC-5,
FR-163-AC-6, FR-163-AC-7.

## Test Procedure

Fixtures: TC-645's `NonInterference` certificate; ADR-023 §8.1's leaky
vault; FR-181-AC-5's HP-5 body over the secure vault.

1. Check the `NonInterference` certificate with one member removed; with
   its keys unsorted; with an added member whose `l` components differ, in
   a component marked `MissingAcceptance`; with one member listed in two
   components; with two components joined by an edge listed in the reverse
   order; with the initial product state removed.
2. Check a certificate for the leaky `NonInterference` listing its 14
   product states with the secure run's components; check a certificate for
   FR-181-AC-5's body listing its reached product states.
3. Check the `NonInterference` certificate with `max_states` 1, then twice
   with the default limits.
4. Remove `components` from HP-2 and HP-3 certificates, then repeat with
   an empty closure. Observe the successor-recomputation boundary: it is
   never called. Every case rejects `Malformed` at certificate member
   `components`, with no state key.
5. Check HP-1 certificates over FR-179-AC-2's nondeterministic `Det` and
   FR-179-AC-4's undefined relation. Compare every edge of the reported
   tuple (pre-state, transition label, post-state and result), in binding
   order, with the first failing tuple of FR-179's canonical enumeration.
   Projecting to pre-state keys alone must fail this comparison.
6. Serialize and read an HP-3 key for zero universal variables with a
   nonempty witness set, then with an empty witness set. Expect `states`
   to remain `[]` in both, and the witness set to retain its actual value.
   Removing `states` or the HP-3 witness set is a different, invalid shape;
   inserting a fabricated component is never a repair.

Tag the tests `#[trace("TC-646", "<AC id>")]`.

## Expected Results

- Step 1: `Rejected` with rule `SuccessorMissing` naming the member whose successor is missing;
  `Rejected` with rule `Malformed` before any recomputation; `Rejected` with rule `WitnessFails`;
  `Rejected` with rule `NotPartition`; `Rejected` with rule `BackwardEdge`;
  `Rejected` with rule `InitialMissing`.
- Step 2: `Rejected` with rule `NotPartition`; `Rejected` with rule `UndefinedMember` at the first state
  with `l = 1`.
- Step 3: `Stopped` with `{MaxStates, 1}` naming the limit and its
  value; equal results.
