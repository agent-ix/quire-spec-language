---
id: TC-252
title: "Checked type node to kernel ValueType is total, tested per type-node form including a sum"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-252: Checked type node to kernel ValueType is total, tested per type-node form including a sum

## Description

ADR-013 C-26's validation row requires "test per type-node form, including
a sum." This test enumerates every checked type-node form C-26 converts and
gives the sum form its own dedicated coverage of the node-id/`VariantId`
preservation rule. Scope: FR-088-AC-9, FR-088-AC-10.

## Test Procedure

1. Enumerate every checked type-node form the checker admits:
   `scalar_type`, `composite_type`, `bounded_domain`, and the sum form
   (ADR-013 O-14).
2. For each non-sum form, construct a checked node of that form, run C-26's
   conversion, and confirm the result is a kernel `ValueType` of the
   matching shape with no panic and no substitution of a different form.
3. For the sum form: construct a checked sum-type node with at least two
   variants (one a plain member, one wrapping another composite type),
   record the node's own checked node id, and run C-26's conversion.
4. Confirm the source checked sum-type node's own id, read from
   `CheckedGraph`/`CheckedPackage` after the conversion, is unchanged from
   what it was before the conversion (not re-minted by C-26) — this is a
   property of the source node, not a value the kernel `ValueType` carries;
   separately, confirm each variant's `VariantId` is present in the
   converted kernel `ValueType` and computed from the declaring sum node and
   that variant's member (ADR-013 O-06, QC-15) — not from the variant's
   position in a list.
5. Confirm the kernel sum shape carries no `NodeKey` anywhere in its
   variant representation, and no `NodeKey` anywhere else in the converted
   `ValueType` either.
6. Construct a sum value from the converted type and confirm it carries its
   `VariantId`, never a bare integer variant index in its place. Then, from
   source, reorder the declared variants of an unordered enum and confirm
   the enum's node id and every variant's `VariantId` are unchanged; reorder
   the declared variants of an ordered enum and confirm the enum's node id
   and every variant's `VariantId` change, because the ordered member list
   is part of the enum's declaration identity (QSpec FR-141).
7. Add a checked type-node form to the checker's admitted set (a test-only
   addition) with no corresponding C-26 match arm, and confirm the crate
   fails to compile (`clippy::wildcard_enum_match_arm` or an equivalent
   exhaustiveness failure), rather than the conversion falling through to a
   default or panicking at runtime.

## Expected Results

- Step 2: every non-sum form converts correctly with no panic.
- Step 4: the source node's own id is unchanged (not re-minted) after
  conversion; each variant's `VariantId` is present in the kernel output and
  derived from (declaring sum, member), not position.
- Step 5: no `NodeKey` appears anywhere in the converted kernel `ValueType`.
- Step 6: sum values carry `VariantId`, never an index in its place;
  reordering an unordered enum changes no `VariantId`, and reordering an
  ordered enum changes its node id and every `VariantId`.
- Step 7: the added form fails to compile against C-26's conversion,
  demonstrating no `_`-arm fallback exists.
