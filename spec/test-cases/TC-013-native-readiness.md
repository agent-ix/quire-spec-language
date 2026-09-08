---
id: TC-013
title: "Token-preserving formatting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
---

## Description

Token-preserving formatting. Type: Integration; priority P1. Traces: FR-003-AC-1, FR-003-AC-2, FR-003-AC-3.

## Test Procedure

Format/reparse the admitted expression corpus, compare ordered token spellings and comments, and format again.

## Expected Results

Ordered spellings/comments are retained, syntax kinds agree and the second formatting pass is byte-identical.

