---
id: TC-417
title: "Model declaration, Reference and Population nodes key to the golden vectors under ModelOwner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: verifies
---
# TC-417: Model declaration, Reference and Population nodes key to the golden vectors under ModelOwner

## Description

Verify that `check` keys each model declaration node with a `ModelOwner` and
a `null` declaration, keys the `Reference<T>` and `Population<T>[N]` type
nodes as anonymous structural nodes over that model node, names the static
object type's node in every model member, records the model correspondence
itself, and treats an unmapped `EffectiveId` or domain package as an internal
fault.

This catches a key computed from an `EffectiveId` (which moves when the
normalization rules change), an owner that carries the domain-package
version (which gives one `Order` declaration two ids across a version-only
change), a member naming the supertype that
declares an operation instead of the receiver's static type, and a
correspondence left caller-supplied.

Scope: FR-094-AC-1, FR-094-AC-2, FR-094-AC-3, FR-094-AC-4, FR-094-AC-7 (its
`Reference` and `DeclarationKey` cases), FR-094-AC-9, FR-094-AC-10,
FR-094-CON-1, FR-094-CON-2.

## Test Procedure

Every fixture admits the domain package `acme/orders` that FR-094's Golden
vectors section describes, at version `1.0.0` unless a step says otherwise,
with `M::Order`, `M::Invoice` and `M::Sub` resolving to its object types.
`Sub` has supertype `Order`, inherits `total` and redefines `size`.

1. Check `function g using v(r: Reference<M::Order>, s: Reference<M::Invoice>): Boolean pure { true }`.
   Read the type nodes of `r` and `s`, their model nodes, and the model
   correspondence.
2. Repeat step 1 with `acme/orders` `2.0.0` admitted.
3. Key the model node of the relationship `ix://acme/orders/billedTo`.
4. Check functions over `p: Population<M::Order>[3]` and `r: Reference<M::Order>`
   whose bodies are `size(allInstances<M::Order>(p))`,
   `lookup<M::Order>(p, r) absent undefined`,
   `lookup<M::Order>(p, r) absent empty`, `deref(r).total` and `r.size()`.
   Read each parameter node, type node and application node.
5. Check `p: Population<M::Order>[5]` and `q: Population<M::Invoice>[3]`, and
   read their type nodes.
6. Check `deref(s).total` and `s.size()` over `s: Reference<M::Sub>`, and read
   each member's `declaration`.
7. Call the model-node keying function with an `EffectiveId` that no admitted
   view's `type_identities` holds, and with a `DeclarationKey` whose `package`
   is `acme/other`, and with a `DeclarationKey` whose `node` is empty. Call
   the record-kind `match` with a field member record.
8. Scan the model-declaration-node `match` for a `_` arm, and scan every
   model-owned preimage for an owner of kind `source` or `definition`.
9. Repeat step 4 with source aliases `Ref = Reference<M::Order>` and
   `Bucket = Population<M::Order>[3]`, then resolve an alias naming no type.
10. Apply `RM-REF-TARGET` to R1, `RM-POP-MAX` to PO1 and `RM-CU-ORDER` to U2,
    retaining each original node id while refreshing the package projection
    and package id. Repeat each mutation after replacing the node id with the
    changed preimage digest.

Tag the tests `#[trace("FR-094-AC-n", "TC-417")]` with the AC each backs.

## Expected Results

- Step 1: `r`'s type node keys to R1 over M1 and `s`'s model node to M3,
  byte for byte. M1's preimage has the `ModelOwner` of FR-094-AC-1 and
  `declaration` `null`; R1's has no `owner`. The correspondence holds exactly
  (M1, `Order`'s key) and (M3, `Invoice`'s key).
- Step 2: the same M1 and R1 as step 1, byte for byte, and the same
  correspondence entry for M1.
- Step 3: M4.
- Step 4: P5 over PO1, whose `semantic_type` is S1, and P6. E4 with
  `result_type` S2, E5 with R1, E6 with R3, E7 and E8 (member `field` naming
  M1 and `total`) and E9 (member `operation` naming M1 and `size`).
- Step 5: PO2 and PO3, which differs from PO1.
- Step 6: `s`'s type node is R4, and both members name `Sub`'s model node
  M5, not M1.
- Step 7: each of the four refuses as an internal fault naming that value,
  and yields no key.
- Step 8: no `_` arm, and no `source` or `definition` owner.
- Step 9: both aliases name the already settled R1 or PO1 node, no alias node
  is emitted, and the unknown alias refuses `missing_declaration` /
  `missing-name` before lowering.
- Step 10: the retained-key candidates are schema-valid but refuse
  `invalid_package` / `stale-node-key`; the QSpec executable vector test also
  records that each changed preimage has a fresh digest when its node id is
  recomputed. Semantic admission or refusal of a malformed recomputed form is
  outside this key-stage procedure.

## Status

Implemented (#384). The tests back every step.
