---
id: TC-1231
title: "Bytes, JsonObject and Duration natives: payloads, literals, equality, length and concat"
type: TC
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-056"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-091"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-093"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-106"
    type: verifies
---
# TC-1231: Bytes, JsonObject and Duration natives: payloads, literals, equality, length and concat

## Description

Verify the K2 canonical payloads of native `Bytes` and `JsonObject`, the
checked types of `Bytes`, `JsonObject` and `Duration`, the literal forms
`Bytes("...")` and `JsonObject("...")`, `=` and `!=` over each kind, the
`length` and `concat` operations over `Bytes`, the refusals of unlike types,
ordering and arithmetic, and the lowering of each admitted form.

## Test Procedure

1. Admit each payload vector P1 to P24 through the kernel constructor and
   again as a runtime-input wire object.
2. Check and evaluate the functions `eqb using v(a: Bytes, b: Bytes): Boolean
   pure { a = b }`, its `!=` counterpart `neb`, `lenb using v(a: Bytes):
   Integer pure { length(a) }`, `catb using v(a: Bytes, b: Bytes): Bytes pure
   { concat(a, b) }`, `eqj using v(x: JsonObject, y: JsonObject): Boolean pure
   { x = y }` and its `!=` counterpart `nej`, over the vectors E1 to E22.
3. Check each refusal vector R1 to R14.
4. Check the literal vectors L1 to L12.
5. With the unit declaring `dimension Time; unit s : Time = rational(1, 1);
   unit ms : Time = rational(1, 1000) * s;` check the Duration vectors D1 to
   D10.
6. Lower the admitted functions and the literal bodies and inspect the nodes.
7. Admit a snapshot whose object type has `blob: Bytes`, `doc: JsonObject` and
   `span: Duration` with the values of the S vectors.

| Vector | Payload | Expected |
| --- | --- | --- |
| P1 | Bytes `""` | admitted, the empty sequence |
| P2 | Bytes `00ff` and `deadbeef` | admitted |
| P3 | Bytes `00FF` | noncanonical Bytes payload |
| P4 | Bytes `0`, `0x00`, `00 ff`, ` 00`, `gg` | noncanonical Bytes payload |
| P5 | Bytes given as the JSON number `0` | `wrong-value-kind` at the wire, no kernel call |
| P6 | JsonObject `{}`, `[]`, `3`, `"x"`, `null`, `true` | admitted |
| P7 | JsonObject `{"a":1,"b":[true,null]}` | admitted |
| P8 | JsonObject `{"b":1,"a":1}` | noncanonical JsonObject payload |
| P9 | JsonObject `{"a": 1}` | noncanonical JsonObject payload |
| P10 | JsonObject `{"a":1.0}`, `{"a":1e2}`, `{"a":-0}` | noncanonical JsonObject payload |
| P11 | JsonObject `{"a":9007199254740993}` | noncanonical JsonObject payload (the shortest spelling of its binary64 is `9007199254740992`) |
| P12 | JsonObject `{"a":9007199254740992}` | admitted |
| P13 | JsonObject `{"a":1e999}` | noncanonical JsonObject payload |
| P14 | JsonObject `{"a":1,"a":2}` | noncanonical JsonObject payload |
| P15 | JsonObject `""` and `{` | noncanonical JsonObject payload |
| P16 | JsonObject `{"é":1,"z":2}` spelled as the JCS text with the raw character | admitted; members in UTF-16 code unit order |
| P17 | JsonObject 100000 nested arrays within the input limit | admitted |
| P18 | the same one byte past the input limit | the limit's outcome (FR-255), no overflow |
| P19 | wire `{"kind":"bytes","text":"00ff"}` for a `Bytes` input | admitted |
| P20 | wire `{"kind":"json_object","text":"{\"a\":1}"}` for a `JsonObject` input | admitted |
| P21 | wire Bytes object into a `JsonObject` input, a String object into a `Bytes` input, a Bytes object into a `String` input | `wrong-value-kind` |
| P22 | wire Bytes object with an extra member, a missing `text`, a non-string `text`, an unknown `kind` | the String object's envelope causes |
| P23 | the type-owned key of Bytes `00`, `00ff` and `0100` | ordered `00`, `00ff`, `0100` (octets bytewise, proper prefix first) |
| P24 | the type-owned key of JsonObject `{"a":1}` and `{"é":1}` | `{"a":1}` first, by the UTF-8 bytes of the JCS text |

| Vector | Function | Operands | Expected |
| --- | --- | --- | --- |
| E1 | `eqb` | `""`, `""` | `true` |
| E2 | `eqb` | `00ff`, `00ff` | `true` |
| E3 | `eqb` | `00ff`, `00fe` | `false` |
| E4 | `eqb` | `00ff`, `00ff00` | `false` |
| E5 | `eqb` | `""`, `00` | `false` |
| E6 | `neb` | the operands of E1 to E5 | the negation of each |
| E7 | `lenb` | `""` | `0` |
| E8 | `lenb` | `deadbeef` | `4` |
| E9 | `catb` | `00ff`, `10` | `00ff10` |
| E10 | `catb` | `""`, `00ff` | `00ff` |
| E11 | `catb` | `00ff`, `""` | `00ff` |
| E12 | `lenb(catb(a, b))` against `lenb(a) + lenb(b)` | `00`, `ffee` | both `3` |
| E13 | `catb(catb(a, b), c)` against `catb(a, catb(b, c))` | `01`, `02`, `03` | both `010203` |
| E14 | `eqj` | `{"a":1,"b":[true,null]}`, the same | `true` |
| E15 | `eqj` | `{"a":1,"b":[true,null]}`, `{"a":1,"b":[null,true]}` | `false` |
| E16 | `eqj` | `{"a":1,"b":[true,null]}`, `{"a":2,"b":[true,null]}` | `false` |
| E17 | `eqj` | `{"a":1,"b":[true,null]}`, `{"a":1}` | `false` |
| E18 | `eqj` | `{}`, `{}` | `true` |
| E19 | `eqj` | `[]`, `{}` | `false` (arrays and objects differ in kind) |
| E20 | `eqj` | `null`, `0` | `false` |
| E21 | `nej` | the operands of E14 to E20 | the negation of each |
| E22 | `eqb`, `eqj` over record fields, call results and `let` bindings | the operands of E2 and E14 | `true` |

| Vector | Expression | Expected |
| --- | --- | --- |
| R1 | `a = x`, `a: Bytes`, `x: JsonObject` | `ill_typed`/`type-mismatch` |
| R2 | `a = s`, `s: String` | `ill_typed`/`type-mismatch` |
| R3 | `x = t`, `t: Text[0, 8; nfc]` | `ill_typed`/`type-mismatch` |
| R4 | `a = n`, `n: Integer`; `a = u`, `u: UUID` | `ill_typed`/`type-mismatch` |
| R5 | `a < b`, `a >= b` over two `Bytes` | `ill_typed`/`operator-ineligible` |
| R6 | `x <= y`, `x > y` over two `JsonObject` | `ill_typed`/`operator-ineligible` |
| R7 | `a + b`, `a - b` over two `Bytes` | `ill_typed`/`operator-ineligible` |
| R8 | `length(x)`, `concat(x, y)` over `JsonObject` | `ill_typed`/`operator-ineligible` |
| R9 | `length(s)` over `String`; `concat(a, s)` | `ill_typed`/`type-mismatch` |
| R10 | `length(a, a)`; `concat(a)` | `ill_typed`/`type-mismatch` at the call |
| R11 | a member access `x.a` over `x: JsonObject` | `ill_typed`/`operator-ineligible` |
| R12 | a unit that also declares `function length using v(b: Bytes): Integer pure { 0 }`, then `length(b)` | `ambiguous_declaration`/`ambiguous-name` with the native operation as a candidate without a span |
| R13 | `a = d`, `a: Bytes`, `d: Duration` | `ill_typed`/`type-mismatch` |
| R14 | `a = b` over `Bytes` and `Reference<T>` | `ill_typed`/`type-mismatch` |

| Vector | Literal | Expected |
| --- | --- | --- |
| L1 | `Bytes("")`, `Bytes("00ff")` | admitted, type `Bytes` |
| L2 | `Bytes("00FF")`, `Bytes("0")`, `Bytes("0x00")`, `Bytes("00 ff")` | `ill_typed`/`type-mismatch`, cause `MalformedBytesLiteral` |
| L3 | `JsonObject("{}")`, `JsonObject("null")`, `JsonObject("{\"a\":1}")` | admitted, type `JsonObject` |
| L4 | `JsonObject("{\"b\":1,\"a\":1}")`, `JsonObject("{\"a\": 1}")`, `JsonObject("1.0")`, `JsonObject("")` | `ill_typed`/`type-mismatch`, cause `MalformedJsonObjectLiteral` |
| L5 | `Bytes(x)` with `x: String`; `Bytes(5)`; `JsonObject(x)` | `ill_typed`/`type-mismatch` |
| L6 | `Bytes("00", "11")`; `Bytes()` | `ill_typed`/`type-mismatch` at the call |
| L7 | `"00ff"` at a `Bytes` slot | `ill_typed`/`type-mismatch` |
| L8 | a unit declaring `function Bytes using v(x: Integer): Integer pure { x }`, then `Bytes("00")` | `ambiguous_declaration`/`ambiguous-name` |
| L9 | `M::Bytes("00")` naming nothing | `missing_declaration`/`missing-name` |
| L10 | `eqb(Bytes("00ff"), Bytes("00ff"))` | `true` |
| L11 | `length(Bytes("deadbeef"))` | `4` |
| L12 | `JsonObject("{\"a\":1}") = JsonObject("{\"a\":1}")` | `true` |

| Vector | Expression | Expected |
| --- | --- | --- |
| D1 | a parameter `d: Duration` | checked type `Quantity<s>`, the type of a `Quantity<s>` parameter |
| D2 | `d = rational(3, 2) s` | `true` for `3/2` seconds, `false` for `2` |
| D3 | `d = convert<Quantity<s>>(m)` with `m: Quantity<ms>` | `true` for `d` of `3/2` and `m` of `1500` |
| D4 | `d = m` with `m: Quantity<ms>` | `ill_typed`/`type-mismatch` |
| D5 | the unit declares no `s` | `missing_declaration`/`missing-name` at the member |
| D6 | a selected package and the unit both declare `s` | `ambiguous_declaration`/`ambiguous-name` |
| D7 | `unit s : Length = rational(1, 1)` with no dimension `Time` | `ill_typed`/`operator-ineligible` |
| D8 | `d < e`, `d + e` over two Durations | admitted as the quantity ordering and addition of FR-094 |
| D9 | `unit s : Time = rational(2, 1)` (scale other than 1) | `ill_typed`/`operator-ineligible` |
| D10 | a `Duration` field of an admitted package declaring its own base dimension `Time` and root unit `s` of scale 1, with the unit of compilation declaring no `s` | the type `Quantity<s>` over that package's `s` |

| Vector | Snapshot field | Expected |
| --- | --- | --- |
| S1 | `blob` `{"kind":"bytes","text":"00ff"}`, `doc` `{"kind":"json_object","text":"{\"a\":1}"}`, `span` the quantity form of `3/2` over `s` | admitted |
| S2 | `blob` `00FF`; `doc` `{"a": 1}` | `invalid_runtime_input`/`invalid-value` |
| S3 | the Bytes object in `doc`; the JsonObject object in `blob`; a String object in either | `wrong-value-kind` |
| S4 | the same snapshot with its document members in another JSON order | one digest |

## Expected Results

- Step 1: each vector gives its stated outcome; a refusal produces no value and
  no `CheckedInvariant` fault, P5 and P21 refuse before the kernel, P17 and P18
  run without a stack overflow.
- Step 2: every function checks, with result types `Boolean`, `Boolean`,
  `Integer`, `Bytes`, `Boolean` and `Boolean`, and each E-row gives its stated
  result.
- Step 3: each refusal carries the stated code and cause at the stated locus,
  yields no Boolean and no coercion, and builds no node.
- Step 4: each literal gives its stated type or refusal; two equal literals in
  one unit are one value.
- Step 5: each D-row gives its stated result; a Duration is the type
  `Quantity<s>` and has no node of its own.
- Step 6: the comparisons lower to `quire.op.bytes.eq`/`.ne` and
  `quire.op.json_object.eq`/`.ne` with result `boolean`, arguments
  `[ref(a), ref(b)]` and no law, mode, member or leaves; `length` lowers to
  `quire.op.bytes.length` with result `Integer`, `concat` to
  `quire.op.bytes.concat` with result `bytes`; the literals lower to
  `value`/`literal` nodes of `value_kind` `bytes` and `json_object` with the
  canonical text as `value`; no text-profile definition is in the lock
  evidence.
- Step 7: each S-row gives its stated outcome.
