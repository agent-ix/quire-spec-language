---
id: TC-1196
title: "A text-to-number convert lowers to its quire.op.text identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-478
    type: references
---
# TC-1196: A text-to-number convert lowers to its quire.op.text identity

## Description

Verify that `check` builds a `convert<T>(e)` over a `Text[..]` operand as a
`ConvertText` node only for an integer or decimal target, and that lowering
writes `quire.op.text.to_integer` or `quire.op.text.to_decimal` with the
operand's `text_profile` law and mode, the target as `type_argument`, and no
`rounding` mode.

Scope: FR-093-AC-35.

## Test Procedure

1. Check and lower FR-093-AC-35's functions `n` and `d`, and read each
   body's `convert` node.
2. Check `convert<Rational>(t)`, `convert<Text[0, 8; nfc]>(t)` and, for a
   `String` parameter `s`, `convert<Integer>(s)`.

Tag the tests `#[trace("TC-1196", "FR-093-AC-35")]`.

## Expected Results

- Step 1: one `convert` node over `[ref(t)]` each, identity
  `quire.op.text.to_integer` for `n` and `quire.op.text.to_decimal` for
  `d`, law `text_profile` with the `quire.value.text.unicode-17.0.0/v1`
  definition, mode `text_profile` `nfc`, member `type_argument` of the
  target type node, no `rounding` mode, and no body obligation.
- Step 2: `ill_typed`/`operator-ineligible` at each `convert`, and no node.
