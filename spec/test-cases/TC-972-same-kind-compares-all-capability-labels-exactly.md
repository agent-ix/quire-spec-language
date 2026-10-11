---
id: TC-972
title: "same_kind compares all capability labels exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-972: same_kind compares all capability labels exactly

## Description

Verify `same_kind` is total over the twelve canonical kinds and determines
equality by exact label identity. Scope: FR-057-AC-4.

## Test Procedure

1. Compare each `Capability` value with itself.
2. Compare every distinct pair of the twelve values, including each new value
   against every existing value.
3. Repeat set comparisons after permuting the order of the same values.

## Expected Results

- Each value compares equal to itself, and every distinct pair compares
  unequal.
- The result does not depend on enum declaration order or display text.
- Permuting values leaves set equality unchanged.
