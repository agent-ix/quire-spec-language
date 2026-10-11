---
id: TC-970
title: "Capability admits exactly the twelve FR-290 labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-970: Capability admits exactly the twelve FR-290 labels

## Description

Verify the canonical `Capability` type admits each value in FR-057's table,
including `probabilistic-satisfaction` and `schedulability`, with no additional
label. Scope: FR-057-AC-1.

## Test Procedure

1. Use the twelve labels in FR-057's admitted-vocabulary table as the fixture.
2. Construct a `Capability` from each label and serialize it as a JSON string.
3. Compare the constructed set with the fixture and try the Rust variant names
   and display forms as additional inputs.

## Expected Results

- Every listed label is admitted and serializes to the same UTF-8 bytes.
- The admitted set equals the twelve table values, with no missing or extra
  label.
- Variant names and display forms are refused as `unknown-kind`; no alias is
  added for either new label.
