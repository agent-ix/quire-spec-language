---
id: TC-010
title: "Owned verification language inventory"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: verifies
---

## Description

Owned verification language inventory. Scope: NFR-005-M-1. Type: Manual; priority P1.

## Test Procedure

Inspect all owned executable audit files, CI steps and documented verification commands after migration.

## Expected Results

No Python audit executable/CI invocation or embedded non-Rust verifier remains; historical records retain their provenance.
