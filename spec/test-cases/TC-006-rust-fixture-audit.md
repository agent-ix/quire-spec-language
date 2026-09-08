---
id: TC-006
title: "Malformed JSON and field types"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: verifies
---

## Description

Malformed JSON and field types. Scope: FR-012-AC-6. Type: Property; priority P1.

## Test Procedure

Generate bounded malformed JSON and field-type mutations including escaped duplicate keys, invalid UTF-8 and trailing JSON.

## Expected Results

Each invalid input errors without panic or a success summary; valid neighboring inputs remain accepted.
