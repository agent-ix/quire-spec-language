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
6. Over TC-765 step 1's trace, monitor `always p` with accounting `work_units` 0. Over TC-765 step 9's samples, monitor `always[1,1] (count over past[0, 9] min_count 1 >= 1)` `on origin` with `max_window_values` 1, then 2. Over a two-position `Document` trace with `c.value` 0, 2, monitor FR-330-AC-7's clause `always[0,1] holds(InUnit(c.value))` with `sources` holding the unit's source, then with `sources` empty, then with `sources` holding a `Source` of the unit's reference whose bytes are shorter than the condition's span.
7. Over TC-765 step 3's lasso, monitor `eventually[0,2] holds(q)` under event-position false-extension, alone and after a clause that evaluates; monitor a selection `Absent`; monitor TC-765 step 3's clause with an `over` naming key `ghost`, then with no `over`; monitor FR-330-AC-17's record-trace-subject clause `always[0,2] A` over TC-765 step 1's `Document` trace; monitor `Absent` and then the bounded clause in one request. Over FR-501-AC-7's trace (stamps 0, 1, 1, no order keys) as a `Typed` `Timestamped` trace with an agreeing `binding`, monitor `holds(p) until[0,1] holds(q)`. Then, with a test trace evaluator returning `TemporalRefusal::Fault` for one of two selected clauses, monitor both.

Tag the tests `#[trace("TC-766", "<AC id>")]`.

## Expected Results

- Step 1: `StageFailure::Refused(MonitorRefusal::Admission)` with the admission failure and position index 1, and no clause record.
- Step 2: `MonitorRefusal::TraceDocument` with `invalid_runtime_input`/`invalid-value` at `/loop`, with no clause record.
- Step 3: each pair of verdicts is equal.
- Step 4: the fairness clause settles unsupported, cause `unsupported_projection`/`missing-fairness-premise` naming `fair weak whole inc`; the clause without fairness reports `tested`; exit 21.
- Step 5: each document refuses with `MonitorRefusal::TraceDocument` and the code and pointer FR-283-AC-8 gives, no position is admitted, and no record is returned.
- Step 6: `Incomplete` naming `work_units`, exit 22; `WindowLimitReached` with `max_window_values`, value 1 and setting `monitor.max_window_values`, exit 22, then `Tested`; `ConditionRefused` at position 1 with `integer_out_of_domain`/`outside-domain`, its `expected` field and a `locus` whose `file` is the unit's authored path and whose `span` is the `holds` condition's, category refusal, exit 20; with `sources` empty, `MonitorRefusal::SourceNotSupplied` naming the clause and the unit's source identity, `invalid_runtime_input`/`missing-member` naming `sources`, no record, exit 20, for both the empty and the short `sources`.
- Step 7: `MonitorRefusal::ClauseRefused` naming the bounded clause with `invalid_runtime_input`/`invalid-value` and no record, in both requests; `MonitorRefusal::UnknownClause` naming `Absent`; `ClauseRefused` with `invalid_runtime_input`/`wrong-role-mapping` for `ghost` and for no `over`; `ClauseRefused` with `invalid_runtime_input`/`invalid-value` for the record-trace-subject clause; `UnknownClause` for the request selecting `Absent` first; a `TraceRefused` record with `invalid_runtime_input`/`invalid-value` at position 2, category refusal, exit 20; then `StageFailure::Fault`, no record for either clause, exit 30.
