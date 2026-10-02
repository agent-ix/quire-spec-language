---
id: TC-766
title: "monitor refuses inadmissible traces and agrees with replay's trace evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: verifies
---
# TC-766: monitor refuses inadmissible traces and agrees with replay's trace evaluation

## Description

Verify `monitor`'s admission refusals, its missing-fairness-premise settlement and its agreement with layer-6 replay.

Scope: FR-283-AC-3, FR-283-AC-4, FR-283-AC-6.

## Test Procedure

1. Monitor TC-765 step 1's clause over a trace whose second position's snapshot fails FR-106 admission.
2. Monitor TC-765 step 3's clause over a lasso with an empty loop.
3. For each TC-765 trace, evaluate the clause through layer-6 replay's trace evaluation and compare with `monitor`'s verdict.
4. Over TC-765 step 3's lasso, monitor `always eventually q` under `fair weak inc` together with the same clause without fairness, and map the outcome's category through FR-285.

Tag the tests `#[trace("TC-766", "<AC id>")]`.

## Expected Results

- Step 1: the request is refused with the admission cause and position index 1, and the outcome holds no clause record.
- Step 2: refused with `invalid_runtime_input`/`invalid-value`, with no clause record.
- Step 3: each pair of verdicts is equal.
- Step 4: the fairness clause settles unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`; the clause without fairness reports `tested`; exit 21.
