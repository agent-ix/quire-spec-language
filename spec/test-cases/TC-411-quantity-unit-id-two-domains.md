---
id: TC-411
title: "A quantity UnitId is a declared unit's node key or a compound unit's digest, and the two never compare equal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-411: A quantity UnitId is a declared unit's node key or a compound unit's digest, and the two never compare equal

## Description

Verify FR-088-AC-12. A kernel `UnitId` is a domain-labelled digest record
over the two unit identities QSpec FR-142 defines: a declared unit's node
key under the `quire.checked-semantic-node/v1` label, and a compound unit's
`quire.value.compound-unit/v1` digest (ADR-013 T-6, OQ-B ruling). Equality
is lexical on domain, then bytes. Scope: FR-088-AC-12.

This catches four faults: a single-domain `UnitId` that drops the label; a
declared unit collapsed to its root, so that `km` equals `m`; a compound
digest that differs from QSpec's vectors; and a non-unit node key accepted
as a declared-arm `UnitId` (ADR-013 C-30).

## Test Procedure

1. Check a package that declares the dimension `Length`, the unit `m` of
   `Length`, and the unit `km` with target `m` and scale 1000.
2. Build the `UnitId`s of `m` and `km` through C-26, and the compound
   `UnitId` of `m^1` through `semantic_value`.
3. For every entry of QSpec's compound-unit vectors
   (`ix://agent-ix/quire-specification/proposals/quire-v1/definitions/value-compound-unit.md`),
   build the compound `UnitId` from the entry's terms.
4. Pass `Length`'s node key to C-30 as a declared-arm `UnitId`.

Tag the test `#[trace("FR-088-AC-12", "TC-411")]`.

## Expected Results

- Step 2: the three `UnitId`s are pairwise unequal, and `m`'s carries `m`'s
  node key under the `quire.checked-semantic-node/v1` label.
- Step 3: each `UnitId` carries the vector's digest under the
  `quire.value.compound-unit/v1` label.
- Step 4: C-30 refuses the key.

## Status

Passed locally: `tests/it/quantities.rs`
`tc_411_unit_ids_are_declared_node_keys_or_compound_digests` (steps 1, 2 and
4) and `tc_411_compound_unit_ids_match_qspec_vectors` (step 3, which reads the
vectors from `$QSPEC_DIR` at run time; `make conformance` runs it and fails
unless it reports a nonzero vector count), with the
kernel equality rule in `quire-exact/src/identity.rs`
`tc_411_unit_id_is_a_two_domain_record_compared_on_label_then_bytes`.
