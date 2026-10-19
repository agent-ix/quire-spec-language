---
id: TC-1082
title: "The export result partitions the requested items into bound and refused, including when none is bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-1082: The export result partitions the requested items into bound and refused, including when none is bound

## Description

Verify that the items carrying bindings are exactly the bound items, so a driver can select them for the CG request, and that a request with no bound item succeeds with one refusal per item.

Scope: FR-307-AC-6.

## Test Procedure

1. With no element bound, request a clause item reading only ConfigVersion fields and an item whose extent domain ranges over the ConfigVersion population.
2. Bind the ConfigVersion object type and repeat the request.

## Expected Results

1. The export succeeds and returns two `missing_declaration`/`missing-name` refusals, one per item, each with no bindings; no item carries bindings.
2. The clause item carries its `ObjectBinding`; the extent-domain item is still refused with no bindings. The items carrying bindings are exactly the one bound item.
