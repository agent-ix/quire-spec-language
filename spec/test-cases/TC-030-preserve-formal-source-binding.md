---
id: TC-030
title: "Preserve exact source and formal artifact binding"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Preserve exact source and formal artifact binding. Integration/property controls, priority P1. Traces: FR-013-AC-1, FR-013-AC-2.
Authored before implementation; now executed with the public link API and actual IR constructors.

## Test Procedure

Construct the real IR BoundedCounter environment with self State value and count field. Link a parsed current invariant against the native import of its existing V1 environment, identified by its identity labels. Inspect the owned source bytes/opaque labels, selected RequirementRef, type/field identities and both native and IR spans. Substitute a different environment's identity and a foreign semantic identity independently.

## Expected Results

The identified environment links with unchanged native source and correct owner-qualified loci. Both foreign substitutions refuse without a LinkedPackage; no semantic identity is relabeled as a source identity.
