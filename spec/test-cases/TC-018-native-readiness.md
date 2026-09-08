---
id: TC-018
title: "Native diagnostic error interoperability"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---

## Description

Native diagnostic error interoperability. Type: Integration; priority P1. Traces: FR-010-AC-8.

## Test Procedure

Cause a real parser diagnostic in a caller returning a boxed standard Error and propagate it with ?. Inspect Display and structured code; enumerate the catalog and reject unknown spellings.

## Expected Results

Standard error propagation compiles and returns the original Diagnostic; display identifies its stable code/message. All code spellings are unique and round-trip; unknown spellings return None.

