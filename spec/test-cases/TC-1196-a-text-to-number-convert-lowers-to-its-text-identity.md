---
id: TC-1196
title: "A text-to-number convert lowers to its text or string identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-478
    type: references
---
# TC-1196: A text-to-number convert lowers to its text or string identity

## Description

Verify that `check` builds a `convert<T>(e)` over a `Text[..]` or `String`
operand as a `ConvertText` node for every integer, decimal, rational, IEEE
and quantity target, inside or outside an equality. Lowering writes the
`quire.op.text.to_*` or `quire.op.string.to_*` identity of the target with:

- the `text_profile` law for a text operand only;
- the `ieee_profile` law for an IEEE target;
- the `rounding` mode the target pins;
- the target as `type_argument`.

Every other target refuses.

Scope: FR-093-AC-35, FR-093-AC-47.

## Test Procedure

1. Check and lower FR-093-AC-35's six bodies over `t: Text[0, 8; nfc]`,
   then over `s: String`, and read each body's `convert` node.
2. Check and lower FR-093-AC-47's `g` and read its comparison node and the
   comparison's first argument.
3. Check FR-093-AC-47's refused converts and `convert<Text[0, 8; nfc]>(t)`.

Tag the tests `#[trace("TC-1196", "FR-093-AC-35")]` and
`#[trace("TC-1196", "FR-093-AC-47")]`.

## Expected Results

- Step 1, over `t`: one `convert` node over `[ref(t)]` per body.
  - Identities, in body order: `quire.op.text.to_integer`, `to_decimal`,
    `to_rational`, `to_float32`, `to_float64`, `to_quantity`.
  - Laws: `text_profile` with the `quire.value.text.unicode-17.0.0/v1`
    definition on each node; `ieee_profile` also on the two IEEE nodes.
  - Modes: none, `rounding` `nearest-even`, none, `rounding` `toward-zero`,
    `rounding` `exact`, `rounding` `exact`. No node has a `text_profile` mode.
  - Member `type_argument` of the target type node, and no body obligation.
- Step 1, over `s`: the `quire.op.string.*` identity of the same name per
  body, with the same members and modes. The IEEE nodes carry `ieee_profile`.
  No node carries a `text_profile` law.
- Step 2: `quire.op.integer.eq` over `[ref(convert node), ref(literal 3)]`.
  The convert node is `quire.op.text.to_integer` over `[ref(t)]`. No
  `quire.op.numeric.convert` node is built.
- Step 3: `ill_typed`/`operator-ineligible` at each refused `convert`, and no
  node. `convert<Text[0, 8; nfc]>(t)` builds no node and its parent
  references `t`.
