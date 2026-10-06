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
T15, that existing vectors are unchanged, and that a counter is a JSON
number up to 2^53-1 and a decimal string beyond it, never a refusal.

Scope: FR-092-AC-14, FR-092-AC-15.

## Test Procedure

1. In one unit under owner (`a`, `u`), check parameters typed
   `Int[0, 18446744073709551615]`, `Int[0, 9223372036854775808]`,
   `Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]`, `Int[0, 18446744073709551616]`
   and `Int[0, 9]`.
2. Re-run the existing TC-413 and TC-414 vector checks.
3. Read QSpec's `integer_encoding_vectors` from a quire-specification
   checkout at run time (as the `conformance` test reads its vectors) and
   key `recursion-size-safe-maximum` and `recursion-size-two-pow-53` through
   QSL's key function.
4. Encode preimages whose `group_reference` `ordinal` and `operation.member`
   `position` are 9007199254740991 and then 9007199254740992.

## Expected Results

1. The type nodes' preimage bytes and keys equal T13, T14, T15, T16 and T4
   exactly; every bound is a decimal string.
2. Every existing vector is unchanged.
3. The keys are `bc6fe35a2c34a7162fe0ed0a20473ffb3284bd70236f96feb75b386c55992874`
   and `89ec1ac87a8b82a05d5414f0c3b314f363e49328988ff5d8c3fc9a88014839bc`,
   the recorded ones.
4. At 9007199254740991 the canonical bytes hold the JSON number; at
   9007199254740992 they hold the string `"9007199254740992"`. No encoding
   refuses, and `NodeKeyRefusal` has no magnitude variant.
