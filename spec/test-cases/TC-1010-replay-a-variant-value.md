---
id: TC-1010
title: "Replay admits and encodes a variant value and refuses each malformed form at its nested path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: verifies
---
# TC-1010: Replay admits and encodes a variant value and refuses each malformed form at its nested path

## Description

Verify the observation form of a variant value. Scope: FR-360-AC-9, FR-360-AC-10 and FR-360-AC-11.

## Test Procedure

1. Admit a snapshot holding the variant form at a field, in `present`, in a `sequence`, and as an operation parameter and result.
2. Admit each malformed form of FR-360-AC-10, including `{"absent":{}}` at a required field and the three multi-defect values.
3. Decode then encode the admitted values with object members in each order; round-trip a counterexample witness holding a `Config::Tier` parameter.
4. Run the same package under aliases `Config` and `Cfg` and compare keys and values.

## Expected Results

- Step 1 admits every value, equal to the literal of its case.
- Step 2 refuses with the stated `invalid_runtime_input` cause at the nested path, the first defect in item 9's order.
- Step 3 returns the same typed observation and digest, and the witness decodes to the same value.
- Step 4 finds equal keys and equal values.
- Assertions compare typed codes, causes, keys and paths.
