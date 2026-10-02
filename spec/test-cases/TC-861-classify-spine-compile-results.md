---
id: TC-861
title: "Spine compile results classify into exactly one refinement class, reading every cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: verifies
---
# TC-861: Spine compile results classify into exactly one refinement class, reading every cause

## Description

Verify FR-341's compile classification over real spine `compile` results,
and over `CompileRefusal` values built in the test for the variants a
fixture cannot reach, including multi-cause vectors whose first cause is
not the deciding one.

Scope: FR-341-AC-1 to FR-341-AC-6.

## Test Procedure

1. Compile the ConfigVersion unit.
2. Compile the unit with the header profile identity
   `test:unknown-profile`; without its
   `model` package; with a dependency input that refuses; with an `import`
   of a library the input does not hold. Build a `Check` refusal holding an
   `ill_typed` cause and a `runtime_invariant` cause, in both orders.
3. Compile a unit with an ill-typed function body. Build a `Check` refusal
   whose causes are `unsupported_construct`/`not-yet-implemented` then
   `ill_typed`.
4. Compile the unit with a forms-stage limit of one; with a type limit of
   one.
5. Compile a unit with a protocol `compensate` construct (refused
   `unsupported_construct`/`not-yet-implemented`). Build an `Omitted`
   refusal and a refusal holding `unsupported_projection`.
6. Build a refusal whose causes are `unsupported_construct`/
   `declaration-form` and `unsupported_construct`/`expression-form`.

Every expected class below is a literal in the test. Tag the tests
`#[trace("TC-861", "FR-341-AC-n")]`.

## Expected Results

- Step 1: `admitted`.
- Step 2: `tool failure`, six times.
- Step 3: `refused` carrying `ill_typed`; `refused` carrying both causes in
  order.
- Step 4: `incomplete`, naming the forms limit and value one; `incomplete`,
  naming the type limit and value one.
- Step 5: `unsupported`, three times.
- Step 6: `prohibited`.
