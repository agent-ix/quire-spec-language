---
id: TC-806
title: "The export refuses each item with an unbound element and returns the others"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-806: The export refuses each item with an unbound element and returns the others

## Description

Scope: FR-307-AC-1 to FR-307-AC-3.

## Test Procedure

With only ConfigVersion's object type bound, request:

1. a clause item reading only ConfigVersion fields, and an item whose extent
   domain ranges over the ConfigVersion population;
2. an item that references the population and the `attemptUpdate`
   operation, both unbound;
3. the items of step 1 together with an occurrence key that names no
   requirement record.

## Expected Results

1. The first item returns its one `ObjectBinding`; the second refuses
   `missing_declaration`/`missing-name`, naming the population key and its
   `DomainPackageRef` identity, with no bindings.
2. One refusal naming both elements in ascending key order, each with its
   `DomainPackageRef` identity.
3. The unknown key refuses `missing_declaration`/`missing-name` naming
   it; the step-1 items' results are unchanged.
