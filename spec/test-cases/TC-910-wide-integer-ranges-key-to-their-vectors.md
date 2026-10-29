---
id: TC-910
title: "Wide integer ranges key to their vectors, and counters keep one exact spelling"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-910: Wide integer ranges key to their vectors, and counters keep one exact spelling

## Description

Verify that `Int` ranges up to the i128 extremes key to FR-092's T13 to
T16, existing vectors are unchanged, and QSL's counter encodings conform to
QSpec FR-322 without refusing a counter for its magnitude.

Scope: FR-092-AC-14, FR-092-AC-15.

## Test Procedure

1. In one unit under owner (`a`, `u`), check parameters typed
   `Int[0, 18446744073709551615]`, `Int[0, 9223372036854775808]`,
   `Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]`, `Int[0, 18446744073709551616]`
   and `Int[0, 9]`.
2. Re-run the existing TC-413 and TC-414 vector checks.
3. Run every QSpec FR-322 `integer_encoding_vectors` preimage through QSL's
   key function.
4. Encode `group_reference` ordinals and `operation.member` positions at the
   safe-integer boundary and the first value beyond it.

## Expected Results

1. The type nodes' preimage bytes and keys equal T13, T14, T15, T16 and T4
   exactly; every bound is a decimal string.
2. Every existing vector is unchanged.
3. Each key equals its QSpec FR-322 published expected key.
4. Each encoded counter follows QSpec FR-322's representation; no encoding
   refuses, and `NodeKeyRefusal` has no magnitude variant.
