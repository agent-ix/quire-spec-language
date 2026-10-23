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

Scope: FR-283-AC-3, FR-283-AC-4, FR-283-AC-6, FR-283-AC-8, FR-283-AC-12, FR-283-AC-13.

## Test Procedure

1. Monitor TC-765 step 1's clause over a trace whose second position's snapshot fails FR-106 admission.
2. Monitor TC-765 step 3's clause over a lasso with an empty loop.
3. For each TC-765 trace, evaluate the clause through layer-6 replay's trace evaluation and compare with `monitor`'s verdict.
4. Over TC-765 step 3's lasso, monitor `always eventually q` under `fair weak inc` together with the same clause without fairness, and map the outcome's category through FR-285.
5. Monitor TC-765 step 1's clause over each FR-283-AC-8 trace document, its snapshots provision holding TC-765 step 1's snapshots.
6. Over TC-765 step 1's trace, monitor `always p` with accounting `work_units` 0, and a window aggregate clause whose window holds 2 values with `max_window_values` 1; over FR-327-AC-6's trace, monitor its `always[0,2] holds(...)` clause.
7. Over TC-765 step 3's lasso, monitor `eventually[0,2] holds(q)` under event-position false-extension; then, with a test trace evaluator returning `TemporalRefusal::Fault` for one of two selected clauses, monitor both.

Tag the tests `#[trace("TC-766", "<AC id>")]`.

## Expected Results

- Step 1: `StageFailure::Refused(MonitorRefusal::Admission)` with the admission failure and position index 1, and no clause record.
- Step 2: `MonitorRefusal::TraceDocument` with `invalid_runtime_input`/`invalid-value` at `/loop`, with no clause record.
- Step 3: each pair of verdicts is equal.
- Step 4: the fairness clause settles unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`; the clause without fairness reports `tested`; exit 21.
- Step 5: each document refuses with `MonitorRefusal::TraceDocument` and the code and pointer FR-283-AC-8 gives, no position is admitted, and no record is returned.
- Step 6: `Incomplete` naming `work_units`, exit 22; `WindowLimitReached` with `max_window_values`, value 1 and setting `monitor.max_window_values`, exit 22; `ConditionRefused` at position 1 with `integer_out_of_domain`/`outside-domain` and its locus, category refusal, exit 20.
- Step 7: `TraceRefused` with `LassoUnderBoundedProfile` naming the profile; then `StageFailure::Fault`, no record for either clause, exit 30.
