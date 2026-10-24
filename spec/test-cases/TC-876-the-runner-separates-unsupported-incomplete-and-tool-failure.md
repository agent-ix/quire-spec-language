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
5. Use two distinct vector ids over identical successful run inputs. Give
   each exactly the budget its independently derived expected accounting
   requires. Run in both vector orders. Compare the original corpus and
   named input bytes directly before and after all runs, including the
   corpus's recorded `results` and `coverage`.
6. Run TC-875's positive compile vector with `--limit s3.nodes=0`, then
   with that setting raised to fit. Compare each actual typed result to a
   direct public compile call under limits from the same FR-255 settings
   operation.

Tag the tests `#[trace("TC-876", "FR-350-AC-n")]`.

## Expected Results

- Step 1: `unsupported` holding `unknown_required_feature`/`unknown-feature`; then `passed`; then `unsupported`.
- Step 2: `incomplete` retaining the evaluation budget kind, bound 0,
  actual denied count and the subject's setting or request field that
  raises it; then `passed`; the boundary vector is `passed`.
- Step 3: `tool-failure` holding the I/O error; the two neighbouring vectors are `passed` in both runs.
- Step 4: equal per-vector outcomes.
- Step 5: both vectors pass in both orders with independent accounting;
  all compared corpus and input bytes are unchanged.
- Step 6: `incomplete` naming `s3.nodes`, then `passed`; actual results
  equal the corresponding direct public calls.
