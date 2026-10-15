---
id: TC-1028
title: "Compare native String, UUID and Timestamp with = and != in the unit language"
type: TC
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-056"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-091"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-093"
    type: verifies
---
# TC-1028: Compare native String, UUID and Timestamp with = and != in the unit language

## Description

Verify that a unit function can declare `String`, `UUID` and `Timestamp`
parameters and results, that `=` and `!=` over two operands of one of these
types check and evaluate by the native payload, that unlike operand types and
ordering refuse, and that each admitted comparison lowers to its native
equality identity. Operand values are supplied as runtime inputs or read from
fields; the unit language has no literal for these types.

## Test Procedure

1. Check `function eqs using v(a: String, b: String): Boolean pure { a = b }`,
   `function equ using v(a: UUID, b: UUID): Boolean pure { a = b }` and
   `function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }`,
   and the three `!=` counterparts. Evaluate each vector in the table.
2. Check each refusal vector in the table.
3. Lower the six admitted functions and inspect each comparison node.
4. Declare `record UUID { x: Integer; }` in a unit that also uses `UUID` as a
   parameter type.
5. With `record Event { id: UUID; moment: Timestamp; label: String; }`, check and
   evaluate the field, call-result and `let` vectors F1-F7 below.

| Vector | Function | Operands | Expected |
| --- | --- | --- | --- |
| E1 | `eqs` | `""`, `""` | `true` |
| E2 | `eqs` | `"abc"`, `"abc"` | `true` |
| E3 | `eqs` | `"é"` (U+00E9), `"é"` (`e` U+0301) | `false` (no normalization) |
| E4 | `eqs` | `"A"`, `"a"` | `false` (no case folding) |
| E5 | `eqs` | `"00112233-4455-6677-8899-aabbccddeeff"`, the same text | `true` |
| E6 | `equ` | `00112233-4455-6677-8899-aabbccddeeff`, the same | `true` |
| E7 | `equ` | `00112233-4455-6677-8899-aabbccddeeff`, `00112233-4455-6677-8899-aabbccddeefe` | `false` |
| E8 | `equ` | `00112233-4455-6677-8899-aabbccddeeff`, `00112233-4455-6677-8899-AABBCCDDEEFF` | the second operand is refused at admission as `invalid_runtime_input`/`invalid-value`; no comparison runs |
| E9 | `eqt` | `0`, `0` | `true` |
| E10 | `eqt` | `0`, `1` | `false` |
| E11 | `eqt` | `-1`, `0` | `false` |
| E12 | `eqt` | `-170141183460469231731687303715884105728`, itself | `true` |
| E13 | `eqt` | `170141183460469231731687303715884105727`, `-170141183460469231731687303715884105728` | `false` |
| E14 | each `!=` counterpart | the operands of E1 to E13 | the negation of the row's result |
| R1 | `a = b` over `a: String`, `b: UUID` | n/a | `ill_typed`/`type-mismatch`, no Boolean |
| R2 | `a = b` over `a: UUID`, `b: Timestamp` | n/a | `ill_typed`/`type-mismatch` |
| R3 | `a != b` over `a: String`, `b: Timestamp` | n/a | `ill_typed`/`type-mismatch` |
| R4 | `a = b` over `a: String`, `b: Text[0, 8; nfc]` | n/a | `ill_typed`/`type-mismatch` |
| R5 | `a = b` over `a: Timestamp`, `b: Integer` | n/a | `ill_typed`/`type-mismatch` |
| R6 | `a < b`, `a <= b`, `a > b`, `a >= b` over two `String`, two `UUID` or two `Timestamp` operands | n/a | each `ill_typed`/`operator-ineligible` |
| R7 | `a = b` over `a: UUID`, `b: Reference<T>` | n/a | `ill_typed`/`type-mismatch` |

| Vector | Function | Operands | Expected |
| --- | --- | --- | --- |
| F1 | `function ft using v(x: Event, y: Event): Boolean pure { x.moment = y.moment }` | `moment` `5` and `5` | `true`; `x.moment` has checked type `Timestamp` |
| F2 | `ft` | `moment` `5` and `6` | `false` |
| F3 | `function fu using v(x: Event, y: Event): Boolean pure { x.id != y.id }` | `id` `00112233-4455-6677-8899-aabbccddeeff` and `00112233-4455-6677-8899-aabbccddeefe` | `true` |
| F4 | `function fs using v(x: Event, y: Event): Boolean pure { x.label = y.label }` | `label` `a` and `a` | `true` |
| F5 | `function fc using v(x: Event, y: Event): Boolean pure { stamp(x) = stamp(y) }` with `function stamp using v(e: Event): Timestamp pure { e.moment }` | `moment` `5` and `5` | `true` (call results) |
| F6 | `function fl using v(x: Event, y: Event): Boolean pure { let p = x.id in let q = y.id in p = q }` | the same `id` twice | `true` (let bindings) |
| F7 | `function fm using v(x: Event, y: Event): Boolean pure { x.moment = y.id }` | n/a | `ill_typed`/`type-mismatch` |

## Expected Results

- Step 1: every function checks; each E-row gives its stated result, and E8
  produces no Boolean. The parameters and results resolve to the native
  `String`, `UUID` and `Timestamp` value types, distinct from `Text`.
- Step 2: each R-row refuses with the stated code and cause (`type-mismatch` for unlike types, `operator-ineligible` for ordering) at the comparison
  and yields no Boolean, no coercion and no node.
- Step 3: the String comparisons lower to `quire.op.string.eq` and `.ne`, the
  UUID comparisons to `quire.op.uuid.eq` and `.ne`, the Timestamp comparisons
  to `quire.op.timestamp.eq` and `.ne`, each `binary` with result `boolean`,
  arguments `[ref(a), ref(b)]`, and no law, mode, member or leaves, with no
  text-profile definition in the lock evidence.
- Step 4: the use of `UUID` refuses `ambiguous_declaration`/`ambiguous-name`,
  naming the use, the span of the record declaration and the native `UUID`
  candidate with no span.
- Step 5: each F-row gives its stated result; the field, call-result and `let`
  operands compare by the native rules, and the uppercase-UUID, Timestamp
  field mapping and F7 refusal hold as in Steps 1 and 2.
