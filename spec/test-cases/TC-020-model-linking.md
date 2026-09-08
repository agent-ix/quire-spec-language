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
Planned, not executed. Requires the qualified inputs and actual native APIs
specified by IT-005; a setup refusal is not an observed application outcome.

## Test Procedure

Use IT-005's qualified ConfigVersion model and its recorded public export. Parse a native import selecting the exact compiled IR byte digest, package identity and version; link the clause through the real shared adapter.

## Expected Results

The import resolves to ix://example/config/type/ConfigVersion and the parent/versionNumber references retain their stable declaration IDs and original native occurrence spans. A distinct LinkedPackage exists; no evaluation or backend qualification is inferred.
