---
id: TC-1052
title: "Write String, UUID, Timestamp and Text constants as literals in the unit language"
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
# TC-1052: Write String, UUID, Timestamp and Text constants as literals in the unit language

## Description

Verify that the quoted literal, `UUID("...")`, `Timestamp(n)` and the explicit
`Text("...")` constructor build, check, lower and evaluate as the native
`String`, `UUID` and `Timestamp` values and the slot's `Text[..]` value; that
malformed literals refuse with the stated code and cause; and that a literal
never adapts to a slot of another type; and that a `Text("...")` value keeps
the quoted spelling it was written with as provenance without the spelling
reaching its equality, node identity or lowering.

Catches a quoted literal typed `Text`, a normalized or case-folded String, a
UUID literal that accepts a non-canonical spelling, a Timestamp read through
Integer coercion, a literal that reaches lowering unvalidated, a leading-zero count
accepted, and a call of `UUID` or `Timestamp` that is not resolved by name
resolution.

## Test Procedure

Every unit starts with the complete-V1 header and one profile selection `v`.

1. Run S2 on the bodies of FR-091-AC-41 and check them; read each form's value.
2. Run S1, S2 and check on the refusals of FR-091-AC-42, including
   `Timestamp(007)` and the unit that also declares `function UUID`.
3. Check the literals of FR-091-AC-43 and read each refusal.
4. Check and evaluate the FR-056-AC-40 vectors, then the FR-056-AC-41
   refusals.
5. Lower the functions of FR-093-AC-29 and inspect the nodes.
6. Replay `function s using v(x: String): Boolean pure { x = "abc" }` and its
   `!=` counterpart `ns` through the real spine (FR-098) with an `Input`
   assignment carrying the String wire object `{"kind":"string","text":...}`
   for the texts `abc`, `ABC` and the empty text.
7. Check and evaluate the `Text("...")` bodies of FR-056-AC-51 over the
   `nfc`, `unicode-scalars` and `binary-utf8` slots, with the composed and
   decomposed spellings of `\u00e9`.
8. Check the refusals of FR-056-AC-52 and FR-091-AC-69: bounds, positions that
   expect no `Text[..]`, argument forms, the ambiguous name and the lexical
   escape.
9. Lower the functions of FR-093-AC-50 and inspect the nodes.
10. Check and evaluate the FR-091-AC-70 vectors: `Text("é")` and
    `Text("\u00e9")` and the other spelling groups under a scalar and a
    normalizing profile, compare each with the runtime payload bytes `c3 a9`,
    read each value's provenance, lower the functions and read the literal
    nodes, and deduplicate the guard literals of two spellings.

## Expected Results

- Step 1: the String forms hold the stated scalar sequences, and the UUID and
  Timestamp calls resolve to their literals with the stated counts;
  `Timestamp(-0)` equals `Timestamp(0)`.
- Step 2: each row refuses with the stated code and cause at the stated
  position and yields no form (`Timestamp(007)` is
  `invalid_syntax`/`invalid-token`); a parameter named `timestamp` or `uuid`
  stays valid.
- Step 3: `MalformedUuidLiteral` and `IntegerOutsideI128` carry
  `ill_typed`/`type-mismatch`, the argument or the positive literal as written, the limit
  for a count, and the span; the in-range maximum checks.
- Step 4: every FR-056-AC-40 vector gives its stated Boolean; every
  FR-056-AC-41 vector refuses with the stated cause and yields no Boolean.
- Step 5: each literal lowers to one `value`/`literal` node of the stated
  `value_kind` and value; equal literals share a node; the comparison lowers
  to the native equality identity with the literal's node as an argument.
- Step 6: `s` evaluates `true` for `abc` and `false` for `ABC` and the empty
  text, `ns` the negation. Each `true` settles `inconclusive` with cause
  `Verdicts` and each `false` `reproduced-without-witness`, as in TC-1028
  Step 6.
- Step 7: under `nfc` both spellings are the one retained sequence U+00E9 and
  both bodies give the same Boolean; under `unicode-scalars` they differ; the
  retained length, not the written length, is checked against the bounds.
- Step 8: each row refuses with the stated code, cause and position and yields
  no form, no value and no coercion; `TextLiteralOutsideBounds` names the
  argument as written, the bounds, the profile and the retained length.
- Step 9: each `Text("...")` lowers to one `value`/`literal` node of
  `value_kind` `text` typed by the slot's `Text[..]` node; equal retained
  sequences share a node; the comparison is the text equality with the
  `text_profile` law.
- Step 10: every spelling of a text holds that text's payload bytes; the raw,
  escaped and runtime values compare equal; each literal's provenance is its
  own quoted spelling, equal for equal spellings and distinct for distinct
  ones; two spellings of one text lower to one literal node per slot type and
  are one data literal.
