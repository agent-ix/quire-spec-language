---
id: TC-816
title: "S1 parses a case scrutinee without a top-level record and keeps protocol case apart"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-313
    type: verifies
---
# TC-816: S1 parses a case scrutinee without a top-level record and keeps protocol case apart

## Description

Scope: FR-313-AC-3.

## Test Procedure

1. Parse `case R { f: 1 } { A: 0; }` in a function body.
2. Parse `case (R { f: 1 }) { A: 0; }`.
3. Parse a unit holding an expression `case` and a protocol `choice` with
   `case` arms.

Tag each test `#[trace("TC-816", "<AC id>")]`.

## Expected Results

- Step 1: the scrutinee is `R`, the following `{` opens the arm list, and
  the parse ends in a diagnostic inside that arm list; `R { f: 1 }` is never
  the scrutinee.
- Step 2: parses, scrutinee the parenthesized record value.
- Step 3: the expression `case` and the protocol `case` produce their two
  different CST productions; the protocol parse is unchanged.

## Status

Planned.
