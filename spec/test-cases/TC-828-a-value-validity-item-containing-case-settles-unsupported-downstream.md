---
id: TC-828
title: "A value-validity item containing case settles unsupported downstream"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-320
    type: verifies
---
# TC-828: A value-validity item containing case settles unsupported downstream

## Description

Scope: FR-320-AC-3.

## Test Procedure

1. Emit `area` (FR-318-AC-1)'s `value-validity` item and route it through the IR and CG
   arms.

Tag each test `#[trace("TC-828", "<AC id>")]`.

## Expected Results

- The item is emitted by QSL and settles `unsupported` with its catalog
  code at the IR or CG arm; QSL records no settlement of its own.

