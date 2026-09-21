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
these are distinct values, and that the `Input`-arm and `Witness`-arm
result types are distinct with no conversion between them — the property
FR-072 actually builds, and the one CG's separately-owned sealed
backend-evidence-verdict type (AD-016, owned by the CG parity comparator,
not by this ticket) depends on. This test does not construct that CG-owned
sealed type; it verifies the QSL-side type boundary CG's type relies on. A
wrong implementation this test would catch: a result type with one shared
`agreed: bool` flag instead of two distinct arm-settlement values; under
that shape, a caller checking only `agreed` cannot tell a corpus-sourced
"reproduced without a witness" result from real backend-witnessed evidence,
which is exactly the AD-016 WP9 distinction this envelope exists to
preserve; or a result type that implements `From<InputArmResult> for
WitnessArmResult` (or the reverse), which would let CG's comparator be
fed an `Input`-arm result through the conversion even though its own
construction path is nominally restricted to the `Witness` arm. Scope:
FR-072-AC-1.

## Test Procedure

1. Construct a `Witness`-arm result whose native replay agrees with the
   proved verdict.
2. Construct an `Input`-arm result (a corpus counterexample with no
   backend transcript) whose native replay agrees with the proved verdict.
3. Read each result's settlement value.
4. Inspect the `Witness`-arm and `Input`-arm result types for any `From`,
   `TryFrom`, `Into` or other conversion between them, in either direction,
   and attempt to compile a call site that passes the `Input`-arm result
   from step 2 where the type system requires a `Witness`-arm result.

## Expected Results

- The `Witness`-arm result's settlement is `reproduced-with-evaluated-witness`.
- The `Input`-arm result's settlement is `reproduced-without-witness`, a
  distinct value from step 3's `Witness`-arm settlement.
- Step 4 finds no conversion between the two arm-result types in either
  direction, and the call site passing the `Input`-arm result where a
  `Witness`-arm result is required fails to compile.
