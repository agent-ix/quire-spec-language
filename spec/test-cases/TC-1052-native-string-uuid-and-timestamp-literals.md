---
id: TC-1052
title: "Write String, UUID and Timestamp constants as literals in the unit language"
type: TC
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-056"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-091"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-093"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-098"
    type: verifies
---
# TC-1052: Write String, UUID and Timestamp constants as literals in the unit language

## Description

Verify that the quoted literal, `uuid("...")` and `timestamp(n)` build, check,
lower and evaluate as the native `String`, `UUID` and `Timestamp` values; that
malformed literals refuse with the stated code and cause; and that a literal
never adapts to a slot of another type.

Catches a quoted literal typed `Text`, a normalized or case-folded String, a
UUID literal that accepts a non-canonical spelling, a Timestamp read through
Integer coercion, a literal that reaches lowering unvalidated, and a reserved
word that breaks a member named `timestamp`.

## Test Procedure

Every unit starts with the complete-V1 header and one profile selection `v`.

1. Run S2 on the bodies of FR-091-AC-41 and check them; read each form's value.
2. Run S1/S2 on the syntax refusals of FR-091-AC-42.
3. Check the literals of FR-091-AC-43 and read each refusal.
4. Check and evaluate the FR-056-AC-40 vectors, then the FR-056-AC-41
   refusals.
5. Lower the functions of FR-093-AC-29 and inspect the nodes.
6. Replay `function s using v(x: String): Boolean pure { x = "abc" }` and its
   `!=` counterpart `ns` through the real spine (FR-098) with an `Input`
   assignment carrying the String wire object `{"kind":"string","text":...}`
   for the texts `abc`, `ABC` and the empty text.

## Expected Results

- Step 1: the forms hold the stated scalar sequences, UUID text and counts;
  `timestamp(-0)` equals `timestamp(0)`.
- Step 2: each row refuses with the stated code and cause at the stated
  position and yields no form; `x.timestamp`, `x.uuid` and `M::timestamp`
  parse.
- Step 3: `MalformedUuidLiteral` and `TimestampOutsideI128` carry
  `ill_typed`/`type-mismatch`, the argument or integer as written, the limit
  for a count, and the literal's span; the in-range maximum checks.
- Step 4: every FR-056-AC-40 vector gives its stated Boolean; every
  FR-056-AC-41 vector refuses with the stated cause and yields no Boolean.
- Step 5: each literal lowers to one `value`/`literal` node of the stated
  `value_kind` and value; equal literals share a node; the comparison lowers
  to the native equality identity with the literal's node as an argument.
- Step 6: `s` evaluates `true` for `abc` and `false` for `ABC` and the empty
  text, `ns` the negation. Each `true` settles `inconclusive` with cause
  `Verdicts` and each `false` `reproduced-without-witness`, as in TC-1028
  Step 6.
