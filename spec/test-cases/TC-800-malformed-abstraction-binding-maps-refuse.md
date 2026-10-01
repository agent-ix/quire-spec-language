---
id: TC-800
title: "Abstraction bindings with malformed parameter or field maps refuse malformed-declaration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-800: Abstraction bindings with malformed parameter or field maps refuse malformed-declaration

## Description

Scope: FR-304-AC-5.

## Test Procedure

Check four relations, each with one defect:

1. a `FrameBinding` for `attemptUpdate` omitting a declared parameter;
2. one naming a parameter `attemptUpdate` does not declare;
3. one mapping two parameters to the same Rust parameter;
4. an `ObjectBinding` naming a field ConfigVersion does not declare.

## Expected Results

Each refuses `invalid_model_binding`/`malformed-declaration`, naming the
defective entry.
