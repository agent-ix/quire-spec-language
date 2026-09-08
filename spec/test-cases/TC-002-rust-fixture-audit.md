---
id: TC-002
title: "Selected invocation packet audit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Selected invocation packet audit. Scope: FR-012-AC-2. Type: Integration; priority P1.

## Test Procedure

Run review over the pinned standard fixture directory and independently corrupt a temporary selected input.

## Expected Results

The real packet reports 23 files and seven cases; the corrupted input refuses before success.
