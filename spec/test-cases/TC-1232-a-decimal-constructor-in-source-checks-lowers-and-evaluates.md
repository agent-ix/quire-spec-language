---
id: TC-1232
title: "A decimal constructor in source checks, lowers and evaluates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-1232: A decimal constructor in source checks, lowers and evaluates

## Description

Verify that `decimal(c, s)` is a source form: S1 parses it and refuses the
malformed spellings, S2 builds one `DecimalValue` per constructor, check types
it at its expected `Decimal[..]` type, lowers it to a `decimal` literal node,
discharges the nonzero obligation of a divisor for a nonzero coefficient, and
the evaluator reads the coefficient and scale as written. Scope: FR-091-AC-57,
FR-091-AC-58 and FR-093-AC-39 to FR-093-AC-42.

## Test Procedure

1. Parse and run S2 on the bodies of FR-091-AC-57; read each form; run S2 on
   `unit cm : Length = decimal(1, 2) * m;` and on a quantity literal with a
   `decimal(1, 2)` magnitude.
2. Parse each malformed constructor of FR-091-AC-58 and the function named
   `decimal`.
3. Compile `add`, `k`, `z` and the constructors of FR-093-AC-39 through
   `compile_admitted`; read the literal nodes and the `quire.op.decimal.add`
   node; run `k` with no arguments and `add` with a parameter holding
   coefficient 100 and scale 2.
4. Check each ambiguous, mismatched and out-of-range constructor of
   FR-093-AC-40, and the oversize coefficient.
5. Compile and run the divisions of FR-093-AC-41.
6. Compile and run the conversions of FR-093-AC-42.

## Expected Results

1. One `DecimalValue` each, with the coefficient, scale and span as written, no
   `UnrepresentedConstruct`; the unit and the quantity literal build their own
   forms.
2. Each malformed constructor refuses as FR-091-AC-58 lists, before S2.
3. The literal nodes have the values `"1.25"`, `"0.05"`, `"10"`, `"0.00"` and
   `"-0.5"` at the parameter's type node; the equal constructors are one node
   and the two spellings of one magnitude are two; `add` evaluates the
   parameter 1.00 plus 1.25 to coefficient 225 and scale 2; `k` evaluates to
   coefficient 125 and scale 2; the `convert` of a constructor builds no node.
4. Each constructor refuses as FR-093-AC-40 lists.
5. `x / decimal(4, 0)` is one `quire.op.decimal.div` node and evaluates to
   coefficient 150 and scale 2; the zero constructors and the parameter
   divisor refuse `undefined_expression`/`unproved-nonzero`.
6. The conversions lower to the listed operations and evaluate to the listed
   values; the float-to-decimal conversion refuses `ill_typed`/`type-mismatch`.
