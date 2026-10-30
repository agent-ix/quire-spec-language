---
id: TC-910
title: "Wide integer ranges produce structural type nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-910: Wide integer ranges produce structural type nodes

## Description

Verify FR-092-AC-14's exact wide integer bounds and QSpec FR-322
structural ids. FR-092-AC-15 is withdrawn because the preimage-counter
spelling it tested no longer exists.

## Test Procedure

1. In one package, check parameters typed `Int[0, 18446744073709551615]`,
   `Int[0, 9223372036854775808]`,
   `Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]`,
   `Int[0, 18446744073709551616]` and `Int[0, 9]`.
2. Inspect each bounded-domain node's exact `min` and `max` integer values
   and its `{package, path, ordinal}` id; compare ids componentwise and
   check ordinal bounds and uniqueness among equal package and path.
3. Repeat the same source under another stack size and compare the nodes.

## Expected Results

The bounds retain their exact values, including both signed-128 extremes.
The type nodes have the FR-092 shape, their structural ids are valid and
repeatable, and no input is refused solely for bound magnitude. No test
asserts identity-preimage bytes or counter spellings.
