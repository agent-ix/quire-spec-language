---
id: TC-409
title: "An enum value's VariantId is its FR-141 member node key, and its rank orders sets and bags"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-409: An enum value's VariantId is its FR-141 member node key, and its rank orders sets and bags

## Description

Verify FR-088-AC-11. A kernel enum value carries its `VariantId` and its
rank in the QSpec FR-141 canonical member list (ADR-013 O-14, OQ-D and
OQ-F rulings). The `VariantId` is the node key over
`{version: quire.enum-member-node/v1, declaration_node_id, case}`. The rank
is the canonical key, so a set visits an ordered enum's values in
declaration order and an unordered enum's values in case identifier byte
order (QSpec FR-144 enumeration key row, FR-144-AC-9). Identity and
equality use the `VariantId` only, and a rank is a function of its
`VariantId` through the enum shape. Scope: FR-088-AC-11.

This catches five faults: a `VariantId` minted over a private preimage
instead of the FR-141 member node key; a set ordered by `VariantId` digest
(`quire-exact/src/key.rs:103`); an unordered enum ordered by declaration
position; a rank that disagrees with its `VariantId`; and a declaration key
accepted as a `VariantId` (ADR-013 C-30).

## Test Procedure

1. Check a package that declares the ordered enum
   `ordered enum E { b, a, c }` and read `E`'s node key.
2. For each case, compute the expected `VariantId` independently: SHA-256
   over the RFC 8785 JCS bytes of
   `{version: quire.enum-member-node/v1, declaration_node_id: E's node key, case}`.
   Compare with the `VariantId` C-26 produces for that variant.
3. Build the kernel values `E::b`, `E::a` and `E::c`, read each rank, and
   form the kernel set `{E::c, E::a, E::b}`. Record the visiting order.
4. Repeat steps 1 to 3 with the unordered enum `enum U { b, a, c }`.
5. Build a value that pairs `E::a`'s `VariantId` with rank 2, and admit it
   against `E`'s shape.
6. Check the same source again with the declaration renamed from `E` to
   `F`, and repeat steps 2 and 3.
7. Pass `E`'s own declaration node key to C-30 as a `VariantId`.

Tag the test `#[trace("FR-088-AC-11", "TC-409")]`.

## Expected Results

- Step 2: each `VariantId` equals the independently computed member node
  key.
- Step 3: the ranks are `b` 0, `a` 1, `c` 2, and the set visits `b`, `a`,
  `c`.
- Step 4: the ranks are `a` 0, `b` 1, `c` 2, and the set visits `a`, `b`,
  `c`.
- Step 5: admission refuses the value.
- Step 6: every `VariantId` differs from step 2, and every rank and the
  visiting order equal step 3.
- Step 7: C-30 refuses the key.

## Status

Planned; no test backs this case. Remaining work: QSL-131.
