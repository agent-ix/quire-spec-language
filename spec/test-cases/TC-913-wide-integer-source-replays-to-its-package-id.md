---
id: TC-913
title: "A wide-range source recompiles to its package_id and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: verifies
---
# TC-913: A wide-range source recompiles to its package_id and replays

## Description

Verify that a proved package whose source declares a range beyond `i64`
recompiles on replay to the same `package_id` and replays a decimal-string
argument.

Scope: FR-098-AC-8.

## Test Procedure

For each case, compile the source, build a replay request over its bytes
and `package_id` with an `Input` assignment, and call `qsl_replay::replay`:

1. `function wide using v(x: Int[0, 18446744073709551615]): Boolean pure
   { x <= 18446744073709551614 }` with `x` = `"18446744073709551615"`.
2. `x: Int[0, 9223372036854775808]` with body `x <= 9223372036854775807`
   and `x` = `"9223372036854775808"`.
3. `x: Int[-170141183460469231731687303715884105728,
   170141183460469231731687303715884105727]` with body
   `x > -170141183460469231731687303715884105728` and
   `x` = `"-170141183460469231731687303715884105728"`.

## Expected Results

Each recompiled `package_id` equals the request's, and each replay settles
`reproduced-without-witness` with the call's result `false`.
