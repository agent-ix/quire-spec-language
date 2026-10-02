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

Scope: FR-163-AC-2, FR-163-AC-3, FR-163-AC-4.

## Test Procedure

Fixtures: TC-645's `NonInterference` certificate; ADR-023 §8.1's leaky
vault; FR-181-AC-5's HP-5 body over the secure vault.

1. Check the `NonInterference` certificate with one member removed; with
   its keys unsorted; with an added member whose `l` components differ, in
   a component marked `MissingAcceptance`; with the initial product state
   removed.
2. Check a certificate for the leaky `NonInterference` listing its 14
   product states with the secure run's components; check a certificate for
   FR-181-AC-5's body listing its reached product states.
3. Check the `NonInterference` certificate with `max_states` 1, then twice
   with the default limits.

Tag the tests `#[trace("TC-646", "<AC id>")]`.

## Expected Results

- Step 1: `Rejected{Closure}` naming the member whose successor is missing;
  `Rejected{Malformed}` before any recomputation; `Rejected{Component}`;
  `Rejected{Initial}`.
- Step 2: `Rejected{Component}`; `Rejected{Undefined}` at the first state
  with `l = 1`.
- Step 3: `Stopped(ResourceExhausted, MaxStates)` naming the limit and its
  value; equal results.
