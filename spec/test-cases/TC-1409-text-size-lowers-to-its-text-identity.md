---
id: TC-1409
title: "A Text size lowers to quire.op.text.size with text_bounds-derived bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-141
    type: references
---
# TC-1409: A Text size lowers to quire.op.text.size with text_bounds-derived bounds

## Description

Verify that `check` accepts the `Size` unary over a `Text[..]` operand, types
it `Int[min, max]` from the operand's `text_bounds`, and lowers it to a `call`
node `quire.op.text.size` with the `text_profile` law and mode. A non-collection,
non-Text operand refuses.

Scope: FR-093-AC-63.

## Test Procedure

1. Check and lower FR-093-AC-63's `n` over `t: Text[0, 8; nfc]` and read the
   body's node and its `result_type`.
2. Evaluate `n` for `""`, `"abc"` and the input U+0065 U+0301.
3. Check `size` over a `String` operand and over an `Integer` operand.

Tag the tests `#[trace("TC-1409", "FR-093-AC-63")]`.

## Expected Results

- Step 1: the body checks as `Int[0, 8]` with no coercion node. One `call`
  node `quire.op.text.size` over `[ref(t)]`, `result_type` the `Int[0, 8]`
  type node, law `text_profile` with the
  `quire.value.text.unicode-17.0.0/v1` definition, mode `text_profile` `nfc`,
  and no member or leaves.
- Step 2: `0`, `3` and `1`. The last input is two raw scalars that `nfc` retains as the one scalar U+00E9 (two UTF-8 bytes), so a byte count or a pre-normalization scalar count returns `2` and fails.
- Step 3: `ill_typed`/`type-mismatch` at the operand in each case, and no node.
