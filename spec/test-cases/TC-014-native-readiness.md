---
id: TC-014
title: "Exact extracted source correspondence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: verifies
---

## Description

Exact extracted source correspondence. Type: Integration; priority P1. Traces: FR-004-AC-1, FR-004-AC-2, FR-004-AC-3, FR-004-AC-4, NFR-001-M-5.

## Test Procedure

Run all nine existing source-map tests over exact and discontiguous mappings, layout deletions, changed/foreign identities and bytes, scalar boundaries and segment budgets.

## Expected Results

Exact original regions are retained. Interior-token deletion, foreign bindings and malformed ranges refuse; segment exhaustion is incomplete and cannot yield a partial correspondence.

