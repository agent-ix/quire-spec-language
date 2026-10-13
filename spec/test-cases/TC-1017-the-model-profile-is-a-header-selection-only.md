---
id: TC-1017
title: "The model profile is selected by the header only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1017: The model profile is selected by the header only

## Description

Verify the unit-level behaviour of the gate: every refusal is reported, per unit, with one refusal per construct, from a header selection and nothing else; and the lock and `unknown_profile` behaviour. Scope: FR-361-AC-6 to FR-361-AC-12. Steps map to criteria: step 1 AC-6, step 2 AC-6, step 3 AC-7, step 4 AC-8, step 5 AC-9, step 6 AC-10, step 7 AC-11, step 8 AC-12.

## Test Procedure

1. Compile a value-profile unit holding one construct of each family of item 5, one of them inside a declaration nothing uses.
2. Compile a library that selects the model profile imported by a unit that does not, writing a `body` against the import alias; then the reverse.
3. Compile a unit with a `model` declaration, a supplied domain package and a model construct, whose only header profile is `quire.value.complete/v1`.
4. Compile a unit whose profile `v` names `quire.model.complete/v1`; read the emitted lock.
5. Compile a unit holding model constructs with the header profile set to the `ieee_profile` row's identity, then to `test:unknown-profile`.
6. Under `make conformance`, compare the DefinitionLock rows that come from QSpec's complete-model lock with the lock read from `QSPEC_DIR`.
7. Read the S2 forms of the unit of step 1.
8. Compile `body b using V on M::A::add(x) { x }` with `V` the only profile (`quire.value.complete/v1`); then with a second profile `W` naming `quire.model.complete/v1`, still `using V`.

## Expected Results

1. One `unsupported_construct` refusal per construct, in source order, each retaining the selected and required layers.
2. The unit lacking the selection gets its refusals, including for its own forms written against the import alias; the other unit does not.
3. The construct is refused; the unit never compiles.
4. The model layer resolves. `definition_selections` hold the model layer, `quire.state.graph/v1`, `quire.value.complete/v1`, their required layers, the complete-value lock's selected `package_selection` rows and the complete-model lock's `always` rows, written out as a literal in the test, and no layer that requires the model layer.
5. `unknown_profile`/`wrong-selection-role`, then `unknown_profile`/`unsupported-selection`; no S3 refusal.
6. The rows (role, authority, identity, in order) and package-selection rules are equal.
7. S2 holds a form for every construct and reports no refusal.
8. The first compile has one refusal (the family refusal); the second has FR-110's layer refusal alone.
