---
id: TC-799
title: "Duplicate and conflicting abstraction bindings refuse conflicting-binding"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-799: Duplicate and conflicting abstraction bindings refuse conflicting-binding

## Description

Scope: FR-304-AC-4.

## Test Procedure

1. Check a relation with two byte-equal `ObjectBinding`s for ConfigVersion.
2. Check a relation with two `ObjectBinding`s for ConfigVersion whose
   `rust_type` differ.

## Expected Results

Each refuses `invalid_model_binding`/`conflicting-binding`, naming both
bindings and the ConfigVersion key.
