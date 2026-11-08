---
id: TC-1380
title: "The JIT publishes its coverage floor and translates every covered entry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: verifies
---
# TC-1380: The JIT publishes its coverage floor and translates every covered entry

## Description

Verify that `qsl_jit::coverage()` holds the coverage floor and that the JIT
lists no covered entry unsupported.

Scope: FR-560-AC-1 and FR-560-AC-2.

## Test Procedure

1. Read `qsl_jit::coverage()` and compare its forms, value types and
   operations with the floor: the seven `Value` forms, `Boolean`, bounded and
   unbounded `Integer`, `Rational`, `Decimal`, `Text`, records and
   collections of them, and every `Value` operator over them.
2. Prepare the JIT over each of the 10,000 units the FR-295 generator yields
   at the default seed, every function selected, and read `unsupported()`.
3. Prepare the JIT over `add`, `half` and `name` as FR-560-AC-2 writes them.

Tag the tests `#[trace("TC-1380", "<AC id>")]`.

## Expected Results

- Step 1: every floor item is listed.
- Step 2: `unsupported()` is empty for every unit.
- Step 3: none of the three is listed unsupported.
