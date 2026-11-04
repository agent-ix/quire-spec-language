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
FR-091-AC-58 and FR-093-AC-38 to FR-093-AC-42, FR-093-AC-45 and FR-093-AC-52 to FR-093-AC-57.

## Test Procedure

1. Parse and run S2 on the bodies of FR-091-AC-57; read each form; run S2 on
   `unit cm : Length = decimal(1, 2) * m;` and on a quantity literal with a
   `decimal(1, 2)` magnitude.
2. Parse each malformed constructor of FR-091-AC-58 and the function named
   `decimal`.
3. Compile `c1` to `c6`, `n` and `add` of FR-093-AC-39 through
   `compile_admitted`; read the literal nodes, the `quire.op.decimal.negate`
   node and the `quire.op.decimal.add` node; run `c1` with no arguments and
   `add` with a parameter holding coefficient 100 and scale 2.
4. Check each ambiguous, mismatched and non-member constructor of
   FR-093-AC-40, and the constructors that are members by value although their
   written coefficient or scale lies outside the type's bounds.
5. Compile and run the divisions of FR-093-AC-41, and the refusals of
   FR-093-AC-54 and FR-093-AC-55.
6. Compile and run the conversions of FR-093-AC-42 and FR-093-AC-56, and the
   negated constructors of FR-093-AC-57.
7. Check the constructors of FR-093-AC-38, AC-45, AC-52 and AC-53: scales
   65534, 65535, 65536 and 4294967295, a literal-scale setting of 4, and the
   unit totals.

## Expected Results

1. One `DecimalValue` each, with the coefficient, scale and span as written, no
   `UnrepresentedConstruct`; the unit and the quantity literal build their own
   forms.
2. Each malformed constructor refuses as FR-091-AC-58 lists, before S2.
3. The literal nodes have the values `"1.25"`, `"0.05"`, `"10"`, `"0.00"`,
   `"-0.5"` and `"1.250"` at `Q`'s type node; the equal constructors are one
   node and the two spellings of one magnitude are two; `n` is one negate node
   over `"5"`; `add` evaluates the parameter 1.00 plus 1.25 to coefficient 225
   and scale 2; `c1` evaluates to coefficient 125 and scale 2.
4. Each ambiguous, mismatched and non-member constructor refuses as
   FR-093-AC-40 lists; each member by value checks and its literal node keeps
   the value as written.
5. `x / decimal(4, 0)` is one `quire.op.decimal.div` node and evaluates to
   coefficient 150 and scale 2; the zero constructors, the parameter divisor,
   the conditional divisor and the negated literal divisor refuse
   `undefined_expression`/`unproved-nonzero`.
6. The conversions check, lower and evaluate to the listed values; the `exact`
   conversion of 2/3 refuses `inexact_decimal`/`nonzero-discarded-digit`; the
   negated constructors check as their values.
7. Scales 65534 and 65535 of a zero constructor check and spell `0.` and that
   many zeros; 65536 and 4294967295 refuse
   `stage_limit_exceeded`/`work-budget-exceeded` naming bound 65535; at a
   setting of 4, scale 5 refuses naming bound 4; 17 literals of scale 65535
   refuse at the 17th naming bound 1048576.
