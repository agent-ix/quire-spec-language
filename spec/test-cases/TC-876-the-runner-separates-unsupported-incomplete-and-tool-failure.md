---
id: TC-876
title: "The conformance runner reports unsupported, incomplete and tool-failure vectors on their own"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
---
# TC-876: The conformance runner reports unsupported, incomplete and tool-failure vectors on their own

## Description

Verify the `unsupported`, `incomplete` and `tool-failure` outcomes, that a failing vector leaves its siblings alone, and that two runs agree.

Scope: FR-350-AC-3, FR-350-AC-4, FR-350-AC-5.

## Test Procedure

1. Add a vector that expects a success and whose unit requires a capability id QSL does not know, so QSL refuses `unknown_required_feature`/`unknown-feature`; add the same vector expecting that pair; add it again expecting `ill_typed`/`type-mismatch`.
2. Run TC-875's positive `run` vector, which expects a success, with an evaluation budget of zero, then with the default budget. Add a boundary vector that expects the zero-budget limit refusal and run it with a budget of zero.
3. Add a vector naming a source file absent from the corpus, between two passing vectors; run; remove it and run again.
4. Run the TC-875 corpus twice with the same build and limits.

Tag the tests `#[trace("TC-876", "FR-350-AC-n")]`.

## Expected Results

- Step 1: `unsupported` holding `unknown_required_feature`/`unknown-feature`; then `passed`; then `unsupported`.
- Step 2: `incomplete` naming the evaluation budget, the value 0 and the option that raises it; then `passed`; the boundary vector is `passed`.
- Step 3: `tool-failure` holding the I/O error; the two neighbouring vectors are `passed` in both runs.
- Step 4: equal per-vector outcomes.
