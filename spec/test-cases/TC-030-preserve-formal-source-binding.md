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

Construct the real IR BoundedCounter environment with self State value and count field. Hash its existing V1 canonical bytes for the native import and link a parsed current invariant. Inspect the owned source bytes/opaque labels, selected RequirementRef, type/field identities and both native and IR spans. Substitute IR CanonicalDigest and a different environment's canonical byte digest independently.

## Expected Results

The exact byte binding links with unchanged native source and correct owner-qualified loci. Both foreign digest substitutions refuse without a LinkedPackage; no semantic digest is relabeled as a raw artifact digest.
