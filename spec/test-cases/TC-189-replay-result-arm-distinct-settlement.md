---
id: TC-189
title: "The replay result keeps the Witness arm and Input arm distinct, each with its own settlement"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: verifies
---
# TC-189: The replay result keeps the Witness arm and Input arm distinct, each with its own settlement

## Description

Verify that a `Witness`-arm agreement settles `reproduced-with-evaluated-witness`
and an `Input`-arm agreement settles `reproduced-without-witness`, that
these are distinct values, and that the `Input`-arm settlement is never
reported or convertible to a value that counts as backend evidence. A wrong
implementation this test would catch: a result type with one shared
`agreed: bool` flag instead of two distinct arm-settlement values; under
that shape, a caller checking only `agreed` cannot tell a corpus-sourced
"reproduced without a witness" result from real backend-witnessed evidence,
which is exactly the AD-016 WP9 distinction this envelope exists to
preserve. Scope: FR-072-AC-1.

## Test Procedure

1. Construct a `Witness`-arm result whose native replay agrees with the
   proved verdict.
2. Construct an `Input`-arm result (a corpus counterexample with no
   backend transcript) whose native replay agrees with the proved verdict.
3. Read each result's settlement value.
4. Attempt to construct or convert a backend-evidence verdict (the sealed
   type AD-016 names, constructible only from an agreeing `Witness`-arm
   result) from the `Input`-arm result of step 2.

## Expected Results

- The `Witness`-arm result's settlement is `reproduced-with-evaluated-witness`.
- The `Input`-arm result's settlement is `reproduced-without-witness`, a
  distinct value from step 3's `Witness`-arm settlement.
- Step 4 fails: there is no public path from an `Input`-arm result to a
  backend-evidence verdict.
