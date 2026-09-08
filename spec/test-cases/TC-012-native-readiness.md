---
id: TC-012
title: "Located admitted native grammar"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: verifies
---

## Description

Located admitted native grammar. Type: Integration; priority P1. Traces: FR-002-AC-1, FR-002-AC-2, FR-002-AC-3, FR-002-AC-4, FR-002-AC-5, NFR-001-M-2, NFR-001-M-3, NFR-001-M-4.

## Test Procedure

Run the parent/declaration, precedence, grouping/name locus, reserved collect, token/node/depth and joined 512 KiB stack cases through the actual parser.

## Expected Results

Admitted examples parse with original spans; chained comparisons are invalid, balanced collect is unsupported, and exhausted work is incomplete. Flat 20000-operator chains complete on the selected stack.

