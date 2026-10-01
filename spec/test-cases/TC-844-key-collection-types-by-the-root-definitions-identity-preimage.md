---
id: TC-844
title: "Collection and population nodes are keyed by the root definitions' identity preimage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-334
    type: verifies
---
# TC-844: Collection and population nodes are keyed by the root definitions' identity preimage

## Description

Verify that bound presence and value enter collection and population node
identity and `package_id`, and that identities are deterministic.

Scope: FR-334-AC-1 to FR-334-AC-3.

## Test Procedure

1. Compile two units that differ only in `s: Set<Int>` against
   `s: Set<Int>[0, 18446744073709551615]`; compare node keys, `package_id`s
   and node members.
2. Lower `Population<Account>`, `Population<Account>[4]` and an unbounded
   `Set<Reference<Account>>`; compare node keys.
3. Compile one bounded unit twice; read its definition selections.

Tag the tests `#[trace("TC-844", "FR-334-AC-n")]`.

## Expected Results

- Step 1: different node keys and `package_id`s; no `collection_bounds` on
  the unbounded node; `collection_bounds{0, 18446744073709551615}` on the
  bounded one.
- Step 2: three distinct node keys; no `UnrepresentableBound` refusal.
- Step 3: equal node keys and `package_id`; the selections name the linked
  root definitions.
