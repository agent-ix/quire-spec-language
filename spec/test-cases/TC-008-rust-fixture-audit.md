---
id: TC-008
title: "Audit resource ceilings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Audit resource ceilings. Scope: FR-012-AC-9. Type: Property; priority P1.

## Test Procedure

Generate sizes around lowered file/aggregate/record limits and verify the default oversized-file boundary.

## Expected Results

At-ceiling valid input succeeds; beyond-ceiling work reports resource-exhausted without partial success.
