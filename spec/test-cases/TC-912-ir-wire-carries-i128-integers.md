---
id: TC-912
title: "The IR wire round-trips integer bounds and literals up to i128"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: verifies
---
# TC-912: The IR wire round-trips integer bounds and literals up to i128

## Description

Verify that the integer IR target writes integer bounds and literals
beyond `i64` as decimal strings that the strict IR reader reconstructs
exactly.

Scope: FR-033-AC-6.

Planned: this test lands when Linear IR-662 (i128 `IntegerType` and
`IntegerLiteral` in the IR) merges.

## Test Procedure

1. Lower a declaration bounded by 0 and 9223372036854775808 and the
   literal 9223372036854775808 through `integer-ir/v1`, and read the bytes
   back through the strict IR reader.
2. Repeat with bounds `i128::MIN` and `i128::MAX` and literals at both
   extremes.

## Expected Results

1. `value_type.maximum` and `value` are both `"9223372036854775808"`, and
   the reader's declaration bounds and literal value equal the originals.
2. The extremes are spelled `"-170141183460469231731687303715884105728"` and
   `"170141183460469231731687303715884105727"` and reconstruct exactly.
