---
id: TC-005
title: "Real native rule syntax outcomes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Real native rule syntax outcomes. Scope: FR-012-AC-5, FR-012-AC-8. Type: Integration; priority P1.

## Test Procedure

Use rule-syntax on the selected standard packet and exercise malformed case metadata.

## Expected Results

50 cases parse and one is unsupported; metadata changes refuse; no evaluator result is claimed.
