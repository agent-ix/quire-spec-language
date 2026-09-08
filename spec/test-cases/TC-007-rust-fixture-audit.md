---
id: TC-007
title: "Fixture root containment"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Fixture root containment. Scope: FR-012-AC-7. Type: Property; priority P1.

## Test Procedure

Generate nested relative and escaping/absolute paths; include a Unix symlink escape in an isolated temporary tree.

## Expected Results

Contained paths are read; foreign targets refuse and are never used as evidence.
