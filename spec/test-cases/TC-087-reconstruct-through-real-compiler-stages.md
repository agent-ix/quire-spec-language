---
id: TC-087
title: "Reconstruct through real compiler stages"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-087: Reconstruct through real compiler stages

## Description

Integration, priority P1. Verifies FR-020-AC-8, FR-020-AC-9. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Prepare matching byte-selected package metadata and externally selected source variants that reach actual syntax, name, native type and definedness refusals. Lower parser, linker and checker limits independently on a valid rebind request. Assert the native stage is actually invoked after earlier setup succeeds.

## Expected Results

Each failure retains its original native code/phase/source span and returns no package. A serialized checked claim cannot bypass the failing compiler stage; resource limits are incomplete rather than logical false.
