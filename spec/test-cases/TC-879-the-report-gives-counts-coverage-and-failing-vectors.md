---
id: TC-879
title: "The conformance report gives outcome counts, coverage and each failing vector, in a stable order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: verifies
---
# TC-879: The conformance report gives outcome counts, coverage and each failing vector, in a stable order

## Description

Verify the report's counts, coverage, failure detail, order, determinism and version line.

Scope: FR-352-AC-1, FR-352-AC-2, FR-352-AC-3.

## Test Procedure

Build a fixture corpus with four in-scope capabilities: two whose vectors pass, one with a failing vector, one with no vector. Write the capability ids out of order in the corpus. Run twice with JSON output and once with text output.

Then add three vectors to the failing capability: one with a missing input
file (`tool-failure`), one expecting success but returning an unknown-feature
refusal (`unsupported`), and one expecting success but exhausting its
configured accounting budget (`incomplete`). Give the four non-passing
vectors ids out of order and repeat both output modes.

Finally, use the valid corpora with an empty default scope from TC-881
step 7. Run each in JSON and text modes and inspect the outputs for a
coverage report or percentage.

Tag the tests `#[trace("TC-879", "FR-352-AC-n")]`.

## Expected Results

- Counts 2, 1, 0, 0, 0, 1 for `passed`, `failed`, `tool-failure`, `unsupported`, `incomplete`, `uncovered`; coverage 50 per cent.
- The failing capability's entry holds the failing vector's id with expected and actual result; passing entries hold no vector detail.
- The two JSON reports are byte-equal, list capabilities and vectors in ascending id order and name the QSL version that ran. The text report gives the same counts and coverage.
- After adding the three vectors, capability counts and coverage stay the
  same. The failed capability keeps the original failed vector as its only
  deciding id. Both output modes retain all four non-passing vectors in id
  order, each with its own outcome and expected and actual result.
- Empty-scope runs emit the `empty-scope` refusal with no coverage report
  or percentage. In particular they report neither 0% nor 100%, and no
  division-by-zero result, NaN or infinity.
