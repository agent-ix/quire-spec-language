---
id: TC-1027
title: "No label-only match and no composed parser remain"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: verifies
---
# TC-1027: No label-only match and no composed parser remain

## Description

Verify FR-362-AC-5 and FR-362-AC-6 by absence, so that a second route cannot return unnoticed.

## Test Procedure

1. Search the workspace for the label-only unit types and the constructor that compared only authority and identity.
2. Search the root crate for the composed parser entry points, the composed syntax tree and any conversion from `ParsedUnit` into a composed tree.
3. Read the producer and the checked-unit constructor and list every comparison they make on a source reference.

## Expected Results

- Step 1 finds none of them.
- Step 2 finds none of them, and the binding and type phase name `qsl_forms` types as their input.
- Step 3 finds only whole-reference comparisons (authority, identity and digest).
