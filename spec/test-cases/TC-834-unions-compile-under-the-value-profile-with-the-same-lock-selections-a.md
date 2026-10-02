---
id: TC-834
title: "Unions compile under the value profile with the same lock selections as records"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-324
    type: verifies
---
# TC-834: Unions compile under the value profile with the same lock selections as records

## Description

Scope: FR-324-AC-1.

## Test Procedure

1. Compile a unit whose header selects `quire.value.complete/v1` and which
   declares `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` and `area` (FR-318-AC-1), through spine `compile`.
2. Compile the same unit with `Shape` replaced by
   `record Shape { tag: Integer; }` and `area` returning `0`.

Tag each test `#[trace("TC-834", "<AC id>")]`.

## Expected Results

- Step 1: compiles.
- The two emitted locks hold equal `profile_selections` and
  `definition_selections`.

## Status

Planned.
