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

Verify that every refusal is reported, per unit, that the selection is a header selection and nothing else, that the model layer resolves with its lock rows, and that `unknown_profile` still refuses other identities. Scope: FR-361-AC-6 to FR-361-AC-10.

## Test Procedure

1. Compile a value-profile unit holding one construct of each family, one inside an unused declaration.
2. Compile a library that selects the model profile imported by a unit that does not, then the reverse.
3. Compile a unit with a `model` declaration, a supplied domain package and a model construct, whose only header profile is `quire.value.complete/v1`.
4. Compile a unit whose profile `v` names `quire.model.complete/v1`; read the emitted lock.
5. Compile a unit holding model constructs with the header profile set to the `ieee_profile` row's identity, then to `test:unknown-profile`.
6. Under `make conformance`, compare the DefinitionLock rows that come from QSpec's complete-model lock with the lock read from `QSPEC_DIR`.

## Expected Results

1. One `unsupported_construct` refusal per construct, in source order, each retaining the selected and required layers.
2. The unit lacking the selection gets its refusals; the other unit does not.
3. Refused as TC-1012 to TC-1016 state; never compiles.
4. The model layer resolves. `definition_selections` hold the model layer, `quire.state.graph/v1`, `quire.value.complete/v1`, their required layers and the complete-model lock's `always` rows, written out as a literal in the test, and no layer that requires the model layer.
5. `unknown_profile`/`wrong-selection-role` then `unknown_profile`/`unsupported-selection`; no S3 refusal.
6. The rows (role, authority, identity, in order) and package-selection rules are equal.
