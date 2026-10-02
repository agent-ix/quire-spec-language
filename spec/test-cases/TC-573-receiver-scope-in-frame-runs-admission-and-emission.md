---
id: TC-573
title: "The receiver scope holds in the Frame run and admission, and in the emitted frame node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: verifies
---
# TC-573: The receiver scope holds in the Frame run and admission, and in the emitted frame node

## Description

Verify that FR-115's `Frame` run and FR-106 admission share the scoped decision, and that S4 emits the scope.

Scope: FR-154-AC-3 to FR-154-AC-4.

## Test Procedure

Fixtures: An invocation of `increment` on `self = c1` whose post snapshot changes `c2.value`.

1. Run the `Frame` selection and a clause run over the invocation under each frame.
2. Emit the unit under each frame and compare frame node contents.
3. Intake the unit with both `modifies self.value` and `modifies value` on `increment`.

Tag the tests `#[trace("TC-573", "FR-154-AC-n")]`.

## Expected Results

- Step 1: `violation` with the witness naming `c2.value` and a `frame_violation` admission refusal under `modifies self.value`; `success` under `modifies value`.
- Step 2: scopes `receiver` and `every-object`; different contents.
- Step 3: intake refuses `invalid_model_binding`/`conflicting-binding` naming both entries and `increment`; no frame node is emitted.
