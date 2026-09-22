---
id: TC-204
title: "The three legacy lowering target names resolve identically through the registry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-079
    type: verifies
---
# TC-204: The three legacy lowering target names resolve identically through the registry

## Description

Verify that `boolean-oracle/v1`, `integer-ir/v1` and `state-scalar-ir/v1`
each still parse via `FromStr`, round-trip via `Display`, and appear in the
CLI's published target list, identically before and after the registry
swap. Scope: FR-079-AC-2.

Catches a migration that drops one of the three names (for example,
folding `state-scalar-ir/v1` into `integer-ir/v1` because their lowering
happens to overlap), or that changes one name's spelling while updating
only its own reverse-lookup table and not its forward `Display`
implementation, producing an asymmetric round-trip that a `Display`-only or
`FromStr`-only check would miss.

## Test Procedure

1. Before the swap, for each of the three legacy names: parse it via
   `FromStr`, confirm success, then format the parsed value via `Display`
   and confirm it equals the original string; also confirm the name
   appears in `ProjectionTarget::ALL` (or its registry-backed successor)
   in the CLI's published target list.
2. Apply the registry-swap implementation.
3. Repeat step 1's three checks for each of the three names after the
   swap.
4. Additionally confirm that a name outside the three (for example
   `nonexistent-target/v1`) is rejected by `FromStr` both before and after
   the swap.

## Expected Results

All three names parse, round-trip, and appear in the published target list
identically before and after the swap. The rejection behavior for an
unknown name in step 4 is unchanged by the swap.

## Status

Steps 1 and 4 are backed for the current catalog by
`legacy_target_names_parse_round_trip_and_list_identically`
(`tests/it/lowering_registry_isolation.rs`). Steps 2 and 3 are subject to
ADR-011-OQ-1.
