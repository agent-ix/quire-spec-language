---
id: TC-020
title: "Exact qualified import"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
---

## Description

Exact qualified import. Type: Integration; priority P1. Traces: FR-005-AC-1.
Executed through the public formal linker and real IR environments in tests/linking.rs.
The complete typing/projection acceptance of IT-005 remains separate.

## Test Procedure

Use IT-005's BoundedCounter formal model constructed through the public IR API.
Parse the native import selecting its exact reviewed source/import binding;
resolve the context and count field against the explicit formal environment.

## Expected Results

The import resolves to the selected owner-qualified BoundedCounter record and
its count field, retaining exact declaration and native occurrence loci. A
distinct LinkedPackage exists. No Filament reader, object-reference semantics,
evaluation or backend qualification is inferred from this generic case.
