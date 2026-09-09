---
id: TC-032
title: "Refuse unmapped object and operation forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Refuse unmapped object and operation forms. Integration/property controls, priority P1. Traces: FR-013-AC-4.
Authored before implementation; now executed with the public link API and actual IR constructors.

## Test Procedure

Use otherwise exact formal imports and try native deref, reaches and pre/post operation clauses independently. Also link a current invariant with an unguarded optional unwrap and a non-Boolean expression whose names are resolvable.

## Expected Results

Unmapped object/operation forms return unsupported_construct at their native spans and no package. Name-resolvable optional/non-Boolean forms can link but carry no reference-evaluable/Boolean/definedness claim; their mandatory FR-006 check remains separate.
