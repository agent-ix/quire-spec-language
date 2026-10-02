---
id: TC-808
title: "The export partitions an emission's items into bound results and refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: verifies
---
# TC-808: The export partitions an emission's items into bound results and refusals

## Description

Scope: FR-307-AC-6.

The test runs on QSL fixtures only: no driver, no CG and no `negotiate_*`.
QSL never waits on a downstream repository; the true end-to-end run, from the
driver through the QSpec FR-331 request to CG, lives in quire-integration.

## Test Procedure

1. Compile a QSL fixture unit whose relation binds the ConfigVersion object
   type and not its population, with one requirement item that reads only a
   ConfigVersion field and one that ranges over the population.
2. Call the layer-4 export with both items' occurrence keys.
3. Keep the bound results, as a driver forming the FR-331 request does.

## Expected Results

1. The unit compiles.
2. Two results: the field-reading item bound, with the ConfigVersion
   `ObjectBinding`; the population item refused `missing_declaration`/
   `missing-name`, naming the population and its owning `DomainPackageRef`.
3. The kept set holds exactly the field-reading item.
