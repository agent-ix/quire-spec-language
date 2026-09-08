---
id: TC-003
title: "Selected role and source correspondence audit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Selected role and source correspondence audit. Scope: FR-012-AC-3. Type: Integration; priority P1.

## Test Procedure

Run roles over the pinned packet, then exercise mismatched region/digest/role membership in isolated fixtures.

## Expected Results

The real packet reports 17 artifacts and four regions; the changed correspondence refuses.
