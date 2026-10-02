---
id: TC-805
title: "The abstraction relation changes no requirement record, route result or clause run"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: verifies
---
# TC-805: The abstraction relation changes no requirement record, route result or clause run

## Description

Scope: FR-306-AC-3, FR-306-AC-4.

## Test Procedure

Compile the ConfigVersion unit with and without its relation declaration.
Compare the requirement records, the `route` results over the same backend
descriptors, and the dispositions of FR-108's ConfigVersion clause runs.

## Expected Results

Requirement records, `route` results and clause dispositions are equal
between the two compiles.
