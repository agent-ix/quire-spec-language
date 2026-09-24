---
id: TC-418
title: "Clause function nodes key to the golden vectors with the operation member's ModelOwner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: verifies
---
# TC-418: Clause function nodes key to the golden vectors with the operation member's ModelOwner

## Description

Verify that the clause functions `check::checked_dispatch_operation`
synthesizes key under the `ModelOwner` of the operation member whose clause
they are, carry no declaration, tell a precondition from a body by their
`clause` binding, and keep the synthesized lookup name out of the preimage.

This catches a clause function keyed with a `SourceOwner` it has no source
for, a key that merges two domain-package versions' clauses or two members'
equal clauses, and a key that changes when the synthesized name's spelling
does.

Scope: FR-094-AC-5, FR-094-CON-1, FR-094-CON-2.

## Test Procedure

The domain package is `acme/orders` of FR-094's Golden vectors section, with
the query operations `Order.size(): Integer` and `Order.count(): Integer`.
The receiver parameter of each candidate is `self: Reference<M::Order>`.

1. Run `checked_dispatch_operation` for `Order.size` with the authored
   precondition `true` and the body `7`, under `acme/orders` `1.0.0`. Read the
   receiver parameter node and the key and preimage bytes of each clause
   function.
2. Repeat step 1 under `acme/orders` `2.0.0`.
3. Run it for `Order.count` with the authored precondition `true`, under
   `1.0.0`.
4. Add `Sub.size`, which redefines `Order.size` with the authored
   precondition `false` and receiver `self: Reference<M::Sub>`. Run
   `checked_dispatch_operation` for `Order.size` and for `Sub.size`. Read the
   receiver parameter node, `Sub.size`'s authored and effective precondition
   functions from both runs, and `Order.size`'s authored function.
5. Rebuild step 1 with the synthesized labels changed, and read the keys.
6. Scan the `DeclaredClauseKind` spelling `match` for a `_` arm.

Tag the tests `#[trace("FR-094-AC-5", "TC-418")]`.

## Expected Results

- Step 1: the receiver parameter keys to P7, the precondition to C1 and the
  body to C2. Both preimages carry `owner` `{kind: "model", identity:
  "acme/orders", version: "1.0.0", node: "ix://acme/orders/Order/size"}` and
  `declaration` `null`.
- Step 2: the precondition keys to C3.
- Step 3: the precondition keys to C4.
- Step 4: the receiver parameter keys to P8, `Sub.size`'s authored
  precondition to C5 and its effective precondition to C6 in both runs, each
  one node; the effective function's body references E10, and its owner's
  `node` is `ix://acme/orders/Sub/size`. `Order.size`'s authored function
  keys to C1.
- Step 5: the keys equal step 1's.
- Step 6: no `_` arm.

## Status

Planned; QSL-156 A4b.
