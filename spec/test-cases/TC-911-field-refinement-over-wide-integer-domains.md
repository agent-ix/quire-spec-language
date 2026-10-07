---
id: TC-911
title: "Field refinement decides wide integer domains and literals exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-911: Field refinement decides wide integer domains and literals exactly

## Description

Verify that domain-package integer bounds beyond ±2^53 arrive as decimal
strings and that the field-domain refinement obligation decides domains
beyond `i64` and literals above `i64::MAX` exactly.

Scope: FR-056-AC-16, FR-082-AC-9.

## Test Procedure

1. Admit a domain package whose scalar type `Wide` is bound to `Integer`
   with `min` the JSON integer `0` and `max` the string
   `"18446744073709551615"`; then with `max` `"9223372036854775808"`.
2. Admit it with `max` the JSON number `18446744073709551615`, then the
   string `"170141183460469231731687303715884105728"`, then the string
   `"0018"`.
3. With `f: Wide` redefined by a field of scalar type `Narrow` bound to
   `[0, 9223372036854775808]`, check the redefinition when the exposed
   operation writing `f` states the postcondition
   `self.f <= 9223372036854775808`, then `self.f <= 9223372036854775809`.

## Expected Results

1. Both admit; `Wide` resolves to `Int[0, 18446744073709551615]` and
   `Int[0, 9223372036854775808]` exactly.
2. The JSON number refuses `noncanonical_wire`/`inexact-integer` at its
   pointer; each string refuses `invalid_model_binding`/
   `malformed-declaration` at the scalar type, naming the constraint and the
   value.
3. The first admits the redefinition; the second refuses
   `unproved-refinement` with obligation `field-domain`.
