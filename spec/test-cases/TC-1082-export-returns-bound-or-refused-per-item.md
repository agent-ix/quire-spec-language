---
id: TC-1082
title: "The export returns a bound result or a refusal for every requested item"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-1082: The export returns a bound result or a refusal for every requested item

## Description

Verify that the export returns a bound result or a refusal for every requested item, including a request in which nothing is bound and an item that references no element.

Scope: FR-307-AC-6.

## Test Procedure

1. With no element bound, request a clause item reading only ConfigVersion fields and an item whose extent domain ranges over the ConfigVersion population.
2. Request an item that references no model element.

## Expected Results

1. The export succeeds and returns two `missing_declaration`/`missing-name` refusals, one per item, each with no bindings.
2. The item returns a bound result with an empty binding set, not a refusal.
