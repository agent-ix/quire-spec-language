---
id: TC-765
title: "monitor reports violations, pending and tested verdicts over finite traces and lassos"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: verifies
---
# TC-765: monitor reports violations, pending and tested verdicts over finite traces and lassos

## Description

Verify offline `monitor` verdicts under ADR-014 A-4.

Scope: FR-283-AC-1, FR-283-AC-2, FR-283-AC-5.

## Test Procedure

1. Over a three-position finite trace whose `p` is false at position 2, monitor `always p`.
2. Monitor `always p` over a three-position trace whose `p` is true at every position.
3. Over a lasso of a two-position prefix and a two-position loop whose `q` holds at loop position 3, monitor `always eventually q`.
4. Repeat step 3 with `q` false at every loop position.
5. Over a three-position finite trace whose `x` is 0 at position 1, monitor `always (10 / x > 0)` and map the outcome's category through FR-285.

Tag the tests `#[trace("TC-765", "<AC id>")]`.

## Expected Results

- Step 1: a violation at position 2 with its separating witness, category violation.
- Step 2: pending, category inconclusive, exit 0.
- Step 3: `tested`, category success.
- Step 4: a violation, category violation.
- Step 5: a violation at position 1, category violation, cause `UndefinedEvaluation` with `division-by-zero`; no `undefined` label; exit 10.
